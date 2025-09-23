'use strict';

require('../common/cli-utils');

const { printInfo, getAmplifierContractConfig, fetchCodeIdFromContract } = require('./utils');
const { mainQueryProcessor } = require('./processor');
const { Command } = require('commander');
const { addAmplifierOptions } = require('./cli-utils');

const processCommand = async (client, config, options, _args, _fee) => {
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
        mainQueryProcessor(processCommand, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
