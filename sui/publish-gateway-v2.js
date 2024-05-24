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
const { publishPackage } = require('@axelar-network/axelar-cgp-sui/scripts/publish-package');
const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { bcs } = require('@mysten/sui.js/bcs');
const { ethers } = require('hardhat');
const {
    Contract,
    Wallet,
    providers: { JsonRpcProvider },
    utils: { parseEther, keccak256, defaultAbiCoder, arrayify, hexlify, randomBytes },
    constants: { AddressZero },
} = ethers;

const { requestSuiFromFaucetV0 } = require('@mysten/sui.js/faucet');

async function main(options) {
    options.validatorAddresses = JSON.parse(options.validatorAddresses);
    options.weights = JSON.parse(options.weights);
    options.validators = options.validatorAddresses.map((val, index) => {return {signer: arrayify(val), weight: options.weights[index]}});
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
    const published = await publishPackage('axelar_gateway', client, keypair, parseEnv(options.env));

    const creatorCap = published.publishTxn.objectChanges.find(change => change.objectType === `${published.packageId}::gateway::CreatorCap`);
    
    const signerStruct = bcs.struct('WeightedSigners', {
        signer: bcs.vector(bcs.u8()),
        weight: bcs.u128(),
    });
    const bytes32Struct = bcs.vector(bcs.u8());

    const signersStruct = bcs.struct('WeightedSigners', {
        signers: bcs.vector(signerStruct),
        threshold: bcs.u128(),
        nonce: bytes32Struct,
    });

    const nonce = bytes32Struct.serialize(arrayify(options.nonce)).toBytes();

    const encodedSigners = signersStruct.serialize({
        signers: options.validators,
        threshold: options.threshold,
        nonce,
    }).toBytes()

    const tx = new TransactionBlock();

    const domainSeparator = tx.moveCall({
        target: `${published.packageId}::bytes32::new`,
        arguments: [
            tx.pure(arrayify(options.domainSeparator)),
        ],
    });

    const bcsArg = tx.moveCall({
        target: `0x2::bcs::new`,
        arguments: [
            tx.pure(encodedSigners)
        ],
    })

    const initialSigners = tx.moveCall({
        target: `${published.packageId}::weighted_signers::peel`,
        arguments: [
            bcsArg,
        ],
    });

    /*tx.moveCall({
        target: `${published.packageId}::gateway::setup`,
        arguments: [
            tx.object(creatorCap.objectId),
            tx.pure.address(options.operator),
            domainSeparator,
            tx.pure(options.rotationDelay),
            initialSigners,
            tx.object('0x6'),
        ]
    });*/

    const publishTxn = await client.signAndExecuteTransactionBlock({
		transactionBlock: tx,
		signer: keypair,
		options: {
			showEffects: true,
			showObjectChanges: true,
            showContent: true
		},
	});

    const config = loadConfig(options.env);

    if (!config.sui) {
        config.sui = {};
    }

    for (const packageName of ['axelar_gateway']) {
        config.sui[packageName] = getConfig(packageName, options.env);
    }

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
        new Option('--nonce <nonce>', 'nonce for the intiial validator set')
            .makeOptionMandatory(true)
            .env('SUI_INITIAL_NONCE'),
    );
    program.addOption(
        new Option('--operator <operator>', 'operator for the sui gateway')
            .makeOptionMandatory(true)
            .env('SUI_OPERATOR'),
    );
    program.addOption(
        new Option('--rotationDelay <rotationDelay>', 'minimum rotation delay for validators')
            .makeOptionMandatory(true)
            .env('SUI_MINIMUM_ROTATION_DELAY'),
    );
    program.addOption(
        new Option('--domainSeparator <domainSeparator>', 'domain separator')
            .makeOptionMandatory(true)
            .env('SUI_DOMAIN_SEPARATOR'),
    );
    program.addOption(
        new Option('--faucetUrl <faucetUrl>', 'url for a faucet to request funds from')
            .makeOptionMandatory(false)
            .env('SUI_FAUCET_URL'),
    );

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
}
