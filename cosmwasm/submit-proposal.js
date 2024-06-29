'use strict';

require('dotenv').config();

const { prepareWallet, prepareClient, submitStoreCodeProposal } = require('./utils');
const { loadConfig } = require('../evm/utils');

const { Command, Option } = require('commander');

const storeCode = (client, wallet, config, options) => {
    submitStoreCodeProposal(client, wallet, config, options);

    return Promise.resolve({ wallet, client });
};

const main = async (options) => {
    const { env } = options;
    const config = loadConfig(env);

    await prepareWallet(options)
        .then((wallet) => prepareClient(config, wallet))
        .then(({ wallet, client }) => storeCode(client, wallet, config, options));
};

const programHandler = () => {
    const program = new Command();

    program.name('submit-proposal').description('Submit governance proposals');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'devnet-amplifier', 'devnet-verifiers', 'stagenet', 'testnet', 'mainnet'])
            .default('devnet-amplifier')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').makeOptionMandatory(true).env('MNEMONIC'));
    program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').makeOptionMandatory(true).env('ARTIFACT_PATH'));
    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));

    program.addOption(new Option('--aarch64', 'aarch64').env('AARCH64').default(false));
    program.addOption(new Option('-y, --yes', 'skip prompt confirmation').env('YES'));

    program.addOption(new Option('-t, --title <title>', 'title of proposal').makeOptionMandatory(true));
    program.addOption(new Option('-d, --description <description>', 'description of proposal').makeOptionMandatory(true));
    program.addOption(new Option('-r, --runAs <runAs>', 'the address that will execute the message').makeOptionMandatory(true));

    program.addOption(new Option('--source <source>', "a valid HTTPS URI to the contract's source code"));
    program.addOption(
        new Option('--builder <builder>', 'a valid docker image name with tag, such as "cosmwasm/workspace-optimizer:0.16.0'),
    );

    program.action((options) => {
        main(options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
