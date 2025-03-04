'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { hexZeroPad, toUtf8Bytes, keccak256 },
    BigNumber,
    Contract,
} = ethers;
const { Command, Option } = require('commander');
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
    parseTrustedChains,
} = require('./utils');
const { getWallet } = require('./sign-utils');
const IInterchainTokenService = getContractJSON('IInterchainTokenService');
const IMinter = getContractJSON('IMinter');
const InterchainTokenService = getContractJSON('InterchainTokenService');
const InterchainTokenFactory = getContractJSON('InterchainTokenFactory');
const IInterchainTokenDeployer = getContractJSON('IInterchainTokenDeployer');
const ITokenManager = getContractJSON('ITokenManager');
const IOwnable = getContractJSON('IOwnable');
const { addOptionsToCommands } = require('../common');
const { addEvmOptions } = require('./cli-utils');
const { getSaltFromKey } = require('@axelar-network/axelar-gmp-sdk-solidity/scripts/utils');
const tokenManagerImplementations = {
    INTERCHAIN_TOKEN: 0,
    MINT_BURN_FROM: 1,
    LOCK_UNLOCK: 2,
    LOCK_UNLOCK_FEE: 3,
    MINT_BURN: 4,
};

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
            const [sourceChain, sourceAddress, payload] = args;
            validateParameters({ isNonEmptyString: { sourceChain, sourceAddress } });

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
            const [destinationChain, tokenId, destinationAddress, amount, gasValue] = args;
            const { metadata } = options;
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

            if (balance.lt(adjustedAmount)) {
                throw new Error(`Insufficient balance for transfer. Balance: ${balance}, amount: ${adjustedAmount}`);
            }

            if (
                implementationType !== tokenManagerImplementations.MINT_BURN &&
                implementationType !== tokenManagerImplementations.INTERCHAIN_TOKEN
            ) {
                printInfo('Approving ITS for a transfer for token with token manager type', implementationType);
                await token.approve(interchainTokenService.address, adjustedAmount, gasOptions).then((tx) => tx.wait());
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
            const [tokenAddress] = args;
            const { gasValue } = options;
            validateParameters({ isValidAddress: { tokenAddress }, isValidNumber: { gasValue } });

            const tx = await interchainTokenService.registerTokenMetadata(tokenAddress, gasValue, { value: gasValue, ...gasOptions });
            await handleTx(tx, chain, interchainTokenService, action);
            break;
        }

        case 'set-flow-limits': {
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
            const [trustedChain, trustedAddress] = args;
            const owner = await new Contract(interchainTokenService.address, IOwnable.abi, wallet).owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            validateParameters({ isNonEmptyString: { trustedChain } });

            let trustedChains, trustedAddresses;

            if (options.trustedChain === 'all') {
                trustedChains = parseTrustedChains(config, options.trustedChain);
                trustedAddresses = trustedChains.map((_) => options.trustedAddress || chain.contracts?.InterchainTokenService?.address);
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
            const [pauseStatus] = args;
            const owner = await new Contract(interchainTokenService.address, IOwnable.abi, wallet).owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            const tx = await interchainTokenService.setPauseStatus(pauseStatus === 'true', gasOptions);

            await handleTx(tx, chain, interchainTokenService, action, 'Paused', 'Unpaused');

            break;
        }

        case 'execute': {
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
            const [tokenId] = args;
            validateParameters({ isKeccak256Hash: { tokenId } });

            const tx = await interchainTokenService.migrateInterchainToken(tokenId);

            await handleTx(tx, chain, interchainTokenService, action);

            break;
        }

        case 'transfer-mintership': {
            const [tokenAddress, minter] = args;
            validateParameters({ isValidAddress: { tokenAddress, minter } });

            const token = new Contract(tokenAddress, IMinter.abi, wallet);
            const tx = await token.transferMintership(minter);

            await handleTx(tx, chain, token, action, 'RolesRemoved', 'RolesAdded');

            break;
        }

        case 'link-token': {
            const [tokenId, destinationChain, destinationTokenAddress, type, operator] = args;
            const { gasValue } = options;
            const deploymentSalt = getDeploymentSalt(options);
            const tokenManagerType = tokenManagerImplementations[type];

            validateParameters({
                isString: { destinationChain },
                isValidAddress: { destinationTokenAddress, operator },
                isValidNumber: { gasValue, tokenManagerType },
            });
            isValidDestinationChain(config, destinationChain);

            const interchainTokenId = await interchainTokenService.interchainTokenId(wallet.address, deploymentSalt);
            printInfo('Expected tokenId', interchainTokenId);

            try {
                const tokenManagerAddress = await interchainTokenService.deployedTokenManager(tokenId);
                printInfo(`TokenManager for tokenId ${tokenId} exists on the current chain`, tokenManagerAddress);

                const sourceTokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
                printInfo(`Token address on current chain for tokenId ${tokenId}`, sourceTokenAddress);
            } catch (error) {
                printError(`TokenManager for tokenId ${tokenId} does not yet exist on the current chain.`);
                return;
            }

            if (prompt(`Proceed with linking tokenId ${tokenId} to ${destinationTokenAddress} on chain ${destinationChain}?`, yes)) {
                return;
            }

            const linkParams = operator;

            const tx = await interchainTokenService.linkToken(
                deploymentSalt,
                destinationChain,
                destinationTokenAddress,
                tokenManagerType,
                linkParams,
                gasValue,
                gasOptions,
            );
            await handleTx(tx, chain, interchainTokenService, action, 'LinkTokenStarted');
            break;
        }

        case 'set-trusted-chain': {
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

                await handleTx(tx, chain, interchainTokenService, action, 'TrustedChainSet');
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

    const contractIdCmd = new Command()
        .name('contract-id')
        .description('Get contract ID')
        .action((options) => {
            options.action = 'contract-id';
            options.args = [];
            main(options);
        });

    const tokenManagerAddressCmd = new Command()
        .name('token-manager-address')
        .description('Get token manager address')
        .command('token-manager-address <token-id>')
        .action((tokenId, options) => {
            options.action = 'token-manager-address';
            options.args = [tokenId];
            main(options);
        });

    const interchainTokenAddressCmd = new Command()
        .name('interchain-token-address')
        .description('Get interchain token address')
        .command('interchain-token-address <token-id>')
        .action((tokenId, options) => {
            options.action = 'interchain-token-address';
            options.args = [tokenId];
            main(options);
        });

    const interchainTokenIdCmd = new Command()
        .name('interchain-token-id')
        .description('Get interchain token ID')
        .command('interchain-token-id <sender>')
        .action((sender, options) => {
            options.action = 'interchain-token-id';
            options.args = [sender];
            main(options);
        });

    const tokenManagerImplementationCmd = new Command()
        .name('token-manager-implementation')
        .description('Get token manager implementation address')
        .action((options) => {
            options.action = 'token-manager-implementation';
            options.args = [];
            main(options);
        });

    const flowLimitCmd = new Command()
        .name('flow-limit')
        .description('Get flow limit for token')
        .command('flow-limit <token-id>')
        .action((tokenId, options) => {
            options.action = 'flow-limit';
            options.args = [tokenId];
            main(options);
        });

    const flowOutAmountCmd = new Command()
        .name('flow-out-amount')
        .description('Get flow out amount for token')
        .command('flow-out-amount <token-id>')
        .action((tokenId, options) => {
            options.action = 'flow-out-amount';
            options.args = [tokenId];
            main(options);
        });

    const flowInAmountCmd = new Command()
        .name('flow-in-amount')
        .description('Get flow in amount for token')
        .command('flow-in-amount <token-id>')
        .action((tokenId, options) => {
            options.action = 'flow-in-amount';
            options.args = [tokenId];
            main(options);
        });

    const contractCallValueCmd = new Command()
        .name('contract-call-value')
        .description('Get contract call value')
        .command('contract-call-value <source-chain> <source-address> <payload>')
        .action((sourceChain, sourceAddress, payload, options) => {
            options.action = 'contract-call-value';
            options.args = [sourceChain, sourceAddress, payload];
            main(options);
        });

    const expressExecuteCmd = new Command()
        .name('express-execute')
        .description('Execute express command')
        .command('express-execute <command-id> <source-chain> <source-address> <payload>')
        .action((commandID, sourceChain, sourceAddress, payload, options) => {
            options.action = 'express-execute';
            options.args = [commandID, sourceChain, sourceAddress, payload];
            main(options);
        });

    const interchainTransferCmd = new Command()
        .name('interchain-transfer')
        .description('Perform interchain transfer')
        .command('interchain-transfer <destination-chain> <token-id> <destination-address> <amount> <gas-value>')
        .addOption(new Option('--rawSalt <rawSalt>', 'raw deployment salt').env('RAW_SALT'))
        .addOption(new Option('--metadata <metadata>', 'token transfer metadata').default('0x'))
        .action((destinationChain, tokenId, destinationAddress, amount, gasValue, options) => {
            options.action = 'interchain-transfer';
            options.args = [destinationChain, tokenId, destinationAddress, amount, gasValue];
            main(options);
        });

    const registerTokenMetadataCmd = new Command()
        .name('register-token-metadata')
        .description('Register token metadata')
        .command('register-token-metadata <token-address> ')
        .addOption(new Option('--gasValue <gasValue>', 'gas value').default(0))
        .action((tokenAddress, options) => {
            options.action = 'register-token-metadata';
            options.args = [tokenAddress];
            main(options);
        });

    const setFlowLimitsCmd = new Command()
        .name('set-flow-limits')
        .description('Set flow limits for multiple tokens')
        .command('set-flow-limits <token-ids> <flow-limits>')
        .action((tokenIds, flowLimits, options) => {
            options.action = 'set-flow-limits';
            options.args = [tokenIds, flowLimits];
            main(options);
        });

    const trustedAddressCmd = new Command()
        .name('trusted-address')
        .description('Get trusted address for chain')
        .command('trusted-address <trusted-chain>')
        .action((trustedChain, options) => {
            options.action = 'trusted-address';
            options.args = [trustedChain];
            main(options);
        });

    const setTrustedAddressCmd = new Command()
        .name('set-trusted-address')
        .description('Set trusted address')
        .command('set-trusted-address <trusted-chain> <trusted-address>')
        .action((trustedChain, trustedAddress, options) => {
            options.action = 'set-trusted-address';
            options.args = [trustedChain, trustedAddress];
            main(options);
        });

    const removeTrustedAddressCmd = new Command()
        .name('remove-trusted-address')
        .description('Remove trusted address')
        .command('remove-trusted-address <trusted-chain>')
        .action((trustedChain, options) => {
            options.action = 'remove-trusted-address';
            options.args = [trustedChain];
            main(options);
        });

    const setPauseStatusCmd = new Command()
        .name('set-pause-status')
        .description('Set pause status')
        .command('set-pause-status <pause-status>')
        .action((pauseStatus, options) => {
            options.action = 'set-pause-status';
            options.args = [pauseStatus];
            main(options);
        });

    const executeCmd = new Command()
        .name('execute')
        .description('Execute command')
        .command('execute <command-id> <source-chain> <source-address> <payload>')
        .action((commandID, sourceChain, sourceAddress, payload, options) => {
            options.action = 'execute';
            options.args = [commandID, sourceChain, sourceAddress, payload];
            main(options);
        });

    const checksCmd = new Command()
        .name('checks')
        .description('Perform contract checks')
        .action((options) => {
            options.action = 'checks';
            options.args = [];
            main(options);
        });

    const migrateInterchainTokenCmd = new Command()
        .name('migrate-interchain-token')
        .description('Migrate interchain token')
        .command('migrate-interchain-token <token-id>')
        .action((tokenId, options) => {
            options.action = 'migrate-interchain-token';
            options.args = [tokenId];
            main(options);
        });

    const transferMintershipCmd = new Command()
        .name('transfer-mintership')
        .description('Transfer mintership')
        .command('transfer-mintership <token-address> <minter>')
        .action((tokenAddress, minter, options) => {
            options.action = 'transfer-mintership';
            options.args = [tokenAddress, minter];
            main(options);
        });

    const linkTokenCmd = new Command()
        .name('link-token')
        .description('Link token')
        .command('link-token <token-id> <destination-chain> <destination-token-address> <type> <operator> <gas-value>')
        .addOption(new Option('--rawSalt <rawSalt>', 'raw deployment salt').env('RAW_SALT'))
        .addOption(new Option('--gasValue <gasValue>', 'gas value').default(0))
        .action((tokenId, destinationChain, destinationTokenAddress, type, operator, options) => {
            options.action = 'link-token';
            options.args = [tokenId, destinationChain, destinationTokenAddress, type, operator];
            main(options);
        });

    const setTrustedChainCmd = new Command()
        .name('set-trusted-chain')
        .description('Set trusted chain')
        .command('set-trusted-chain <trusted-chain>')
        .action((trustedChain, options) => {
            options.action = 'set-trusted-chain';
            options.args = [trustedChain];
            main(options);
        });

    program.addCommand(contractIdCmd);
    program.addCommand(tokenManagerAddressCmd);
    program.addCommand(interchainTokenAddressCmd);
    program.addCommand(interchainTokenIdCmd);
    program.addCommand(tokenManagerImplementationCmd);
    program.addCommand(flowLimitCmd);
    program.addCommand(flowOutAmountCmd);
    program.addCommand(flowInAmountCmd);
    program.addCommand(contractCallValueCmd);
    program.addCommand(expressExecuteCmd);
    program.addCommand(interchainTransferCmd);
    program.addCommand(registerTokenMetadataCmd);
    program.addCommand(setFlowLimitsCmd);
    program.addCommand(trustedAddressCmd);
    program.addCommand(setTrustedAddressCmd);
    program.addCommand(removeTrustedAddressCmd);
    program.addCommand(setPauseStatusCmd);
    program.addCommand(executeCmd);
    program.addCommand(checksCmd);
    program.addCommand(migrateInterchainTokenCmd);
    program.addCommand(transferMintershipCmd);
    program.addCommand(linkTokenCmd);
    program.addCommand(setTrustedChainCmd);

    addOptionsToCommands(program, addEvmOptions, { address: true, salt: true });

    program.parse();
}

module.exports = { getDeploymentSalt, handleTx, getTrustedChainsAndAddresses, isValidDestinationChain };
