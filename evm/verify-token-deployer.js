'use strict';

const axios = require('axios');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    utils: { defaultAbiCoder, isHexString },
} = ethers;

const { loadConfig, printError } = require('./utils');

async function processCommand(chain, options) {
    try {
        const { signedHash, env, address, message } = options;
        const { txHash } = options;

        if (!(txHash && txHash.startsWith('0x') && isHexString(txHash) && txHash.length === 66)) {
            throw new Error('Invalid transaction hash');
        }

        const apiUrl = env === 'testnet' ? 'https://testnet.api.gmp.axelarscan.io/' : 'https://api.gmp.axelarscan.io/';
        const requestBody = {
            txHash,
            method: 'searchGMP',
        };
        let sender, tokenAddress;

        const response = await axios.post(apiUrl, requestBody);

        if (!(response.data && response.data.data && Array.isArray(response.data.data))) {
            throw new Error('No data found in the response.');
        }

        const data = response.data.data;

        for (const item of data) {
            if (item.call?.chain === chain.name.toLowerCase()) {
                sender = item.call.receipt.from;
                break;
            }
        }

        for (const item of data) {
            let logsData = item.call?.receipt?.logs;

            for (const log of logsData) {
                if (log.topics[0] === '0xf0d7beb2b03d35e597f432391dc2a6f6eb1a621be6cb5b325f55a49090085239') {
                    [tokenAddress] = defaultAbiCoder.decode(['address'], log.data.substring(0, 66));
                    break;
                }
            }

            if (!tokenAddress) {
                logsData = item.executed?.receipt?.logs;

                for (const log of logsData) {
                    if (log.topics[0] === '0xf0d7beb2b03d35e597f432391dc2a6f6eb1a621be6cb5b325f55a49090085239') {
                        [tokenAddress] = defaultAbiCoder.decode(['address'], log.data.substring(0, 66));
                        break;
                    }
                }
            }

            if (tokenAddress) {
                break;
            }
        }

        if (address.toLowerCase() !== tokenAddress.toLowerCase()) {
            throw new Error('Provided token address does not match retrieved deployed interchain token address');
        }

        const recoveredAddress = ethers.utils.verifyMessage(message, signedHash);

        if (recoveredAddress.toLowerCase() !== sender.toLowerCase()) {
            throw new Error('Provided signer address does not match retrieved signer from message signature');
        }
    } catch (error) {
        printError('Error', error.message);
    }
}

async function main(options) {
    const { chainName, env } = options;
    const config = loadConfig(env);

    if (config.chains[chainName.toLowerCase()] === undefined) {
        throw new Error(`Chain ${chainName} is not defined in the info file`);
    }

    const chain = config.chains[chainName.toLowerCase()];

    await processCommand(chain, options);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('verify-token-deployer')
        .description('Script to verify that the signer of a signature corresponds to the deployer address for the provided transaction.');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(
        new Option('-n, --chainName <chainName>', 'origin chain from which transaction started').makeOptionMandatory(true).env('CHAIN'),
    );
    program.addOption(new Option('-a, --address <token address>', 'deployed interchain token address'));
    program.addOption(new Option('-t, --txHash <transaction hash>', 'transaction hash').makeOptionMandatory(true));
    program.addOption(new Option('-m, --message <message>', 'message to be signed').makeOptionMandatory(true));
    program.addOption(new Option('-s, --signedHash <signed hash>', 'signed hash').makeOptionMandatory(true));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
