'use strict';

const chalk = require('chalk');
const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { keccak256, id, defaultAbiCoder, arrayify },
    constants: { HashZero },
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
    getGasOptions,
    httpGet,
    getContractJSON,
    getMultisigProof,
} = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { getWallet, signTransaction } = require('./sign-utils');

const IGateway = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarGateway.json');
const IAxelarExecutable = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IAxelarExecutable.json');
const IAuth = require('@axelar-network/axelar-cgp-solidity/interfaces/IAxelarAuthWeighted.json');
const { getWeightedSignersProof, WEIGHTED_SIGNERS_TYPE } = require('@axelar-network/axelar-gmp-sdk-solidity/scripts/utils');

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

    const gatewayAddress = address || contracts.AxelarGateway?.address;

    if (!isValidAddress(gatewayAddress)) {
        throw new Error(`Contract ${contractName} is not deployed on ${chain.name}`);
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = await getWallet(privateKey, provider, options);
    const { address: walletAddress } = await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', gatewayAddress);

    const gateway = new Contract(gatewayAddress, IGateway.abi, wallet);

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('Action', action);

    if (prompt(`Proceed with action ${action}`, yes)) {
        return;
    }

    let payload = options.payload || '0x';
    if (!payload.startsWith('0x')) {
        payload = '0x' + payload;
    }

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

        case 'params': {
            const governance = await gateway.governance();
            const mintLimiter = await gateway.mintLimiter();
            const authModule = await gateway.authModule();
            const tokenDeployer = await gateway.tokenDeployer();
            const implementation = await gateway.implementation();

            printInfo('Gateway governance', governance);
            printInfo('Gateway mint limiter', mintLimiter);
            printInfo('Gateway auth module', authModule);
            printInfo('Gateway token deployer', tokenDeployer);
            printInfo('Gateway implementation', implementation);

            break;
        }

        case 'operators': {
            const { addresses, weights, threshold, keyID } = await getEVMAddresses(config, chain.axelarId, options);
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
            const batch = await getEVMBatch(config, chain.axelarId, options.batchID);

            printInfo(`Submitting batch: ${options.batchID || 'latest'}`);

            if (batch.status !== 'BATCHED_COMMANDS_STATUS_SIGNED') {
                throw new Error(`Batch status: ${batch.status} is not signed`);
            }

            const tx = await gateway.execute('0x' + batch.execute_data, gasOptions);
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

        case 'approveWithBatch': {
            const { batchID } = options;

            if (!batchID) {
                throw new Error('Batch ID is required for the approve action');
            }

            const batchId = batchID.startsWith('0x') ? batchID.substring(2) : batchID;
            const apiUrl = `${config.axelar.lcd}/axelar/evm/v1beta1/batched_commands/${chain.axelarId}/${batchId}`;

            let executeData, response;

            try {
                response = await httpGet(`${apiUrl}`);
                executeData = '0x' + response.execute_data;
            } catch (error) {
                throw new Error(`Failed to fetch batch data: ${error.message}`);
            }

            if (response == null || !response.execute_data) {
                throw new Error('Response does not contain execute_data');
            }

            if (response.status !== 'BATCHED_COMMANDS_STATUS_SIGNED') {
                throw new Error('Data is not yet signed by operators');
            }

            const tx = {
                to: gatewayAddress,
                data: executeData,
                ...gasOptions,
            };

            const txResponse = await wallet.sendTransaction(tx);
            printInfo('Approve tx', txResponse.hash);

            const receipt = await response.wait(chain.confirmations);
            const eventEmitted = wasEventEmitted(receipt, gateway, 'ContractCallApproved');

            if (!eventEmitted) {
                printWarn('Event not emitted in receipt.');
            }

            break;
        }

        case 'approve':

        // eslint-disable-next-line no-fallthrough
        case 'approveAndExecute': {
            const payloadHash = keccak256(arrayify(payload));

            const commandID = options.commandID.startsWith('0x') ? options.commandID : id(parseInt(options.commandID).toString());

            if (await gateway.isCommandExecuted(commandID)) {
                printWarn('Command already executed');
                return;
            }

            const data = buildCommandBatch(
                chain.chainId,
                [commandID],
                ['approveContractCall'],
                [getApproveContractCall(chain.axelarId, walletAddress, options.destination || walletAddress, payloadHash, id(''), 0)],
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
            const payloadHash = keccak256(arrayify(payload));
            const { sourceChain, sourceAddress } = options;

            let commandId;

            if (options.messageId) {
                // Derive commandId for Amplifier gateway
                commandId = id(`${sourceChain}_${options.messageId}`);
            } else {
                commandId = options.commandID.startsWith('0x') ? options.commandID : id(parseInt(options.commandID).toString());
            }

            if (!options.destination) {
                throw new Error('Missing destination contract address');
            }

            printInfo('Destination app contract', options.destination);
            printInfo('Payload Hash', payloadHash);

            if (
                !(await gateway.isContractCallApproved(
                    commandId,
                    sourceChain,
                    sourceAddress,
                    options.destination,
                    payloadHash,
                ))
            ) {
                printWarn('Contract call not approved at the gateway');
                return;
            }

            const appContract = new Contract(options.destination, IAxelarExecutable.abi, wallet);

            const tx = await appContract.execute(commandId, sourceChain, sourceAddress, payload);
            printInfo('Execute tx', tx.hash);
            await tx.wait(chain.confirmations);

            break;
        }

        case 'transferGovernance': {
            const newGovernance = options.destination || chain.contracts.InterchainGovernance?.address;

            if (!isValidAddress(newGovernance)) {
                throw new Error('Invalid new governor address');
            }

            const currGovernance = await gateway.governance();
            printInfo('Current governance', currGovernance);

            if (!(currGovernance === walletAddress)) {
                throw new Error('Wallet address is not the governor');
            }

            if (prompt(`Proceed with governance transfer to ${chalk.cyan(newGovernance)}`, yes)) {
                return;
            }

            const tx = await gateway.transferGovernance(newGovernance, gasOptions);
            printInfo('Transfer governance tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gateway, 'GovernanceTransferred');

            if (!eventEmitted) {
                throw new Error('Event not emitted in receipt.');
            }

            if (!chain.contracts.InterchainGovernance) {
                chain.contracts.InterchainGovernance = {};
            }

            chain.contracts.InterchainGovernance.address = newGovernance;

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

        case 'transferMintLimiter': {
            const newMintLimiter = options.destination || chain.contracts.Multisig?.address;

            if (!isValidAddress(newMintLimiter)) {
                throw new Error('Invalid address');
            }

            const currMintLimiter = await gateway.mintLimiter();
            printInfo('Current governance', currMintLimiter);

            if (!(currMintLimiter === walletAddress)) {
                throw new Error('Wallet address is not the mint limiter');
            }

            if (prompt(`Proceed with mint limiter transfer to ${chalk.cyan(newMintLimiter)}`, yes)) {
                return;
            }

            const tx = await gateway.transferMintLimiter(newMintLimiter, gasOptions);
            printInfo('Transfer mint limiter tx', tx.hash);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gateway, 'MintLimiterTransferred');

            if (!eventEmitted) {
                throw new Error('Event not emitted in receipt.');
            }

            if (!chain.contracts.Multisig) {
                chain.contracts.Multisig = {};
            }

            chain.contracts.Multisig.address = newMintLimiter;

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

        case 'rotateSigners': {
            // TODO: use args for new signers
            const gateway = new Contract(gatewayAddress, getContractJSON('AxelarAmplifierGateway').abi, wallet);

            const weightedSigners = {
                signers: [
                    {
                        signer: wallet.address,
                        weight: 1,
                    },
                ],
                threshold: 1,
                nonce: HashZero,
            };

            const newSigners = {
                ...weightedSigners,
                nonce: id('1'),
            };

            const data = defaultAbiCoder.encode(['uint8', WEIGHTED_SIGNERS_TYPE], [1, newSigners]);
            console.log(JSON.stringify(newSigners, null, 2));
            const proof = await getWeightedSignersProof(data, HashZero, weightedSigners, [wallet]);
            const tx = await gateway.rotateSigners(newSigners, proof, gasOptions);

            const receipt = await tx.wait(chain.confirmations);

            const eventEmitted = wasEventEmitted(receipt, gateway, 'SignersRotated');

            if (!eventEmitted) {
                throw new Error('Event not emitted in receipt.');
            }

            break;
        }

        case 'submitProof': {
            const { multisigSessionId } = options;

            if (!multisigSessionId) {
                throw new Error('Missing multisig session ID');
            }

            const { status } = await getMultisigProof(config, chain.axelarId, multisigSessionId);

            if (!status.completed) {
                throw new Error('Multisig session not completed');
            }

            const tx = {
                to: gateway.address,
                data: '0x' + status.completed.execute_data,
            };

            await signTransaction(wallet, chain, tx, options);

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

    program.name('gateway').description('Script to perform gateway commands');

    addBaseOptions(program, { address: true });

    program.addOption(new Option('-c, --contractName <contractName>', 'contract name').default('Multisig'));
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
                'transferMintLimiter',
                'mintLimit',
                'params',
                'approveWithBatch',
                'rotateSigners',
                'submitProof',
            ])
            .makeOptionMandatory(true),
    );

    program.addOption(new Option('--payload <payload>', 'gmp payload'));
    program.addOption(new Option('--commandID <commandID>', 'execute command ID'));
    program.addOption(new Option('--messageId <messageId>', 'GMP call message ID'));
    program.addOption(new Option('--sourceChain <sourceChain>', 'GMP source chain'));
    program.addOption(new Option('--sourceAddress <sourceAddress>', 'GMP source address'));
    program.addOption(new Option('--destination <destination>', 'GMP destination address'));
    program.addOption(new Option('--destinationChain <destinationChain>', 'GMP destination chain'));
    program.addOption(new Option('--batchID <batchID>', 'EVM batch ID').default(''));
    program.addOption(new Option('--symbol <symbol>', 'EVM token symbol'));
    program.addOption(new Option('--multisigSessionId <multisigSessionId>', 'Amplifier multisig proof session ID'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
