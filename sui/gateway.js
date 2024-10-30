const { Command, Option } = require('commander');
const { Transaction } = require('@mysten/sui/transactions');
const { bcs } = require('@mysten/sui/bcs');
const { ethers } = require('hardhat');
const { bcsStructs } = require('@axelar-network/axelar-cgp-sui');
const {
    utils: { arrayify, keccak256, toUtf8Bytes },
    constants: { HashZero },
} = ethers;

const { saveConfig, printInfo, loadConfig, getMultisigProof, getChainConfig } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    getSigners,
    getWallet,
    printWalletInfo,
    getRawPrivateKey,
    broadcast,
    suiClockAddress,
} = require('./utils');
const secp256k1 = require('secp256k1');

const COMMAND_TYPE_APPROVE_MESSAGES = 0;
const COMMAND_TYPE_ROTATE_SIGNERS = 1;

function hashMessage(commandType, data) {
    const toHash = new Uint8Array(data.length + 1);
    toHash[0] = commandType;
    toHash.set(data, 1);

    return keccak256(toHash);
}

function getProofSigners(keypair, options) {
    if (options.proof === 'wallet') {
        printInfo('Using wallet to provide proof');

        if (keypair.getKeyScheme() !== 'Secp256k1') {
            throw new Error('Only Secp256k1 pubkeys are supported by the gateway');
        }

        return {
            signers: [{ pub_key: keypair.getPublicKey().toRawBytes(), weight: 1 }],
            threshold: 1,
            nonce: options.currentNonce ? keccak256(toUtf8Bytes(options.currentNonce)) : HashZero,
        };
    } else if (options.proof) {
        printInfo('Using provided proof', options.proof);

        const proof = JSON.parse(options.proof);
        return {
            signers: proof.signers.signers.map(({ pub_key: pubKey, weight }) => {
                return { pub_key: arrayify(pubKey), weight };
            }),
            threshold: proof.signers.threshold,
            nonce: arrayify(proof.signers.nonce) || HashZero,
        };
    }

    throw new Error('Proof not found');
}

function getSignatures(keypair, messageToSign, options) {
    if (options.proof === 'wallet') {
        if (keypair.getKeyScheme() !== 'Secp256k1') {
            throw new Error('Only Secp256k1 pubkeys are supported by the gateway');
        }

        const { signature, recid } = secp256k1.ecdsaSign(arrayify(keccak256(messageToSign)), getRawPrivateKey(keypair));

        return [new Uint8Array([...signature, recid])];
    } else if (options.proof) {
        const proof = JSON.parse(options.proof);
        return proof.signatures.map((signature) => arrayify(signature));
    }

    throw new Error('Proof not found');
}

function getProof(keypair, commandType, data, contractConfig, options) {
    const signers = getProofSigners(keypair, options);

    const dataHash = arrayify(hashMessage(commandType, data));

    const message = bcsStructs.gateway.MessageToSign.serialize({
        domain_separator: contractConfig.domainSeparator,
        signers_hash: keccak256(bcsStructs.gateway.WeightedSigners.serialize(signers).toBytes()),
        data_hash: dataHash,
    }).toBytes();

    const signatures = getSignatures(keypair, message, options);

    const encodedProof = bcsStructs.gateway.Proof.serialize({
        signers,
        signatures,
    }).toBytes();

    return encodedProof;
}

async function callContract(keypair, client, config, chain, contractConfig, args, options) {
    const packageId = contractConfig.address;

    const [destinationChain, destinationAddress, payload] = args;

    const gatewayObjectId = chain.contracts.AxelarGateway.objects.Gateway;

    let channel = options.channel;

    const tx = new Transaction();

    // Create a temporary channel if one wasn't provided
    if (!options.channel) {
        [channel] = tx.moveCall({
            target: `${packageId}::channel::new`,
            arguments: [],
        });
    }

    const messageTicket = tx.moveCall({
        target: `${packageId}::gateway::prepare_message`,
        arguments: [
            channel,
            tx.pure(bcs.string().serialize(destinationChain).toBytes()),
            tx.pure(bcs.string().serialize(destinationAddress).toBytes()),
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(payload)).toBytes()),
        ],
    });

    tx.moveCall({
        target: `${packageId}::gateway::send_message`,
        arguments: [tx.object(gatewayObjectId), messageTicket],
    });

    if (!options.channel) {
        tx.moveCall({
            target: `${packageId}::channel::destroy`,
            arguments: [channel],
        });
    }

    await broadcast(client, keypair, tx, 'Message sent');
}

async function approve(keypair, client, config, chain, contractConfig, args, options) {
    const packageId = contractConfig.address;
    const [sourceChain, messageId, sourceAddress, destinationId, payloadHash] = args;

    const encodedMessages = bcs
        .vector(bcsStructs.gateway.Message)
        .serialize([
            {
                source_chain: sourceChain,
                message_id: messageId,
                source_address: sourceAddress,
                destination_id: destinationId,
                payload_hash: payloadHash,
            },
        ])
        .toBytes();

    const encodedProof = getProof(keypair, COMMAND_TYPE_APPROVE_MESSAGES, encodedMessages, contractConfig, options);

    const tx = new Transaction();

    tx.moveCall({
        target: `${packageId}::gateway::approve_messages`,
        arguments: [
            tx.object(contractConfig.objects.Gateway),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedMessages).toBytes()),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedProof).toBytes()),
        ],
    });

    await broadcast(client, keypair, tx, 'Approved Messages');
}

