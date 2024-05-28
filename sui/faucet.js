'use strict';

const { addBaseOptions } = require('cli-utils');
const { requestSuiFromFaucetV0 } = require('@mysten/sui.js/faucet');
const { getWallet } = require('./utils');

async function processCommand(config, chain, options) {
    await requestSuiFromFaucetV0({
        host: options.faucetUrl,
        recipient: keypair.toSuiAddress(),
    });
}

if (require.main === module) {
    const program = new Command();

    program.name('publish-sui-gateway-v2').description('Publish sui gateway v2');

    addBaseOptions(program);

    program.addOption(
        new Option('--faucetUrl <faucetUrl>', 'url for a faucet to request funds from')
            .makeOptionMandatory(true),
    );

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
