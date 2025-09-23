'use strict';

require('../common/cli-utils');

const { getAmplifierContractConfig, fetchCodeIdFromContract } = require('./utils');
const { mainProcessor } = require('./processor');
const { printInfo } = require('../common');
const { Command } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const processCommand = async (client, _wallet, config, options) => {
    const { contractConfig } = getAmplifierContractConfig(config, options);

    printInfo('Old code id', contractConfig.codeId);

    contractConfig.codeId = await fetchCodeIdFromContract(client, contractConfig);

    printInfo('New code id', contractConfig.codeId);
};

const programHandler = () => {
    const program = new Command();

    program.name('update-code-id').description('Update configured code id of contract with the one currently being used on-chain');

    addAmplifierOptions(program, {
        contractOptions: true,
    });

    program.action((options) => {
        mainProcessor(processCommand, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
