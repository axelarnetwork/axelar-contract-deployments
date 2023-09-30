'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { keccak256, id, defaultAbiCoder, arrayify },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, prompt, printWarn, printWalletInfo, isValidNumber, isValidAddress, wasEventEmitted, mainProcessor } = require('./utils');
const IGateway = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarGateway.json');
const IAxelarExecutable = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarExecutable.json');
const { getWallet } = require('./sign-utils');

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

async function processCommand(_, chain, options) {
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

    const payloadHash = payload.startsWith('0x') ? keccak256(arrayify(payload)) : id(payload);

    const commandID = isValidNumber(options.commandID) ? id(parseInt(options.commandID).toString()) : options.commandID;

    switch (action) {
        case ('approve', 'approveAndExecute'): {
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
        case ('execute', 'approveAndExecute'): {
            if (!options.destination) {
                throw new Error('Missing destination contract address');
            }

            if (!(await gateway.isContractCallApproved(commandID, chain.id, walletAddress, options.destination, payloadHash))) {
                printWarn('Contract call not approved at the gateway');
                return;
            }

            const appContract = new Contract(options.destination, IAxelarExecutable.abi, wallet);

            const tx = await appContract.execute(commandID, chain.id, walletAddress, payload);
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
        .choices(['approve', 'execute', 'approveAndExecute', 'transferGovernance', 'governance', 'mintLimiter'])
        .makeOptionMandatory(true),
);
program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

program.addOption(new Option('--payload <payload>', 'gmp payload'));
program.addOption(new Option('--commandID <commandID>', 'execute command ID'));
program.addOption(new Option('--destination <destination>', 'GMP destination address'));

program.action((options) => {
    main(options);
});

program.parse();
