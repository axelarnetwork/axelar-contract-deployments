'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { keccak256, id, defaultAbiCoder, arrayify },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    prompt,
    printWarn,
    printWalletInfo,
    getEVMBatch,
    getEVMAddresses,
    isValidAddress,
    wasEventEmitted,
    mainProcessor,
    printError,
} = require('./utils');
const { getWallet } = require('./sign-utils');

const IGateway = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarGateway.json');
const IAxelarExecutable = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarExecutable.json');
const IAuth = require('@axelar-network/axelar-cgp-solidity/interfaces/IAxelarAuthWeighted.json');

const getApproveContractCall = (sourceChain, source, destination, payloadHash, sourceTxHash, sourceEventIndex) => {
    return defaultAbiCoder.encode(
        ['string', 'string', 'address', 'bytes32', 'bytes32', 'uint256'],
        [sourceChain, source, destination, payloadHash, sourceTxHash, sourceEventIndex],
    );
};

const buildCommandBatch = (chainId, commandIDs, commandNames, commands) => {
    return arrayify(defaultAbiCoder.encode(['uint256', 'bytes32[]', 'string[]', 'bytes[]'], [chainId, commandIDs, commandNames, commands]));
};

const getWeightedSignaturesProof = async (data, operators, weights, threshold, signers) => {
    const hash = arrayify(keccak256(data));

    // assume sorted order of signers
    const signatures = await Promise.all(signers.map((wallet) => wallet.signMessage(hash)));

    return defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256', 'bytes[]'], [operators, weights, threshold, signatures]);
};

const getSignedWeightedExecuteInput = async (data, operators, weights, threshold, signers) => {
    return defaultAbiCoder.encode(
        ['bytes', 'bytes'],
        [data, await getWeightedSignaturesProof(data, operators, weights, threshold, signers)],
    );
};

