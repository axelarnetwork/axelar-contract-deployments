// test-sdk-version.js
'use strict';

const { Command } = require('commander');
const { loadConfig } = require('./common/utils');
const { isPreV50SDK } = require('./cosmwasm/utils');

const program = new Command();

program
    .name('test-sdk-version')
    .description('Test SDK version detection')
    .option('-e, --env <env>', 'environment')
    .action(async (options) => {
        if (!options.env) {
            console.error('Error: environment is required. Use -e flag');
            process.exit(1);
        }

        try {
            const config = loadConfig(options.env);
            console.log(`Testing environment: ${options.env}`);
            console.log(`LCD endpoint: ${config.axelar.lcd}`);
            
            // Call the new consolidated function
            const isLegacy = await isPreV50SDK(config);
            console.log(`Is pre-v0.50 (legacy): ${isLegacy}`);
            console.log(`Uses ${isLegacy ? 'legacy' : 'modern'} proposal format`);
            
        } catch (error) {
            console.error(`Failed for ${options.env}: ${error.message}`);
        }
    });

program.parse();