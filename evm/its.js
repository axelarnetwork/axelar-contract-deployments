'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { hexZeroPad, toUtf8Bytes, keccak256 },
    BigNumber,
    Contract,
} = ethers;
const { Command } = require('commander');
const {
    printInfo,
    prompt,
    printWarn,
    printError,
    printWalletInfo,
    wasEventEmitted,
    mainProcessor,
    validateParameters,
    getContractJSON,
    isValidTokenId,
    getGasOptions,
    isNonEmptyString,
    isValidChain,
    getChainConfig,
} = require('./utils');
const { getWallet } = require('./sign-utils');
const IInterchainTokenService = getContractJSON('IInterchainTokenService');
const IMinter = getContractJSON('IMinter');
const InterchainTokenService = getContractJSON('InterchainTokenService');
const InterchainTokenFactory = getContractJSON('InterchainTokenFactory');
const IInterchainTokenDeployer = getContractJSON('IInterchainTokenDeployer');
const ITokenManager = getContractJSON('ITokenManager');
const IOwnable = getContractJSON('IOwnable');
const { addEvmOptions } = require('./cli-utils');
const { getSaltFromKey } = require('@axelar-network/axelar-gmp-sdk-solidity/scripts/utils');
const tokenManagerImplementations = {
    INTERCHAIN_TOKEN: 0,
    MINT_BURN_FROM: 1,
    LOCK_UNLOCK: 2,
    LOCK_UNLOCK_FEE: 3,
    MINT_BURN: 4,
};
const { getITSChains } = require('../common/utils');

function getDeploymentSalt(options) {
    const { rawSalt, salt } = options;

    if (rawSalt) {
        validateParameters({ isKeccak256Hash: { rawSalt } });
        return rawSalt;
    }

    validateParameters({ isString: { salt } });
    return getSaltFromKey(salt);
}

async function handleTx(tx, chain, contract, action, firstEvent, secondEvent) {
    printInfo(`${action} tx`, tx.hash);

    const receipt = await tx.wait(chain.confirmations);

    const eventEmitted =
        (firstEvent ? wasEventEmitted(receipt, contract, firstEvent) : true) ||
        (secondEvent ? wasEventEmitted(receipt, contract, secondEvent) : false);

    if (!eventEmitted) {
        printWarn('Event not emitted in receipt.');
    }
}

async function getTrustedChainsAndAddresses(config, interchainTokenService) {
    const allChains = Object.values(config.chains).map((chain) => chain.axelarId);

    // If ITS Hub is deployed, register it as a trusted chain as well
    const itsHubAddress = config.axelar?.contracts?.InterchainTokenService?.address;

    if (itsHubAddress) {
        allChains.push(config.axelar?.axelarId);
    }

    const trustedAddressesValues = await Promise.all(
        allChains.map(async (chainName) => await interchainTokenService.trustedAddress(chainName)),
    );
    const trustedChains = allChains.filter((_, index) => trustedAddressesValues[index] !== '');
    const trustedAddresses = trustedAddressesValues.filter((address) => address !== '');

    return [trustedChains, trustedAddresses];
}

function compare(contractValue, configValue, variableName) {
    contractValue = isNonEmptyString(contractValue) ? contractValue.toLowerCase() : contractValue;
    configValue = isNonEmptyString(configValue) ? configValue.toLowerCase() : configValue;

    if (contractValue === configValue) {
        printInfo(variableName, contractValue);
    } else {
        printError(
            `Error: Value mismatch for '${variableName}'. Config value: ${configValue}, InterchainTokenService value: ${contractValue}`,
        );
    }
}

function compareToConfig(contractConfig, contractName, toCheck) {
    for (const [key, value] of Object.entries(toCheck)) {
        if (contractConfig[key]) {
            const configValue = contractConfig[key];
            compare(value, configValue, key);
        } else {
            printWarn(`Warning: The key '${key}' is not found in the contract config for ${contractName}.`);
        }
    }
}

function isValidDestinationChain(config, destinationChain) {
    if (destinationChain === '') {
        return;
    }

    isValidChain(config, destinationChain);
}

