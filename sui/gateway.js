const { Command, Option } = require('commander');
const { Transaction } = require('@mysten/sui/transactions');
const { bcs } = require('@mysten/sui/bcs');
const { ethers } = require('hardhat');
const { bcsStructs, CLOCK_PACKAGE_ID } = require('@axelar-network/axelar-cgp-sui');
const {
    utils: { arrayify, keccak256, toUtf8Bytes },
    constants: { HashZero },
} = ethers;

const { saveConfig, printInfo, loadConfig, getMultisigProof, getChainConfig, writeJSON } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    getSigners,
    getWallet,
    printWalletInfo,
    getRawPrivateKey,
    broadcast,
    suiClockAddress,
    saveGeneratedTx,
    isAllowed,
} = require('./utils');
const secp256k1 = require('secp256k1');
const chalk = require('chalk');
const { readJSON } = require(`${__dirname}/../axelar-chains-config`);

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

    return {
        tx,
        message: 'Message sent',
    };
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

    return {
        tx,
        message: 'Approved Messages',
    };
}

async function migrate(keypair, client, config, chain, contractConfig, args, options) {
    const packageId = contractConfig.address;
    const data = new Uint8Array(arrayify(options.migrateData || '0x'));
    const tx = new Transaction();

    tx.moveCall({
        target: `${packageId}::gateway::migrate`,
        arguments: [
            tx.object(contractConfig.objects.Gateway),
            tx.object(contractConfig.objects.OwnerCap),
            tx.pure(bcs.vector(bcs.u8()).serialize(data).toBytes()),
        ],
    });

    return {
        tx,
        message: 'Migrate',
    };
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

    return {
        tx,
        message: 'Submitted Amplifier Proof',
    };
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

    return {
        tx,
        message: 'Rotated Signers',
    };
}

async function allowFunctions(keypair, client, config, chain, contractConfig, args, options) {
    const packageId = contractConfig.address;

    const [versionsArg, functionNamesArg] = args;

    const versions = versionsArg.split(',');
    const functionNames = functionNamesArg.split(',');

    if (versions.length !== functionNames.length) throw new Error('Versions and Function Names must have a matching length');

    const tx = new Transaction();

    for (const i in versions) {
        tx.moveCall({
            target: `${packageId}::gateway::allow_function`,
            arguments: [
                tx.object(contractConfig.objects.Gateway),
                tx.object(contractConfig.objects.OwnerCap),
                tx.pure.u64(versions[i]),
                tx.pure.string(functionNames[i]),
            ],
        });
    }

    return {
        tx,
        message: 'Allow Functions',
    };
}

async function disallowFunctions(keypair, client, config, chain, contractConfig, args, options) {
    const packageId = contractConfig.address;

    const [versionsArg, functionNamesArg] = args;

    const versions = versionsArg.split(',');
    const functionNames = functionNamesArg.split(',');

    if (versions.length !== functionNames.length) throw new Error('Versions and Function Names must have a matching length');

    const tx = new Transaction();

    for (const i in versions) {
        tx.moveCall({
            target: `${packageId}::gateway::disallow_function`,
            arguments: [
                tx.object(contractConfig.objects.Gateway),
                tx.object(contractConfig.objects.OwnerCap),
                tx.pure.u64(versions[i]),
                tx.pure.string(functionNames[i]),
            ],
        });
    }

    return {
        tx,
        message: 'Disallow Functions',
    };
}