async function processCommand(config, chain, options) {
    const { privateKey, address, action, yes } = options;

    const contracts = chain.contracts;
    const contractName = 'AxelarGateway';
    const contractConfig = contracts.AxelarGateway;

    const gatewayAddress = address || contracts.AxelarGateway?.address;

    if (!isValidAddress(gatewayAddress)) {
        throw new Error(`Contract ${contractName} is not deployed on ${chain.name}`);
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    printInfo('Chain', chain.name);

    const wallet = await getWallet(privateKey, provider, options);
    const { address: walletAddress } = await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', gatewayAddress);

    const gateway = new Contract(gatewayAddress, IGateway.abi, wallet);

    const gasOptions = contractConfig?.gasOptions || chain?.gasOptions || {};
    printInfo('Gas options', JSON.stringify(gasOptions, null, 2));

    printInfo('Action', action);

    if (prompt(`Proceed with action ${action}`, yes)) {
        return;
    }

    const payload = options.payload || '0x';

    if (!payload) {
        throw new Error('Missing GMP payload');
    }

    switch (action) {
        case 'admins': {
            const adminEpoch = await gateway.adminEpoch();
            const admins = await gateway.admins(adminEpoch);
            const adminThreshold = await gateway.adminThreshold(adminEpoch);
            printInfo('Gateway admins', admins);
            printInfo('Gateway admin threshold', adminThreshold);

            break;
        }

        case 'operators': {
            const { addresses, weights, threshold, keyID } = await getEVMAddresses(config, chain.id, options);
            printInfo('Axelar validator key id', keyID);

            const auth = new Contract(await gateway.authModule(), IAuth.abi, wallet);
            const operators = defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [addresses, weights, threshold]);
            const expectedHash = keccak256(operators);

            const epoch = await auth.currentEpoch();
            const operatorHash = await auth.hashForEpoch(epoch);

            printInfo('Gateway operator epoch', epoch);
            printInfo('Gateway operator hash', operatorHash);

            if (expectedHash !== operatorHash) {
                printError(`Expected operator hash ${expectedHash} but found ${operatorHash}`);
            }

            break;
        }

        case 'submitBatch': {
            const batch = getEVMBatch(config, chain.id, options.batchID);

            printInfo(`Submitting batch: ${options.batchID || 'latest'}`);

            if (batch.status !== 'BATCH_COMMANDS_STATUS_SIGNED') {
                throw new Error(`Batch status: ${batch.status} is not signed`);
            }

            const tx = await gateway.execute(batch.execute_data, gasOptions);
            printInfo('Approve tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gateway, 'Executed');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'callContract': {
            const destination = options.destination || walletAddress;

            printInfo('Call contract destination chain', options.destinationChain);
            printInfo('Call contract destination address', destination);

            const tx = await gateway.callContract(options.destinationChain, destination, payload, gasOptions);
            printInfo('Call contract tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gateway, 'ContractCall');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'approve':

        // eslint-disable-next-line no-fallthrough
        case 'approveAndExecute': {
            const payloadHash = payload.startsWith('0x') ? keccak256(arrayify(payload)) : id(payload);

            const commandID = options.commandID.startsWith('0x') ? options.commandID : id(parseInt(options.commandID).toString());

            if (await gateway.isCommandExecuted(commandID)) {
                printWarn('Command already executed');
                return;
            }

            const data = buildCommandBatch(
                chain.chainId,
                [commandID],
                ['approveContractCall'],
                [getApproveContractCall(chain.id, walletAddress, options.destination || walletAddress, payloadHash, id(''), 0)],
            );

            const signedData = await getSignedWeightedExecuteInput(data, [walletAddress], [1], 1, [wallet]);

            const tx = await gateway.execute(signedData, gasOptions);
            printInfo('Approve tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gateway, 'ContractCallApproved');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            if (action !== 'approveAndExecute') {
                break;
            }
        }

        // eslint-disable-next-line no-fallthrough
        case 'execute':

        // eslint-disable-next-line no-duplicate-case,no-fallthrough
        case 'approveAndExecute': {
            const payloadHash = payload.startsWith('0x') ? keccak256(arrayify(payload)) : id(payload);

            const commandID = options.commandID.startsWith('0x') ? options.commandID : id(parseInt(options.commandID).toString());

            if (!options.destination) {
                throw new Error('Missing destination contract address');
            }

            printInfo('Destination app contract', options.destination);
            printInfo('Payload Hash', payloadHash);

            if (
                !(await gateway.isContractCallApproved(
                    commandID,
                    'Axelarnet',
                    'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj',
                    options.destination,
                    payloadHash,
                ))
            ) {
                printWarn('Contract call not approved at the gateway');
                return;
            }

            const appContract = new Contract(options.destination, IAxelarExecutable.abi, wallet);

            const tx = await appContract.execute(commandID, 'Axelarnet', 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj', payload);
            printInfo('Execute tx', tx.hash);
            await tx.wait(chain.confirmations);

            break;
        }

        case 'transferGovernance': {
            const newGovernance = options.destination;

            if (!isValidAddress(newGovernance)) {
                throw new Error('Invalid new governor address');
            }

            const currGovernance = await gateway.governance();
            printInfo('Current governance', currGovernance);

            if (!(currGovernance === walletAddress)) {
                throw new Error('Wallet address is not the governor');
            }

            if (prompt(`Proceed with governance transfer to ${newGovernance}`, yes)) {
                return;
            }

            const tx = await gateway.transferGovernance(newGovernance, gasOptions);
            printInfo('Transfer governance tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gateway, 'GovernanceTransferred');

            if (!eventEmitted) {
                throw new Error('Event not emitted in receipt.');
            }

            contracts.AxelarGateway.governance = newGovernance;

            break;
        }

        case 'governance': {
            printInfo(`Gateway governance`, await gateway.governance());
            break;
        }

        case 'mintLimiter': {
            printInfo(`Gateway mintLimiter`, await gateway.mintLimiter());
            break;
        }

        case 'mintLimit': {
            if (!options.symbol) {
                throw new Error('Missing symbol');
            }

            printInfo(`Gateway mintLimit ${options.symbol}`, await gateway.tokenMintLimit(options.symbol));
            printInfo(`Gateway mint amount ${options.symbol}`, await gateway.tokenMintAmount(options.symbol));
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

const program = new Command();

program.name('gateway').description('Script to perform gateway commands');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-c, --contractName <contractName>', 'contract name').default('Multisig'));
program.addOption(new Option('-a, --address <address>', 'override address'));
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(
    new Option('--action <action>', 'gateway action')
        .choices([
            'admins',
            'operators',
            'callContract',
            'submitBatch',
            'approve',
            'execute',
            'approveAndExecute',
            'transferGovernance',
            'governance',
            'mintLimiter',
            'mintLimit',
        ])
        .makeOptionMandatory(true),
);
program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

program.addOption(new Option('--payload <payload>', 'gmp payload'));
program.addOption(new Option('--commandID <commandID>', 'execute command ID'));
program.addOption(new Option('--destination <destination>', 'GMP destination address'));
program.addOption(new Option('--destinationChain <destinationChain>', 'GMP destination chain'));
program.addOption(new Option('--batchID <batchID>', 'EVM batch ID').default(''));
program.addOption(new Option('--symbol <symbol>', 'EVM token symbol'));

program.action((options) => {
    main(options);
});

program.parse();
