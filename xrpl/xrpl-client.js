const xrpl = require('xrpl');
const { printInfo, printWarn, printError, prompt } = require('../common');

class XRPLClient {
    constructor(rpcUrl) {
        this.client = new xrpl.Client(rpcUrl);
    }

    async connect() {
        await this.client.connect();
    }

    async disconnect() {
        await this.client.disconnect();
    }

    async request(command, params = {}) {
        const response = await this.client.request({ command, ...params });
        return response.result;
    }

    async autofill(tx) {
        return this.client.autofill(tx);
    }

    async accountInfo(account, ledgerIndex = 'validated') {
        try {
            const accountInfoRes = await this.request('account_info', {
                account,
                ledger_index: ledgerIndex,
            });

            const accountInfo = accountInfoRes.account_data;
            return {
                balance: accountInfo.Balance,
                sequence: accountInfo.Sequence,
            };
        } catch (error) {
            if (error.data?.error === 'actNotFound') {
                return {
                    balance: '0',
                    sequence: '-1',
                };
            }

            throw error;
        }
    }

    async accountObjects(account, params = {}, limit = 1000, ledgerIndex = 'validated') {
        const accountObjectsRes = await this.request('account_objects', {
            account,
            ledger_index: ledgerIndex,
            limit,
            ...params,
        });

        return accountObjectsRes.account_objects;
    }

    async tickets(account, limit = 1000, ledgerIndex = 'validated') {
        const ticketRes = await this.accountObjects(account, { type: 'ticket' }, limit, ledgerIndex);
        return ticketRes.map((ticket) => ticket.TicketSequence);
    }

    async accountLines(account) {
        const accountLinesRes = await this.request('account_lines', {
            account,
            ledger_index: 'validated',
        });

        return accountLinesRes.lines;
    }

    async reserveRequirements() {
        const serverInfoRes = await this.request('server_info');
        const validatedLedger = serverInfoRes.info.validated_ledger;
        return {
            baseReserve: validatedLedger.reserve_base_xrp,
            ownerReserve: validatedLedger.reserve_inc_xrp,
        };
    }

    async fee(feeType = 'open_ledger_fee') {
        const feeRes = await this.request('fee');
        return feeRes.drops[feeType];
    }

    async fundWallet(wallet, amount) {
        return this.client.fundWallet(wallet, { amount });
    }

    handleReceipt(receipt) {
        const result = receipt.engine_result;

        if (result !== 'tesSUCCESS') {
            printError('Transaction failed', `${receipt.engine_result}: ${receipt.engine_result_message}`);
            process.exit(1);
        }

        printInfo(`Transaction sent`, receipt.tx_json.hash);
    }

    async submitTx(txBlob, failHard = true) {
        const result = await this.request('submit', {
            tx_blob: txBlob,
            fail_hard: failHard,
        });
        this.handleReceipt(result);
        return result;
    }

    async buildTx(txType, fields = {}, args = {}) {
        const tx = {
            TransactionType: txType,
            ...fields,
        };

        if (args.account) {
            tx.Account = args.account;
        }

        if (args.fee) {
            tx.Fee = args.fee;
        }

        return this.autofill(tx);
    }

    async signTx(signer, tx, multisign = false) {
        return signer.sign(tx, multisign);
    }

    async signAndSubmitTx(signer, txType, fields = {}, args = {}, options = { multisign: false, yes: false }) {
        const tx = await this.buildTx(txType, fields, {
            ...args,
            account: args.account ?? signer.classicAddress,
            // when multisigning, fee = (N + 1) * normal fee, where N is the number of signatures
            fee: args.fee ?? (options.multisign ? String(Number(await this.fee()) * 2) : undefined),
        });

        printInfo(`${options.multisign ? 'Multi-' : ''}Signing transaction`, JSON.stringify(tx, null, 2));
        const signedTx = await this.signTx(signer, tx, options.multisign);

        if (prompt(`Submit ${txType} transaction?`, options.yes)) {
            printWarn('Transaction cancelled by user.');
            process.exit(0);
        }

        return this.submitTx(signedTx.tx_blob);
    }

    checkRequiredField(field, fieldName) {
        if (!field) {
            throw new Error(`Missing required field: ${fieldName}`);
        }
    }

    async sendPayment(signer, { destination, amount, memos = [], ...restArgs }, options = { multisign: false, yes: false }) {
        this.checkRequiredField(destination, 'destination');
        this.checkRequiredField(amount, 'amount');
        return this.signAndSubmitTx(
            signer,
            'Payment',
            {
                Destination: destination,
                Amount: amount,
                Memos:
                    memos.length > 0
                        ? memos.map((memo) => ({
                              Memo: {
                                  MemoType: memo.memoType,
                                  MemoData: memo.memoData,
                              },
                          }))
                        : undefined,
            },
            restArgs,
            options,
        );
    }

    async sendSignerListSet(signer, { quorum, signers, ...restArgs }, options = { multisign: false, yes: false }) {
        this.checkRequiredField(quorum, 'quorum');
        this.checkRequiredField(signers, 'signers');

        if (signers.length === 0) {
            throw new Error('Signers list cannot be empty');
        }

        return this.signAndSubmitTx(
            signer,
            'SignerListSet',
            {
                SignerQuorum: quorum,
                SignerEntries: signers.map((signer) => ({
                    SignerEntry: {
                        Account: signer.address,
                        SignerWeight: signer.weight,
                    },
                })),
            },
            restArgs,
            options,
        );
    }

    async sendTicketCreate(signer, { ticketCount, ...restArgs }, options = { multisign: false, yes: false }) {
        this.checkRequiredField(ticketCount, 'ticketCount');
        return this.signAndSubmitTx(signer, 'TicketCreate', { TicketCount: ticketCount }, restArgs, options);
    }

    async sendAccountSet(signer, { transferRate, tickSize, domain, flag, ...restArgs }, options = { multisign: false, yes: false }) {
        return this.signAndSubmitTx(
            signer,
            'AccountSet',
            {
                TransferRate: transferRate,
                TickSize: tickSize,
                Domain: domain,
                SetFlag: flag,
            },
            restArgs,
            options,
        );
    }

    async sendTrustSet(signer, { currency, issuer, value, ...restArgs }, options = { multisign: false, yes: false }) {
        this.checkRequiredField(currency, 'currency');
        this.checkRequiredField(issuer, 'issuer');
        this.checkRequiredField(value, 'value');
        return this.signAndSubmitTx(
            signer,
            'TrustSet',
            {
                LimitAmount: {
                    currency,
                    issuer,
                    value,
                },
            },
            restArgs,
            options,
        );
    }
}

module.exports = XRPLClient;
