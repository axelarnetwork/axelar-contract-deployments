'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { hexZeroPad, toUtf8Bytes, keccak256 },
    BigNumber,
    constants: { AddressZero },
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
    itsEdgeContract,
    getChainConfigByAxelarId,
    isConsensusChain,
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
    const { privateKey, address, action, yes } = options;

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

    const tokenId = options.tokenId;

    switch (action) {
        case 'contractId': {
            const contractId = await interchainTokenService.contractId();
            printInfo('InterchainTokenService contract ID', contractId);

            break;
        }

        case 'tokenManagerAddress': {
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

        case 'interchainTokenAddress': {
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

        case 'interchainTokenId': {
            const { sender } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({ isValidAddress: { sender } });

            const interchainTokenId = await interchainTokenService.interchainTokenId(sender, deploymentSalt);
            printInfo(`InterchainTokenId for sender ${sender} and deployment salt: ${deploymentSalt}`, interchainTokenId);

            break;
        }

        case 'tokenManagerImplementation': {
            const tokenManagerImplementation = await interchainTokenService.tokenManager();
            printInfo(`TokenManager implementation address`, tokenManagerImplementation);

            break;
        }

        case 'flowLimit': {
            validateParameters({ isValidTokenId: { tokenId } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const tokenManagerAddress = await interchainTokenService.deployedTokenManager(tokenIdBytes32);

            const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, wallet);

            const flowLimit = await tokenManager.flowLimit();
            printInfo(`Flow limit for TokenManager with tokenId ${tokenId}`, flowLimit);

            break;
        }

        case 'flowOutAmount': {
            validateParameters({ isValidTokenId: { tokenId } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const tokenManagerAddress = await interchainTokenService.deployedTokenManager(tokenIdBytes32);

            const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, wallet);

            const flowOutAmount = await tokenManager.flowOutAmount();
            printInfo(`Flow out amount for TokenManager with tokenId ${tokenId}`, flowOutAmount);

            break;
        }

        case 'flowInAmount': {
            validateParameters({ isValidTokenId: { tokenId } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const tokenManagerAddress = await interchainTokenService.deployedTokenManager(tokenIdBytes32);

            const tokenManager = new Contract(tokenManagerAddress, ITokenManager.abi, wallet);

            const flowInAmount = await tokenManager.flowInAmount();
            printInfo(`Flow in amount for TokenManager with tokenId ${tokenId}`, flowInAmount);

            break;
        }

        case 'contractCallValue': {
            const { sourceChain, sourceAddress, payload } = options;

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

        case 'expressExecute': {
            const { commandID, sourceChain, sourceAddress, payload } = options;

            validateParameters({
                isKeccak256Hash: { commandID },
                isNonEmptyString: { sourceChain, sourceAddress },
                isValidCalldata: { payload },
            });

            const tx = await interchainTokenService.expressExecute(commandID, sourceChain, sourceAddress, payload, gasOptions);

            await handleTx(tx, chain, interchainTokenService, options.action, 'ExpressExecuted');

            break;
        }

        case 'interchainTransfer': {
            const { destinationChain, destinationAddress, metadata, gasValue } = options;
            let amount = options.amount;

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
            amount = BigNumber.from(amount).mul(BigNumber.from(10).pow(decimals));
            const balance = await token.balanceOf(wallet.address);

            if (balance.lt(amount)) {
                throw new Error(`Insufficient balance for transfer. Balance: ${balance}, amount: ${amount}`);
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
                amount,
                metadata,
                gasValue,
                { value: gasValue, ...gasOptions },
            );

            await handleTx(tx, chain, interchainTokenService, options.action, 'InterchainTransfer', 'InterchainTransferWithData');

            break;
        }

        case 'registerTokenMetadata': {
            const { tokenAddress, gasValue } = options;

            validateParameters({ isValidAddress: { tokenAddress }, isValidNumber: { gasValue } });

            const tx = await interchainTokenService.registerTokenMetadata(tokenAddress, gasValue, { value: gasValue, ...gasOptions });

            await handleTx(tx, chain, interchainTokenService, options.action);

            break;
        }

        case 'setFlowLimits': {
            const tokenIds = options.tokenIds.split(',');
            const flowLimitsStrings = options.flowLimits.split(',');
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

            await handleTx(tx, chain, tokenManagers[0], options.action, 'FlowLimitSet');

            break;
        }

        case 'trustedAddress': {
            const trustedChain = options.trustedChain;

            validateParameters({ isNonEmptyString: { trustedChain } });

            const trustedAddress = await interchainTokenService.trustedAddress(trustedChain);

            if (trustedAddress) {
                printInfo(`Trusted address for chain ${trustedChain}`, trustedAddress);
            } else {
                printWarn(`No trusted address for chain ${trustedChain}`);
            }

            break;
        }

        case 'setTrustedAddress': {
            const owner = await new Contract(interchainTokenService.address, IOwnable.abi, wallet).owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            validateParameters({ isNonEmptyString: { trustedChain: options.trustedChain } });

            let trustedChains, trustedAddresses;

            if (options.trustedChain === 'all') {
                const itsChains = Object.values(config.chains).filter((chain) => chain.contracts?.InterchainTokenService?.skip !== true);
                trustedChains = itsChains.map((chain) => chain.axelarId);
                trustedAddresses = itsChains.map((_) => options.trustedAddress || chain.contracts?.InterchainTokenService?.address);
            } else {
                const trustedChain =
                    getChainConfig(config, options.trustedChain.toLowerCase(), { skipCheck: true })?.axelarId ||
                    options.trustedChain.toLowerCase();
                const trustedAddress =
                    options.trustedAddress ||
                    getChainConfig(config, options.trustedChain.toLowerCase())?.contracts?.InterchainTokenService?.address;

                if (trustedChain === undefined || trustedAddress === undefined) {
                    throw new Error(`Invalid chain/address: ${options.trustedChain}`);
                }

                trustedChains = [trustedChain];
                trustedAddresses = [trustedAddress];
            }

            if (prompt(`Proceed with setting trusted address for chain ${trustedChains} to ${trustedAddresses}?`, options.yes)) {
                return;
            }

            for (const [trustedChain, trustedAddress] of trustedChains.map((chain, index) => [chain, trustedAddresses[index]])) {
                const tx = await interchainTokenService.setTrustedAddress(trustedChain, trustedAddress, gasOptions);

                await handleTx(tx, chain, interchainTokenService, options.action, 'TrustedAddressSet');
            }

            break;
        }

        case 'removeTrustedAddress': {
            const owner = await new Contract(interchainTokenService.address, IOwnable.abi, wallet).owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            let trustedChains;

            if (options.trustedChain === 'all') {
                [trustedChains] = await getTrustedChainsAndAddresses(config, interchainTokenService);
            } else {
                const trustedChain = config.chains[options.trustedChain.toLowerCase()]?.axelarId;

                if (trustedChain === undefined) {
                    throw new Error(`Invalid chain: ${options.trustedChain}`);
                }

                if ((await interchainTokenService.trustedAddress(options.trustedChain)) === '') {
                    printError(`No trusted address for chain ${options.trustedChain}`);
                    return;
                }

                trustedChains = [trustedChain];
            }

            printInfo(`Removing trusted address for chains ${trustedChains}`);

            for (const trustedChain of trustedChains) {
                const tx = await interchainTokenService.removeTrustedAddress(trustedChain, gasOptions);

                await handleTx(tx, chain, interchainTokenService, options.action, 'TrustedAddressRemoved');
            }

            break;
        }

        case 'setPauseStatus': {
            const owner = await new Contract(interchainTokenService.address, IOwnable.abi, wallet).owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            const pauseStatus = options.pauseStatus === 'true';

            const tx = await interchainTokenService.setPauseStatus(pauseStatus, gasOptions);

            await handleTx(tx, chain, interchainTokenService, options.action, 'Paused', 'Unpaused');

            break;
        }

        case 'execute': {
            const { commandID, sourceChain, sourceAddress, payload } = options;

            validateParameters({ isKeccak256Hash: { commandID }, isNonEmptyString: { sourceChain, sourceAddress } });

            const isTrustedAddress = await interchainTokenService.isTrustedAddress(sourceChain, sourceAddress);

            if (!isTrustedAddress) {
                throw new Error('Invalid remote service.');
            }

            validateParameters({ isValidCalldata: { payload } });

            const tx = await interchainTokenService.execute(commandID, sourceChain, sourceAddress, payload, gasOptions);

            await handleTx(tx, chain, interchainTokenService, options.action);

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

            // TODO: simplify ITS trusted address checks
            const [trustedChains, trustedAddresses] = await getTrustedChainsAndAddresses(config, interchainTokenService);

            printInfo('Trusted chains', trustedChains);
            printInfo('Trusted addresses', trustedAddresses);

            for (let i = 0; i < trustedAddresses.length; i++) {
                const trustedAddress = trustedAddresses[i];
                const trustedChain = trustedChains[i];
                const chainConfig = getChainConfigByAxelarId(config, trustedChain);

                if ((isConsensusChain(chain) && isConsensusChain(chainConfig)) || chainConfig.axelarId === config.axelar.axelarId) {
                    if (trustedAddress !== itsEdgeContract(chainConfig)) {
                        printError(
                            `Error: Trusted address on ${chain.name}'s ITS contract for ${trustedChain} is ${trustedAddress} which does not match ITS address from the config ${interchainTokenServiceAddress}`,
                        );
                    }
                } else if (trustedAddress !== 'hub') {
                    printError(
                        `Error: Trusted address on ${chain.name}'s ITS contract for ${trustedChain} is ${trustedAddress} which does not match "hub"`,
                    );
                }
            }

            const chainNameHash = await interchainTokenService.chainNameHash();
            const configChainNameHash = keccak256(toUtf8Bytes(chain.axelarId));

            compare(await interchainTokenService.gateway(), chain.contracts.AxelarGateway?.address, 'AxelarGateway');
            compare(await interchainTokenService.gasService(), chain.contracts.AxelarGasService?.address, 'AxelarGasService');
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

        case 'migrateInterchainToken': {
            const { tokenId } = options;

            validateParameters({ isKeccak256Hash: { tokenId } });

            const tx = await interchainTokenService.migrateInterchainToken(tokenId);

            await handleTx(tx, chain, interchainTokenService, options.action);

            break;
        }

        case 'transferMintership': {
            const { tokenAddress, minter } = options;

            validateParameters({ isValidAddress: { tokenAddress, minter } });

            const token = new Contract(tokenAddress, IMinter.abi, wallet);
            const tx = await token.transferMintership(minter);

            await handleTx(tx, chain, token, options.action, 'RolesRemoved', 'RolesAdded');

            break;
        }

        case 'linkToken': {
            const { destinationChain, type, operator, destinationTokenAddress, gasValue } = options;

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

            if (
                prompt(`Proceed with linking tokenId ${tokenId} to ${destinationTokenAddress} on chain ${destinationChain}?`, options.yes)
            ) {
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

            await handleTx(tx, chain, interchainTokenService, options.action, 'LinkTokenStarted');

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

    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').default('InterchainTokenService'));
    program.addOption(
        new Option('--action <action>', 'ITS action')
            .choices([
                'contractId',
                'tokenManagerAddress',
                'tokenAddress',
                'interchainTokenAddress',
                'interchainTokenId',
                'tokenManagerImplementation',
                'flowLimit',
                'flowOutAmount',
                'flowInAmount',
                'contractCallValue',
                'expressExecute',
                'interchainTransfer',
                'setFlowLimits',
                'trustedAddress',
                'setTrustedAddress',
                'removeTrustedAddress',
                'setPauseStatus',
                'execute',
                'checks',
                'migrateInterchainToken',
                'registerTokenMetadata',
                'transferMintership',
                'linkToken',
            ])
            .makeOptionMandatory(true),
    );

    program.addOption(new Option('--commandID <commandID>', 'execute command ID'));
    program.addOption(new Option('--tokenId <tokenId>', 'ID of the token'));
    program.addOption(new Option('--sender <sender>', 'TokenManager deployer address'));
    program.addOption(
        new Option('--type <type>', 'TokenManager implementation type').choices([
            'MINT_BURN',
            'MINT_BURN_FROM',
            'LOCK_UNLOCK',
            'LOCK_UNLOCK_FEE',
        ]),
    );
    program.addOption(new Option('--destinationChain <destinationChain>', 'destination chain'));
    program.addOption(new Option('--destinationAddress <destinationAddress>', 'destination address'));
    program.addOption(new Option('--params <params>', 'params for TokenManager deployment'));
    program.addOption(new Option('--tokenAddress <tokenAddress>', 'token address to use for token manager deployment'));
    program.addOption(
        new Option(
            '--destinationTokenAddress <destinationTokenAddress>',
            'token address on the destination chain to link with the token on the source chain corresponding to the tokenId',
        ),
    );
    program.addOption(new Option('--operator <operator>', 'operator address to use for token manager'));
    program.addOption(new Option('--gasValue <gasValue>', 'gas value').default(0));
    program.addOption(new Option('--name <name>', 'token name'));
    program.addOption(new Option('--symbol <symbol>', 'token symbol'));
    program.addOption(new Option('--decimals <decimals>', 'token decimals'));
    program.addOption(new Option('--minter <minter>', 'token minter').default(AddressZero));
    program.addOption(new Option('--sourceChain <sourceChain>', 'source chain'));
    program.addOption(new Option('--sourceAddress <sourceAddress>', 'source address'));
    program.addOption(new Option('--payload <payload>', 'payload'));
    program.addOption(new Option('--amount <amount>', 'token amount, in terms of symbol'));
    program.addOption(new Option('--metadata <metadata>', 'token transfer metadata').default('0x'));
    program.addOption(new Option('--data <data>', 'token transfer data'));
    program.addOption(new Option('--tokenIds <tokenIds>', 'tokenId array'));
    program.addOption(new Option('--flowLimits <flowLimits>', 'flow limit array'));
    program.addOption(new Option('--trustedChain <trustedChain>', 'chain name for trusted addresses'));
    program.addOption(new Option('--trustedAddress <trustedAddress>', 'trusted address'));
    program.addOption(new Option('--pauseStatus <pauseStatus>', 'pause status').choices(['true', 'false']));
    program.addOption(new Option('--rawSalt <rawSalt>', 'raw deployment salt').env('RAW_SALT'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

module.exports = { getDeploymentSalt, handleTx, getTrustedChainsAndAddresses, isValidDestinationChain };
