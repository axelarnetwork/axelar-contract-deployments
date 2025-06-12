'use strict';

const { Asset, Contract, nativeToScVal } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const {
    loadConfig,
    addOptionsToCommands,
    getChainConfig,
    printInfo,
    validateParameters,
    prompt,
} = require('../common');
const { addBaseOptions, broadcast, getWallet, serializeValue, assetToScVal } = require('./utils');

async function deployStellarAssetContract(wallet, _config, chain, contract, args, options) {
    const [assetCode, issuer] = args;

    validateParameters({
        isNonEmptyString: { assetCode, issuer },
        isValidStellarAddress: { issuer },
    });

    const asset = new Asset(assetCode, issuer);
    const xdrAssetScVal = assetToScVal(asset);

    const operation = contract.call('deploy_stellar_asset_contract', xdrAssetScVal);
    const returnValue = await broadcast(operation, wallet, chain, 'deploy_stellar_asset_contract', options);

    printInfo('Stellar asset contract address', serializeValue(returnValue.value()));
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

    program
        .command('deploy-stellar-asset-contract <assetCode> <issuer>')
        .action((assetCode, issuer, options) => {
            mainProcessor(deployStellarAssetContract, [assetCode, issuer], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
