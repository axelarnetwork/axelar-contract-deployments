const xrpl = require('xrpl');
const { Command, Option } = require('commander');
const {
    mainProcessor,
    getWallet,
    generateWallet,
    getAccountInfo,
    getReserveRequirements,
    hex,
    sendPayment,
    sendSignerListSet,
    sendAccountSet,
    sendTicketCreate,
} = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { printInfo, printWarn, prompt } = require('../common');

const TRANSFER_RATE = 0; // Don't charge a fee for transferring currencies issued by the multisig
const TICK_SIZE = 6; // This determines truncation for order book entries, not payments
const DOMAIN = 'axelar.foundation';

const MAX_TICKET_COUNT = 250;
const MAX_SIGNERS = 32;

const INITIAL_QUORUM = 1;
const INITIAL_WEIGHT_PER_SIGNER = 1;

async function deployMultisig(_, chain, client, options) {
    const wallet = getWallet(options);
    const { balance } = await getAccountInfo(client, wallet.address);
    const { baseReserve, ownerReserve } = await getReserveRequirements(client);

    const multisigReserve = Math.ceil(baseReserve + (MAX_TICKET_COUNT + MAX_SIGNERS) * ownerReserve);

    if (balance < Number(multisigReserve)) {
        printWarn(`Wallet XRP balance (${balance} XRP) is less than required multisig account reserve (${multisigReserve} XRP)`);
        process.exit(0);
    }

    let multisig;

    if (options.generateWallet) {
        multisig = generateWallet();
        printInfo('Generated new multisig account', multisig);
        printInfo(`Funding multisig account with ${multisigReserve} XRP from wallet`);
        await sendPayment(client, wallet, {
            destination: multisig.address,
            amount: xrpl.xrpToDrops(multisigReserve),
        });
        printInfo('Funded multisig account');
    } else {
        if (prompt(`Proceed with turning ${wallet.address} into a multisig account?`, options.yes)) {
            return;
        }

        multisig = wallet;
    }

    printInfo('Adding initial multisig signer set', options.initialSigners);
    await sendSignerListSet(client, multisig, {
        quorum: INITIAL_QUORUM,
        signers: options.initialSigners.map((signer) => ({
            address: signer,
            weight: INITIAL_WEIGHT_PER_SIGNER,
        })),
    });

    printInfo(`Creating tickets`);
    await sendTicketCreate(client, multisig, {
        ticketCount: MAX_TICKET_COUNT,
    });

    const flags = xrpl.AccountSetAsfFlags.asfDisableMaster
        & xrpl.AccountSetAsfFlags.asfDisallowIncomingNFTokenOffer
        & xrpl.AccountSetAsfFlags.asfDisallowIncomingCheck
        & xrpl.AccountSetAsfFlags.asfDisallowIncomingPayChan;

    printInfo('Configuring account settings');
    await sendAccountSet(client, multisig, {
        transferRate: TRANSFER_RATE,
        tickSize: TICK_SIZE,
        domain: hex(DOMAIN),
        flags,
    });

    chain.contracts.AxelarGateway = {
        address: multisig.address,
        transferRate: TRANSFER_RATE,
        tickSize: TICK_SIZE,
        domain: DOMAIN,
        flags,
    };

    printInfo('Successfully created and configured XRPL multisig account', multisig.address);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('deploy-multisig')
        .description('Converts a wallet into an XRPL multisig account.')
        .addOption(new Option('--generate-wallet', 'generate a new wallet account to convert into an XRPL multisig account instead of using active wallet').default(false))
        .addOption(new Option('-y, --yes', 'skip prompt confirmation').env('YES'))
        .addOption(
            new Option(
                '--initial-signers <signers...>',
                'XRPL addresses of initial signers',
            ).makeOptionMandatory(true),
        );

    addBaseOptions(program);

    program.action((options) => {
        mainProcessor(options, deployMultisig);
    });

    program.parse();
}
