'use strict';

const { saveConfig, loadConfig, printInfo, printWarn, getChainConfig } = require('../common/utils');
const { getWallet, addBaseOptions } = require('./utils');
const { Command, Option } = require('commander');
const fs = require('fs');
const path = require('path');

const PLACEHOLDER_TEXT = 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM';
const CONTRACTS_TO_PROCESS_WITH_TIMES = {
    'native-interchain-token.clar': 3,
    'token-manager.clar': 3,
    'verify-onchain.clar': 6,
};

async function processCommand(config, chain, options) {
    const { stacksAddress } = await getWallet(chain, options);

    printInfo('Preparing contracts using address', stacksAddress);

    const contractBasePath = path.resolve('./stacks/contracts');

    for (const filename in CONTRACTS_TO_PROCESS_WITH_TIMES) {
        const times = CONTRACTS_TO_PROCESS_WITH_TIMES[filename];

        const filePath = path.join(contractBasePath, filename);

        if (!fs.existsSync(filePath)) {
            throw new Error(`Warning: File not found: ${filePath}`);
        }

        printInfo(`Reading file: ${filePath}`);
        const originalContent = fs.readFileSync(filePath, 'utf8');

        if ([...originalContent.matchAll(new RegExp(PLACEHOLDER_TEXT, 'g'))].length !== times) {
            throw new Error(`Error finding correct placeholders in contract ${filename}. Re-download the contracts and try again`);
        }

        const newContent = originalContent.replaceAll(PLACEHOLDER_TEXT, stacksAddress);

        printInfo(`Replacing placeholder in and saving file: ${filePath}`);
        fs.writeFileSync(filePath, newContent, 'utf8');
    }

    printInfo(`Finished preparing contracts`);
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
        .name('prepare-contracts')
        .description('Prepare the contracts')
        .action((options) => {
            mainProcessor(options, processCommand);
        });

    addBaseOptions(program);

    program.parse();
}