async function checkVersionControl(version, options) {
    const config = loadConfig(options.env);

    const chain = getChainConfig(config, options.chainName);
    const [keypair, client] = getWallet(chain, options);
    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts?.AxelarGateway) {
        throw new Error('Axelar Gateway package not found.');
    }

    const contractConfig = chain.contracts.AxelarGateway;
    const packageId = contractConfig.versions[version];

    const functions = {};
    functions.approve_messages = (tx) =>
        tx.moveCall({
            target: `${packageId}::gateway::approve_messages`,
            arguments: [tx.object(contractConfig.objects.Gateway), tx.pure.vector('u8', []), tx.pure.vector('u8', [])],
        });
    functions.rotate_signers = (tx) =>
        tx.moveCall({
            target: `${packageId}::gateway::rotate_signers`,
            arguments: [
                tx.object(contractConfig.objects.Gateway),
                tx.object(CLOCK_PACKAGE_ID),
                tx.pure.vector('u8', []),
                tx.pure.vector('u8', []),
            ],
        });
    functions.is_message_approved = (tx) =>
        tx.moveCall({
            target: `${packageId}::gateway::is_message_approved`,
            arguments: [
                tx.object(contractConfig.objects.Gateway),
                tx.pure.string(''),
                tx.pure.string(''),
                tx.pure.string(''),
                tx.pure.address('0x0'),
                tx.moveCall({
                    target: `${packageId}::bytes32::from_address`,
                    arguments: [tx.pure.address('0x0')],
                }),
            ],
        });
    functions.is_message_executed = (tx) =>
        tx.moveCall({
            target: `${packageId}::gateway::is_message_executed`,
            arguments: [tx.object(contractConfig.objects.Gateway), tx.pure.string(''), tx.pure.string('')],
        });
    functions.take_approved_message = (tx) =>
        tx.moveCall({
            target: `${packageId}::gateway::take_approved_message`,
            arguments: [
                tx.object(contractConfig.objects.Gateway),
                tx.pure.string(''),
                tx.pure.string(''),
                tx.pure.string(''),
                tx.pure.address('0x0'),
                tx.pure.vector('u8', []),
            ],
        });

    functions.send_message = (tx) => {
        const channel = tx.moveCall({
            target: `${packageId}::channel::new`,
            arguments: [],
        });

        const message = tx.moveCall({
            target: `${packageId}::gateway::prepare_message`,
            arguments: [channel, tx.pure.string(''), tx.pure.string(''), tx.pure.vector('u8', [])],
        });

        tx.moveCall({
            target: `${packageId}::gateway::send_message`,
            arguments: [tx.object(contractConfig.objects.Gateway), message],
        });

        tx.moveCall({
            target: `${packageId}::channel::destroy`,
            arguments: [channel],
        });
    };

    functions.allow_function = (tx) =>
        tx.moveCall({
            target: `${packageId}::gateway::allow_function`,
            arguments: [
                tx.object(contractConfig.objects.Gateway),
                tx.object(contractConfig.objects.OwnerCap),
                tx.pure.u64(0),
                tx.pure.string(''),
            ],
        });
    functions.disallow_function = (tx) =>
        tx.moveCall({
            target: `${packageId}::gateway::disallow_function`,
            arguments: [
                tx.object(contractConfig.objects.Gateway),
                tx.object(contractConfig.objects.OwnerCap),
                tx.pure.u64(0),
                tx.pure.string(''),
            ],
        });

    if (options.allowedFunctions) {
        const allowedFunctions = options.allowedFunctions === 'all' ? Object.keys(functions) : options.allowedFunctions.split(',');

        for (const allowedFunction of allowedFunctions) {
            const allowed = await isAllowed(client, keypair, chain, functions[allowedFunction]);
            const color = allowed ? chalk.green : chalk.red;
            console.log(`${allowedFunction} is ${color(allowed ? 'allowed' : 'dissalowed')}`);
        }
    }

    if (options.disallowedFunctions) {
        const disallowedFunctions = options.allowedFunctions === 'all' ? Object.keys(functions) : options.disallowedFunctions.split(',');

        for (const disallowedFunction of disallowedFunctions) {
            const allowed = await isAllowed(client, keypair, chain, functions[disallowedFunction]);
            const color = allowed ? chalk.red : chalk.green;
            console.log(`${disallowedFunction} is ${color(allowed ? 'allowed' : 'dissalowed')}`);
        }
    }
}

async function testNewField(value, options) {
    const config = loadConfig(options.env);

    const chain = getChainConfig(config, options.chainName);
    const [keypair, client] = getWallet(chain, options);
    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts?.AxelarGateway) {
        throw new Error('Axelar Gateway package not found.');
    }

    const contractConfig = chain.contracts.AxelarGateway;
    const packageId = contractConfig.address;

    let tx = new Transaction();

    tx.moveCall({
        target: `${packageId}::gateway::set_new_field`,
        arguments: [tx.object(contractConfig.objects.Gateway), tx.pure.u64(value)],
    });

    await broadcast(client, keypair, tx, 'Set new_field');
    await new Promise((resolve) => setTimeout(resolve, 1000));

    tx = new Transaction();

    tx.moveCall({
        target: `${packageId}::gateway::new_field`,
        arguments: [tx.object(contractConfig.objects.Gateway)],
    });

    const response = await client.devInspectTransactionBlock({
        transactionBlock: tx,
        sender: keypair.toSuiAddress(),
    });
    const returnedValue = bcs.U64.parse(new Uint8Array(response.results[0].returnValues[0][0]));
    console.log(`Set the value to ${value} and it was set to ${returnedValue}.`);
}

