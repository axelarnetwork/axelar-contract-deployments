'use strict';

const { Command } = require('commander');
const { addBaseOptions } = require('./utils');
const { addOptionsToCommands } = require('../common');
const { getDeployContractCommands, getUpgradeContractCommands, getUploadContractCommands } = require('./deploy-contract/commands');

require('./cli-utils');

function main() {
    const command = new Command('deploy-contract').description('Deploy/Upgrade Stellar contracts');

    const deployCommand = new Command('deploy').description('Deploy a Stellar contract');
    const upgradeCommand = new Command('upgrade').description('Upgrade a Stellar contract').addHelpText(
        'after',
        `
Examples:
# using Vec<Address> as migration data:
$ deploy-contract upgrade <contractName> deploy --artifact-dir <artifactDirectoryPath> --version 2.1.7 --migration-data '["GDYBNA2LAWDKRSCIR4TKCB5LJCDRVUWKHLMSKUWMJ3YX3BD6DWTNT5FW"]'

# default void migration data:
$ deploy-contract upgrade <contractName> deploy --artifact-dir <artifactDirectoryPath> --version 1.0.1

# equivalent explicit void migration data:
$ deploy-contract upgrade <contractName> deploy --artifact-dir <artifactDirectoryPath> --version 1.0.1 --migration-data '()'

# artifactDirectoryPath example: ../axelar-amplifier-stellar/target/wasm32-unknown-unknown/release/
`,
    );
    const uploadCommand = new Command('upload').description('Upload a Stellar contract');

    const deployContractCommand = getDeployContractCommands();
    const upgradeContractCommands = getUpgradeContractCommands();
    const uploadContractCommands = getUploadContractCommands();

    deployContractCommand.forEach((command) => deployCommand.addCommand(command));
    upgradeContractCommands.forEach((command) => upgradeCommand.addCommand(command));
    uploadContractCommands.forEach((command) => uploadCommand.addCommand(command));

    addOptionsToCommands(deployCommand, addBaseOptions);
    addOptionsToCommands(upgradeCommand, addBaseOptions);
    addOptionsToCommands(uploadCommand, addBaseOptions);

    command.addCommand(deployCommand);
    command.addCommand(upgradeCommand);
    command.addCommand(uploadCommand);

    command.parse();
}

if (require.main === module) {
    main();
}
