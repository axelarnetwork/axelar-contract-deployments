'use strict';

require('dotenv').config();

const axios = require('axios');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {loadConfig} = require('../common');
const { Contract, getDefaultProvider } = ethers;
const {
    validateParameters,
    printError,
    getContractJSON,
    printInfo,
    printWarn,
    printObj,
    isValidAddress,
    isStringArray,
} = require('./utils');

const interchainTokenFactoryABI = getContractJSON('InterchainTokenFactory').abi;
const interchainTokenABI = getContractJSON('InterchainToken').abi;
const interchainTokenServiceABI = getContractJSON('InterchainTokenService').abi;
const erc20ABI = getContractJSON('IERC20Named').abi;

async function processCommand(config, options) {
    try {
        const { deployer, address, its, rpc, api } = options;
        let { source, destination } = options;

        validateParameters({ isValidAddress: { address }, isNonEmptyString: { source, destination } });

        const sourceChain = config.chains[source.toLowerCase()];

        if (!sourceChain) {
            throw new Error(`Chain ${source} is not defined in the info file`);
        }

        try {
            destination = JSON.parse(destination);
        } catch (error) {
            throw new Error(`Unable to parse destination chains: ${error}`);
        }

        if (!isStringArray(destination)) {
            throw new Error(`Invalid destination chains type, expected string`);
        }

        const invalidDestinations = destination.filter((chain) => !config.chains[chain.toLowerCase()]);

        if (invalidDestinations.length > 0) {
            throw new Error(`Chains ${invalidDestinations.join(', ')} are not defined in the info file`);
        }

        const provider = getDefaultProvider(rpc || sourceChain.rpc);
        let itsAddress;

        if (its) {
            if (isValidAddress(its)) {
                itsAddress = its;
            } else {
                throw new Error(`Invalid ITS address: ${its}`);
            }
        } else {
            itsAddress = sourceChain.contracts.InterchainTokenService?.address;
        }

        if (deployer === 'gateway') {
            const gatewayTokens = await fetchGatewayTokens(address, sourceChain.name.toLowerCase(), destination, provider, api);
            printInfo(`Gateway Tokens on destination chains`);
            printObj(gatewayTokens);
            return;
        }

        if (await isTokenCanonical(address, itsAddress, provider)) {
            printInfo(`Provided address ${address} is a canonical token`);
            return;
        }

        const interchainToken = new Contract(address, interchainTokenABI, provider);
        const tokenId = await isNativeInterchainToken(interchainToken);
        const interchainTokens = await fetchNativeInterchainTokens(address, config, tokenId, destination, its);
        printInfo(`Native Interchain Tokens on destination chains`);
        printObj(interchainTokens);
    } catch (error) {
        printError('Error', error.message);
    }
}

async function isTokenCanonical(address, itsAddress, provider) {
    let isCanonicalToken;
    const its = new Contract(itsAddress, interchainTokenServiceABI, provider);
    const itsFactory = new Contract(await its.interchainTokenFactory(), interchainTokenFactoryABI, provider);
    const canonicalTokenId = await itsFactory.canonicalInterchainTokenId(address);

    try {
        const validCanonicalAddress = await its.validTokenAddress(canonicalTokenId);
        isCanonicalToken = address.toLowerCase() === validCanonicalAddress.toLowerCase();
    } catch {}

    return isCanonicalToken;
}

async function fetchNativeInterchainTokens(address, config, tokenId, destination, itsAddress) {
    const interchainTokens = [];

    try {
        for (const chain of destination) {
            const chainConfig = config.chains[chain];
            itsAddress = itsAddress || chainConfig.contracts.InterchainTokenService?.address;
            const provider = getDefaultProvider(chainConfig.rpc);
            const its = new Contract(itsAddress, interchainTokenServiceABI, provider);

            if ((await its.validTokenAddress(tokenId)).toLowerCase() === address.toLowerCase()) {
                interchainTokens.push({ [chain]: address });
            } else {
                printWarn(`No native Interchain token found for tokenId ${tokenId} on chain ${chain}`);
            }
        }

        if (destination.length !== interchainTokens.length) {
            printError('Native Interchain tokens not found on all destination chains');
        }

        return interchainTokens;
    } catch (error) {
        throw new Error('Unable to fetch native interchain tokens on destination chains');
    }
}

async function isNativeInterchainToken(token) {
    try {
        return await token.interchainTokenId();
    } catch {
        throw new Error(`The token at address ${await token.address} is not a Interchain Token`);
    }
}

async function isGatewayToken(apiUrl, address) {
    try {
        const { data: sourceData } = await axios.get(apiUrl);

        if (!(sourceData.confirmed && !sourceData.is_external)) {
            throw new Error();
        }
    } catch {
        throw new Error(`The token at address ${address} is not deployed through Axelar Gateway.`);
    }
}

async function fetchGatewayTokens(address, source, destination, provider, api) {
    const gatewayTokens = [];
    const apiUrl = api || 'https://lcd-axelar.imperator.co/axelar/evm/v1beta1/token_info/';

    await isGatewayToken(`${apiUrl}${source}?address=${address}`, address);

    try {
        const token = new Contract(address, erc20ABI, provider);
        const symbol = await token.symbol();

        for (const chain of destination) {
            const { data: chainData } = await axios.get(`${apiUrl}${chain}?symbol=${symbol}`);

            if (!(chainData.confirmed && !chainData.is_external && chainData.address)) {
                printWarn(`No Gateway token found for token symbol ${symbol} on chain ${chain}`);
            } else {
                gatewayTokens.push({ [chain]: chainData.address });
            }
        }

        if (destination.length !== gatewayTokens.length) {
            printError('Gateway tokens not found on all destination chains');
        }

        return gatewayTokens;
    } catch (error) {
        throw new Error('Unable to fetch gateway tokens on destination chains');
    }
}

async function main(options) {
    const env = 'mainnet';
    const config = loadConfig(env);

    await processCommand(config, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('check-ownership-request').description('Script to check token ownership claim request');

    program.addOption(
        new Option('--deployer <deployer>', 'deployed through which axelar product')
            .choices(['gateway', 'its'])
            .makeOptionMandatory(true)
            .env('DEPLOYER'),
    );
    program.addOption(
        new Option('-s, --source <sourceChain>', 'source chain on which provided contract address is deployed')
            .makeOptionMandatory(true)
            .env('SOURCE'),
    );
    program.addOption(
        new Option('-d, --destination <destinationChains>', 'destination chains on which other tokens are deployed')
            .makeOptionMandatory(true)
            .env('DESTINATION'),
    );
    program.addOption(
        new Option('-a, --address <token address>', 'deployed token address on source chain').makeOptionMandatory(true).env('ADDRESS'),
    );
    program.addOption(
        new Option('-r, --rpc <rpc>', 'The rpc url for creating a provider on source chain to fetch token information').env('RPC'),
    );
    program.addOption(new Option('-i, --its <its>', 'Interchain token service override address'));
    program.addOption(new Option('--api <apiUrl>', 'api url to check token deployed through gateway and the token details'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