async function pause(keypair, client, config, chain, contracts, args, options) {
    const response = await client.getObject({
        id: contracts.objects.Gatewayv0,
        options: {
            showContent: true,
            showBcs: true,
        },
    });
    let allowedFunctionsArray = response.data.content.fields.value.fields.version_control.fields.allowed_functions;
    allowedFunctionsArray = allowedFunctionsArray.map((allowedFunctions) => allowedFunctions.fields.contents);

    const versionsArg = [];
    const allowedFunctionsArg = [];

    for (const version in allowedFunctionsArray) {
        const allowedFunctions = allowedFunctionsArray[version];

        // Do not dissalow `allow_function` because that locks the gateway forever.
        if (Number(version) === allowedFunctionsArray.length - 1) {
            const index = allowedFunctions.indexOf('allow_function');

            if (index > -1) {
                // only splice array when item is found
                allowedFunctions.splice(index, 1); // 2nd parameter means remove one item only
            }
        }

        printInfo(`Functions that will be disallowed for version ${version}`, allowedFunctions);

        versionsArg.push(new Array(allowedFunctions.length).fill(version).join());
        allowedFunctionsArg.push(allowedFunctions.join());
    }

    // Write the
    writeJSON(
        {
            versions: versionsArg,
            disallowedFunctions: allowedFunctionsArg,
        },
        `${__dirname}/../axelar-chains-config/info/sui-gateway-allowed-functions-${options.env}.json`,
    );

    return disallowFunctions(keypair, client, config, chain, contracts, [versionsArg.join(), allowedFunctionsArg.join()], options);
}

async function unpause(keypair, client, config, chain, contracts, args, options) {
    const dissalowedFunctions = readJSON(`${__dirname}/../axelar-chains-config/info/sui-gateway-allowed-functions-${options.env}.json`);

    return allowFunctions(
        keypair,
        client,
        config,
        chain,
        contracts,
        [dissalowedFunctions.versions.join(), dissalowedFunctions.disallowedFunctions.join()],
        options,
    );
}

async function mainProcessor(processor, args, options) {
    const config = loadConfig(options.env);

    const chain = getChainConfig(config, options.chainName);
    const [keypair, client] = getWallet(chain, options);
    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts?.AxelarGateway) {
        throw new Error('Axelar Gateway package not found.');
    }

    const { tx, message } = await processor(keypair, client, config, chain, chain.contracts.AxelarGateway, args, options);

    saveConfig(config, options.env);

    if (options.offline) {
        const sender = options.sender || keypair.toSuiAddress();
        tx.setSender(sender);
        await saveGeneratedTx(tx, message, client, options);
    } else {
        await broadcast(client, keypair, tx, message);
    }
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
        .command('migrate')
        .description('Migrate the gateway after upgrade')
        .addOption(new Option('--migrate-data <migrateData>', 'bcs encoded data to pass to the migrate function'))
        .action((options) => {
            mainProcessor(migrate, null, options);
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

    program
        .command('allow-functions <versions> <functionNames>')
        .description('Allow certain funcitons on the gateway')
        .action((versions, functionNames, options) => {
            mainProcessor(allowFunctions, [versions, functionNames], options);
        });

    program
        .command('disallow-functions <versions> <functionNames>')
        .description('Allow certain funcitons on the gateway')
        .action((versions, functionNames, options) => {
            mainProcessor(disallowFunctions, [versions, functionNames], options);
        });

    program
        .command('check-version-control <version>')
        .description('Check if version control works on a certain version')
        .addOption(new Option('--allowed-functions <allowed-functions>', 'Functions that should be allowed on this version'))
        .addOption(new Option('--disallowed-functions <disallowed-functions>', 'Functions that should be disallowed on this version'))
        .action((version, options) => {
            checkVersionControl(version, options);
        });

    program
        .command('test-new-field <value>')
        .description('Test the new field added for upgrade-versioned')
        .action((value, options) => {
            testNewField(value, options);
        });

    program
        .command('pause')
        .description('Pause the gateway')
        .action((options) => {
            mainProcessor(pause, [], options);
        });

    program
        .command('unpause')
        .description('Unpause the gateway')
        .action((options) => {
            mainProcessor(unpause, [], options);
        });

    addOptionsToCommands(program, addBaseOptions, { offline: true });

    program.parse();
}
