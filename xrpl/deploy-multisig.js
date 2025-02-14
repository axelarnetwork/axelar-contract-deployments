const xrpl = require('xrpl');
const { Command, Option } = require('commander');
const { mainProcessor, getWallet, getAccountInfo, sendTransaction, hex } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { printInfo, printWarn } = require('../common');

const MAX_TICKET_COUNT = 250;
const KEY_TYPE = xrpl.ECDSA.secp256k1;

const BASE_RESERVE_XRP = 10;
const OWNER_RESERVE_XRP = 0.2;

async function processCommand(_, chain, options) {
    const client = new xrpl.Client(chain.rpc);
    await client.connect();

    const wallet = await getWallet(chain, options);
    const balance = Number((await getAccountInfo(client, wallet.address)).Balance) / 1e6;
    const multisigReserve = BASE_RESERVE_XRP + MAX_TICKET_COUNT * OWNER_RESERVE_XRP;
    if (balance < Number(multisigReserve)) {
        printWarn(`Wallet XRP balance is insufficient to fund the multisig account reserve (${multisigReserve} XRP)`);
        process.exit(0);
    }

    printInfo('Creating multisig account');
    const multisig = xrpl.Wallet.generate(KEY_TYPE);
    printInfo('Created multisig account', multisig.address);

    const paymentTx = {
        TransactionType: "Payment",
        Account: wallet.address,
        Destination: multisig.address,
        Amount: xrpl.xrpToDrops(multisigReserve),
    };

    printInfo(`Funding multisig account with ${multisigReserve} XRP from wallet`, JSON.stringify(paymentTx, null, 2));
    await sendTransaction(client, wallet, paymentTx);
    printInfo('Funded multisig account');

    const signerListSetTx = {
        TransactionType: "SignerListSet",
        Account: multisig.address,
        SignerQuorum: 1,
        SignerEntries: options.initialSigners.map((signer) => ({
            SignerEntry: {
                Account: signer,
                SignerWeight: 1,
            },
        })),
    };

    printInfo('Adding initial multisig signer set', JSON.stringify(signerListSetTx, null, 2));
    await sendTransaction(client, multisig, signerListSetTx);
    printInfo('Added initial multisig signer set', options.initialSigners);

    const ticketCreateTx = {
        TransactionType: "TicketCreate",
        Account: multisig.address,
        TicketCount: MAX_TICKET_COUNT,
    };

    printInfo('Creating tickets', JSON.stringify(ticketCreateTx, null, 2));
    await sendTransaction(client, multisig, ticketCreateTx);
    printInfo('Tickets created');

    const accountSetTx = {
        TransactionType: "AccountSet",
        Account: multisig.address,
        TransferRate: 0,
        TickSize: 6,
        Domain: hex("axelar.network"),
        SetFlag: xrpl.AccountSetAsfFlags.asfDisableMaster
            & xrpl.AccountSetAsfFlags.asfDisallowIncomingNFTokenOffer
            & xrpl.AccountSetAsfFlags.asfDisallowIncomingCheck
            & xrpl.AccountSetAsfFlags.asfDisallowIncomingPayChan,
    };

    printInfo('Disabling master key pair', JSON.stringify(accountSetTx, null, 2));
    await sendTransaction(client, multisig, accountSetTx);
    printInfo('Master key pair disabled');

    chain.multisigAddress = multisig.address;

    printInfo('Created and configured XRPL multisig account successfully', multisig.address);
    await client.disconnect();
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('deploy-multisig')
        .description('Create & configure XRPL multisig account.')
        .addOption(
            new Option(
                '--initialSigners <signers...>',
                'XRPL addresses of initial signers',
            ).makeOptionMandatory(true),
        );

    addBaseOptions(program);

    program.action((options) => {
        main(options);
    });

    program.parse();
}
