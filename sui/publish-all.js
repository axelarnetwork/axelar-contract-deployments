const { SuiClient, getFullnodeUrl } = require('@mysten/sui.js/client');
const { Ed25519Keypair } = require('@mysten/sui.js/keypairs/ed25519');
const {
    saveConfig,
    loadConfig,
} = require('../evm/utils');
const { publishAll } = require('@axelar-network/axelar-cgp-sui/scripts/publish-all');
const { getConfig, parseEnv } = require('@axelar-network/axelar-cgp-sui/scripts/utils');
const { setTrustedAddresses } = require('@axelar-network/axelar-cgp-sui/scripts/its/set-trusted-address');
const { setItsDiscovery } = require('@axelar-network/axelar-cgp-sui/scripts/its/discovery');
const { Command, Option } = require('commander');

async function main(options) {
    const privKey =
        Buffer.from(
            options.privateKey,
            "hex"
        );
    const keypair = Ed25519Keypair.fromSecretKey(privKey);
    const client = new SuiClient({ url: getFullnodeUrl(options.env) });
    await publishAll(client, keypair, parseEnv(options.env));

    const config = loadConfig(options.env);

    if(!config.sui) {
        config.sui = {};
    }

    for(const package of ['axelar', 'governance', 'gas_service', 'its', 'abi']) {
        config.sui[package] = getConfig(package, options.env);
    }

    saveConfig(config, options.env);

    await setTrustedAddresses(client, keypair, options.env, [], []);

    await setItsDiscovery(client, keypair, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('publish-sui').description('Publish all the packaged for sui');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['localnet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    //program.addOption(new Option('--gasOptions <gasOptions>', 'gas options cli override'));

    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('SUI_PRIVATE_KEY'));
    
    program.addOption(
        new Option('--validators <validatorAddresses>', 'addresses of the intiial validator set').makeOptionMandatory(true).env('SUI_INITIAL_VALIDATOR_ADDRESSES'),
    );
    program.addOption(
        new Option('--weights <validatorWeights>', 'wieghts of the intiial validator set').makeOptionMandatory(true).env('SUI_INITIAL_VALIDATOR_WEIGHTS'),
    );
    program.addOption(
        new Option('--threshold <threshold>', 'threshold for the intiial validator set').makeOptionMandatory(true).env('SUI_INITIAL_VALIDATOR_THRESHOLD'),
    );

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
} else {
    module.exports = { deployITS: deploy };
}
