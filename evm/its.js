'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { hexZeroPad, toUtf8Bytes, keccak256, parseUnits, formatUnits },
    BigNumber,
    Contract,
} = ethers;
const { Command, Option, Argument } = require('commander');
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
    encodeITSDestination,
    printTokenInfo,
    INTERCHAIN_TRANSFER_WITH_METADATA,
    isTrustedChain,
    loadConfig,
    scaleGasValue,
} = require('./utils');
const {
    getChainConfigByAxelarId,
    validateDestinationChain,
    validateChain,
    tokenManagerTypes,
    validateLinkType,
    calculateItsCrossChainGas,
} = require('../common/utils');
const { getWallet } = require('./sign-utils');
const IInterchainTokenService = getContractJSON('IInterchainTokenService');
const IMinter = getContractJSON('IMinter');
const InterchainTokenService = getContractJSON('InterchainTokenService');
const InterchainTokenFactory = getContractJSON('InterchainTokenFactory');
const IInterchainTokenDeployer = getContractJSON('IInterchainTokenDeployer');
const ITokenManager = getContractJSON('ITokenManager');
const { addOptionsToCommands } = require('../common');
const { addEvmOptions } = require('./cli-utils');
const { getSaltFromKey } = require('@axelar-network/axelar-gmp-sdk-solidity/scripts/utils');

const IInterchainTokenServiceV211 = getContractJSON(
    'IInterchainTokenService',
    '@axelar-network/interchain-token-service-v2.1.1/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json',
);

function createInterchainTokenServiceContract(address, wallet, version) {
    if (version === '2.1.1') {
        return new Contract(address, IInterchainTokenServiceV211.abi, wallet);
    } else {
        return new Contract(address, IInterchainTokenService.abi, wallet);
    }
}

async function validateOwner(contract, walletAddress, action) {
    const owner = await contract.owner();
    if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
        throw new Error(`${action} can only be performed by contract owner: ${owner}`);
    }
}

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

async function getTrustedChains(chains, interchainTokenService, version) {
    const chainIds = Object.values(chains)
        .filter((chain) => chain.contracts.InterchainTokenService !== undefined)
        .map((chain) => chain.axelarId);

    const trustedChains = [];

    for (const chain of chainIds) {
        if (await isTrustedChain(chain, interchainTokenService, version)) {
            trustedChains.push(chain);
        }
    }

    return trustedChains;
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
    for (const [key, value] of Object.entries(toCheck).filter(([_key, value]) => typeof value !== 'undefined')) {
        if (contractConfig[key]) {
            const configValue = contractConfig[key];
            compare(value, configValue, key);
        } else {
            printWarn(`Warning: The key '${key}' is not found in the contract config for ${contractName}.`);
        }
    }
}

