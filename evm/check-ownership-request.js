'use strict';

const axios = require('axios');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const { Contract, getDefaultProvider } = ethers;
const { loadConfig, printError, getContractJSON, printInfo, printWarn, isValidAddress, printObj } = require('./utils');

const interchainTokenFactoryABI = getContractJSON('InterchainTokenFactory').abi;
const interchainTokenABI = getContractJSON('InterchainToken').abi;
const interchainTokenServiceABI = getContractJSON('InterchainTokenService').abi;
const erc20ABI = getContractJSON('IERC20Named').abi;

async function processCommand(config, options) {
    try {
        const { deployer, address, its, rpc, api } = options;
        let { source, destination } = options;
        destination = JSON.parse(destination);

        if (!isValidAddress(address)) {
            throw new Error('Invalid address parameter.');
        }

        const sourceChain = config.chains[source.toLowerCase()];

        if (!sourceChain) {
            throw new Error(`Chain ${source} is not defined in the info file`);
        }

        const invalidDestinations = destination.filter((chain) => !config.chains[chain.toLowerCase()]);

        if (invalidDestinations.length > 0) {
            throw new Error(`Chains ${invalidDestinations.join(', ')} are not defined in the info file`);
        }

        const provider = getDefaultProvider(rpc || sourceChain.rpc);
        const itsAddress = its || sourceChain.contracts.InterchainTokenService?.address;

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
        isCanonicalToken = address === validCanonicalAddress;
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

            if ((await its.validTokenAddress(tokenId)) === address) {
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

        if (!(sourceData.confirmed === true && sourceData.is_external === false)) {
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

            if (!(chainData.confirmed === true && chainData.is_external === false && chainData.address)) {
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

    program
        .name('verify-token-deployer')
        .description('Script to verify that the signer of a signature corresponds to the deployer address for the provided transaction.');
    program.addOption(
        new Option('--deployer <deployer>', 'deployed through which axelar product').choices(['gateway', 'its']).makeOptionMandatory(true),
    );
    program.addOption(
        new Option('-s, --source <sourceChain>', 'source chain on which provided contract address is deployed').makeOptionMandatory(true),
    );
    program.addOption(
        new Option('-d, --destination <destinationChain>', 'destination chains on which other tokens are deployed').makeOptionMandatory(
            true,
        ),
    );
    program.addOption(new Option('-a, --address <token address>', 'deployed token address on source chain').makeOptionMandatory(true));
    program.addOption(
        new Option('-r, --rpc <rpc>', 'The rpc url for creating a provider on source chain to fetch token information').env('RPC'),
    );
    program.addOption(new Option('-i, --its <its>', 'The Interchain token service address to be used if not want to use config address'));
    program.addOption(new Option('--api <api>', 'api to check token deployed through gateway and the token details'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
