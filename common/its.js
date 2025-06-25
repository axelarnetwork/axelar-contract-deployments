'use strict';

const { Command } = require('commander');
const { execSync } = require('child_process');
const { addBaseOptions, addOptionsToCommands, encodeITSDestination, loadConfig, printInfo } = require('../common');

async function encodeRecipient(config, args, _) {
    const [destinationChain, destinationAddress] = args;

    const itsDestinationAddress = encodeITSDestination(config, destinationChain, destinationAddress);

    printInfo('Human-readable destination address', destinationAddress);
    printInfo('Encoded ITS destination address', itsDestinationAddress);
}

async function setTrustedChainsAll(config, args, options) {
    const chain = process.env.CHAIN;
    if (!chain) {
        throw new Error('CHAIN environment variable must be set');
    }
    
    const requiredKeys = [
        'PRIVATE_KEY_EVM', 
        'PRIVATE_KEY_SUI', 
        'PRIVATE_KEY_STELLAR'
    ];
    for (const key of requiredKeys) {
        if (!process.env[key]) {
            throw new Error(`${key} must be set in .env file`);
        }
    }
    
    const commands = [
        { 
            cmd: `ts-node evm/its.js set-trusted-chains ${chain} hub -n all`,
            privateKeyEnv: 'PRIVATE_KEY_EVM'
        },
        { 
            cmd: `ts-node sui/its.js add-trusted-chains ${chain}`,
            privateKeyEnv: 'PRIVATE_KEY_SUI'
        },
        { 
            cmd: `ts-node stellar/its.js add-trusted-chains ${chain}`,
            privateKeyEnv: 'PRIVATE_KEY_STELLAR'
        }
    ];
    
    for (const { cmd, privateKeyEnv } of commands) {
        execSync(cmd, { 
            stdio: 'inherit',
            env: {
                ...process.env,
                PRIVATE_KEY: process.env[privateKeyEnv]
            }
        });
    }
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);

    await processor(config, args, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('its').description('Interchain Token Service common operations.');

    program
        .command('encode-recipient <destination-chain> <destination-address')
        .description('Encode ITS recipient based on destination chain in config')
        .action((destinationChain, destinationAddress, options) => {
            mainProcessor(encodeRecipient, [destinationChain, destinationAddress], options);
        });
    
    program
        .command('set-trusted-chains-all')
        .description('Set trusted chains for all chains')
        .action((options) => {
            mainProcessor(setTrustedChainsAll, [], options);
        });

    addOptionsToCommands(program, addBaseOptions, { ignoreChainNames: true });

    program.parse();
}
