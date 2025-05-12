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
} = require('@stacks/transactions');

async function processCommand(config, chain, options) {
    const { privateKey, stacksAddress, networkType } = await getWallet(chain, options);

    if (chain.contracts[kebabToPascal(options.contract)]?.address) {
        throw new Error(`Contract ${options.contract} already exists`);
    }

    printInfo('Deploying contracts using address', stacksAddress);

    const contractBasePath = path.resolve('./stacks/contracts');
    const filePath = path.join(contractBasePath, `${options.contract}.clar`);

    if (!fs.existsSync(filePath)) {
        throw new Error(`Warning: File not found: ${filePath}`);
    }

    const source = fs.readFileSync(filePath, 'utf8');
    const deployTx = await makeContractDeploy({
        contractName: options.contract,
        codeBody: source,
        senderKey: privateKey,
        network: networkType,
        postConditionMode: PostConditionMode.Allow,
        anchorMode: AnchorMode.Any,
        clarityVersion: ClarityVersion.Clarity3,
        fee: 1_000_000,
    });
    const result = await broadcastTransaction({
        transaction: deployTx,
        network: networkType,
    });

    chain.contracts[kebabToPascal(options.contract)] = {
        address: `${stacksAddress}.${options.contract}`,
    };

    printInfo(`Finished deploying contract`, result.txid);
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(config, chain, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('deploy-contract')
        .description('Deploy a contract')
        .addOption(new Option('-c, --contract <contract>', 'The contract to deploy'))
        .action((options) => {
            mainProcessor(options, processCommand);
        });

    addBaseOptions(program);

    program.parse();
}
