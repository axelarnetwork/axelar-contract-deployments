const { saveConfig, printInfo } = require('../evm/utils');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, keccak256 },
    constants: { HashZero },
} = ethers;

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, getRawPrivateKey } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');
const { getSigners } = require('./deploy-gateway');
const secp256k1 = require('secp256k1');
const { TxBuilder } = require('@axelar-network/axelar-cgp-sui/scripts/tx-builder');
const { axelarStructs } = require('@axelar-network/axelar-cgp-sui/scripts/bcs');

const COMMAND_TYPE_ROTATE_SIGNERS = 1;

function hashMessage(data) {
    const toHash = new Uint8Array(data.length + 1);
    toHash[0] = COMMAND_TYPE_ROTATE_SIGNERS;
    toHash.set(data, 1);
``
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
            nonce: HashZero,
        };
    } else if (options.proof) {
        printInfo('Using provided proof', options.proof);

        const proof = JSON.parse(options.proof);
        return {
            signers: proof.signers.signers.map(({ pubkey, weight }) => {
                return { pubkey, weight };
            }),
            threshold: proof.signers.threshold,
            nonce: proof.signers.nonce || HashZero,
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
        return proof.signatures;
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

    const encodedSigners = axelarStructs.WeightedSigners
        .serialize(signers)
        .toBytes();

    const proofSigners = getProofSigners(keypair, options);

    const hashed = hashMessage(encodedSigners);

    const message = axelarStructs.MessageToSign
        .serialize({
            domain_separator: contractConfig.domainSeparator,
            signers_hash: keccak256(axelarStructs.WeightedSigners.serialize(proofSigners).toBytes()),
            data_hash: hashed,
        })
        .toBytes();

    const signatures = getSignatures(keypair, options, message);
        console.log(proofSigners);
    const encodedProof = axelarStructs.Proof
        .serialize({
            signers: proofSigners,
            signatures,
        })
        .toBytes();

    const builder = new TxBuilder(client);

    await builder.moveCall({
        target: `${packageId}::gateway::rotate_signers`,
        arguments: [
            contractConfig.objects.gateway,
            '0x6',
            encodedSigners,
            encodedProof,
        ],
    });

    await builder.signAndExecute(keypair);

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

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
