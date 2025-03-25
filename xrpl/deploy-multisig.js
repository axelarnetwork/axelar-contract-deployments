const xrpl = require('xrpl');
const { Command, Option } = require('commander');
const { mainProcessor, generateWallet, hex } = require('./utils');
const { addBaseOptions, addSkipPromptOption } = require('./cli-utils');
const { printInfo, printWarn, prompt } = require('../common');

const DISABLE_MASTER_FLAG = xrpl.AccountSetAsfFlags.asfDisableMaster;

const DEFAULTS = {
    TRANSFER_RATE: 0, // Don't charge a fee for transferring currencies issued by the multisig
    TICK_SIZE: 6,     // Determines truncation for order book entries, not payments
    DOMAIN: 'axelar.foundation',
    FLAGS: [
        xrpl.AccountSetAsfFlags.asfDisallowIncomingNFTokenOffer,
        xrpl.AccountSetAsfFlags.asfDisallowIncomingCheck,
        xrpl.AccountSetAsfFlags.asfDisallowIncomingPayChan,
        xrpl.AccountSetAsfFlags.asfDefaultRipple,
        xrpl.AccountSetAsfFlags.asfNoFreeze,
    ],
};

const MAX_TICKET_COUNT = 250;

const INITIAL_QUORUM = 1;
const INITIAL_SIGNER_WEIGHT = 1;

async function deployMultisig(_config, wallet, client, chain, options) {
    const { balance } = await client.accountInfo(wallet.address);
    const { baseReserve, ownerReserve } = await client.reserveRequirements();
    const multisigReserve = Math.ceil(baseReserve + (MAX_TICKET_COUNT + 1) * ownerReserve);

    if (balance < Number(multisigReserve)) {
        printWarn(`Wallet XRP balance (${balance} XRP) is less than required multisig account reserve (${multisigReserve} XRP)`);
        process.exit(0);
    }

    let multisig;

    if (options.generateWallet) {
        multisig = generateWallet(options);
        printInfo('Generated new multisig account', multisig);
        printInfo(`Funding multisig account with ${multisigReserve} XRP from wallet`);
        await client.sendPayment(wallet, { destination: multisig.address, amount: xrpl.xrpToDrops(multisigReserve) });
        printInfo('Funded multisig account');
    } else {
        if (prompt(`Proceed with turning ${wallet.address} into a multisig account?`, options.yes)) {
            return;
        }

        multisig = wallet;
    }

    printInfo('Setting initial multisig signer', options.initialSigner);
    await client.sendSignerListSet(multisig, {
        quorum: INITIAL_QUORUM,
        signers: [{ address: options.initialSigner, weight: INITIAL_SIGNER_WEIGHT }],
    }, options);

    printInfo('Creating tickets');
    await client.sendTicketCreate(multisig, { ticketCount: MAX_TICKET_COUNT }, options);

    for (const flag of options.flags) {
        printInfo(`Setting flag ${flag}`);
        await client.sendAccountSet(multisig, { flag }, options);
    }

    printInfo('Configuring remaining account settings');
    await client.sendAccountSet(multisig, {
        transferRate: options.transferRate,
        tickSize: options.tickSize,
        domain: hex(options.domain),
        flag: DISABLE_MASTER_FLAG,
    }, options);

    chain.contracts.AxelarGateway = chain.contracts.InterchainTokenService = {
        address: multisig.address,
        initialSigner: options.initialSigner,
        transferRate: options.transferRate,
        tickSize: options.tickSize,
        domain: options.domain,
        flags: [DISABLE_MASTER_FLAG, ...options.flags],
    };

    printInfo('Successfully created and configured XRPL multisig account', multisig.address);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('deploy-multisig')
        .description('Converts a wallet into an XRPL multisig account.')
        .addOption(new Option('--generateWallet', 'convert a new wallet (instead of the active wallet) into an XRPL multisig account').default(false))
        .addOption(new Option('--transferRate <transferRate>', 'account transfer rate').default(DEFAULTS.TRANSFER_RATE))
        .addOption(new Option('--tickSize <tickSize>', 'account tick size').default(DEFAULTS.TICK_SIZE))
        .addOption(new Option('--domain <domain>', 'account domain').default(DEFAULTS.DOMAIN))
        .addOption(new Option('--flags <flags...>', 'extra account flags (beyond asfDisableMaster)').default(DEFAULTS.FLAGS))
        .addOption(
            new Option(
                '--initialSigner <signer>',
                'XRPL address of the multisig\'s initial signer',
            ).makeOptionMandatory(true),
        );

    addBaseOptions(program);
    addSkipPromptOption(program);

    program.action((options) => {
        mainProcessor(deployMultisig, options);
    });

    program.parse();
}
