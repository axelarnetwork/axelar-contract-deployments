const { saveConfig, printInfo } = require('../evm/utils');
const { Command, Option } = require('commander');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, hexlify, keccak256, toUtf8Bytes },
    constants: { HashZero },
} = ethers;

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, getRawPrivateKey } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');
const { getSigners } = require('./deploy-gateway');
const secp256k1 = require('secp256k1');

const COMMAND_TYPE_ROTATE_SIGNERS = 1;

function hashMessage(data) {
    const toHash = new Uint8Array(data.length + 1);
    toHash[0] = COMMAND_TYPE_ROTATE_SIGNERS;
    toHash.set(data, 1);

    return keccak256(toHash);
}

function getProofSigners(keypair, options) {
    if (options.proof === 'wallet') {
        console.log('Using wallet to provide proof');

        if (keypair.getKeyScheme() !== 'Secp256k1') {
            throw new Error('Only Secp256k1 pubkeys are supported by the gateway');
        }

        return {
            signers: [{ pubkey: keypair.getPublicKey().toRawBytes(), weight: 1 }],
            threshold: 1,
            nonce: options.currentNonce ? keccak256(toUtf8Bytes(options.currentNonce)) : HashZero,
        };
    } else if (options.proof) {
        printInfo('Using provided proof', options.proof);

        const proof = JSON.parse(options.proof);
        return {
            signers: proof.signers.signers.map(({ pubkey, weight }) => {
                return { pubkey: arrayify(pubkey), weight };
            }),
            threshold: proof.signers.threshold,
            nonce: arrayify(proof.signers.nonce) || HashZero,
        };
    }

    throw new Error('Proof not found');
}

function getSignatures(keypair, options, messageToSign) {
    if (options.proof === 'wallet') {
        if (keypair.getKeyScheme() !== 'Secp256k1') {
            throw new Error('Only Secp256k1 pubkeys are supported by the gateway');
        }

        const { signature, recid } = secp256k1.ecdsaSign(arrayify(keccak256(messageToSign)), getRawPrivateKey(keypair));

        return [new Uint8Array([...signature, recid])];
    } else if (options.proof) {
        const proof = JSON.parse(options.proof);
        return proof.signatures.map((signatrue) => arrayify(signatrue));
    }

    throw new Error('Proof not found');
}

async function processCommand(config, chain, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts.axelar_gateway) {
        throw new Error('Axelar Gateway package not found.');
    }

    const contractConfig = chain.contracts.axelar_gateway;
    const packageId = contractConfig.address;
    const signers = await getSigners(keypair, config, chain, options);

    const signerStruct = bcs.struct('WeightedSigner', {
        pubkey: bcs.vector(bcs.u8()),
        weight: bcs.u128(),
    });
    const bytes32Struct = bcs.fixedArray(32, bcs.u8()).transform({
        input: (id) => arrayify(id),
        output: (id) => hexlify(id),
    });

    const signersStruct = bcs.struct('WeightedSigners', {
        signers: bcs.vector(signerStruct),
        threshold: bcs.u128(),
        nonce: bytes32Struct,
    });
    const newNonce = options.newNonce ? keccak256(toUtf8Bytes(options.newNonce)) : signers.nonce;
    const encodedSigners = signersStruct
        .serialize({
            ...signers,
            nonce: bytes32Struct.serialize(newNonce).toBytes(),
        })
        .toBytes();

    const proofSigners = getProofSigners(keypair, options);

    const hashed = arrayify(hashMessage(encodedSigners));

    const messageToSignStruct = bcs.struct('MessageToSign', {
        domain_separator: bytes32Struct,
        signers_hash: bytes32Struct,
        data_hash: bytes32Struct,
    });

    const message = messageToSignStruct
        .serialize({
            domain_separator: contractConfig.domainSeparator,
            signers_hash: keccak256(signersStruct.serialize(proofSigners).toBytes()),
            data_hash: hashed,
        })
        .toBytes();

    const signatures = getSignatures(keypair, options, message);

    const proofStruct = bcs.struct('Proof', {
        signers: signersStruct,
        signatures: bcs.vector(bcs.vector(bcs.u8())),
    });

    const encodedProof = proofStruct
        .serialize({
            signers: proofSigners,
            signatures,
        })
        .toBytes();

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

    await client.signAndExecuteTransactionBlock({
        transactionBlock: tx,
        signer: keypair,
        options: {
            showEffects: true,
            showObjectChanges: true,
            showContent: true,
        },
    });

    printInfo('Signers rotated succesfully');
}

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);

    await processor(config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('rotate-signers').description('Rotates signers on the gateway contract.');

    addBaseOptions(program);

    program.addOption(new Option('--signers <signers>', 'JSON with the initial signer set').makeOptionMandatory(true).env('SIGNERS'));
    program.addOption(new Option('--proof <proof>', 'JSON of the proof').env('PROOF'));
    program.addOption(new Option('--currentNonce <currentNonce>', 'nonce of the existing signers'));
    program.addOption(new Option('--newNonce <newNonce>', 'nonce of the new signers (useful for test rotations)'));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
