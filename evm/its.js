'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { hexZeroPad },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    prompt,
    printWarn,
    printWalletInfo,
    isValidAddress,
    isKeccak256Hash,
    wasEventEmitted,
    mainProcessor,
    isValidTokenId,
    isValidNumber,
    isString,
    isValidCalldata,
    isValidBytesAddress,
    isNumberArray,
} = require('./utils');
const { getWallet } = require('./sign-utils');
const IInterchainTokenService = require('@axelar-network/interchain-token-service/dist/interchain-token-service/InterchainTokenService.sol');
const tokenManagerImplementations = {
    MINT_BURN: 0,
    MINT_BURN_FROM: 1,
    LOCK_UNLOCK: 2,
    LOCK_UNLOCK_FEE: 3,
};

async function processCommand(chain, options) {
    const { privateKey, address, action, yes } = options;

    const contracts = chain.contracts;
    const contractName = 'InterchainTokenService';
    const contractConfig = contracts.InterchainTokenService;

    const interchainTokenServiceAddress = address || contracts.interchainTokenService?.address;

    if (!isValidAddress(interchainTokenServiceAddress)) {
        throw new Error(`Contract ${contractName} is not deployed on ${chain.name}`);
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    printInfo('Chain', chain.name);

    const wallet = await getWallet(privateKey, provider, options);
    const { address: walletAddress } = await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', interchainTokenServiceAddress);

    const interchainTokenService = new Contract(interchainTokenServiceAddress, IInterchainTokenService.abi, wallet);

    const gasOptions = contractConfig?.gasOptions || chain?.gasOptions || {};
    printInfo('Gas options', JSON.stringify(gasOptions, null, 2));

    printInfo('Action', action);

    if (prompt(`Proceed with action ${action}`, yes)) {
        return;
    }

    switch (action) {
        case 'contractId': {
            const contractId = await interchainTokenService.contractId();
            printInfo('InterchainTokenService contract ID', contractId);

            break;
        }

        case 'tokenManagerAddress': {
            const tokenId = options.tokenId;

            if (!isValidTokenId(tokenId)) {
                throw new Error(`Invalid tokenId value: ${tokenId}`);
            }

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const tokenManagerAddress = await interchainTokenService.tokenManagerAddress(tokenIdBytes32);
            printInfo(`TokenManager address for tokenId: ${tokenId}:`, tokenManagerAddress);

            break;
        }

        case 'validTokenManagerAddress': {
            const tokenId = options.tokenId;

            if (!isValidTokenId(tokenId)) {
                throw new Error(`Invalid tokenId value: ${tokenId}`);
            }

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            try {
                const tokenManagerAddress = await interchainTokenService.validTokenManagerAddress(tokenIdBytes32);
                printInfo(`TokenManager for tokenId: ${tokenId} exists at address:`, tokenManagerAddress);
            } catch (error) {
                printInfo(`TokenManager for tokenId: ${tokenId} does not exist.`);
            }

            break;
        }

        case 'interchainTokenAddress': {
            const tokenId = options.tokenId;

            if (!isValidTokenId(tokenId)) {
                throw new Error(`Invalid tokenId value: ${tokenId}`);
            }

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const interchainTokenAddress = await interchainTokenService.interchainTokenAddress(tokenIdBytes32);
            printInfo(`InterchainToken address for tokenId: ${tokenId}:`, interchainTokenAddress);

            break;
        }

        case 'interchainTokenId': {
            const sender = options.sender;

            if (!isValidAddress(sender)) {
                throw new Error(`Invalid sender address: ${sender}`);
            }

            const salt = options.salt;

            if (!isKeccak256Hash(salt)) {
                throw new Error(`Invalid salt: ${salt}`);
            }

            const interchainTokenId = await interchainTokenService.interchainTokenId(sender, salt);
            printInfo(`InterchainTokenId for sender ${sender} and deployment salt: ${salt}`, interchainTokenId);

            break;
        }

        case 'tokenManagerImplementation': {
            const type = options.type;

            const tokenManagerImplementation = await interchainTokenService.tokenManagerImplementation(tokenManagerImplementations[type]);
            printInfo(`${type} TokenManager implementation address:`, tokenManagerImplementation);

            break;
        }

        case 'flowLimit': {
            const tokenId = options.tokenId;

            if (!isValidTokenId(tokenId)) {
                throw new Error(`Invalid tokenId value: ${tokenId}`);
            }

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const flowLimit = await interchainTokenService.flowLimit(tokenIdBytes32);
            printInfo(`Flow limit for TokenManager with tokenId: ${tokenId}`, flowLimit);

            break;
        }

        case 'flowOutAmount': {
            const tokenId = options.tokenId;

            if (!isValidTokenId(tokenId)) {
                throw new Error(`Invalid tokenId value: ${tokenId}`);
            }

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const flowOutAmount = await interchainTokenService.flowOutAmount(tokenIdBytes32);
            printInfo(`Flow out amount for TokenManager with tokenId: ${tokenId}`, flowOutAmount);

            break;
        }

        case 'flowInAmount': {
            const tokenId = options.tokenId;

            if (!isValidTokenId(tokenId)) {
                throw new Error(`Invalid tokenId value: ${tokenId}`);
            }

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            const flowInAmount = await interchainTokenService.flowInAmount(tokenIdBytes32);
            printInfo(`Flow out amount for TokenManager with tokenId: ${tokenId}`, flowInAmount);

            break;
        }

        case 'deployTokenManager': {
            const isPaused = await interchainTokenService.paused();

            if (isPaused) {
                throw new Error(`${action} invalid while service is paused.`);
            }

            const { salt, destinationChain, type, params, gasValue } = options;

            if (!isKeccak256Hash(salt)) {
                throw new Error(`Invalid salt: ${salt}`);
            }

            if (!isString(destinationChain)) {
                throw new Error(`Invalid destinationChain: ${destinationChain}`);
            }

            if (!isValidCalldata(params)) {
                throw new Error(`Invalid params: ${params}`);
            }

            if (!isValidNumber(gasValue)) {
                throw new Error(`Invalid gas value: ${gasValue}`);
            }

            const tx = await interchainTokenService.deployTokenManager(
                salt,
                destinationChain,
                tokenManagerImplementations[type],
                params,
                gasValue,
            );
            printInfo('deploy TokenManager tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted =
                wasEventEmitted(receipt, interchainTokenService, 'TokenManagerDeployed') ||
                wasEventEmitted(receipt, interchainTokenService, 'TokenManagerDeploymentStarted');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'deployInterchainToken': {
            const isPaused = await interchainTokenService.paused();

            if (isPaused) {
                throw new Error(`${action} invalid while service is paused.`);
            }

            const { salt, destinationChain, name, symbol, decimals, distributor, gasValue } = options;

            if (!isKeccak256Hash(salt)) {
                throw new Error(`Invalid salt: ${salt}`);
            }

            if (!isString(destinationChain)) {
                throw new Error(`Invalid destinationChain: ${destinationChain}`);
            }

            if (!isString(name)) {
                throw new Error(`Invalid name: ${name}`);
            }

            if (!isString(symbol)) {
                throw new Error(`Invalid symbol: ${symbol}`);
            }

            if (!isValidNumber(decimals)) {
                throw new Error(`Invalid decimals value: ${decimals}`);
            }

            if (!isValidBytesAddress(distributor)) {
                throw new Error(`Invalid distributor address: ${distributor}`);
            }

            if (!isValidNumber(gasValue)) {
                throw new Error(`Invalid gas value: ${gasValue}`);
            }

            const tx = await interchainTokenService.deployInterchainToken(
                salt,
                destinationChain,
                name,
                symbol,
                decimals,
                distributor,
                gasValue,
            );
            printInfo('deploy InterchainToken tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted =
                wasEventEmitted(receipt, interchainTokenService, 'TokenManagerDeployed') ||
                wasEventEmitted(receipt, interchainTokenService, 'InterchainTokenDeploymentStarted');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'contractCallValue': {
            const isPaused = await interchainTokenService.paused();

            if (isPaused) {
                throw new Error(`${action} invalid while service is paused.`);
            }

            const { sourceChain, sourceAddress, payload } = options;

            if (!isString(sourceChain)) {
                throw new Error(`Invalid sourceChain: ${sourceChain}`);
            }

            if (!isString(sourceAddress)) {
                throw new Error(`Invalid sourceAddress: ${sourceAddress}`);
            }

            const isTrustedAddress = await interchainTokenService.isTrustedAddress(sourceChain, sourceAddress);

            if (!isTrustedAddress) {
                throw new Error('Invalid remote service.');
            }

            if (!isValidCalldata(payload)) {
                throw new Error(`Invalid payload: ${payload}`);
            }

            const [tokenAddress, tokenAmount] = await interchainTokenService.contractCallValue(sourceChain, sourceAddress, payload);
            printInfo(`Amount of tokens with address ${tokenAddress} that the call is worth:`, tokenAmount);

            break;
        }

        case 'expressExecute': {
            const isPaused = await interchainTokenService.paused();

            if (isPaused) {
                throw new Error(`${action} invalid while service is paused.`);
            }

            const { commandID, sourceChain, sourceAddress, payload } = options;

            if (!isKeccak256Hash(commandID)) {
                throw new Error(`Invalid commandID: ${commandID}`);
            }

            if (!isString(sourceChain)) {
                throw new Error(`Invalid sourceChain: ${sourceChain}`);
            }

            if (!isString(sourceAddress)) {
                throw new Error(`Invalid sourceAddress: ${sourceAddress}`);
            }

            if (!isValidCalldata(payload)) {
                throw new Error(`Invalid payload: ${payload}`);
            }

            const tx = await interchainTokenService.expressExecute(commandID, sourceChain, sourceAddress, payload);
            printInfo('expressExecute tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, interchainTokenService, 'ExpressExecuted');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'interchainTransfer': {
            const isPaused = await interchainTokenService.paused();

            if (isPaused) {
                throw new Error(`${action} invalid while service is paused.`);
            }

            const { tokenId, destinationChain, destinationAddress, amount, metadata } = options;

            if (!isValidTokenId(tokenId)) {
                throw new Error(`Invalid tokenId value: ${tokenId}`);
            }

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            if (!isString(destinationChain)) {
                throw new Error(`Invalid destinationChain: ${destinationChain}`);
            }

            if (!isString(destinationAddress)) {
                throw new Error(`Invalid destinationAddress: ${destinationAddress}`);
            }

            if (!isValidNumber(amount)) {
                throw new Error(`Invalid token amount: ${amount}`);
            }

            if (!isValidCalldata(metadata)) {
                throw new Error(`Invalid metadata: ${metadata}`);
            }

            const tx = await interchainTokenService.interchainTransfer(
                tokenIdBytes32,
                destinationChain,
                destinationAddress,
                amount,
                metadata,
            );
            printInfo('interchainTransfer tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted =
                wasEventEmitted(receipt, interchainTokenService, 'InterchainTransfer') ||
                wasEventEmitted(receipt, interchainTokenService, 'InterchainTransferWithData');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'callContractWithInterchainToken': {
            const isPaused = await interchainTokenService.paused();

            if (isPaused) {
                throw new Error(`${action} invalid while service is paused.`);
            }

            const { tokenId, destinationChain, destinationAddress, amount, data } = options;

            if (!isValidTokenId(tokenId)) {
                throw new Error(`Invalid tokenId value: ${tokenId}`);
            }

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            if (!isString(destinationChain)) {
                throw new Error(`Invalid destinationChain: ${destinationChain}`);
            }

            if (!isString(destinationAddress)) {
                throw new Error(`Invalid destinationAddress: ${destinationAddress}`);
            }

            if (!isValidNumber(amount)) {
                throw new Error(`Invalid token amount: ${amount}`);
            }

            if (!isValidCalldata(data)) {
                throw new Error(`Invalid data: ${data}`);
            }

            const tx = await interchainTokenService.callContractWithInterchainToken(
                tokenIdBytes32,
                destinationChain,
                destinationAddress,
                amount,
                data,
            );
            printInfo('callContractWithInterchainToken tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted =
                wasEventEmitted(receipt, interchainTokenService, 'InterchainTransfer') ||
                wasEventEmitted(receipt, interchainTokenService, 'InterchainTransferWithData');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

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

            if (!isNumberArray(flowLimits)) {
                throw new Error(`Invalid flowLimits array: ${flowLimits}`);
            }

            const tx = await interchainTokenService.setFlowLimits(tokenIdsBytes32, flowLimits);
            printInfo('setFlowLimits tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, interchainTokenService, 'FlowLimitSet');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'setTrustedAddress': {
            const owner = await interchainTokenService.owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            const { trustedChain, trustedAddress } = options;

            if (!isString(trustedChain)) {
                throw new Error(`Invalid chain name: ${trustedChain}`);
            }

            if (!isString(trustedAddress)) {
                throw new Error(`Invalid trusted address: ${trustedAddress}`);
            }

            const tx = await interchainTokenService.setTrustedAddress(trustedChain, trustedAddress);
            printInfo('setTrustedAddress tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, interchainTokenService, 'TrustedAddressSet');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'removeTrustedAddress': {
            const owner = await interchainTokenService.owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            const trustedChain = options.trustedChain;

            if (!isString(trustedChain)) {
                throw new Error(`Invalid chain name: ${trustedChain}`);
            }

            const tx = await interchainTokenService.removeTrustedAddress(trustedChain);
            printInfo('removeTrustedAddress tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, interchainTokenService, 'TrustedAddressRemoved');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'setPauseStatus': {
            const owner = await interchainTokenService.owner();

            if (owner.toLowerCase() !== walletAddress.toLowerCase()) {
                throw new Error(`${action} can only be performed by contract owner: ${owner}`);
            }

            const pauseStatus = options.pauseStatus;

            const tx = await interchainTokenService.setPauseStatus(pauseStatus);
            printInfo('setPauseStatus tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = pauseStatus
                ? wasEventEmitted(receipt, interchainTokenService, 'Paused')
                : wasEventEmitted(receipt, interchainTokenService, 'Unpaused');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'execute': {
            const isPaused = await interchainTokenService.paused();

            if (isPaused) {
                throw new Error(`${action} invalid while service is paused.`);
            }

            const { commandID, sourceChain, sourceAddress, payload } = options;

            if (!isKeccak256Hash(commandID)) {
                throw new Error(`Invalid commandID: ${commandID}`);
            }

            if (!isString(sourceChain)) {
                throw new Error(`Invalid sourceChain: ${sourceChain}`);
            }

            if (!isString(sourceAddress)) {
                throw new Error(`Invalid sourceAddress: ${sourceAddress}`);
            }

            const isTrustedAddress = await interchainTokenService.isTrustedAddress(sourceChain, sourceAddress);

            if (!isTrustedAddress) {
                throw new Error('Invalid remote service.');
            }

            if (!isValidCalldata(payload)) {
                throw new Error(`Invalid payload: ${payload}`);
            }

            const tx = await interchainTokenService.execute(commandID, sourceChain, sourceAddress, payload);
            printInfo('execute tx', tx.hash);

            await tx.wait(chain.confirmations);

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

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-a, --address <address>', 'override address'));
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true).env('CHAINS'));
    program.addOption(new Option('--skipChains <skipChains>', 'chains to skip over'));
    program.addOption(
        new Option('--action <action>', 'ITS action')
            .choices([
                'contractId',
                'tokenManagerAddress',
                'validTokenManagerAddress',
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
                'setTrustedAddress',
                'removeTrustedAddress',
                'setPauseStatus',
                'execute',
            ])
            .makeOptionMandatory(true),
    );
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

    program.addOption(new Option('--commandID <commandID>', 'execute command ID'));
    program.addOption(new Option('--tokenId <tokenId>', 'ID of the token'));
    program.addOption(new Option('--sender <sender>', 'TokenManager deployer address'));
    program.addOption(new Option('--salt <salt>', 'deployment salt'));
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
    program.addOption(new Option('--gasValue <gasValue>', 'gas value'));
    program.addOption(new Option('--name <name>', 'token name'));
    program.addOption(new Option('--symbol <symbol>', 'token symbol'));
    program.addOption(new Option('--decimals <decimals>', 'token decimals'));
    program.addOption(new Option('--distributor <distributor>', 'token distributor'));
    program.addOption(new Option('--sourceChain <sourceChain>', 'source chain'));
    program.addOption(new Option('--sourceAddress <sourceAddress>', 'source address'));
    program.addOption(new Option('--payload <payload>', 'payload'));
    program.addOption(new Option('--amount <amount>', 'token amount'));
    program.addOption(new Option('--metadata <metadata>', 'token transfer metadata'));
    program.addOption(new Option('--data <data>', 'token transfer data'));
    program.addOption(new Option('--tokenIds <tokenIds>', 'tokenId array'));
    program.addOption(new Option('--flowLimits <flowLimits>', 'flow limit array'));
    program.addOption(new Option('--trustedChain <trustedChain>', 'chain name for trusted addresses'));
    program.addOption(new Option('--trustedAddress <trustedAddress>', 'trusted address'));
    program.addOption(new Option('--pauseStatus <pauseStatus>', 'pause status').choices(['true', 'false']));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
