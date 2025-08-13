'use strict';

const { saveConfig, loadConfig, printInfo, printWarn, getChainConfig, kebabToPascal } = require('../common/utils');
const { getWallet, addBaseOptions } = require('./utils');
const { Command, Option } = require('commander');
const fs = require('fs');
const path = require('path');
const {
    makeContractDeploy,
    PostConditionMode,
    AnchorMode,
    ClarityVersion,
    broadcastTransaction,
    fetchCallReadOnlyFunction,
} = require('@stacks/transactions');

async function getContractSource(chain, options, networkType) {
    // If deploying a token manager, we need to get the code from the verify-onchain contract
    if (options.contract === 'token-manager') {
        if (!chain.contracts.VerifyOnchain?.address) {
            throw new Error(`Contract VerifyOnchain is not deployed`);
        }

        const verifyOnchainAddress = chain.contracts.VerifyOnchain.address.split('.');
        const source = await fetchCallReadOnlyFunction({
            contractAddress: verifyOnchainAddress[0],
            contractName: verifyOnchainAddress[1],
            functionName: 'get-token-manager-source',
            functionArgs: [],
            senderAddress: verifyOnchainAddress[0],
            network: networkType,
        });

        return source.value;
    }

    const contractBasePath = path.resolve(options.basePath);
    const filePath = path.join(contractBasePath, `${options.contract}.clar`);

    if (!fs.existsSync(filePath)) {
        throw new Error(`Warning: File not found: ${filePath}`);
    }

    return fs.readFileSync(filePath, 'utf8');
}

async function processCommand(config, chain, options) {
    const { privateKey, stacksAddress, networkType } = await getWallet(chain, options);

    const contractName = options.name || options.contract;
    const configName = options.configName || kebabToPascal(contractName);

    if (chain.contracts[configName]?.address) {
        throw new Error(`Contract ${contractName} already exists`);
    }

    printInfo('Deploying contracts using address', stacksAddress);

    const source = await getContractSource(chain, options, networkType);
    const deployTx = await makeContractDeploy({
        contractName,
        codeBody: source,
        senderKey: privateKey,
        network: networkType,
        postConditionMode: PostConditionMode.Allow,
        anchorMode: AnchorMode.Any,
        clarityVersion: ClarityVersion.Clarity3,
    });
    const result = await broadcastTransaction({
        transaction: deployTx,
        network: networkType,
    });

    chain.contracts[configName] = {
        address: `${stacksAddress}.${contractName}`,
        deployer: stacksAddress,
        ...(configName === 'AxelarGateway' ? { connectionType: 'amplifier' } : {}),
    };

    if (options.version) {
        chain.contracts[kebabToPascal(contractName)].version = options.version;
    }

    printInfo(`Finished deploying contract`, result.txid);
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(config, chain, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('deploy-contract')
        .description('Deploy a contract')
        .addOption(new Option('-c, --contract <contract>', 'The contract to deploy').makeOptionMandatory(true))
        .addOption(new Option('-n, --name <name>', 'The name of the contract'))
        .addOption(new Option('-cn, --configName <configName>', 'The config name of the contract'))
        .addOption(new Option('-v, --version <version>', 'The version of the contract'))
        .addOption(new Option('-bp, --basePath <basePath>', 'The base path from where to get the contracts').makeOptionMandatory(true))
        .action((options) => {
            mainProcessor(options, processCommand);
        });

    addBaseOptions(program);

    program.parse();
}
