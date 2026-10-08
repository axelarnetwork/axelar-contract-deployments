'use strict';

const { Asset, Contract, Operation, Address, nativeToScVal, authorizeInvocation, xdr } = require('@stellar/stellar-sdk');
const { Command } = require('commander');
const {
    loadConfig,
    addOptionsToCommands,
    getChainConfig,
    printInfo,
    printError,
    validateParameters,
    prompt,
    isNonEmptyString,
} = require('../common');
const {
    addBaseOptions,
    broadcast,
    broadcastHorizon,
    getWallet,
    serializeValue,
    assetToScVal,
    createAuthorizedFunc,
    getNetworkPassphrase,
    getAuthValidUntilLedger,
} = require('./utils');

async function createStellarAssetContract(wallet, _config, chain, contract, args, options) {
    const [assetCode, issuer] = args;

    validateParameters({
        isNonEmptyString: { assetCode, issuer },
        isValidStellarAddress: { issuer },
    });

    const asset = new Asset(assetCode, issuer);
    const xdrAssetScVal = assetToScVal(asset);

    const operation = contract.call('create_stellar_asset_contract', xdrAssetScVal);
    const response = await broadcast(operation, wallet, chain, 'create_stellar_asset_contract', options);

    printInfo('Stellar asset contract address', serializeValue(response.value()));
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

async function owner(wallet, _config, chain, _contract, args, options) {
    const [tokenAddress] = args;

    validateParameters({
        isValidStellarAddress: { tokenAddress },
    });

    const tokenContract = new Contract(tokenAddress);
    const operation = tokenContract.call('owner');

    const returnValue = await broadcast(operation, wallet, chain, 'Get Owner', options);
    const ownerAddress = serializeValue(returnValue.value());

    printInfo('Token address', tokenAddress);
    printInfo('Owner address', ownerAddress);

    return ownerAddress;
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

async function addMinter(wallet, _config, chain, _contract, args, options) {
    const [tokenAddress, minter] = args;

    validateParameters({
        isValidStellarAddress: { tokenAddress, minter },
    });

    const tokenContract = new Contract(tokenAddress);
    const operation = tokenContract.call('add_minter', nativeToScVal(minter, { type: 'address' }));

    await broadcast(operation, wallet, chain, 'Add Minter', options);
    printInfo('Successfully added minter for token', tokenAddress);
    printInfo('New minter address', minter);
}

async function createAuths(tokenAddress, functionName, args, wallet, chain) {
    const publicKey = wallet.publicKey();
    const networkPassphrase = getNetworkPassphrase(chain.networkType);

    const validUntil = await getAuthValidUntilLedger(chain);

    const contractAuth = createAuthorizedFunc(Address.fromString(tokenAddress), functionName, args);
    const [contractInvocation] = [new xdr.SorobanAuthorizedInvocation({ function: contractAuth, subInvocations: [] })];

    return Promise.all([authorizeInvocation(wallet, validUntil, contractInvocation, publicKey, networkPassphrase)]);
}

async function mint(wallet, _config, chain, _contract, args, options) {
    const [tokenAddress, recipient, amount] = args;

    validateParameters({
        isValidStellarAddress: { tokenAddress, recipient },
        isValidNumber: { amount },
    });

    const recipientScVal = nativeToScVal(recipient, { type: 'address' });
    const amountScVal = nativeToScVal(amount, { type: 'i128' });

    const tokenContract = new Contract(tokenAddress);

    let operation = Operation.invokeContractFunction({
        contract: tokenContract.contractId(),
        function: 'mint',
        args: [recipientScVal, amountScVal],
        auth: await createAuths(tokenAddress, 'mint', [recipientScVal, amountScVal], wallet, chain),
    });

    await broadcast(operation, wallet, chain, 'Mint', options);
    printInfo('Successfully minted tokens');
    printInfo('To recipient', recipient);
    printInfo('Token address', tokenAddress);
    printInfo('Amount minted', amount);
}

async function mintFrom(wallet, _config, chain, _contract, args, options) {
    const [tokenAddress, recipient, amount] = args;

    validateParameters({
        isValidStellarAddress: { tokenAddress, recipient },
        isValidNumber: { amount },
    });

    const minterScVal = nativeToScVal(wallet.publicKey(), { type: 'address' });
    const recipientScVal = nativeToScVal(recipient, { type: 'address' });
    const amountScVal = nativeToScVal(amount, { type: 'i128' });

    const tokenContract = new Contract(tokenAddress);

    let operation = Operation.invokeContractFunction({
        contract: tokenContract.contractId(),
        function: 'mint_from',
        args: [minterScVal, recipientScVal, amountScVal],
        auth: await createAuths(tokenAddress, 'mint_from', [minterScVal, recipientScVal, amountScVal], wallet, chain),
    });

    await broadcast(operation, wallet, chain, 'Mint From', options);
    printInfo('Successfully minted tokens from', wallet.publicKey());
    printInfo('To recipient', recipient);
    printInfo('Token address', tokenAddress);
    printInfo('Amount minted', amount);
}

async function mainProcessor(processor, args, options) {
    const { yes } = options;
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
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
        .command('change-trust <assetCode> <issuer> [limit]')
        .description('Change or create a trustline for a Stellar asset')
        .action((assetCode, issuer, limit, options) => {
            mainProcessor(changeTrust, [assetCode, issuer, limit || '1000000000'], options);
        });

    program
        .command('admin <tokenAddress>')
        .description('Get the admin address for a token contract')
        .action((tokenAddress, options) => {
            mainProcessor(admin, [tokenAddress], options);
        });

    program
        .command('owner <tokenAddress>')
        .description('Get the owner address for a token contract')
        .action((tokenAddress, options) => {
            mainProcessor(owner, [tokenAddress], options);
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

    program
        .command('add-minter <tokenAddress> <minter>')
        .description('Add a minter for a token contract')
        .action((tokenAddress, minter, options) => {
            mainProcessor(addMinter, [tokenAddress, minter], options);
        });

    program
        .command('mint <tokenAddress> <recipient> <amount>')
        .description('Mint tokens to a recipient address')
        .action((tokenAddress, recipient, amount, options) => {
            mainProcessor(mint, [tokenAddress, recipient, amount], options);
        });

    program
        .command('mint-from <tokenAddress> <recipient> <amount>')
        .description('Mint tokens from the minter to a recipient address')
        .action((tokenAddress, recipient, amount, options) => {
            mainProcessor(mintFrom, [tokenAddress, recipient, amount], options);
        });

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
