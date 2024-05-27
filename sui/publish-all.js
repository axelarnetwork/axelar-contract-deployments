const { SuiClient, getFullnodeUrl } = require('@mysten/sui.js/client');
const { Ed25519Keypair } = require('@mysten/sui.js/keypairs/ed25519');
const secp256k1 = require('secp256k1');
const { saveConfig, loadConfig } = require('../evm/utils');
const { publishAll } = require('@axelar-network/axelar-cgp-sui/scripts/publish-all');
const { getConfig, parseEnv } = require('@axelar-network/axelar-cgp-sui/scripts/utils');
const { setTrustedAddresses } = require('@axelar-network/axelar-cgp-sui/scripts/its/set-trusted-address');
const { setItsDiscovery } = require('@axelar-network/axelar-cgp-sui/scripts/its/discovery');
const { transferOperatorship } = require('@axelar-network/axelar-cgp-sui/scripts/gateway');
const { Command, Option } = require('commander');

async function main(options) {
    options.validators = JSON.parse(options.validators).map((privKey) => secp256k1.publicKeyCreate(Buffer.from(privKey, 'hex')));
    options.weights = JSON.parse(options.weights);
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
    await publishAll(client, keypair, parseEnv(options.env));

    const config = loadConfig(options.env);

    if (!config.sui) {
        config.sui = {};
    }

    for (const packageName of ['axelar', 'governance', 'gas_service', 'its', 'abi']) {
        config.sui[packageName] = getConfig(packageName, options.env);
    }

    saveConfig(config, options.env);

    await setTrustedAddresses(client, keypair, options.env, [], []);

    await setItsDiscovery(client, keypair, options.env);
    await transferOperatorship(getConfig('axelar', options.env), client, keypair, options.validators, options.weights, options.threshold);
}

if (require.main === module) {
    const program = new Command();

    program.name('publish-sui').description('Publish all the packaged for sui');

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
        new Option('--validators <validatorAddresses>', 'addresses of the initial validator set')
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
        new Option('--faucetUrl <faucetUrl>', 'url for a faucet to request funds from')
            .makeOptionMandatory(false)
    );

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
}
