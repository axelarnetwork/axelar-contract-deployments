const { SuiClient, getFullnodeUrl } = require('@mysten/sui.js/client');
const { Ed25519Keypair } = require('@mysten/sui.js/keypairs/ed25519');
const { saveConfig, loadConfig } = require('../evm/utils');
const { getConfig, parseEnv } = require('@axelar-network/axelar-cgp-sui/scripts/utils');
const { Command, Option } = require('commander');
const { publishPackage } = require('@axelar-network/axelar-cgp-sui/scripts/publish-package');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, hexlify },
} = ethers;

const { requestSuiFromFaucetV0 } = require('@mysten/sui.js/faucet');

async function main(options) {
    options.validatorAddresses = JSON.parse(options.validatorAddresses);
    options.weights = JSON.parse(options.weights);
    options.validators = options.validatorAddresses.map((val, index) => {
        return { signer: arrayify(val), weight: options.weights[index] };
    });
    options.threshold = JSON.parse(options.threshold);
    const privKey = Buffer.from(options.privateKey, 'hex');
    const keypair = Ed25519Keypair.fromSecretKey(privKey);
    const client = new SuiClient({ url: getFullnodeUrl(options.env) });

    if (options.faucetUrl) {
        await requestSuiFromFaucetV0({
            host: options.faucetUrl,
            recipient: keypair.toSuiAddress(),
        });
    }

    const published = await publishPackage('test', client, keypair, parseEnv(options.env));

    const singleton = published.publishTxn.objectChanges.find((change) => change.objectType === `${published.packageId}::test::Singleton`);

    const tx = new TransactionBlock();

    const config = loadConfig(options.env);

    tx.moveCall({
        target: `${published.packageId}::test::register_transaction`,
        arguments: [tx.object(config.sui.relayerDiscovery), tx.object(singleton.objectId)],
    });

    const publishTxn = await client.signAndExecuteTransactionBlock({
        transactionBlock: tx,
        signer: keypair,
        options: {
            showEffects: true,
            showObjectChanges: true,
            showContent: true,
        },
    });

    config.sui.testSingleton = singleton.objectId;

    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('publish-sui-gateway-v2').description('Publish sui gateway v2');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['localnet', 'devnet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('SUI_PRIVATE_KEY'));

    program.addOption(
        new Option('--validatorAddresses <validatorAddresses>', 'addresses of the intiial validator set')
            .makeOptionMandatory(true)
            .env('SUI_INITIAL_VALIDATOR_ADDRESSES'),
    );
    program.addOption(
        new Option('--weights <validatorWeights>', 'wieghts of the intiial validator set')
            .makeOptionMandatory(true)
            .env('SUI_INITIAL_VALIDATOR_WEIGHTS'),
    );
    program.addOption(
        new Option('--threshold <threshold>', 'threshold for the intiial validator set')
            .makeOptionMandatory(true)
            .env('SUI_INITIAL_VALIDATOR_THRESHOLD'),
    );
    program.addOption(
        new Option('--nonce <nonce>', 'nonce for the intiial validator set').makeOptionMandatory(true).env('SUI_INITIAL_NONCE'),
    );
    program.addOption(new Option('--operator <operator>', 'operator for the sui gateway').makeOptionMandatory(true).env('SUI_OPERATOR'));
    program.addOption(
        new Option('--rotationDelay <rotationDelay>', 'minimum rotation delay for validators')
            .makeOptionMandatory(true)
            .env('SUI_MINIMUM_ROTATION_DELAY'),
    );
    program.addOption(
        new Option('--domainSeparator <domainSeparator>', 'domain separator').makeOptionMandatory(true).env('SUI_DOMAIN_SEPARATOR'),
    );
    program.addOption(
        new Option('--faucetUrl <faucetUrl>', 'url for a faucet to request funds from').makeOptionMandatory(false).env('SUI_FAUCET_URL'),
    );

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
}
