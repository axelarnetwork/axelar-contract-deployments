const { saveConfig, loadConfig, prompt } = require('../evm/utils');
const { Command, Option } = require('commander');
const { publishPackage, updateMoveToml } = require('@axelar-network/axelar-cgp-sui/scripts/publish-package');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { ethers } = require('hardhat');
const {
    constants: { HashZero },
} = ethers;

const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');



async function processCommand(config, chain, options) {
    const [keypair, client] = await getWallet(chain, options);

    if (!chain.contracts) {
        chain.contracts = {
            axelar_gateway: {},
        };
    }

    const contractConfig = chain.contracts.test;

    if (prompt(`Proceed with deployment on ${chain.name}?`, options.yes)) {
        return;
    }

    const published = await publishPackage('test', client, keypair, parseEnv(options.env));
    updateMoveToml('axelar_gateway', published.packageId);

    const singleton = published.publishTxn.objectChanges.find((change) => change.objectType === `${published.packageId}::test::Singleton`);

    const tx = new TransactionBlock();

    const config = loadConfig(options.env);

    tx.moveCall({
        target: `${published.packageId}::test::register_transaction`,
        arguments: [tx.object(config.sui.relayerDiscovery), tx.object(singleton.objectId)],
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

    contractConfig.singleton = singleton.objectId;
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);

    if (!config.sui) {
        config.sui = {
        networkType: "localnet",
        name: "Sui",
        contracts: {
            "axelar_gateway": {}
        }
    }

    await processor(config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-gateway').description('Deploys/publishes the Sui gateway');

    addBaseOptions(program);

    program.addOption(new Option('--signers <signers>', 'JSON with the initial signer set').env('SIGNERS'));
    program.addOption(new Option('--operator <operator>', 'operator for the gateway (defaults to the deployer address)'));
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
