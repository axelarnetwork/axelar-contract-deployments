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

async function processCommand(config, chain, options) {
    const wallet = new Wallet(options.privateKey, new JsonRpcProvider(chain.rpc));
    const deploymentMethod = options.env === 'testnet' ? 'create' : 'create2';
    const collector = wallet.address;
    const signers = [wallet.address];
    const threshold = 1;
    const minimumTimeDelay = 300;
    const symbol = `W${chain.tokenSymbol}`;
    const refundIssuer = wallet.address;
    const argsAxelarGasService = JSON.stringify({ collector });
    const argsMultisig = JSON.stringify({ signers, threshold });
    const argsInterchainGovernance = JSON.stringify({ minTimeDelay: minimumTimeDelay });
    const argsAxelarDepositService = JSON.stringify({ symbol, refundIssuer });

    const cmds = [
        `node evm/deploy-contract.js -c ConstAddressDeployer -m create --artifactPath ../evm/legacy/ConstAddressDeployer.json`,
        `node evm/deploy-contract.js -c Create3Deployer -m create2`,
        `node evm/deploy-gateway-v6.2.x.js -m create3 --keyID ${wallet.address} --mintLimiter ${wallet.address} --governance ${wallet.address}`,
        `node evm/gateway.js --action params`,
        `node evm/deploy-contract.js -c Operators -m create2`,
        `node evm/deploy-upgradable.js -c AxelarGasService -m ${deploymentMethod} --args '${argsAxelarGasService}'`,
        `node evm/deploy-contract.js -c Multisig -m create3 -s 'testSalt' --args '${argsMultisig}'`,
        `node evm/deploy-contract.js -c InterchainGovernance -m create3 --args '${argsInterchainGovernance}'`,
        `node evm/deploy-its.js -s "testSalt" --proxySalt 'testSalt'`,
        `node evm/gateway.js --action transferMintLimiter`,
        `node evm/gateway.js --action transferGovernance`,
    ];

    if (options.deployDepositService) {
        cmds.push(
            `node evm/deploy-test-gateway-token.js`,
            `node evm/deploy-upgradable.js -c AxelarDepositService -m create --salt 'testSalt' --args '${argsAxelarDepositService}'`,
        );
    }

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
    program.addOption(
        new Option('--deployDepositService', 'include AxelarDepositService in deployment tests')
            .makeOptionMandatory(true)
            .env('DEPLOY_DEPOSIT_SERVICE'),
    );
    addBaseOptions(program);

    program.action((options) => {
        main(options);
    });

    program.parse();
}
