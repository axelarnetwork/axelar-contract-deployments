'use strict';

require('dotenv').config();

const { printInfo, loadConfig, saveConfig } = require('../common');
const { prepareWallet, prepareClient, initContractConfig, getAmplifierContractConfig, fetchCodeIdFromContract } = require('./utils');

const { Command } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const processCommand = async (options) => {
    const { env } = options;
    const config = loadConfig(env);

    initContractConfig(config, options);

    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);

    const { contractConfig } = getAmplifierContractConfig(config, options);

    printInfo('Old code id', contractConfig.codeId);

    contractConfig.codeId = await fetchCodeIdFromContract(client, contractConfig);

    printInfo('New code id', contractConfig.codeId);

    saveConfig(config, env);
};

const programHandler = () => {
    const program = new Command();

    program.name('update-code-id').description('Update configured code id of contract with the one currently being used on-chain');

    addAmplifierOptions(program, {
        contractOptions: true,
    });

    program.action((options) => {
        processCommand(options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
