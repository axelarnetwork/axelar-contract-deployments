const { saveConfig, prompt, printInfo, validateParameters, writeJSON } = require('../evm/utils');
const { Command, Option } = require('commander');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, hexlify, toUtf8Bytes, keccak256 },
    constants: { HashZero },
} = ethers;

const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo, broadcast } = require('./sign-utils');
const { bytes32Struct, signersStruct } = require('./types-utils');
const { getAmplifierSigners, loadSuiConfig, deployPackage } = require('./utils');
const { upgradePackage } = require('./deploy-utils');
const { toB64 } = require('@mysten/sui.js/utils');

async function getSigners(keypair, config, chain, options) {
    if (options.signers === 'wallet') {
        const pubKey = keypair.getPublicKey().toRawBytes();
        printInfo('Using wallet pubkey as the signer for the gateway', hexlify(pubKey));

        if (keypair.getKeyScheme() !== 'Secp256k1') {
            throw new Error('Only Secp256k1 pubkeys are supported by the gateway');
        }

        return {
            signers: [{ pub_key: pubKey, weight: 1 }],
            threshold: 1,
            nonce: options.nonce ? keccak256(toUtf8Bytes(options.nonce)) : HashZero,
        };
    } else if (options.signers) {
        printInfo('Using provided signers', options.signers);

        const signers = JSON.parse(options.signers);
        return {
            signers: signers.signers.map(({ pub_key: pubKey, weight }) => {
                return { pub_key: arrayify(pubKey), weight };
            }),
            threshold: signers.threshold,
            nonce: arrayify(signers.nonce) || HashZero,
        };
    }

    return getAmplifierSigners(config, chain);
}

async function deployGateway(config, chain, options, keypair, client) {
    const contractConfig = chain.contracts.axelar_gateway;
    const { minimumRotationDelay, domainSeparator } = options;
    const signers = await getSigners(keypair, config, chain, options);
    const operator = options.operator || keypair.toSuiAddress();
    const { previousSigners } = options;

    validateParameters({ isNonEmptyString: { previousSigners } });

    if (prompt(`Proceed with deployment on ${chain.name}?`, options.yes)) {
        return;
    }

    const { packageId, publishTxn } = await deployPackage('axelar_gateway', client, keypair);

    const creatorCap = publishTxn.objectChanges.find((change) => change.objectType === `${packageId}::gateway::CreatorCap`);
    const relayerDiscovery = publishTxn.objectChanges.find((change) => change.objectType === `${packageId}::discovery::RelayerDiscovery`);

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
            tx.pure(options.previousSigners),
            tx.pure(bcs.vector(bcs.u8()).serialize(encodedSigners).toBytes()),
            tx.object('0x6'),
        ],
    });
    const result = await broadcast(client, keypair, tx);

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

async function upgradeGateway(chain, options, keypair, client) {
    const contractsConfig = chain.contracts;
    const packageName = 'axelar_gateway';
    const builder = new TxBuilder(client);
    contractsConfig[packageName].objects.UpgradeCap = options.upgradeCap;

    await upgradePackage(client, keypair, packageName, contractsConfig[packageName], builder, options);
}

async function processCommand(config, chain, options) {
    const [keypair, client] = getWallet(chain, options);
    await printWalletInfo(keypair, client, chain, options);

    chain.contracts.axelar_gateway = chain.contracts.axelar_gateway ?? {};

    if (!options.upgrade) {
        await deployGateway(config, chain, options, keypair, client);
    } else {
        await upgradeGateway(chain, options, keypair, client);
    }

    if (options.offline) {
        const { txFilePath } = options;
        validateParameters({ isNonEmptyString: { txFilePath } });

        const txB64Bytes = toB64(options.txBytes);

        writeJSON({ status: 'PENDING', bytes: txB64Bytes }, txFilePath);
        printInfo(`The unsigned transaction is`, txB64Bytes);
    }
}

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);

    await processor(config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-gateway').description('Deploys/Upgrades the Sui gateway');

    addBaseOptions(program);

    program.addOption(new Option('--signers <signers>', 'JSON with the initial signer set').env('SIGNERS'));
    program.addOption(new Option('--operator <operator>', 'operator for the gateway (defaults to the deployer address)').env('OPERATOR'));
    program.addOption(new Option('--minimumRotationDelay <minimumRotationDelay>', 'minium delay for signer rotations (in ms)').default(0));
    program.addOption(new Option('--domainSeparator <domainSeparator>', 'domain separator').default(HashZero));
    program.addOption(new Option('--nonce <nonce>', 'nonce for the signer (defaults to HashZero)'));
    program.addOption(new Option('--previousSigners <previousSigners>', 'number of previous signers to retain').default(0));
    program.addOption(new Option('--upgradeCap <upgradeCap>', 'gateway UpgradeCap id'));
    program.addOption(new Option('--upgrade', 'upgrade a deployed contract'));
    program.addOption(new Option('--policy <policy>', 'new policy to upgrade'));
    program.addOption(new Option('--sender <sender>', 'transaction sender'));
    program.addOption(new Option('--digest <digest>', 'digest hash for upgrade'));
    program.addOption(new Option('--offline', 'store tx block for sign'));
    program.addOption(new Option('--txFilePath <file>', 'unsigned transaction will be stored'));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}

module.exports = {
    getSigners,
};
