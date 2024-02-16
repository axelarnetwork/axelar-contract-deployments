'use strict';

const { ethers } = require('hardhat');
const fs = require('fs');
const path = require('path');
const {
    getDefaultProvider,
    utils: { computePublicKey, keccak256, getAddress, arrayify, defaultAbiCoder, hashMessage, recoverAddress, hexZeroPad, hexlify },
    constants: { HashZero, MaxUint256 },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const {
    mainProcessor,
    printInfo,
    printWalletInfo,
    getGasOptions,
    printError,
    validateParameters,
    getContractJSON,
    getEVMAddresses,
} = require('./utils');
const { handleTx } = require('./its');
const { getWallet } = require('./sign-utils');
const { addBaseOptions } = require('./cli-utils');
const IAxelarGateway = getContractJSON('IAxelarGateway');

function readSignatures() {
    const signaturesDir = path.join(__dirname, '../signatures');
    const signatureFiles = fs.readdirSync(signaturesDir);
    const signatures = [];

    signatureFiles.forEach((file) => {
        const filePath = path.join(signaturesDir, file);
        const fileContent = fs.readFileSync(filePath, 'utf8');

        try {
            const signature = JSON.parse(fileContent);
            signatures.push(signature);
        } catch (error) {
            printError(`Error parsing JSON in file ${file}`, error.message);
        }
    });

    return signatures;
}

function getAddressFromPublicKey(publicKey) {
    const uncompressedPublicKey = computePublicKey(publicKey, false);
    const addressHash = keccak256(`0x${uncompressedPublicKey.slice(4)}`);

    return getAddress('0x' + addressHash.slice(-40));
}

async function getCommandId(gateway) {
    let currentValue = MaxUint256;

    while (true) {
        const isCommandIdExecuted = await gateway.isCommandExecuted(hexZeroPad(hexlify(currentValue), 32));

        if (!isCommandIdExecuted) {
            break;
        }

        currentValue = currentValue.sub(1);
    }

    return hexZeroPad(hexlify(currentValue), 32);
}

async function processCommand(config, chain, options) {
    const { address, action, privateKey } = options;

    const contracts = chain.contracts;

    if (address) {
        validateParameters({ isValidAddress: { address } });
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = await getWallet(privateKey, provider, options);
    await printWalletInfo(wallet);

    const gatewayAddress = address || contracts.AxelarGateway?.address;
    const gateway = new Contract(gatewayAddress, IAxelarGateway.abi, wallet);

    printInfo('Batch Action', action);

    switch (action) {
        case 'createBatchData': {
            const { commandId, payloadHash } = options;

            const sourceChain = options.sourceChain || 'Axelarnet';
            const sourceAddress = options.sourceAddress || 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';
            const contractAddress = options.contractAddress || contracts.InterchainGovernance?.address;
            const commandID = commandId || (await getCommandId(gateway));

            printInfo('Command ID', commandID);

            validateParameters({
                isNonEmptyString: { sourceChain, sourceAddress },
                isValidAddress: { contractAddress },
                isKeccak256Hash: { commandID, payloadHash },
            });

            const chainId = chain.chainId;
            const command = 'approveContractCall';
            const params = defaultAbiCoder.encode(
                ['string', 'string', 'address', 'bytes32', 'bytes32', 'uint256'],
                [sourceChain, sourceAddress, contractAddress, payloadHash, HashZero, 0],
            );

            const data = defaultAbiCoder.encode(
                ['uint256', 'bytes32[]', 'string[]', 'bytes[]'],
                [chainId, [commandID], [command], [params]],
            );

            const dataHash = hashMessage(arrayify(keccak256(data)));

            const { keyID } = await getEVMAddresses(config, chain.id, options);

            printInfo('Original bytes message (pre-hash)', data);
            printInfo('Message hash for validators to sign', dataHash);
            printInfo('Vald sign command', `axelard vald-sign ${keyID} [validator-addr] ${dataHash}`);

            break;
        }

        case 'constructBatch': {
            const { batchData, execute } = options;

            validateParameters({ isValidCalldata: { batchData } });

            const { addresses: validatorAddresses, weights, threshold } = await getEVMAddresses(config, chain.id, options);

            const validatorWeights = {};

            validatorAddresses.forEach((address, index) => {
                validatorWeights[address.toLowerCase()] = weights[index];
            });

            const signatures = readSignatures();

            const sortedSignatures = signatures.sort((a, b) => {
                const addressA = getAddressFromPublicKey(`0x${a.pub_key}`).toLowerCase();
                const addressB = getAddressFromPublicKey(`0x${b.pub_key}`).toLowerCase();
                return addressA.localeCompare(addressB);
            });

            const batchSignatures = [];
            const checkedAddresses = [];

            const expectedMessageHash = hashMessage(arrayify(keccak256(batchData)));

            const prevKeyId = sortedSignatures[0].key_id;
            let totalWeight = 0;

            for (const signatureJSON of sortedSignatures) {
                const keyId = signatureJSON.key_id;
                const msgHash = signatureJSON.msg_hash.startsWith('0x') ? signatureJSON.msg_hash : `0x${signatureJSON.msg_hash}`;
                const pubKey = signatureJSON.pub_key.startsWith('0x') ? signatureJSON.pub_key : `0x${signatureJSON.pub_key}`;
                const signature = signatureJSON.signature.startsWith('0x') ? signatureJSON.signature : `0x${signatureJSON.signature}`;

                validateParameters({
                    isNonEmptyString: { keyId },
                    isKeccak256Hash: { msgHash },
                    isValidCalldata: { pubKey, signature },
                });

                if (prevKeyId !== keyId) {
                    printError('Signatures do not contain consistent key IDs', keyId);
                    return;
                }

                if (msgHash.toLowerCase() !== expectedMessageHash.toLowerCase()) {
                    printError('Message hash does not equal expected message hash', msgHash);
                    return;
                }

                const validatorAddress = getAddressFromPublicKey(pubKey).toLowerCase();

                if (checkedAddresses.includes(validatorAddress)) {
                    printError('Duplicate validator address', validatorAddress);
                    return;
                }

                checkedAddresses.push(validatorAddress);

                const signer = recoverAddress(msgHash, signature);

                if (signer.toLowerCase() !== validatorAddress) {
                    printError('Signature is invalid for the given validator address', validatorAddress);
                    return;
                }

                const validatorWeight = validatorWeights[validatorAddress];

                if (!validatorWeight) {
                    printError('Validator does not belong to current epoch', validatorAddress);
                    return;
                }

                totalWeight += validatorWeight;

                batchSignatures.push(signature);

                if (totalWeight >= threshold) {
                    break;
                }
            }

            if (totalWeight < threshold) {
                printError(`Total signer weight ${totalWeight} less than threshold`, threshold);
                return;
            }

            const proof = defaultAbiCoder.encode(
                ['address[]', 'uint256[]', 'uint256', 'bytes[]'],
                [validatorAddresses, weights, threshold, batchSignatures],
            );

            const IAxelarAuth = getContractJSON('IAxelarAuth');
            const authAddress = address || contracts.AxelarGateway?.authModule;
            const auth = new Contract(authAddress, IAxelarAuth.abi, wallet);

            let isValidProof;

            try {
                isValidProof = await auth.validateProof(expectedMessageHash, proof);
            } catch (error) {
                printError('Invalid batch proof', error);
                return;
            }

            if (!isValidProof) {
                printError('Invalid batch proof');
                return;
            }

            const input = defaultAbiCoder.encode(['bytes', 'bytes'], [batchData, proof]);

            printInfo('Batch input (data and proof) for gateway execute function', input);

            if (execute) {
                printInfo('Executing gateway batch on chain', chain.name);

                const contractName = 'AxelarGateway';

                const gasOptions = await getGasOptions(chain, options, contractName);

                const tx = await gateway.execute(input, gasOptions);

                await handleTx(tx, chain, gateway, action, 'Executed');
            }

            break;
        }

        default: {
            throw new Error(`Unknown signature action ${action}`);
        }
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('combine-signatures').description('script to combine manually created signatures and construct gateway batch');

    addBaseOptions(program, { address: true });

    program.addOption(new Option('--action <action>', 'signature action').choices(['createBatchData', 'constructBatch']));
    program.addOption(
        new Option('-i, --batchData <batchData>', 'batch data to be combined with proof for gateway execute command').env('BATCH_DATA'),
    );
    program.addOption(new Option('--commandId <commandId>', 'gateway command id').env('COMMAND_ID'));
    program.addOption(new Option('--sourceChain <sourceChain>', 'source chain for contract call').env('SOURCE_CHAIN'));
    program.addOption(new Option('--sourceAddress <sourceAddress>', 'source address for contract call').env('SOURCE_ADDRESS'));
    program.addOption(new Option('--contractAddress <contractAddress>', 'contract address on current chain').env('CONTRACT_ADDRESS'));
    program.addOption(new Option('--payloadHash <payloadHash>', 'payload hash').env('PAYLOAD_HASH'));
    program.addOption(new Option('--execute', 'whether or not to immediately execute the batch').env('EXECUTE'));
    program.addOption(new Option('--keyID <keyID>', 'key ID for operators that have signed the message').env('KEY_ID'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
