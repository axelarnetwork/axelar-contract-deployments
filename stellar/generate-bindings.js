'use strict';

const { Command, Option } = require('commander');
const { execSync } = require('child_process');
const { loadConfig } = require('../evm/utils');
const path = require('path');
const { getNetworkPassphrase } = require('./utils');
require('./cli-utils');

function processCommand(options, _, chain) {
    const { wasmPath, contractId, outputDir } = options;
    const overwrite = true;

    const { rpc, networkType } = chain;
    const networkPassphrase = getNetworkPassphrase(networkType);

    const cmd = `soroban contract bindings typescript --wasm ${wasmPath} --rpc-url ${rpc} --network-passphrase "${networkPassphrase}" --contract-id ${contractId} --output-dir ${outputDir} ${
        overwrite ? '--overwrite' : ''
    }`;
    console.log(`Executing command: ${cmd}`);

    execSync(cmd, { stdio: 'inherit' });
    console.log('Bindings generated successfully!');
}

function main() {
    const program = new Command();
    program.name('Generate TypeScript Bindings for Soroban contract').description('Generates TypeScript bindings for a Soroban contract.');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('--wasmPath <wasmPath>', 'path to the WASM file').makeOptionMandatory(true));
    program.addOption(new Option('--contractId <contractId>', 'contract ID').makeOptionMandatory(true));
    program.addOption(
        new Option('--outputDir <outputDir>', 'output directory for the generated bindings').default(path.join(__dirname, 'bindings')),
    );

    program.action((options) => {
        const config = loadConfig(options.env);
        processCommand(options, config, config.stellar);
    });

    program.parse();
}

if (require.main === module) {
    main();
}
