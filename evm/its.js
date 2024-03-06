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
} = require('./utils');
const { getWallet } = require('./sign-utils');
const IInterchainTokenService = getContractJSON('IInterchainTokenService');
const InterchainTokenService = getContractJSON('InterchainTokenService');
const InterchainTokenFactory = getContractJSON('InterchainTokenFactory');
const IInterchainTokenDeployer = getContractJSON('IInterchainTokenDeployer');
const IOwnable = getContractJSON('IOwnable');
const { addExtendedOptions } = require('./cli-utils');
const { getSaltFromKey } = require('@axelar-network/axelar-gmp-sdk-solidity/scripts/utils');
const tokenManagerImplementations = {
    MINT_BURN: 0,
    MINT_BURN_FROM: 1,
    LOCK_UNLOCK: 2,
    LOCK_UNLOCK_FEE: 3,
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
                await interchainTokenService.validTokenManagerAddress(tokenIdBytes32);
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
                await interchainTokenService.validTokenAddress(tokenIdBytes32);
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

            const flowLimit = await interchainTokenService.flowLimit(tokenIdBytes32);
            printInfo(`Flow limit for TokenManager with tokenId ${tokenId}`, flowLimit);

            break;
        }

        case 'flowOutAmount': {
            validateParameters({ isValidTokenId: { tokenId } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const flowOutAmount = await interchainTokenService.flowOutAmount(tokenIdBytes32);
            printInfo(`Flow out amount for TokenManager with tokenId ${tokenId}`, flowOutAmount);

            break;
        }

        case 'flowInAmount': {
            validateParameters({ isValidTokenId: { tokenId } });

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const flowInAmount = await interchainTokenService.flowInAmount(tokenIdBytes32);
            printInfo(`Flow out amount for TokenManager with tokenId ${tokenId}`, flowInAmount);

            break;
        }

        case 'deployTokenManager': {
            const { destinationChain, type, params, gasValue } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isString: { destinationChain },
                isValidCalldata: { params },
                isValidNumber: { gasValue },
            });

            isValidDestinationChain(config, destinationChain);

            const tx = await interchainTokenService.deployTokenManager(
                deploymentSalt,
                destinationChain,
                tokenManagerImplementations[type],
                params,
                gasValue,
                gasOptions,
            );

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'TokenManagerDeploymentStarted');

            break;
        }

        case 'deployInterchainToken': {
            const { destinationChain, name, symbol, decimals, minter, gasValue } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isNonEmptyString: { name, symbol },
                isString: { destinationChain },
                isAddress: { minter },
                isValidNumber: { decimals, gasValue },
            });

            isValidDestinationChain(config, destinationChain);

            const tx = await interchainTokenService.deployInterchainToken(
                deploymentSalt,
                destinationChain,
                name,
                symbol,
                decimals,
                minter,
                gasValue,
                { value: gasValue, ...gasOptions },
            );

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

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

            isValidDestinationChain(config, destinationChain);

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const tokenManager = new Contract(
                await interchainTokenService.validTokenManagerAddress(tokenIdBytes32),
                getContractJSON('ITokenManager').abi,
                wallet,
            );
            const token = new Contract(
                await interchainTokenService.validTokenAddress(tokenIdBytes32),
                getContractJSON('InterchainToken').abi,
                wallet,
            );

            const implementationType = await tokenManager.implementationType();
            const decimals = await token.decimals();
            amount = BigNumber.from(amount).mul(BigNumber.from(10).pow(decimals));
            const balance = await token.balanceOf(wallet.address);

            if (balance.lt(amount)) {
                throw new Error(`Insufficient balance for transfer. Balance: ${balance}, amount: ${amount}`);
            }

            if (implementationType !== tokenManagerImplementations.MINT_BURN) {
                printInfo('Approving ITS for a transfer');
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

        case 'callContractWithInterchainToken': {
            const { destinationChain, destinationAddress, amount, data, gasValue } = options;

            validateParameters({
                isValidTokenId: { tokenId },
                isNonEmptyString: { destinationChain, destinationAddress },
                isValidNumber: { amount, gasValue },
                isValidCalldata: { data },
            });

            isValidDestinationChain(config, destinationChain);

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const tx = await interchainTokenService.callContractWithInterchainToken(
                tokenIdBytes32,
                destinationChain,
                destinationAddress,
                amount,
                data,
                gasValue,
                { value: gasValue, ...gasOptions },
            );

            await handleTx(tx, chain, interchainTokenService, options.action, 'InterchainTransfer', 'InterchainTransferWithData');

            break;
        }

        case 'setFlowLimits': {
            const { tokenIds, flowLimits } = options;
            const tokenIdsBytes32 = [];

            for (const tokenId of tokenIds) {
                if (!isValidTokenId(tokenId)) {
                    throw new Error(`Invalid tokenId value: ${tokenId}`);
                }

                const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);
                tokenIdsBytes32.push(tokenIdBytes32);
            }

            validateParameters({ isNumberArray: { flowLimits } });

            const tx = await interchainTokenService.setFlowLimits(tokenIdsBytes32, flowLimits, gasOptions);

            await handleTx(tx, chain, interchainTokenService, options.action, 'FlowLimitSet');

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
                trustedAddresses = itsChains.map((_) => chain.contracts?.InterchainTokenService?.address);
            } else {
                const trustedChain = config.chains[options.trustedChain.toLowerCase()]?.axelarId;
                const trustedAddress =
                    options.trustedAddress || config.chains[options.trustedChain.toLowerCase()]?.contracts?.InterchainTokenService?.address;

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

            const pauseStatus = options.pauseStatus;

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

    addExtendedOptions(program, { address: true, salt: true });

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
                'deployTokenManager',
                'deployInterchainToken',
                'contractCallValue',
                'expressExecute',
                'interchainTransfer',
                'callContractWithInterchainToken',
                'setFlowLimits',
                'trustedAddress',
                'setTrustedAddress',
                'removeTrustedAddress',
                'setPauseStatus',
                'execute',
                'checks',
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