async function processCommand(config, chain, options) {
    const { privateKey, address, action, yes, args } = options;

    const contracts = chain.contracts;
    const contractName = 'InterchainTokenService';

    const interchainTokenServiceAddress = address || contracts.InterchainTokenService?.address;

    if (!interchainTokenServiceAddress) {
        printWarn(`No InterchainTokenService address found for chain ${chain.name}`);
        return;
    }

    validateParameters({ isValidAddress: { interchainTokenServiceAddress } });

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = await getWallet(privateKey, provider, options);
    const { address: walletAddress } = await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', interchainTokenServiceAddress);

    const interchainTokenService = new Contract(interchainTokenServiceAddress, IInterchainTokenService.abi, wallet);

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('Action', action);

    if (prompt(`Proceed with action ${action}`, yes)) {
        return;
    }

    switch (action) {
        case 'contract-id': {
            const contractId = await interchainTokenService.contractId();
            printInfo('InterchainTokenService contract ID', contractId);

            break;
        }

        case 'token-manager-address': {
            if (args.length < 1) {
                throw new Error('Missing required argument: <token-id>');
            }

            const [tokenId] = args;
            validateParameters({ isValidTokenId: { tokenId } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const tokenManagerAddress = await interchainTokenService.tokenManagerAddress(tokenIdBytes32);
            printInfo(`TokenManager address for tokenId: ${tokenId}`, tokenManagerAddress);

            try {
                await interchainTokenService.deployedTokenManager(tokenIdBytes32);
                printInfo(`TokenManager for tokenId: ${tokenId} exists at address:`, tokenManagerAddress);
            } catch (error) {
                printInfo(`TokenManager for tokenId: ${tokenId} does not yet exist.`);
            }

            break;
        }

        case 'interchain-token-address': {
            if (args.length < 1) {
                throw new Error('Missing required argument: <token-id>');
            }

            const [tokenId] = args;
            validateParameters({ isValidTokenId: { tokenId } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const interchainTokenAddress = await interchainTokenService.interchainTokenAddress(tokenIdBytes32);
            printInfo(`InterchainToken address for tokenId: ${tokenId}`, interchainTokenAddress);

            try {
                await interchainTokenService.registeredTokenAddress(tokenIdBytes32);
                printInfo(`Token for tokenId: ${tokenId} exists at address:`, interchainTokenAddress);
            } catch (error) {
                printInfo(`Token for tokenId: ${tokenId} does not yet exist.`);
            }

            break;
        }

        case 'interchain-token-id': {
            if (args.length < 1) {
                throw new Error('Missing required argument: <sender>');
            }

            const [sender] = args;
            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({ isValidAddress: { sender } });

            const interchainTokenId = await interchainTokenService.interchainTokenId(sender, deploymentSalt);
            printInfo(`InterchainTokenId for sender ${sender} and deployment salt: ${deploymentSalt}`, interchainTokenId);

            break;
        }

        case 'token-manager-implementation': {
            const tokenManagerImplementation = await interchainTokenService.tokenManager();
            printInfo(`TokenManager implementation address`, tokenManagerImplementation);

            break;
        }

        case 'flow-limit': {
            if (args.length < 1) {
                throw new Error('Missing required argument: <token-id>');
            }

            const [tokenId] = args;
            validateParameters({ isValidTokenId: { tokenId } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const tokenManagerAddress = await interchainTokenService.deployedTokenManager(tokenIdBytes32);

            const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, wallet);

            const flowLimit = await tokenManager.flowLimit();
            printInfo(`Flow limit for TokenManager with tokenId ${tokenId}`, flowLimit);
            break;
        }

        case 'flow-out-amount': {
            if (args.length < 1) {
                throw new Error('Missing required argument: <token-id>');
            }

            const [tokenId] = args;
            validateParameters({ isValidTokenId: { tokenId } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const tokenManagerAddress = await interchainTokenService.deployedTokenManager(tokenIdBytes32);

            const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, wallet);

            const flowOutAmount = await tokenManager.flowOutAmount();
            printInfo(`Flow out amount for TokenManager with tokenId ${tokenId}`, flowOutAmount);

            break;
        }

        case 'flow-in-amount': {
            if (args.length < 1) {
                throw new Error('Missing required argument: <token-id>');
            }

            const [tokenId] = args;
            validateParameters({ isValidTokenId: { tokenId } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const tokenManagerAddress = await interchainTokenService.deployedTokenManager(tokenIdBytes32);

            const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, wallet);

            const flowInAmount = await tokenManager.flowInAmount();
            printInfo(`Flow in amount for TokenManager with tokenId ${tokenId}`, flowInAmount);

            break;
        }

        case 'contract-call-value': {
            if (args.length < 3) {
                throw new Error('Missing required arguments: <source-chain> <source-address> <payload>');
            }

            const [sourceChain, sourceAddress, payload] = args;
            validateParameters({ isNonEmptyString: { sourceChain, sourceAddress }, isValidCalldata: { payload } });
            const isTrustedAddress = await interchainTokenService.isTrustedAddress(sourceChain, sourceAddress);

            if (!isTrustedAddress) {
                throw new Error('Invalid remote service.');
            }

            validateParameters({ isValidCalldata: { payload } });

            const [tokenAddress, tokenAmount] = await interchainTokenService.contractCallValue(sourceChain, sourceAddress, payload);
            printInfo(`Amount of tokens with address ${tokenAddress} that the call is worth:`, tokenAmount);

            break;
        }

        case 'express-execute': {
            if (args.length < 4) {
                throw new Error('Missing required arguments: <command-id> <source-chain> <source-address> <payload>');
            }

            const [commandID, sourceChain, sourceAddress, payload] = args;
            validateParameters({
                isKeccak256Hash: { commandID },
                isNonEmptyString: { sourceChain, sourceAddress },
                isValidCalldata: { payload },
            });

            const tx = await interchainTokenService.expressExecute(commandID, sourceChain, sourceAddress, payload, gasOptions);

            await handleTx(tx, chain, interchainTokenService, action, 'ExpressExecuted');

            break;
        }

        case 'interchain-transfer': {
            if (args.length < 5) {
                throw new Error(
                    'Missing required arguments: <token-id> <destination-chain> <destination-address> <amount> <metadata> <gas-value>',
                );
            }

            const [tokenId, destinationChain, destinationAddress, amount, metadata, gasValue] = args;
            validateParameters({
                isValidTokenId: { tokenId },
                isNonEmptyString: { destinationChain, destinationAddress },
                isValidNumber: { amount, gasValue },
                isValidCalldata: { metadata },
            });

            if ((await interchainTokenService.trustedAddress(destinationChain)) === '') {
                throw new Error(`Destination chain ${destinationChain} is not trusted by ITS`);
            }

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const tokenManager = new Contract(
                await interchainTokenService.deployedTokenManager(tokenIdBytes32),
                getContractJSON('ITokenManager').abi,
                wallet,
            );
            const token = new Contract(
                await interchainTokenService.registeredTokenAddress(tokenIdBytes32),
                getContractJSON('InterchainToken').abi,
                wallet,
            );

            const implementationType = (await tokenManager.implementationType()).toNumber();
            const decimals = await token.decimals();
            const adjustedAmount = BigNumber.from(amount).mul(BigNumber.from(10).pow(decimals));
            const balance = await token.balanceOf(wallet.address);

            if (balance.lt(amount)) {
                throw new Error(`Insufficient balance for transfer. Balance: ${balance}, amount: ${adjustedAmount}`);
            }

            if (
                implementationType !== tokenManagerImplementations.MINT_BURN &&
                implementationType !== tokenManagerImplementations.INTERCHAIN_TOKEN
            ) {
                printInfo('Approving ITS for a transfer for token with token manager type', implementationType);
                await token.approve(interchainTokenService.address, amount, gasOptions).then((tx) => tx.wait());
            }

            const tx = await interchainTokenService.interchainTransfer(
                tokenIdBytes32,
                destinationChain,
                destinationAddress,
                adjustedAmount,
                metadata,
                gasValue,
                { value: gasValue, ...gasOptions },
            );

            await handleTx(tx, chain, interchainTokenService, action, 'InterchainTransfer', 'InterchainTransferWithData');

            break;
        }

        case 'register-token-metadata': {
            if (args.length < 2) {
                throw new Error('Missing required arguments: <token-address> <gas-value>');
            }

            const [tokenAddress, gasValue] = args;
            validateParameters({ isValidAddress: { tokenAddress }, isValidNumber: { gasValue } });

            const tx = await interchainTokenService.registerTokenMetadata(tokenAddress, gasValue, { value: gasValue, ...gasOptions });

            await handleTx(tx, chain, interchainTokenService, action);

            break;
        }

        case 'set-flow-limits': {
            if (args.length < 2) {
                throw new Error('Missing required arguments: <token-ids> <flow-limits>');
            }

            const [tokenIdsArg, flowLimitsArg] = args;
            const flowLimitsStrings = flowLimitsArg.split(',');
            const tokenIds = tokenIdsArg.split(',');
            const flowLimits = [];

            for (const flowLimit of flowLimitsStrings) {
                flowLimits.push(Number(flowLimit));
            }

            const tokenIdsBytes32 = [];
            const tokenManagers = [];

            for (const tokenId of tokenIds) {
                if (!isValidTokenId(tokenId)) {
                    throw new Error(`Invalid tokenId value: ${tokenId}`);
                }

                const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);
                tokenIdsBytes32.push(tokenIdBytes32);

                const tokenManager = new Contract(
                    await interchainTokenService.deployedTokenManager(tokenIdBytes32),
                    getContractJSON('ITokenManager').abi,
                    wallet,
                );
                tokenManagers.push(tokenManager);
            }

            validateParameters({ isNumberArray: { flowLimits } });

            const tx = await interchainTokenService.setFlowLimits(tokenIdsBytes32, flowLimits, gasOptions);

            await handleTx(tx, chain, tokenManagers[0], action, 'FlowLimitSet');

            break;
        }

        case 'trusted-address': {
            if (args.length < 1) {
                throw new Error('Missing required argument: <trusted-chain>');
            }

            const [trustedChain] = args;
            validateParameters({ isNonEmptyString: { trustedChain } });

            const trustedAddress = await interchainTokenService.trustedAddress(trustedChain);

            if (trustedAddress) {
                printInfo(`Trusted address for chain ${trustedChain}`, trustedAddress);
            } else {
                printWarn(`No trusted address for chain ${trustedChain}`);
            }

            break;
        }

        case 'set-trusted-address': {
            if (args.length < 2) {
                throw new Error('Missing required arguments: <trustedChain> <trustedAddress>');
            }

            const [trustedChain, trustedAddress] = args;
            const owner = await new Contract(interchainTokenService.address, IOwnable.abi, wallet).owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            validateParameters({ isNonEmptyString: { trustedChain } });

            let trustedChains, trustedAddresses;

            if (trustedChain === 'all') {
                trustedChains = getITSChains(config);
                trustedAddresses = trustedChains.map((_) => trustedAddress || chain.contracts?.InterchainTokenService?.address);
            } else {
                const trustedChainFinal =
                    getChainConfig(config, trustedChain.toLowerCase(), { skipCheck: true })?.axelarId || trustedChain.toLowerCase();
                const trustedAddressFinal =
                    trustedAddress || getChainConfig(config, trustedChain.toLowerCase())?.contracts?.InterchainTokenService?.address;

                if (trustedChainFinal === undefined || trustedAddressFinal === undefined) {
                    throw new Error(`Invalid chain/address: ${trustedChain}`);
                }

                trustedChains = [trustedChainFinal];
                trustedAddresses = [trustedAddressFinal];
            }

            if (prompt(`Proceed with setting trusted address for chain ${trustedChains} to ${trustedAddresses}?`, yes)) {
                return;
            }

            for (const [trustedChain, trustedAddress] of trustedChains.map((chain, index) => [chain, trustedAddresses[index]])) {
                const tx = await interchainTokenService.setTrustedAddress(trustedChain, trustedAddress, gasOptions);

                await handleTx(tx, chain, interchainTokenService, action, 'TrustedAddressSet');
            }

            break;
        }

        case 'remove-trusted-address': {
            if (args.length < 1) {
                throw new Error('Missing required argument: <trusted-chain|all>');
            }

            const [trustedChainArg] = args;
            const owner = await new Contract(interchainTokenService.address, IOwnable.abi, wallet).owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            let trustedChains;

            if (trustedChainArg === 'all') {
                [trustedChains] = await getTrustedChainsAndAddresses(config, interchainTokenService);
            } else {
                const trustedChain = config.chains[trustedChainArg.toLowerCase()]?.axelarId;

                if (trustedChain === undefined) {
                    throw new Error(`Invalid chain: ${trustedChainArg}`);
                }

                if ((await interchainTokenService.trustedAddress(trustedChainArg)) === '') {
                    printError(`No trusted address for chain ${trustedChainArg}`);
                    return;
                }

                trustedChains = [trustedChain];
            }

            printInfo(`Removing trusted address for chains ${trustedChains}`);

            for (const trustedChain of trustedChains) {
                const tx = await interchainTokenService.removeTrustedAddress(trustedChain, gasOptions);
                await handleTx(tx, chain, interchainTokenService, action, 'TrustedAddressRemoved');
            }

            break;
        }

        case 'set-pause-status': {
            if (args.length < 1) {
                throw new Error('Missing required argument: <pause-status>');
            }

            const owner = await new Contract(interchainTokenService.address, IOwnable.abi, wallet).owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            const pauseStatus = args[0] === 'true';
            const tx = await interchainTokenService.setPauseStatus(pauseStatus, gasOptions);

            await handleTx(tx, chain, interchainTokenService, action, 'Paused', 'Unpaused');

            break;
        }

        case 'execute': {
            if (args.length < 4) {
                throw new Error('Missing required arguments: <command-id> <source-chain> <source-address> <payload>');
            }

            const [commandID, sourceChain, sourceAddress, payload] = args;
            validateParameters({
                isKeccak256Hash: { commandID },
                isNonEmptyString: { sourceChain, sourceAddress },
                isValidCalldata: { payload },
            });
            const isTrustedAddress = await interchainTokenService.isTrustedAddress(sourceChain, sourceAddress);

            if (!isTrustedAddress) {
                throw new Error('Invalid remote service.');
            }

            const tx = await interchainTokenService.execute(commandID, sourceChain, sourceAddress, payload, gasOptions);

            await handleTx(tx, chain, interchainTokenService, action);

            break;
        }

        case 'checks': {
            const interchainTokenService = new Contract(interchainTokenServiceAddress, InterchainTokenService.abi, wallet);

            const contractConfig = chain.contracts[contractName];

            const interchainTokenDeployer = await interchainTokenService.interchainTokenDeployer();
            const interchainTokenFactory = await interchainTokenService.interchainTokenFactory();

            const interchainTokenFactoryContract = new Contract(interchainTokenFactory, InterchainTokenFactory.abi, wallet);
            const interchainTokenFactoryImplementation = await interchainTokenFactoryContract.implementation();

            const interchainTokenDeployerContract = new Contract(interchainTokenDeployer, IInterchainTokenDeployer.abi, wallet);
            const interchainToken = await interchainTokenDeployerContract.implementationAddress();

            const [trustedChains, trustedAddresses] = await getTrustedChainsAndAddresses(config, interchainTokenService);

            printInfo('Trusted chains', trustedChains);
            printInfo('Trusted addresses', trustedAddresses);

            // check if all trusted addresses match ITS address
            for (const trustedAddress of trustedAddresses) {
                if (trustedAddress !== interchainTokenServiceAddress) {
                    printError(
                        `Error: Trusted address ${trustedAddress} does not match InterchainTokenService address ${interchainTokenServiceAddress}`,
                    );

                    break;
                }
            }

            const gateway = await interchainTokenService.gateway();
            const gasService = await interchainTokenService.gasService();

            const configGateway = chain.contracts.AxelarGateway?.address;
            const configGasService = chain.contracts.AxelarGasService?.address;

            const chainNameHash = await interchainTokenService.chainNameHash();
            const configChainNameHash = keccak256(toUtf8Bytes(chain.axelarId));

            compare(gateway, configGateway, 'AxelarGateway');
            compare(gasService, configGasService, 'AxelarGasService');
            compare(chainNameHash, configChainNameHash, 'chainNameHash');

            const toCheck = {
                tokenManagerDeployer: await interchainTokenService.tokenManagerDeployer(),
                interchainTokenDeployer,
                interchainToken,
                tokenManager: await interchainTokenService.tokenManager(),
                tokenHandler: await interchainTokenService.tokenHandler(),
                implementation: await interchainTokenService.implementation(),
            };

            compareToConfig(contractConfig, contractName, toCheck);

            const itsFactoryContractName = 'InterchainTokenFactory';
            const itsFactoryContractConfig = chain.contracts[itsFactoryContractName];

            const toCheckFactory = {
                address: interchainTokenFactory,
                implementation: interchainTokenFactoryImplementation,
            };

            compareToConfig(itsFactoryContractConfig, itsFactoryContractName, toCheckFactory);

            break;
        }

        case 'migrate-interchain-token': {
            if (args.length < 1) {
                throw new Error('Missing required argument: <token-id>');
            }

            const [tokenId] = args;
            validateParameters({ isKeccak256Hash: { tokenId } });

            const tx = await interchainTokenService.migrateInterchainToken(tokenId);

            await handleTx(tx, chain, interchainTokenService, action);

            break;
        }

        case 'transfer-mintership': {
            if (args.length < 2) {
                throw new Error('Missing required arguments: <token-address> <minter>');
            }

            const [tokenAddress, minter] = args;
            validateParameters({ isValidAddress: { tokenAddress, minter } });

            const token = new Contract(tokenAddress, IMinter.abi, wallet);
            const tx = await token.transferMintership(minter);

            await handleTx(tx, chain, token, action, 'RolesRemoved', 'RolesAdded');

            break;
        }

        case 'set-trusted-chain': {
            if (args.length < 1) {
                throw new Error('Missing required arguments: <trusted-chain>');
            }

            const [trustedChain] = args;
            const owner = await new Contract(interchainTokenService.address, IOwnable.abi, wallet).owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            validateParameters({ isNonEmptyString: { trustedChain } });

            const trustedChainFinal =
                getChainConfig(config, trustedChain.toLowerCase(), { skipCheck: true })?.axelarId || trustedChain.toLowerCase();
            const trustedAddressFinal =
                'hub' || getChainConfig(config, trustedChain.toLowerCase())?.contracts?.InterchainTokenService?.address;

            if (trustedChainFinal === undefined || trustedAddressFinal === undefined) {
                throw new Error(`Invalid chain/address: ${trustedChain}`);
            }

            const trustedChains = [trustedChainFinal];
            const trustedAddresses = [trustedAddressFinal];

            if (prompt(`Proceed with setting trusted address for chain ${trustedChains} to ${trustedAddresses}?`, yes)) {
                return;
            }

            for (const [trustedChain, trustedAddress] of trustedChains.map((chain, index) => [chain, trustedAddresses[index]])) {
                const tx = await interchainTokenService.setTrustedAddress(trustedChain, trustedAddress, gasOptions);

                await handleTx(tx, chain, interchainTokenService, action, 'TrustedAddressSet');
            }

            break;
        }

        default: {
            throw new Error(`Unknown action ${action}`);
        }
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('ITS').description('Script to perform ITS commands');

    addEvmOptions(program, { address: true, salt: true });

    program.argument('<command>', 'ITS command to execute');
    program.argument('[args...]', 'Arguments for the command');

    program.action((command, args, options) => {
        options.action = command;
        options.args = args;
        main(options);
    });

    program.parse();
}

module.exports = { getDeploymentSalt, handleTx, getTrustedChainsAndAddresses, isValidDestinationChain };
