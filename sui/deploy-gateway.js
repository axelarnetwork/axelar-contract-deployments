const { saveConfig, prompt, printInfo } = require('../evm/utils');
const { Command, Option } = require('commander');
const { publishPackage, updateMoveToml } = require('@axelar-network/axelar-cgp-sui/scripts/publish-package');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, hexlify },
    constants: { HashZero },
} = ethers;

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo } = require('./sign-utils');
const { getAmplifierSigners, loadSuiConfig } = require('./utils');

async function getSigners(config, chain, options) {
    if (options.signers) {
        printInfo('Using provided signers', options.signers);

        const signers = JSON.parse(options.signers);
        return {
            signers: signers.signers.map(({ pubkey, weight }) => {
                return { pubkey: arrayify(pubkey), weight };
            }),
            threshold: signers.threshold,
            nonce: signers.nonce || HashZero,
        };
    }

    return getAmplifierSigners(config, chain);
}

async function processCommand(config, chain, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    if (!chain.contracts.axelar_gateway) {
        chain.contracts.axelar_gateway = {};
    }

    const contractConfig = chain.contracts.axelar_gateway;
    const { minimumRotationDelay, domainSeparator } = options;
    const signers = await getSigners(config, chain, options);
    const operator = options.operator || keypair.toSuiAddress();

    if (prompt(`Proceed with deployment on ${chain.name}?`, options.yes)) {
        return;
    }

    const published = await publishPackage('axelar_gateway', client, keypair);
    const packageId = published.packageId;

    updateMoveToml('axelar_gateway', packageId);

    const creatorCap = published.publishTxn.objectChanges.find((change) => change.objectType === `${packageId}::gateway::CreatorCap`);
    const relayerDiscovery = published.publishTxn.objectChanges.find(
        (change) => change.objectType === `${packageId}::discovery::RelayerDiscovery`,
    );

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

    const encodedSigners = signersStruct
        .serialize({
            ...signers,
            nonce: bytes32Struct.serialize(signers.nonce).toBytes(),
        })
        .toBytes();

    const tx = new TransactionBlock();

    const separator = tx.moveCall({
        target: `${packageId}::bytes32::new`,
        arguments: [tx.pure(arrayify(domainSeparator))],
    });

    tx.moveCall({
        target: `${packageId}::gateway::setup`,
        arguments: [
            tx.object(creatorCap.objectId),
            tx.pure.address(operator),
            separator,
            tx.pure(minimumRotationDelay),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedSigners).toBytes()),
            tx.object('0x6'),
        ],
    });
    const result = await client.signAndExecuteTransactionBlock({
        transactionBlock: tx,
        signer: keypair,
        options: {
            showEffects: true,
            showObjectChanges: true,
            showContent: true,
        },
    });

    const gateway = result.objectChanges.find((change) => change.objectType === `${packageId}::gateway::Gateway`);

    contractConfig.address = packageId;
    contractConfig.objects = {
        gateway: gateway.objectId,
        relayerDiscovery: relayerDiscovery.objectId,
    };
    contractConfig.domainSeparator = domainSeparator;
    contractConfig.operator = operator;
    contractConfig.minimumRotationDelay = minimumRotationDelay;

    printInfo('Gateway deployed', JSON.stringify(contractConfig, null, 2));
}

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);

    await processor(config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-gateway').description('Deploys/publishes the Sui gateway');

    addBaseOptions(program);

    program.addOption(new Option('--signers <signers>', 'JSON with the initial signer set').env('SIGNERS'));
    program.addOption(new Option('--operator <operator>', 'operator for the gateway (defaults to the deployer address)').env('OPERATOR'));
    program.addOption(
        new Option('--minimumRotationDelay <minimumRotationDelay>', 'minium delay for signer rotations (in ms)').default(
            24 * 60 * 60 * 1000,
        ),
    ); // 1 day (in ms)
    program.addOption(new Option('--domainSeparator <domainSeparator>', 'domain separator').default(HashZero));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
