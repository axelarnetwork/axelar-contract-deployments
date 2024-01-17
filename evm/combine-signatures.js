'use strict';

const { ethers } = require('hardhat');
const fs = require('fs');
const path = require('path');
const axios = require('axios');
const {
    getDefaultProvider,
    utils: { computePublicKey, keccak256, getAddress, verifyMessage, arrayify, concat, toUtf8Bytes, defaultAbiCoder },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const { mainProcessor, printInfo, printWalletInfo, getGasOptions, printError, validateParameters, getContractJSON } = require('./utils');
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
    const addressHash = keccak256(uncompressedPublicKey.slice(4));

    return getAddress('0x' + addressHash.slice(-40));
}

async function getValidatorsAndThreshold(chain) {
    const url = `https://lcd-axelar.imperator.co/axelar/evm/v1beta1/key_address/${chain.id}`;

    try {
        const response = await axios.get(url);
        const data = response.data;

        return [data.key_id, data.threshold, data.addresses];
    } catch (error) {
        printError('Error fetching data', error);
    }
}

function getEthSignedMessageHash(message) {
    const messageHash = keccak256(arrayify(message));

    const prefix = '\x19Ethereum Signed Message:\n32';

    const ethSignedMessageHash = keccak256(concat([toUtf8Bytes(prefix), arrayify(messageHash)]));

    return ethSignedMessageHash;
}

async function processCommand(_, chain, options) {
    const { address, action, privateKey } = options;

    const contracts = chain.contracts;

    const contractName = 'AxelarGateway';

    const gatewayAddress = address || contracts.AxelarGateway?.address;

    validateParameters({ isValidAddress: { gatewayAddress } });

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = await getWallet(privateKey, provider, options);
    await printWalletInfo(wallet);

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('Batch Action', action);

    switch (action) {
        case 'constructBatch': {
            const { message } = options;

            validateParameters({ isValidCalldata: { message } });

            const [chainKeyId, threshold, validatorAddresses] = await getValidatorsAndThreshold(chain);

            const signatures = readSignatures();

            const batchSignatures = [];
            const batchValidators = [];
            const batchWeights = [];

            let totalWeight = 0;

            const expectedMessageHash = getEthSignedMessageHash(message);

            for (const signatureJSON of signatures) {
                const keyId = signatureJSON.key_id;
                const validatorAddress = signatureJSON.validator;
                const msgHash = signatureJSON.msg_hash;
                const pubKey = signatureJSON.pub_key;
                const signature = signatureJSON.signature;

                validateParameters({
                    isNonEmptyString: { keyId },
                    isValidAddress: { validatorAddress },
                    isKeccak256Hash: { msgHash },
                    isValidCalldata: { pubKey, signature },
                });

                if (chainKeyId !== keyId) {
                    printError('Signature contains invalid key_id', keyId);
                    return;
                }

                if (msgHash.toLowerCase() !== expectedMessageHash.toLowerCase()) {
                    printError('Message hash does not equal expected message hash', msgHash);
                    return;
                }

                const expectedAddress = getAddressFromPublicKey(pubKey);

                if (expectedAddress.toLowerCase() !== validatorAddress.toLowerCase()) {
                    printError('Public key does not match validator address', validatorAddress);
                    return;
                }

                const signer = verifyMessage(msgHash, signature);

                if (signer.toLowerCase() !== validatorAddress.toLowerCase()) {
                    printError('Signature is invalid for the given validator address', validatorAddress);
                    return;
                }

                const addressInfo = validatorAddresses.find(
                    (addressObj) => addressObj.address.toLowerCase() === validatorAddress.toLowerCase(),
                );
                const validatorWeight = addressInfo.weight;

                totalWeight += validatorWeight;

                batchValidators.push(validatorAddress);
                batchSignatures.push(signature);
                batchWeights.push(validatorWeight);

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
                [batchValidators, batchWeights, threshold, batchSignatures],
            );

            const input = defaultAbiCoder.encode(['bytes', 'bytes'], [message, proof]);

            printInfo('Batch input data', input);

            break;
        }

        case 'executeBatch': {
            const { input } = options;

            validateParameters({ isValidCalldata: { input } });

            const gateway = new Contract(gatewayAddress, IAxelarGateway.abi, wallet);

            const tx = await gateway.execute(input, gasOptions);

            await handleTx(tx, chain, gateway, action, 'Executed');

            break;
        }

        default: {
            throw new Error(`Unknown batch action ${action}`);
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
    program.addOption(new Option('-m, --message <message>', 'bytes message for validators to sign').env('MESSAGE'));
    program.addOption(new Option('-i, --input <input>', 'batch input consisting of bytes message (data) and bytes proof').env('INPUT'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