async function processCommand(_axelar, chain, chains, action, options) {
    const { privateKey, address, yes, args } = options;

    const config = loadConfig(options.env);
    const contracts = chain.contracts;
    const contractName = 'InterchainTokenService';

    const interchainTokenServiceAddress = address || contracts.InterchainTokenService?.address;

    const itsVersion = contracts.InterchainTokenService?.version;

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

    const interchainTokenService = createInterchainTokenServiceContract(interchainTokenServiceAddress, wallet, itsVersion);

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

            // Check if interchainTokenAddress function exists (predictable token address)
            const predictableAddress = 'interchainTokenAddress' in interchainTokenService;

            if (predictableAddress) {
                const interchainTokenAddress = await interchainTokenService.interchainTokenAddress(tokenIdBytes32);
                printInfo(`InterchainToken address for tokenId: ${tokenId}`, interchainTokenAddress);
            }

            try {
                const interchainTokenAddress = await interchainTokenService.registeredTokenAddress(tokenIdBytes32);
                printInfo(`Token for tokenId: ${tokenId} exists at address:`, interchainTokenAddress);
                return interchainTokenAddress;
            } catch (error) {
                printInfo(`Token for tokenId: ${tokenId} does not yet exist.`);
            }

            return;
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

            if (!(await isTrustedChain(sourceChain, interchainTokenService, itsVersion))) {
                throw new Error(`Invalid remote service: ${sourceChain} is not a trusted chain.`);
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
            const [destinationChain, tokenId, destinationAddress, amount] = args;
            let { gasValue, metadata, env } = options;
            validateParameters({
                isValidTokenId: { tokenId },
                isNonEmptyString: { destinationChain, destinationAddress },
                isValidNumber: { amount, gasValue },
                isValidCalldata: { metadata },
            });

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

            await printTokenInfo(await interchainTokenService.registeredTokenAddress(tokenIdBytes32), provider);

            const implementationType = (await tokenManager.implementationType()).toNumber();
            const decimals = await token.decimals();
            const amountInUnits = parseUnits(amount, decimals);
            const balance = await token.balanceOf(wallet.address);

            if (balance.lt(amountInUnits)) {
                throw new Error(`Insufficient balance for transfer. Balance: ${balance}, amount: ${amountInUnits}`);
            }

            if (implementationType !== tokenManagerTypes.MINT_BURN && implementationType !== tokenManagerTypes.NATIVE_INTERCHAIN_TOKEN) {
                printInfo('Approving ITS for a transfer for token with token manager type', implementationType);
                await token.approve(interchainTokenService.address, amountInUnits, gasOptions).then((tx) => tx.wait());
            }

            const itsDestinationAddress = encodeITSDestination(chains, destinationChain, destinationAddress);
            printInfo('Human-readable destination address', destinationAddress);

            if (gasValue === 0) {
                gasValue = await calculateItsCrossChainGas(chain.axelarId, destinationChain, env, 'InterchainTransfer');
            }
            const tx = await interchainTokenService[INTERCHAIN_TRANSFER_WITH_METADATA](
                tokenIdBytes32,
                destinationChain,
                itsDestinationAddress,
                amountInUnits,
                metadata,
                gasValue,
                { value: scaleGasValue(chain, gasValue), ...gasOptions },
            );
            await handleTx(tx, chain, interchainTokenService, action, 'InterchainTransfer');
            return tx.hash;
        }

        case 'register-token-metadata': {
            const [tokenAddress] = args;
            let { gasValue, env } = options;
            validateParameters({ isValidAddress: { tokenAddress }, isValidNumber: { gasValue } });

            if (gasValue === 0) {
                gasValue = await calculateItsCrossChainGas(chain.axelarId, 'axelar', env, 'TokenMetadataRegistered');
            }

            const tx = await interchainTokenService.registerTokenMetadata(tokenAddress, gasValue, {
                value: scaleGasValue(chain, gasValue),
                ...gasOptions,
            });
            await handleTx(tx, chain, interchainTokenService, action);
            break;
        }

        case 'set-flow-limits': {
            const [tokenIdsArg, flowLimitsArg] = args;
            const flowLimitsStrings = flowLimitsArg.split(' ');
            const tokenIds = tokenIdsArg.split(' ');
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

        case 'is-trusted-chain': {
            const [itsChain] = args;

            validateParameters({ isNonEmptyString: { itsChain } });

            if (await isTrustedChain(itsChain, interchainTokenService, itsVersion)) {
                printInfo(`${itsChain} is a trusted chain`);
            } else {
                printInfo(`${itsChain} is not a trusted chain`);
            }

            break;
        }

        case 'set-trusted-chains': {
            const trustedChains = args;

            await validateOwner(interchainTokenService, walletAddress, action);

            if (prompt(`Proceed with setting trusted chain(s): ${Array.from(trustedChains).join(', ')}?`, yes)) {
                return;
            }

            const data = [];
            for (const trustedChain of trustedChains) {
                if (itsVersion === '2.1.1') {
                    const tx = await interchainTokenService.populateTransaction.setTrustedAddress(trustedChain, 'hub', gasOptions);
                    data.push(tx.data);
                } else {
                    const tx = await interchainTokenService.populateTransaction.setTrustedChain(trustedChain, gasOptions);
                    data.push(tx.data);
                }
            }

            const multicall = await interchainTokenService.multicall(data);
            await handleTx(multicall, chain, interchainTokenService, action, 'TrustedAddressSet', 'TrustedChainSet');

            break;
        }

        case 'remove-trusted-chains': {
            const trustedChains = args;

            await validateOwner(interchainTokenService, walletAddress, action);

            if (prompt(`Proceed with removing trusted chain(s): ${Array.from(trustedChains).join(', ')}?`, yes)) {
                return;
            }

            const data = [];
            for (const trustedChain of trustedChains) {
                if (itsVersion === '2.1.1') {
                    const tx = await interchainTokenService.populateTransaction.removeTrustedAddress(trustedChain, gasOptions);
                    data.push(tx.data);
                } else {
                    const tx = await interchainTokenService.populateTransaction.removeTrustedChain(trustedChain, gasOptions);
                    data.push(tx.data);
                }
            }

            const multicall = await interchainTokenService.multicall(data);
            await handleTx(multicall, chain, interchainTokenService, action, 'TrustedAddressRemoved', 'TrustedChainRemoved');

            break;
        }

        case 'set-pause-status': {
            const [pauseStatus] = args;

            await validateOwner(interchainTokenService, walletAddress, action);

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

            if (!(await isTrustedChain(sourceChain, interchainTokenService, itsVersion))) {
                throw new Error(`Invalid remote service: ${sourceChain} is not a trusted chain.`);
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

            // Note: only get `interchainToken` if the contract supports it
            let interchainToken;
            if ('implementationAddress' in interchainTokenDeployerContract) {
                try {
                    interchainToken = await interchainTokenDeployerContract.implementationAddress();
                } catch (error) {
                    printWarn(`Warning: implementationAddress() method not implemented in deployed contract at ${interchainTokenDeployer}`);
                    interchainToken = undefined;
                }
            } else {
                interchainToken = undefined;
            }

            const trustedChains = await getTrustedChains(chains, interchainTokenService, itsVersion);
            printInfo('Trusted chains', trustedChains);

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

        case 'mint-token': {
            const [tokenId, to, amount] = args;
            validateParameters({ isValidTokenId: { tokenId }, isValidAddress: { to }, isValidNumber: { amount } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            // Get token manager address
            const tokenManagerAddress = await interchainTokenService.deployedTokenManager(tokenIdBytes32);
            printInfo(`TokenManager address for tokenId: ${tokenId}`, tokenManagerAddress);

            // Get token address
            const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenIdBytes32);
            printInfo(`Token address for tokenId: ${tokenId}`, tokenAddress);

            const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, wallet);

            const amountInUnits = ethers.BigNumber.from(amount.toString());

            if (prompt(`Proceed with minting ${amount} to ${to}?`, yes)) {
                return;
            }

            // Execute mint
            const tx = await tokenManager.mintToken(tokenAddress, to, amountInUnits, gasOptions);
            await handleTx(tx, chain, tokenManager, action);

            break;
        }

        case 'approve': {
            const [tokenId, spender, amount] = args;
            validateParameters({ isValidTokenId: { tokenId }, isValidAddress: { spender }, isValidNumber: { amount } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            // Get token address
            const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenIdBytes32);
            printInfo(`Token address for tokenId: ${tokenId}`, tokenAddress);

            // Create token contract instance
            const token = new Contract(tokenAddress, getContractJSON('InterchainToken').abi, wallet);

            const amountInUnits = ethers.BigNumber.from(amount.toString());
            printInfo(`Approving ${spender} to spend ${amount} of token ${tokenId}`);

            if (prompt(`Proceed with approving ${spender} to spend ${amount}?`, yes)) {
                return;
            }

            // Execute approval
            const tx = await token.approve(spender, amountInUnits, gasOptions);
            await handleTx(tx, chain, token, action, 'Approval');

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
            let { gasValue, env } = options;
            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isValidTokenId: { tokenId },
                isNonEmptyString: { destinationChain, type },
                isValidAddress: { destinationTokenAddress, operator },
                isValidNumber: { gasValue },
            });
            validateChain(chains, destinationChain);

            const chainType = getChainConfigByAxelarId(config, destinationChain)?.chainType;
            const tokenManagerType = validateLinkType(chainType, type);
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

            if (gasValue === 0) {
                gasValue = await calculateItsCrossChainGas(chain.axelarId, destinationChain, env, 'LinkToken');
            }

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

        default: {
            throw new Error(`Unknown action ${action}`);
        }
    }
}

async function main(action, args, options) {
    options.args = args;
    return mainProcessor(options, (axelar, chain, chains, options) => processCommand(axelar, chain, chains, action, options));
}

if (require.main === module) {
    const program = new Command();
    program.name('ITS').description('Script to perform ITS commands');

    program
        .command('contract-id')
        .description('Get contract ID')
        .action((options, cmd) => {
            main(cmd.name(), [], options);
        });

    program
        .command('token-manager-address')
        .description('Get token manager address')
        .argument('<token-id>', 'Token ID')
        .action((tokenId, options, cmd) => {
            main(cmd.name(), [tokenId], options);
        });

    program
        .command('interchain-token-address')
        .description('Get interchain token address')
        .argument('<token-id>', 'Token ID')
        .action((tokenId, options, cmd) => {
            main(cmd.name(), [tokenId], options);
        });

    program
        .command('interchain-token-id')
        .description('Get interchain token ID')
        .argument('<sender>', 'Sender address')
        .action((sender, options, cmd) => {
            main(cmd.name(), [sender], options);
        });

    program
        .command('token-manager-implementation')
        .description('Get token manager implementation address')
        .action((options, cmd) => {
            main(cmd.name(), [], options);
        });

    program
        .command('flow-limit')
        .description('Get flow limit for token')
        .argument('<token-id>', 'Token ID')
        .action((tokenId, options, cmd) => {
            main(cmd.name(), [tokenId], options);
        });

    program
        .command('flow-out-amount')
        .description('Get flow out amount for token')
        .argument('<token-id>', 'Token ID')
        .action((tokenId, options, cmd) => {
            main(cmd.name(), [tokenId], options);
        });

    program
        .command('flow-in-amount')
        .description('Get flow in amount for token')
        .argument('<token-id>', 'Token ID')
        .action((tokenId, options, cmd) => {
            main(cmd.name(), [tokenId], options);
        });

    program
        .command('contract-call-value')
        .description('Get contract call value')
        .argument('<source-chain>', 'Source chain')
        .argument('<source-address>', 'Source address')
        .argument('<payload>', 'Payload')
        .action((sourceChain, sourceAddress, payload, options, cmd) => {
            main(cmd.name(), [sourceChain, sourceAddress, payload], options);
        });

    program
        .command('express-execute')
        .description('Execute express command')
        .argument('<command-id>', 'Command ID')
        .argument('<source-chain>', 'Source chain')
        .argument('<source-address>', 'Source address')
        .argument('<payload>', 'Payload')
        .action((commandID, sourceChain, sourceAddress, payload, options, cmd) => {
            main(cmd.name(), [commandID, sourceChain, sourceAddress, payload], options);
        });

    program
        .command('interchain-transfer')
        .description('Perform interchain transfer')
        .argument('<destination-chain>', 'Destination chain')
        .argument('<token-id>', 'Token ID')
        .argument('<destination-address>', 'Destination address')
        .argument('<amount>', 'Amount')
        .addOption(new Option('--rawSalt <rawSalt>', 'raw deployment salt').env('RAW_SALT'))
        .addOption(new Option('--metadata <metadata>', 'token transfer metadata').default('0x'))
        .addOption(new Option('--gasValue <gasValue>', 'gas value').default(0))
        .action((destinationChain, tokenId, destinationAddress, amount, options, cmd) => {
            main(cmd.name(), [destinationChain, tokenId, destinationAddress, amount], options);
        });

    program
        .command('register-token-metadata')
        .description('Register token metadata')
        .argument('<token-address>', 'Token address')
        .addOption(new Option('--gasValue <gasValue>', 'gas value').default(0))
        .action((tokenAddress, options, cmd) => {
            main(cmd.name(), [tokenAddress], options);
        });

    program
        .command('set-flow-limits')
        .description('Set flow limits for multiple tokens')
        .argument('<token-ids>', 'Comma-separated token IDs')
        .argument('<flow-limits>', 'Comma-separated flow limits')
        .action((tokenIds, flowLimits, options, cmd) => {
            main(cmd.name(), [tokenIds, flowLimits], options);
        });

    program
        .command('is-trusted-chain')
        .description('Is trusted chain')
        .argument('<its-chain>', 'ITS chain')
        .action((itsChain, options, cmd) => {
            main(cmd.name(), [itsChain], options);
        });

    program
        .command('set-trusted-chains')
        .description('Set trusted chains')
        .argument('<chains...>', 'Chains to trust')
        .action((chains, options, cmd) => {
            main(cmd.name(), chains, options);
        });

    program
        .command('remove-trusted-chains')
        .description('Remove trusted chains')
        .argument('<chains...>', 'Chains to not trust')
        .action((chains, options, cmd) => {
            main(cmd.name(), chains, options);
        });

    program
        .command('set-pause-status')
        .description('Set pause status')
        .argument('<pause-status>', 'Pause status (true/false)')
        .action((pauseStatus, options, cmd) => {
            main(cmd.name(), [pauseStatus], options);
        });

    program
        .command('execute')
        .description('Execute command')
        .argument('<command-id>', 'Command ID')
        .argument('<source-chain>', 'Source chain')
        .argument('<source-address>', 'Source address')
        .argument('<payload>', 'Payload')
        .action((commandID, sourceChain, sourceAddress, payload, options, cmd) => {
            main(cmd.name(), [commandID, sourceChain, sourceAddress, payload], options);
        });

    program
        .command('checks')
        .description('Perform contract checks')
        .action((options, cmd) => {
            main(cmd.name(), [], options);
        });

    program
        .command('migrate-interchain-token')
        .description('Migrate interchain token')
        .argument('<token-id>', 'Token ID')
        .action((tokenId, options, cmd) => {
            main(cmd.name(), [tokenId], options);
        });

    program
        .command('mint-token')
        .description('Mint tokens using token manager')
        .argument('<token-id>', 'Token ID')
        .argument('<to>', 'Recipient address')
        .argument('<amount>', 'Amount to mint')
        .action((tokenId, to, amount, options, cmd) => {
            main(cmd.name(), [tokenId, to, amount], options);
        });

    program
        .command('approve')
        .description('Approve spender to spend tokens')
        .argument('<token-id>', 'Token ID')
        .argument('<spender>', 'Spender address')
        .argument('<amount>', 'Amount to approve (in wei)')
        .action((tokenId, spender, amount, options, cmd) => {
            main(cmd.name(), [tokenId, spender, amount], options);
        });

    program
        .command('transfer-mintership')
        .description('Transfer mintership')
        .argument('<token-address>', 'Token address')
        .argument('<minter>', 'Minter address')
        .action((tokenAddress, minter, options, cmd) => {
            main(cmd.name(), [tokenAddress, minter], options);
        });

    program
        .command('link-token')
        .description('Link token')
        .argument('<token-id>', 'Token ID')
        .argument('<destination-chain>', 'Destination chain')
        .argument('<destination-token-address>', 'Destination token address')
        .addArgument(new Argument('<type>', 'Token manager type').choices(Object.keys(tokenManagerTypes)))
        .argument('<operator>', 'Operator address')
        .addOption(new Option('--rawSalt <rawSalt>', 'raw deployment salt').env('RAW_SALT'))
        .addOption(new Option('--gasValue <gasValue>', 'gas value, will default to estimate gas fee if none provided').default(0))
        .action((tokenId, destinationChain, destinationTokenAddress, type, operator, options, cmd) => {
            main(cmd.name(), [tokenId, destinationChain, destinationTokenAddress, type, operator], options);
        });

    addOptionsToCommands(program, addEvmOptions, { address: true, salt: true });

    program.parse();
}

module.exports = { its: main, getDeploymentSalt, handleTx, getTrustedChains };
