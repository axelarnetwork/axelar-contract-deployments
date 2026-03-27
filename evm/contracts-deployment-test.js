'use strict';

const { ethers } = require('hardhat');
const { execSync } = require('child_process');
const {
    Wallet,
    providers: { JsonRpcProvider },
} = ethers;
const { Command, Option } = require('commander');

const { mainProcessor } = require('./utils');
const { addBaseOptions } = require('./cli-utils');

async function processCommand(_axelar, chain, _chains, options) {
    const wallet = new Wallet(options.privateKey, new JsonRpcProvider(chain.rpc));
    const deploymentMethod = options.env === 'testnet' ? 'create' : 'create2';
    const collector = wallet.address;
    const signers = [wallet.address];
    const threshold = 1;
    const minimumTimeDelay = 300;
    const argsAxelarGasService = JSON.stringify({ collector });
    const argsMultisig = JSON.stringify({ signers, threshold });
    const argsInterchainGovernance = JSON.stringify({ minimumTimeDelay });

    const cmds = [
        `ts-node evm/deploy-contract.js -c ConstAddressDeployer -m create --artifactPath ../evm/legacy/ConstAddressDeployer.json`,
        `ts-node evm/deploy-contract.js -c Create3Deployer -m create2`,
        `ts-node evm/deploy-gateway-v6.2.x.js -m create3 --keyID ${wallet.address} --mintLimiter ${wallet.address} --governance ${wallet.address}`,
        `ts-node evm/gateway.js --action params`,
        `ts-node evm/deploy-contract.js -c Operators -m create2`,
        `ts-node evm/deploy-upgradable.js -c AxelarGasService -m ${deploymentMethod} --args '${argsAxelarGasService}'`,
        `ts-node evm/deploy-contract.js -c Multisig -m create3 -s 'testSalt' --args '${argsMultisig}'`,
        `ts-node evm/deploy-contract.js -c InterchainGovernance -m create3 --args '${argsInterchainGovernance}'`,
        `ts-node evm/deploy-its.js -s "testSalt" --proxySalt 'testSalt'`,
        `ts-node evm/gateway.js --action transferMintLimiter`,
        `ts-node evm/gateway.js --action transferGovernance`,
    ];

    for (let i = 0; i < cmds.length; i++) {
        execSync(`${cmds[i]} -n ${options.chainNames} -p ${options.privateKey} ${options.yes ? '-y' : ''}`, {
            stdio: 'inherit',
        });
    }
}

async function main(options) {
    await mainProcessor(options, processCommand, false);
}

if (require.main === module) {
    const program = new Command();

    program.name('contracts-deployment-test').description('Deploy contracts to test deployment on chain');
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    addBaseOptions(program);

    program.action((options) => {
        main(options);
    });

    program.parse();
}
