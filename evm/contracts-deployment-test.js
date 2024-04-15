'use strict';

const { ethers } = require('hardhat');
const { execSync } = require('child_process');
const { readFileSync } = require('fs');
const {
    Wallet,
    providers: { JsonRpcProvider },
} = ethers;
const { Command, Option } = require('commander');

const { mainProcessor, saveConfig } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const path = require('path');

async function processCommand(config, chain, options) {
    const wallet = new Wallet(options.privateKey, new JsonRpcProvider(chain.rpc));

    if (
        !config.chains[options.chainNames].contracts.Multisig ||
        !config.chains[options.chainNames].contracts.Multisig.signers ||
        !config.chains[options.chainNames].contracts.Multisig.threshold
    ) {
        config.chains[options.chainNames].contracts.Multisig = { signers: [wallet.address], threshold: 1 };
        saveConfig(config, options.env);
    }

    if (
        !config.chains[options.chainNames].contracts.InterchainGovernance ||
        !config.chains[options.chainNames].contracts.InterchainGovernance.minimumTimeDelay
    ) {
        config.chains[options.chainNames].contracts.InterchainGovernance = { minimumTimeDelay: 300 };
        saveConfig(config, options.env);
    }

    if (
        !config.chains[options.chainNames].contracts.AxelarDepositService ||
        !config.chains[options.chainNames].contracts.AxelarDepositService.wrappedSymbol ||
        !config.chains[options.chainNames].contracts.AxelarDepositService.refundIssuer
    ) {
        config.chains[options.chainNames].contracts.AxelarDepositService = {
            wrappedSymbol: `W${chain.tokenSymbol}`,
            refundIssuer: wallet.address,
        };
        saveConfig(config, options.env);
    }

    const cmds = [
        `node evm/deploy-contract.js -c ConstAddressDeployer -m create --artifactPath ../evm/legacy/ConstAddressDeployer.json`,
        `node evm/deploy-contract.js -c Create3Deployer -m create2`,
        `node evm/deploy-gateway-v6.2.x.js -m create3 --keyID ${wallet.address} --mintLimiter ${wallet.address} --governance ${wallet.address}`,
        `node evm/gateway.js --action params`,
        `node evm/deploy-contract.js -c Operators -m create2`,
        `node evm/deploy-upgradable.js -c AxelarGasService -m create${options.env === 'testnet' ? '' : '2'} --args ${wallet.address}`,
        `node evm/deploy-contract.js -c Multisig -m create3 -s 'testSalt'`,
        `node evm/deploy-contract.js -c InterchainGovernance -m create3`,
        `node evm/deploy-its.js -s "testSalt" --proxySalt 'testSalt'`,
        `node evm/gateway.js --action transferMintLimiter`,
        `node evm/gateway.js --action transferGovernance`,
    ];

    if (options.deployDepositService) {
        cmds.push(
            `node evm/deploy-test-gateway-token.js`,
            `node evm/deploy-upgradable.js -c AxelarDepositService -m create --salt "testSalt"`,
        );
    }

    const _path = path.join(__dirname, '..', 'axelar-chains-config', 'info');
    const file = `${options.env}.json`;

    for (let i = 0; i < cmds.length; i++) {
        config = JSON.parse(readFileSync(path.join(_path, file)));
        execSync(`${cmds[i]} -n ${options.chainNames} -p ${options.privateKey} ${options.yes ? '-y' : ''} ${i === 5 ? `` : ''}`, {
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
    program.addOption(new Option('--deployDepositService', 'include AxelarDepositService in deployment tests').env('deployDepositService'));
    addBaseOptions(program);

    program.action((options) => {
        main(options);
    });

    program.parse();
}
