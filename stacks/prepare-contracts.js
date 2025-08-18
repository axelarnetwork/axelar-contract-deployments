'use strict';

const { saveConfig, loadConfig, printInfo, printWarn, getChainConfig } = require('../common/utils');
const { getWallet, addBaseOptions } = require('./utils');
const { Command, Option } = require('commander');
const fs = require('fs');
const path = require('path');

const CONTRACTS_TO_STACKS_ADDRESS_WITH_TIMES = {
    'native-interchain-token.clar': 3,
    'token-manager.clar': 3,
    'verify-onchain.clar': 6,
    'hello-world.clar': 9,
};

function processClarityStacksContract(contractBasePath, config, chain) {
    const filename = 'clarity-stacks.clar';

    const filePath = path.join(contractBasePath, filename);

    if (!fs.existsSync(filePath)) {
        throw new Error(`Warning: File not found: ${filePath}`);
    }

    printInfo(`Reading file: ${filePath}`);
    const originalContent = fs.readFileSync(filePath, 'utf8');

    // Make sure debug mode is set to false
    const debugModeText = '\\(define-constant debug-mode \\(not is-in-mainnet\\)\\)';

    if ([...originalContent.matchAll(new RegExp(debugModeText, 'g'))].length !== 1) {
        throw new Error(`Error finding correct placeholders in contract ${filename}. Re-download the contracts and try again`);
    }

    const newContent = originalContent.replaceAll(new RegExp(debugModeText, 'g'), '(define-constant debug-mode false)');

    printInfo(`Replacing placeholder in and saving file: ${filePath}`);
    fs.writeFileSync(filePath, newContent, 'utf8');
}

async function processCommand(config, chain, options) {
    const { stacksAddress } = await getWallet(chain, options);

    printInfo('Preparing contracts using address', stacksAddress);

    const contractBasePath = path.resolve(options.basePath);
    const placeholderAddress = options.placeholderAddress;

    for (const filename in CONTRACTS_TO_STACKS_ADDRESS_WITH_TIMES) {
        const times = CONTRACTS_TO_STACKS_ADDRESS_WITH_TIMES[filename];

        const filePath = path.join(contractBasePath, filename);

        if (!fs.existsSync(filePath)) {
            throw new Error(`Warning: File not found: ${filePath}`);
        }

        printInfo(`Reading file: ${filePath}`);
        const originalContent = fs.readFileSync(filePath, 'utf8');

        if ([...originalContent.matchAll(new RegExp(placeholderAddress, 'g'))].length !== times) {
            throw new Error(`Error finding correct placeholders in contract ${filename}. Re-download the contracts and try again`);
        }

        const newContent = originalContent.replaceAll(placeholderAddress, stacksAddress);

        printInfo(`Replacing placeholder in and saving file: ${filePath}`);
        fs.writeFileSync(filePath, newContent, 'utf8');
    }

    processClarityStacksContract(contractBasePath, config, chain);

    printInfo(`Finished preparing contracts`);
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
        .name('prepare-contracts')
        .description('Prepare the contracts')
        .addOption(new Option('-bp, --basePath <basePath>', 'The base path from where to get the contracts').makeOptionMandatory(true))
        .addOption(
            new Option('-pt, --placeholderAddress <placeholderAddress>', 'The placeholder address to replace').makeOptionMandatory(true),
        )
        .action((options) => {
            mainProcessor(options, processCommand);
        });

    addBaseOptions(program);

    program.parse();
}
