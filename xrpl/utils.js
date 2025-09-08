'use strict';

const xrpl = require('xrpl');
const { decodeAccountID } = require('ripple-address-codec');
const { decode: decodeTxBlob } = require('ripple-binary-codec');
const chalk = require('chalk');
const { loadConfig, saveConfig, printInfo, printWarn, printError, prompt, getChainConfig } = require('../common');

function hex(str) {
    return Buffer.from(str).toString('hex');
}

function roundUpToNearestXRP(amountInDrops) {
    return Math.ceil(amountInDrops / 1e6) * 1e6;
}

function generateWallet(options) {
    return xrpl.Wallet.generate(options.walletKeyType);
}

function getWallet(options) {
    if (options.privateKeyType === 'hex' || (options.privateKey && /^[0-9A-Fa-f]{64,}$/.test(options.privateKey))) {
        const entropy = Buffer.from(options.privateKey, 'hex').slice(0, 16);
        return xrpl.Wallet.fromEntropy(entropy, {
            algorithm: options.walletKeyType,
        });
    }
    return xrpl.Wallet.fromSeed(options.privateKey, {
        algorithm: options.walletKeyType,
    });
}

function deriveAddress(publicKey) {
    return new xrpl.Wallet(publicKey).address;
}

function decodeAccountIDToHex(accountId) {
    return hex(decodeAccountID(accountId));
}

// XRPL token is either:
// (1) "XRP"
// (2) "<currency>.<issuer-address>"
function parseTokenAmount(token, amount) {
    let parsedAmount;

    if (token === 'XRP') {
        parsedAmount = xrpl.xrpToDrops(amount);
    } else {
        const [currency, issuer] = token.split('.');
        parsedAmount = {
            currency,
            issuer,
            value: amount,
        };
    }

    return parsedAmount;
}

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

async function printWalletInfo(client, wallet, chain) {
    const address = wallet.address;
    const { balance, sequence } = await client.accountInfo(address);
    printInfo('Wallet address', address);

    if (balance === '0') {
        printError('Wallet balance', '0');
    } else {
        printInfo('Wallet balance', `${xrpl.dropsToXrp(balance)} ${chain.tokenSymbol || ''}`);
    }

    if (sequence === '-1') {
        printWarn('Wallet is not active because it does not meet the base reserve requirement');
        return;
    }

    printInfo('Wallet sequence', sequence);

    const lines = await client.accountLines(address);

    if (lines.length === 0) {
        printInfo('Wallet IOU balances', 'No IOU balances found');
        return;
    }

    printInfo('Wallet IOU balances', lines.map((line) => `${line.balance} ${line.currency}.${line.account}`).join('  '));
}

async function mainProcessor(processor, options, args, save = true, catchErr = false) {
    if (!options.env) {
        throw new Error('Environment was not provided');
    }

    printInfo('Environment', options.env);

    const config = loadConfig(options.env);

    if (!options.chainName) {
        throw new Error('Chain name was not provided');
    }

    const chainName = options.chainName.toLowerCase();

    const chain = getChainConfig(config.chains, chainName);

    if (!chain) {
        throw new Error(`Chain ${chainName} is not defined in the info file`);
    }

    if (chain.chainType !== 'xrpl') {
        throw new Error(`Cannot run script for a non XRPL chain: ${chainName}`);
    }

    console.log('');
    printInfo('Chain', chain.name, chalk.cyan);

    const wallet = getWallet(options);

    const client = new XRPLClient(chain.wssRpc);
    await client.connect();

    try {
        await processor(config, wallet, client, chain, options, args);
    } catch (error) {
        printError(`Failed with error on ${chainName}`, error.message);

        if (!catchErr && !options.ignoreError) {
            throw error;
        }
    } finally {
        await client.disconnect();
    }

    if (save) {
        saveConfig(config, options.env);
    }
}

module.exports = {
    ...require('../common/utils'),
    XRPLClient,
    generateWallet,
    getWallet,
    printWalletInfo,
    mainProcessor,
    hex,
    roundUpToNearestXRP,
    deriveAddress,
    parseTokenAmount,
    decodeAccountIDToHex,
    decodeTxBlob,
};
