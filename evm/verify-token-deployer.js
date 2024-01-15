'use strict';

const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    utils: { defaultAbiCoder, Interface, verifyMessage },
    providers: { JsonRpcProvider },
    Contract,
} = ethers;

const { loadConfig, printError, getContractJSON, printInfo, isKeccak256Hash } = require('./utils');

const EVENT_SIGNATURE_INDEX = 0;
const TOKEN_ID_INDEX = 1;
const EVENT_FILTER_START_BLOCK = 0;

async function processCommand(config, options, destChainName) {
    try {
        const { signedHash, address, message, txHash } = options;

        if (config.chains[destChainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${destChainName} is not defined in the info file`);
        }

        const destChain = config.chains[destChainName.toLowerCase()];

        const interchainTokenServiceABI = getContractJSON('InterchainTokenService').abi;
        const interchainTokenServiceInterface = new Interface(interchainTokenServiceABI);
        const interchainTokenDeployedEventHash = interchainTokenServiceInterface.getEventTopic('InterchainTokenDeployed');
        const interchainTokenDeploymentStartedEventHash = interchainTokenServiceInterface.getEventTopic('InterchainTokenDeploymentStarted');

        if (!isKeccak256Hash(txHash)) {
            throw new Error('Invalid transaction hash');
        }

        const provider = new JsonRpcProvider(destChain.rpc);

        const interchainTokenABI = getContractJSON('InterchainToken').abi;
        const interchainToken = new Contract(address, interchainTokenABI, provider);

        let tokenId;

        try {
            tokenId = await interchainToken.interchainTokenId();
        } catch (error) {
            throw new Error('Unable to fetch interchain token ID');
        }

        const tx = await provider.getTransaction(txHash);

        const [, sourceChainName] = interchainTokenServiceInterface.decodeFunctionData('execute', tx.data);

        const receipt = await tx.wait();

        const tokenAddress = fetchTokenAddress(receipt.logs, interchainTokenDeployedEventHash, tokenId);

        if (address.toLowerCase() !== tokenAddress.toLowerCase()) {
            throw new Error('Provided token address does not match retrieved deployed interchain token address');
        }

        if (config.chains[sourceChainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${sourceChainName} is not defined in the info file`);
        }

        const sourceChain = config.chains[sourceChainName.toLowerCase()];

        const sourceChainProvider = new JsonRpcProvider(sourceChain.rpc);

        const filter = {
            address: sourceChain.contracts.InterchainTokenService.address,
            topics: [interchainTokenDeploymentStartedEventHash, tokenId],
            fromBlock: EVENT_FILTER_START_BLOCK,
        };

        const logs = await sourceChainProvider.getLogs(filter);

        let sourceTxHash;

        for (const log of logs) {
            if (log.topics[EVENT_SIGNATURE_INDEX] === interchainTokenDeploymentStartedEventHash && log.topics[TOKEN_ID_INDEX] === tokenId) {
                const { destinationChain } = interchainTokenServiceInterface.decodeEventLog(
                    'InterchainTokenDeploymentStarted',
                    log.data,
                    log.topics,
                );

                if (destinationChain === destChain.id) {
                    sourceTxHash = log.transactionHash;
                    break;
                }
            }
        }

        if (!sourceTxHash) {
            throw new Error('Specied source chain tx not found');
        }

        const sourceTx = await sourceChainProvider.getTransaction(sourceTxHash);
        const sender = sourceTx.from;

        const recoveredAddress = verifyMessage(message, signedHash);

        if (recoveredAddress.toLowerCase() !== sender.toLowerCase()) {
            throw new Error('Provided signer address does not match retrieved signer from message signature');
        }

        printInfo('Sender address matches recovered address.');
    } catch (error) {
        printError('Error', error.message);
    }
}

function fetchTokenAddress(logs, eventHash, tokenId) {
    let tokenAddress;

    for (const log of logs) {
        if (log.topics[EVENT_SIGNATURE_INDEX] === eventHash && log.topics[TOKEN_ID_INDEX] === tokenId) {
            [tokenAddress] = defaultAbiCoder.decode(['address'], log.data.substring(0, 66));
            break;
        }
    }

    if (!tokenAddress) {
        throw new Error('No interchain token deployment found with specified token id');
    }

    return tokenAddress;
}

async function main(options) {
    const { chainName, env } = options;
    const config = loadConfig(env);

    await processCommand(config, options, chainName);
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
        new Option('-n, --chainName <chainName>', 'destination chain on which token is deployed').makeOptionMandatory(true).env('CHAIN'),
    );
    program.addOption(new Option('-a, --address <token address>', 'deployed interchain token address').makeOptionMandatory(true));
    program.addOption(new Option('-t, --txHash <transaction hash>', 'transaction hash on destinatino chain').makeOptionMandatory(true));
    program.addOption(
        new Option('-m, --message <message>', 'message to be signed').makeOptionMandatory(true).makeOptionMandatory(true).env('MESSAGE'),
    );
    program.addOption(
        new Option('-s, --signedHash <signed hash>', 'signed hash').makeOptionMandatory(true).makeOptionMandatory(true).env('SIGNED_HASH'),
    );

    program.action((options) => {
        main(options);
    });

    program.parse();
}
