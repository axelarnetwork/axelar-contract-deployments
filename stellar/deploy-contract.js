'use strict';

const { Command } = require('commander');
const { addBaseOptions } = require('./utils');
const { addOptionsToCommands } = require('../common');
const { getDeployContractCommands, getUpgradeContractCommands, getUploadContractCommands } = require('./commands');

require('./cli-utils');

function main() {
    const command = new Command('deploy-contract').description('Deploy/Upgrade Stellar contracts');

    const deployCommand = new Command('deploy').description('Deploy a Stellar contract');
    const upgradeCommand = new Command('upgrade').description('Upgrade a Stellar contract');
    const uploadCommand = new Command('upload').description('Upload a Stellar contract');

    const deployContractCommand = getDeployContractCommands();
    const upgradeContractCommands = getUpgradeContractCommands();
    const uploadContractCommands = getUploadContractCommands();

    deployContractCommand.forEach((command) => deployCommand.addCommand(command));
    upgradeContractCommands.forEach((command) => upgradeCommand.addCommand(command));
    uploadContractCommands.forEach((command) => uploadCommand.addCommand(command));

    addOptionsToCommands(deployCommand, addBaseOptions);
    addOptionsToCommands(upgradeCommand, addBaseOptions);

    command.addCommand(deployCommand);
    command.addCommand(upgradeCommand);

    command.parse();
}

if (require.main === module) {
    main();
}
