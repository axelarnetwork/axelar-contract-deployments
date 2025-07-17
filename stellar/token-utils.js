'use strict';

const { Asset, Contract, Operation, nativeToScVal, StrKey } = require('@stellar/stellar-sdk');
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

async function changeTrust(wallet, _config, chain, _contract, args, options) {
    const [assetCode, issuer, limit] = args;

    validateParameters({
        isValidStellarAddress: { issuer },
        isNonEmptyString: { assetCode },
    });

    const asset = new Asset(assetCode, issuer);

    const changeTrustOperation = Operation.changeTrust({
        asset: asset,
        limit: limit,
        source: wallet.publicKey(),
    });

    await broadcastHorizon(changeTrustOperation, wallet, chain, 'Change Trust', options);

    printInfo(`Successfully changed trustline for ${assetCode} issued by ${issuer} with limit ${limit}`);
}

async function admin(wallet, _config, chain, _contract, args, options) {
    const [tokenAddress] = args;

    validateParameters({
        isValidStellarAddress: { tokenAddress },
    });

    const tokenContract = new Contract(tokenAddress);
    const operation = tokenContract.call('admin');

    const returnValue = await broadcast(operation, wallet, chain, 'Get Admin', options);
    const adminAddress = serializeValue(returnValue.value());

    printInfo('Token address', tokenAddress);
    printInfo('Admin address', adminAddress);

    return adminAddress;
}

async function balance(wallet, _config, chain, _contract, args, options) {
    const [tokenAddress, accountAddress] = args;

    validateParameters({
        isValidStellarAddress: { tokenAddress, accountAddress },
    });

    const tokenContract = new Contract(tokenAddress);
    const operation = tokenContract.call('balance', nativeToScVal(accountAddress, { type: 'address' }));

    const returnValue = await broadcast(operation, wallet, chain, 'Get Balance', options);
    const balanceValue = serializeValue(returnValue.value());

    printInfo('Token address', tokenAddress);
    printInfo('Account address', accountAddress);
    printInfo('Balance', balanceValue);

    return balanceValue;
}

async function setAdmin(wallet, _config, chain, _contract, args, options) {
    const [tokenAddress, newAdmin] = args;

    validateParameters({
        isValidStellarAddress: { tokenAddress, newAdmin },
    });

    const tokenContract = new Contract(tokenAddress);
    const operation = tokenContract.call('set_admin', nativeToScVal(newAdmin, { type: 'address' }));

    await broadcast(operation, wallet, chain, 'Set Admin', options);
    printInfo('Successfully set admin for token', tokenAddress);
    printInfo('New admin address', newAdmin);
}

async function isMinter(wallet, _config, chain, _contract, args, options) {
    const [tokenAddress, minter] = args;

    validateParameters({
        isValidStellarAddress: { tokenAddress, minter },
    });

    const tokenContract = new Contract(tokenAddress);
    const operation = tokenContract.call('is_minter', nativeToScVal(minter, { type: 'address' }));

    const returnValue = await broadcast(operation, wallet, chain, 'Check Is Minter', options);
    const isMinterResult = returnValue.value();

    printInfo('Token address', tokenAddress);
    printInfo('Minter address', minter);
    printInfo('Is minter', isMinterResult);

    return isMinterResult;
}

async function mainProcessor(processor, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    const wallet = await getWallet(chain, options);

    if (prompt(`Proceed with action ${processor.name}`, yes)) {
        return;
    }

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
        .command('change-trust <assetCode> <issuer> <limit>')
        .description('Change or create a trustline for a Stellar asset')
        .action((assetCode, issuer, limit, options) => {
            mainProcessor(changeTrust, [assetCode, issuer, limit], options);
        });

    program
        .command('admin <tokenAddress>')
        .description('Get the admin address for a token contract')
        .action((tokenAddress, options) => {
            mainProcessor(admin, [tokenAddress], options);
        });

    program
        .command('balance <tokenAddress> <accountAddress>')
        .description('Get the balance of an account for a token contract')
        .action((tokenAddress, accountAddress, options) => {
            mainProcessor(balance, [tokenAddress, accountAddress], options);
        });

    program
        .command('set-admin <tokenAddress> <newAdmin>')
        .description('Set the admin address for a token contract')
        .action((tokenAddress, newAdmin, options) => {
            mainProcessor(setAdmin, [tokenAddress, newAdmin], options);
        });

    program
        .command('is-minter <tokenAddress> <minter>')
        .description('Check if an address is a minter for a token contract')
        .action((tokenAddress, minter, options) => {
            mainProcessor(isMinter, [tokenAddress, minter], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