async function submitProof(keypair, client, config, chain, contractConfig, args, options) {
    const packageId = contractConfig.address;
    const [multisigSessionId] = args;
    const { payload, status } = await getMultisigProof(config, chain.axelarId, multisigSessionId);

    if (!status.completed) {
        throw new Error('Multisig session not completed');
    }

    const executeData = bcsStructs.gateway.ExecuteData.parse(arrayify('0x' + status.completed.execute_data));

    const tx = new Transaction();

    if (payload.verifier_set) {
        printInfo('Submitting rotate_signers');

        tx.moveCall({
            target: `${packageId}::gateway::rotate_signers`,
            arguments: [
                tx.object(contractConfig.objects.Gateway),
                tx.object(suiClockAddress),
                tx.pure(bcs.vector(bcs.u8()).serialize(new Uint8Array(executeData.payload)).toBytes()),
                tx.pure(bcs.vector(bcs.u8()).serialize(new Uint8Array(executeData.proof)).toBytes()),
            ],
        });
    } else if (payload.messages) {
        printInfo('Submitting approve_messages');

        tx.moveCall({
            target: `${packageId}::gateway::approve_messages`,
            arguments: [
                tx.object(contractConfig.objects.Gateway),
                tx.pure(bcs.vector(bcs.u8()).serialize(new Uint8Array(executeData.payload)).toBytes()),
                tx.pure(bcs.vector(bcs.u8()).serialize(new Uint8Array(executeData.proof)).toBytes()),
            ],
        });
    } else {
        throw new Error(`Unknown payload type: ${payload}`);
    }

    await broadcast(client, keypair, tx, 'Submitted Amplifier Proof');
}

async function rotate(keypair, client, config, chain, contractConfig, args, options) {
    const packageId = contractConfig.address;
    const signers = await getSigners(keypair, config, chain.axelarId, options);

    const newNonce = options.newNonce ? keccak256(toUtf8Bytes(options.newNonce)) : signers.nonce;
    const encodedSigners = bcsStructs.gateway.WeightedSigners.serialize({
        ...signers,
        nonce: bcsStructs.common.Bytes32.serialize(newNonce).toBytes(),
    }).toBytes();

    const encodedProof = getProof(keypair, COMMAND_TYPE_ROTATE_SIGNERS, encodedSigners, contractConfig, options);

    const tx = new Transaction();

    tx.moveCall({
        target: `${packageId}::gateway::rotate_signers`,
        arguments: [
            tx.object(contractConfig.objects.Gateway),
            tx.object(suiClockAddress),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedSigners).toBytes()),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedProof).toBytes()),
        ],
    });

    await broadcast(client, keypair, tx, 'Rotated Signers');
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);

    const chain = getChainConfig(config, options.chainName);
    const [keypair, client] = getWallet(chain, options);
    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts?.AxelarGateway) {
        throw new Error('Axelar Gateway package not found.');
    }

    await processor(keypair, client, config, chain, chain.contracts.AxelarGateway, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('gateway').description('Gateway contract operations.');

    program
        .command('rotate')
        .description('Rotate signers of the gateway contract')
        .addOption(new Option('--signers <signers>', 'JSON with the initial signer set'))
        .addOption(new Option('--proof <proof>', 'JSON of the proof'))
        .addOption(new Option('--currentNonce <currentNonce>', 'nonce of the existing signers'))
        .addOption(new Option('--newNonce <newNonce>', 'nonce of the new signers (useful for test rotations)'))
        .action((options) => {
            mainProcessor(rotate, [], options);
        });

    program
        .command('approve <sourceChain> <messageId> <sourceAddress> <destinationId> <payloadHash>')
        .description('Approve messages at the gateway contract')
        .addOption(new Option('--proof <proof>', 'JSON of the proof'))
        .addOption(new Option('--currentNonce <currentNonce>', 'nonce of the existing signers'))
        .action((sourceChain, messageId, sourceAddress, destinationId, payloadHash, options) => {
            mainProcessor(approve, [sourceChain, messageId, sourceAddress, destinationId, payloadHash], options);
        });

    program
        .command('submitProof <multisigSessionId>')
        .description('Submit proof for the provided amplifier multisig session id')
        .action((multisigSessionId, options) => {
            mainProcessor(submitProof, [multisigSessionId], options);
        });

    program
        .command('call-contract <destinationChain> <destinationAddress> <payload>')
        .description('Initiate sending a cross-chain message via the gateway')
        .addOption(new Option('--channel <channel>', 'Existing channel ID to initiate a cross-chain message over'))
        .action((destinationChain, destinationAddress, payload, options) => {
            mainProcessor(callContract, [destinationChain, destinationAddress, payload], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
