'use strict';

const { Asset, Contract, Operation } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const { loadConfig, addOptionsToCommands, getChainConfig, printInfo, printError, validateParameters, prompt } = require('../common');
const { addBaseOptions, broadcast, broadcastHorizon, getWallet, serializeValue, assetToScVal } = require('./utils');

async function createStellarAssetContract(wallet, _config, chain, contract, args, options) {
    const [assetCode, issuer] = args;

    validateParameters({
        isNonEmptyString: { assetCode, issuer },
        isValidStellarAddress: { issuer },
    });

    const asset = new Asset(assetCode, issuer);
    const xdrAssetScVal = assetToScVal(asset);

    const operation = contract.call('create_stellar_asset_contract', xdrAssetScVal);
    const returnValue = await broadcast(operation, wallet, chain, 'create_stellar_asset_contract', options);

    printInfo('Stellar asset contract address', serializeValue(returnValue.value()));
}

async function createStellarClassicAsset(wallet, _config, chain, _contract, args, options) {
    const [assetCode, issuer, limit] = args;

    validateParameters({
        isValidStellarAddress: { issuer },
        isNonEmptyString: { assetCode },
    });

    const trustLimit = limit || '1000000000'; // Default to a large limit if not specified

    const asset = new Asset(assetCode, issuer);

    const changeTrustOperation = Operation.changeTrust({
        asset: asset,
        limit: trustLimit,
        source: wallet.publicKey(),
    });

    await broadcastHorizon(changeTrustOperation, wallet, chain, 'Create Trustline', options);

    printInfo(`Successfully created trustline for ${assetCode} issued by ${issuer} with limit ${trustLimit}`);
}

async function mainProcessor(processor, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);

    if (!chain.contracts?.TokenUtils) {
        throw new Error('Token Utils package not found.');
    }

    if (prompt(`Proceed with action ${processor.name}`, yes)) {
        return;
    }

    const contract = new Contract(chain.contracts.TokenUtils.address);

    await processor(wallet, config, chain, contract, args, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('token-utils').description('token utils');

    addBaseOptions(program);

    program.command('create-stellar-asset-contract <assetCode> <issuer>').action((assetCode, issuer, options) => {
        mainProcessor(createStellarAssetContract, [assetCode, issuer], options);
    });

    program
        .command('create-stellar-classic-asset <assetCode> <issuer> [limit]')
        .description('Create a trustline for a Stellar classic asset')
        .action((assetCode, issuer, limit, options) => {
            mainProcessor(createStellarClassicAsset, [assetCode, issuer, limit], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
