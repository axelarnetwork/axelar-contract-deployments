'use strict';

const axios = require('axios');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    utils: { defaultAbiCoder, isHexString, keccak256, toUtf8Bytes },
} = ethers;

const { loadConfig, printError } = require('./utils');

async function processCommand(chain, options) {
    try {
        const { signedHash, env, address, message, api } = options;
        const { txHash } = options;
        const eventHash = keccak256(toUtf8Bytes('InterchainTokenDeployed(bytes32,address,address,string,string,uint8)'));

        if (!(txHash && txHash.startsWith('0x') && isHexString(txHash) && txHash.length === 66)) {
            throw new Error('Invalid transaction hash');
        }

        const apiUrl = api || (env === 'testnet' ? 'https://testnet.api.gmp.axelarscan.io/' : 'https://api.gmp.axelarscan.io/');
        const requestBody = {
            txHash,
            method: 'searchGMP',
        };

        const response = await axios.post(apiUrl, requestBody);

        if (!(response.data && response.data.data && Array.isArray(response.data.data))) {
            throw new Error('No data found in the response.');
        }

        const data = response.data.data;

        const sender = fetchSenderAddress(data, chain.name.toLowerCase());
        const tokenAddress = fetchTokenAddress(data, eventHash);

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

function fetchSenderAddress(data, chainName) {
    const sender = data.find((item) => item.call?.chain === chainName)?.call.receipt.from;

    if (!sender) {
        throw new Error('Sender not found in the provided transaction details');
    }

    return sender;
}

function fetchTokenAddress(data, eventHash) {
    let tokenAddress;
    const EVENT_SIGNATURE_INDEX = 0;

    for (const item of data) {
        let logsData = item.call?.receipt?.logs;

        for (const log of logsData) {
            if (log.topics[EVENT_SIGNATURE_INDEX] === eventHash) {
                [tokenAddress] = defaultAbiCoder.decode(['address'], log.data.substring(0, 66));
                break;
            }
        }

        if (!tokenAddress) {
            logsData = item.executed?.receipt?.logs;

            for (const log of logsData) {
                if (log.topics[EVENT_SIGNATURE_INDEX] === eventHash) {
                    [tokenAddress] = defaultAbiCoder.decode(['address'], log.data.substring(0, 66));
                    break;
                }
            }
        }

        if (tokenAddress) {
            break;
        }
    }

    if (!tokenAddress) {
        throw new Error('No interchain token deployment found');
    }

    return tokenAddress;
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
    program.addOption(new Option('--api <gmp api>', 'gmp api to query transaction details').env('API_URL'));
    program.addOption(new Option('-a, --address <token address>', 'deployed interchain token address'));
    program.addOption(new Option('-t, --txHash <transaction hash>', 'transaction hash').makeOptionMandatory(true));
    program.addOption(new Option('-m, --message <message>', 'message to be signed').makeOptionMandatory(true));
    program.addOption(new Option('-s, --signedHash <signed hash>', 'signed hash').makeOptionMandatory(true));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
