const { Command, Option } = require('commander');
const { Contract, Address, nativeToScVal, xdr } = require('@stellar/stellar-sdk');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, keccak256, id },
} = ethers;

const { saveConfig, loadConfig, addOptionsToCommands, getMultisigProof, printInfo, getChainConfig } = require('../common');
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
    const signersHash = keccak256(signers.toXDR());

    const domainSeparator = chain.contracts.axelar_gateway?.initializeArgs?.domainSeparator;

    if (!domainSeparator) {
        throw new Error('Domain separator not found');
    }

    const msg = '0x' + domainSeparator + signersHash.slice(2) + dataHash.slice(2);
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

    await broadcast(operation, wallet, chain, 'Contract Called', options);
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

    await broadcast(operation, wallet, chain, 'Messages Approved', options);
}

async function validateMessage(wallet, _, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const [sourceChain, messageId, sourceAddress, payload] = args;
    const caller = Address.fromString(wallet.publicKey());
    const callArgs = [caller, messageId, sourceChain, sourceAddress, Buffer.from(arrayify(keccak256(payload)))].map(nativeToScVal);

    const operation = contract.call('validate_message', ...callArgs);

    await broadcast(operation, wallet, chain, 'Message validated', options);
}

async function rotate(wallet, config, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);

    const newSigners = weightedSignersToScVal(await getNewSigners(wallet, config, chain, options));

    const dataHash = encodeDataHash('RotateSigners', newSigners);
    const proof = getProof(dataHash, wallet, chain, options);
    const bypassRotationDelay = nativeToScVal(false); // only operator can bypass rotation delay.

    const operation = contract.call('rotate_signers', newSigners, proof, bypassRotationDelay);

    await broadcast(operation, wallet, chain, 'Signers Rotated', options);
}

async function submitProof(wallet, config, chain, contractConfig, args, options) {
    const contract = new Contract(contractConfig.address);
    const [multisigSessionId] = args;
    const { payload, status } = await getMultisigProof(config, chain.axelarId, multisigSessionId);

    if (!status.completed) {
        throw new Error('Multisig session not completed');
    }

    const executeData = Buffer.from(arrayify('0x' + status.completed.execute_data));
    const [data, proof] = xdr.ScVal.fromXDR(executeData).vec();

    let operation;

    if (payload.verifier_set) {
        printInfo('Submitting rotate_signers');

        operation = contract.call('rotate_signers', data, proof);
    } else if (payload.messages) {
        printInfo('Submitting approve_messages');

        operation = contract.call('approve_messages', data, proof);
    } else {
        throw new Error(`Unknown payload type: ${payload}`);
    }

    await broadcast(operation, wallet, chain, 'Amplifier Proof Submitted', options);
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);

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
        .addOption(new Option('--current-nonce <currentNonce>', 'nonce of the existing signers'))
        .addOption(new Option('--new-nonce <newNonce>', 'nonce of the new signers (useful for test rotations)'))
        .action((options) => {
            mainProcessor(rotate, [], options);
        });

    program
        .command('approve <sourceChain> <messageId> <sourceAddress> <destinationAddress> <payload>')
        .description('Approve messages at the gateway contract')
        .addOption(new Option('--current-nonce <currentNonce>', 'nonce of the existing signers'))
        .action((sourceChain, messageId, sourceAddress, destinationAddress, payload, options) => {
            mainProcessor(approve, [sourceChain, messageId, sourceAddress, destinationAddress, payload], options);
        });

    program
        .command('validate-message <sourceChain> <messageId> <sourceAddress> <payload>')
        .description('Validate an approved message at the gateway contract. The signer will be treated as the destination address')
        .action((sourceChain, messageId, sourceAddress, payload, options) => {
            mainProcessor(validateMessage, [sourceChain, messageId, sourceAddress, payload], options);
        });

    program
        .command('submit-proof <multisigSessionId>')
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
