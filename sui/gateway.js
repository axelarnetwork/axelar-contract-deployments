const { saveConfig, printInfo } = require('../evm/utils');
const { Command, Option } = require('commander');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, keccak256, toUtf8Bytes },
    constants: { HashZero },
} = ethers;

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, getRawPrivateKey, broadcast } = require('./sign-utils');
const { bytes32Struct, signersStruct, messageToSignStruct, messageStruct, proofStruct } = require('./types-utils');
const { loadSuiConfig } = require('./utils');
const { getSigners } = require('./deploy-gateway');
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

    const message = messageToSignStruct
        .serialize({
            domain_separator: contractConfig.domainSeparator,
            signers_hash: keccak256(signersStruct.serialize(signers).toBytes()),
            data_hash: dataHash,
        })
        .toBytes();

    const signatures = getSignatures(keypair, message, options);

    const encodedProof = proofStruct
        .serialize({
            signers,
            signatures,
        })
        .toBytes();

    return encodedProof;
}

async function callContract(keypair, client, config, chain, args, options) {
    if (!chain.contracts.axelar_gateway) {
        throw new Error('Axelar Gateway package not found.');
    }

    const contractConfig = chain.contracts.axelar_gateway;
    const packageId = contractConfig.address;

    const [destinationChain, destinationAddress, payload] = args;

    let channel = options.channel;

    const tx = new TransactionBlock();

    // Create a temporary channel if one wasn't provided
    if (!options.channel) {
        [channel] = tx.moveCall({
            target: `${packageId}::channel::new`,
            arguments: [],
        });
    }

    tx.moveCall({
        target: `${packageId}::gateway::call_contract`,
        arguments: [
            channel,
            tx.pure(bcs.string().serialize(destinationChain).toBytes()),
            tx.pure(bcs.string().serialize(destinationAddress).toBytes()),
            tx.pure(bcs.vector(bcs.u8()).serialize(arrayify(payload)).toBytes()),
        ],
    });

    if (!options.channel) {
        tx.moveCall({
            target: `${packageId}::channel::destroy`,
            arguments: [channel],
        });
    }

    await broadcast(client, keypair, tx);

    printInfo('Contract called');
}

async function approveMessages(keypair, client, config, chain, args, options) {
    if (!chain.contracts.axelar_gateway) {
        throw new Error('Axelar Gateway package not found.');
    }

    const contractConfig = chain.contracts.axelar_gateway;
    const packageId = contractConfig.address;
    const [sourceChain, messageId, sourceAddress, destinationId, payloadHash] = args;

    const encodedMessages = bcs
        .vector(messageStruct)
        .serialize([
            {
                source_chain: sourceChain,
                message_id: messageId,
                source_address: sourceAddress,
                destination_id: destinationId,
                payload_hash: bytes32Struct.serialize(arrayify(payloadHash)).toBytes(),
            },
        ])
        .toBytes();

    const encodedProof = getProof(keypair, COMMAND_TYPE_APPROVE_MESSAGES, encodedMessages, contractConfig, options);

    const tx = new TransactionBlock();

    tx.moveCall({
        target: `${packageId}::gateway::approve_messages`,
        arguments: [
            tx.object(contractConfig.objects.gateway),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedMessages).toBytes()),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedProof).toBytes()),
        ],
    });

    await broadcast(client, keypair, tx);

    printInfo('Approved messages');
}

async function rotateSigners(keypair, client, config, chain, args, options) {
    if (!chain.contracts.axelar_gateway) {
        throw new Error('Axelar Gateway package not found.');
    }

    const contractConfig = chain.contracts.axelar_gateway;
    const packageId = contractConfig.address;
    const signers = await getSigners(keypair, config, chain.axelarId, options);

    const newNonce = options.newNonce ? keccak256(toUtf8Bytes(options.newNonce)) : signers.nonce;
    const encodedSigners = signersStruct
        .serialize({
            ...signers,
            nonce: bytes32Struct.serialize(newNonce).toBytes(),
        })
        .toBytes();

    const encodedProof = getProof(keypair, COMMAND_TYPE_ROTATE_SIGNERS, encodedSigners, contractConfig, options);

    const tx = new TransactionBlock();

    tx.moveCall({
        target: `${packageId}::gateway::rotate_signers`,
        arguments: [
            tx.object(contractConfig.objects.gateway),
            tx.object('0x6'),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedSigners).toBytes()),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedProof).toBytes()),
        ],
    });

    await broadcast(client, keypair, tx);

    printInfo('Signers rotated succesfully');
}

async function mainProcessor(processor, args, options) {
    const config = loadSuiConfig(options.env);

    const [keypair, client] = getWallet(config.sui, options);
    await printWalletInfo(keypair, client, config.sui, options);

    await processor(keypair, client, config, config.sui, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('gateway').description('Gateway contract operations.');

    const rotateSignersCmd = program
        .command('rotate')
        .description('Rotate signers of the gateway contract')
        .addOption(new Option('--signers <signers>', 'JSON with the initial signer set'))
        .addOption(new Option('--proof <proof>', 'JSON of the proof'))
        .addOption(new Option('--currentNonce <currentNonce>', 'nonce of the existing signers'))
        .addOption(new Option('--newNonce <newNonce>', 'nonce of the new signers (useful for test rotations)'))
        .action((options) => {
            mainProcessor(rotateSigners, [], options);
        });

    const approveMessagesCmd = program
        .command('approve <sourceChain> <messageId> <sourceAddress> <destinationId> <payloadHash>')
        .description('Approve messages at the gateway contract')
        .addOption(new Option('--proof <proof>', 'JSON of the proof'))
        .addOption(new Option('--currentNonce <currentNonce>', 'nonce of the existing signers'))
        .action((sourceChain, messageId, sourceAddress, destinationId, payloadHash, options) => {
            mainProcessor(approveMessages, [sourceChain, messageId, sourceAddress, destinationId, payloadHash], options);
        });

    const callContractCmd = program
        .command('call-contract <destinationChain> <destinationAddress> <payload>')
        .description('Initiate sending a cross-chain message via the gateway')
        .addOption(new Option('--channel <channel>', 'Existing channel ID to initiate a cross-chain message over'))
        .action((destinationChain, destinationAddress, payload, options) => {
            mainProcessor(callContract, [destinationChain, destinationAddress, payload], options);
        });

    addBaseOptions(program);
    addBaseOptions(rotateSignersCmd);
    addBaseOptions(callContractCmd);
    addBaseOptions(approveMessagesCmd);

    program.parse();
}
