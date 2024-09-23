const { Command, Option } = require('commander');
const { Contract, Address, nativeToScVal } = require('@stellar/stellar-sdk');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, keccak256, id },
} = ethers;

const { saveConfig, loadConfig, addOptionsToCommands } = require('../common');
const { addBaseOptions, getWallet, broadcast, getAmplifierVerifiers } = require('./utils');
const { messagesToScVal, commandTypeToScVal, proofToScVal, weightedSignersToScVal } = require('./type-utils');

const getNewSigners = async (wallet, config, chain, options) => {
    if (options.signers === 'wallet') {
        return {
            nonce: options.newNonce ? arrayify(id(options.newNonce)) : Array(32).fill(0),
            signers: [
                {
                    signer: wallet.publicKey(),
                    weight: 1,
                },
            ],
            threshold: 1,
        };
    }

    return await getAmplifierVerifiers(config, chain.axelarId);
};

function encodeDataHash(commandType, command) {
    const data = nativeToScVal([commandTypeToScVal(commandType), command]);
    return keccak256('0x' + data.toXDR('hex'));
}

function getProof(dataHash, wallet, chain, options) {
    const nonce = options.currentNonce ? arrayify(id(options.currentNonce)) : Array(32).fill(0);
    const signers = weightedSignersToScVal({
        nonce,
        signers: [
            {
                signer: wallet.publicKey(),
                weight: 1,
            },
        ],
        threshold: 1,
    });
    const signerHash = keccak256(signers.toXDR());

    const domainSeparator = chain.contracts.axelar_auth_verifier?.initializeArgs?.domainSeparator;

    if (!domainSeparator) {
        throw new Error('Domain separator not found');
    }

    const msg = '0x' + domainSeparator + signerHash.slice(2) + dataHash.slice(2);
    const messageHash = keccak256(msg);
    const signature = wallet.sign(arrayify(messageHash));

    return proofToScVal({
        signers: [
            {
                signer: {
                    signer: wallet.publicKey(),
                    weight: 1,
                },
                signature,
            },
        ],
        threshold: 1,
        nonce,
    });
}

async function callContract(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const caller = nativeToScVal(Address.fromString(wallet.publicKey()), { type: 'address' });

    const [destinationChain, destinationAddress, payload] = args;

    const operation = contract.call(
        'call_contract',
        caller,
        nativeToScVal(destinationChain, { type: 'string' }),
        nativeToScVal(destinationAddress, { type: 'string' }),
        nativeToScVal(Buffer.from(arrayify(payload)), { type: 'bytes' }),
    );

    await broadcast(operation, wallet, chain, 'Contract called', options);
}

async function approve(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const [sourceChain, messageId, sourceAddress, destinationAddress, payload] = args;

    const messages = messagesToScVal([
        {
            messageId,
            sourceChain,
            sourceAddress,
            contractAddress: destinationAddress === 'wallet' ? wallet.publicKey() : destinationAddress,
            payloadHash: arrayify(keccak256(payload)),
        },
    ]);

    const dataHash = encodeDataHash('ApproveMessages', messages);
    const proof = getProof(dataHash, wallet, chain, options);

    const operation = contract.call('approve_messages', messages, proof);

    await broadcast(operation, wallet, chain, 'Approved Messages', options);
}

async function rotate(wallet, config, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);

    const newSigners = weightedSignersToScVal(await getNewSigners(wallet, config, chain, options));

    const dataHash = encodeDataHash('RotateSigners', newSigners);
    const proof = getProof(dataHash, wallet, chain, options);

    const operation = contract.call('rotate_signers', newSigners, proof);

    await broadcast(operation, wallet, chain, 'Rotated signers', options);
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);
    const chain = config.stellar;

    const wallet = await getWallet(chain, options);

    if (!chain.contracts?.axelar_gateway) {
        throw new Error('Axelar Gateway package not found.');
    }

    await processor(wallet, config, chain, chain.contracts.axelar_gateway, args, options);

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('gateway').description('Gateway contract operations.');

    program
        .command('rotate')
        .description('Rotate signers of the gateway contract')
        .addOption(new Option('--signers <signers>', 'Either use `wallet` or provide a JSON with the new signer set'))
        .addOption(new Option('--currentNonce <currentNonce>', 'nonce of the existing signers'))
        .addOption(new Option('--newNonce <newNonce>', 'nonce of the new signers (useful for test rotations)'))
        .action((options) => {
            mainProcessor(rotate, [], options);
        });

    program
        .command('approve <sourceChain> <messageId> <sourceAddress> <destinationAddress> <payload>')
        .description('Approve messages at the gateway contract')
        .addOption(new Option('--currentNonce <currentNonce>', 'nonce of the existing signers'))
        .action((sourceChain, messageId, sourceAddress, destinationAddress, payload, options) => {
            mainProcessor(approve, [sourceChain, messageId, sourceAddress, destinationAddress, payload], options);
        });

    program
        .command('submitProof <multisigSessionId>')
        .description('Submit proof for the provided amplifier multisig session id')
        .action((multisigSessionId, options) => {
            mainProcessor(undefined, [multisigSessionId], options);
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
