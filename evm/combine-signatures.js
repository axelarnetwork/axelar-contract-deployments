'use strict';

const { ethers } = require('hardhat');
const fs = require('fs');
const path = require('path');
const {
    getDefaultProvider,
    utils: { computePublicKey, keccak256, getAddress, arrayify, defaultAbiCoder, hashMessage, recoverAddress },
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
    getRandomBytes32,
    getEVMAddresses,
} = require('./utils');
const { handleTx } = require('./its');
const { getWallet } = require('./sign-utils');
const { addBaseOptions } = require('./cli-utils');
const IAxelarGateway = getContractJSON('IAxelarGateway');
const IAxelarAuth = getContractJSON('IAxelarAuth');

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

    printInfo('Batch Action', action);

    switch (action) {
        case 'computeMessageHash': {
            const { commandId, sourceChain, sourceAddress, contractAddress, payloadHash, sourceTxHash, sourceEventIndex } = options;

            validateParameters({
                isNonEmptyString: { sourceChain, sourceAddress },
                isValidAddress: { contractAddress },
                isKeccak256Hash: { commandId, payloadHash, sourceTxHash },
                isValidNumber: { sourceEventIndex },
            });

            const chainId = chain.chainId;
            const commandID = commandId || getRandomBytes32();
            const command = 'approveContractCall';
            const params = defaultAbiCoder.encode(
                ['string', 'string', 'address', 'bytes32', 'bytes32', 'uint256'],
                [sourceChain, sourceAddress, contractAddress, payloadHash, sourceTxHash, sourceEventIndex],
            );

            const data = defaultAbiCoder.encode(
                ['uint256', 'bytes32[]', 'string[]', 'bytes[]'],
                [chainId, [commandID], [command], [params]],
            );

            const dataHash = hashMessage(arrayify(keccak256(data)));

            printInfo('Original bytes message (pre-hash)', data);
            printInfo('Message hash for validators to sign', dataHash);

            break;
        }

        case 'constructBatch': {
            const { message } = options;

            validateParameters({ isValidCalldata: { message } });

            const {
                addresses: validatorAddresses,
                weights,
                threshold,
                keyID: expectedKeyId,
            } = await getEVMAddresses(config, chain.id, options);

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

            let totalWeight = 0;

            const expectedMessageHash = hashMessage(arrayify(keccak256(message)));

            for (const signatureJSON of sortedSignatures) {
                const keyId = signatureJSON.key_id;
                const msgHash = `0x${signatureJSON.msg_hash}`;
                const pubKey = `0x${signatureJSON.pub_key}`;
                const signature = `0x${signatureJSON.signature}`;

                validateParameters({
                    isNonEmptyString: { keyId },
                    isKeccak256Hash: { msgHash },
                    isValidCalldata: { pubKey, signature },
                });

                if (expectedKeyId !== keyId) {
                    printError('Signature contains invalid key_id', keyId);
                    return;
                }

                if (msgHash.toLowerCase() !== expectedMessageHash.toLowerCase()) {
                    printError('Message hash does not equal expected message hash', msgHash);
                    return;
                }

                const validatorAddress = getAddressFromPublicKey(pubKey);

                const signer = recoverAddress(msgHash, signature);

                if (signer.toLowerCase() !== validatorAddress.toLowerCase()) {
                    printError('Signature is invalid for the given validator address', validatorAddress);
                    return;
                }

                const validatorWeight = validatorWeights[validatorAddress.toLowerCase()];

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
                printError('Total signer weight less than threshold', totalWeight);
                return;
            }

            const proof = defaultAbiCoder.encode(
                ['address[]', 'uint256[]', 'uint256', 'bytes[]'],
                [validatorAddresses, weights, threshold, batchSignatures],
            );

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

            const input = defaultAbiCoder.encode(['bytes', 'bytes'], [message, proof]);

            printInfo('Batch input data for gateway execute function', input);

            break;
        }

        case 'executeBatch': {
            const { input } = options;

            validateParameters({ isValidCalldata: { input } });

            const contractName = 'AxelarGateway';

            const gatewayAddress = address || contracts.AxelarGateway?.address;
            const gateway = new Contract(gatewayAddress, IAxelarGateway.abi, wallet);

            const gasOptions = await getGasOptions(chain, options, contractName);

            const tx = await gateway.execute(input, gasOptions);

            await handleTx(tx, chain, gateway, action, 'Executed');

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

    program.addOption(
        new Option('--action <action>', 'signature action').choices(['computeMessageHash', 'constructBatch', 'executeBatch']),
    );
    program.addOption(new Option('-m, --message <message>', 'bytes message (validators sign the hash of this message)').env('MESSAGE'));
    program.addOption(new Option('-i, --input <input>', 'batch input consisting of bytes message (data) and bytes proof').env('INPUT'));

    program.addOption(new Option('--commandId <commandId>', 'gateway command id').env('COMMAND_ID'));
    program.addOption(new Option('--sourceChain <sourceChain>', 'source chain for contract call').env('SOURCE_CHAIN'));
    program.addOption(new Option('--sourceAddress <sourceAddress>', 'source address for contract call').env('SOURCE_ADDRESS'));
    program.addOption(new Option('--contractAddress <contractAddress>', 'contract address on current chain').env('CONTRACT_ADDRESS'));
    program.addOption(new Option('--payloadHash <payloadHash>', 'payload hash').env('PAYLOAD_HASH'));
    program.addOption(new Option('--sourceTxHash <sourceTxHash>', 'source transaction hash').env('SOURCE_TX_HASH'));
    program.addOption(new Option('--sourceEventIndex <sourceEventIndex>', 'source event index').env('SOURCE_EVENT_INDEX'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
