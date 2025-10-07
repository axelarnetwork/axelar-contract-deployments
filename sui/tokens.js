const { Transaction } = require('@mysten/sui/transactions');
const { Command } = require('commander');
const { loadConfig, saveConfig, printInfo, printError, getChainConfig, validateParameters } = require('../common/');

const {
    addBaseOptions,
    addOptionsToCommands,
    broadcast,
    broadcastFromTxBuilder,
    createLockedCoinManagement,
    deployTokenFromInfo,
    getWallet,
    isGasToken,
    paginateAll,
    parseSuiUnitAmount,
    printWalletInfo,
    saveTokenDeployment,
    suiCoinId,
} = require('./utils');
const {
    utils: { formatUnits },
} = require('ethers');

class CoinManager {
    static async getAllCoins(client, account) {
        const coinTypeToCoins = {};

        try {
            // Fetch all coins using pagination
            const coins = await paginateAll(client, 'getAllCoins', { owner: account });

            // Iterate over each coin and organize them by coin type
            for (const coin of coins) {
                const coinsByType = coinTypeToCoins[coin.coinType] || {
                    data: [],
                    totalBalance: 0n,
                };

                coinsByType.data.push(coin);
                coinsByType.totalBalance += BigInt(coin.balance);

                coinTypeToCoins[coin.coinType] = coinsByType;
            }
        } catch (e) {
            printError('Failed to fetch coins', e.message);
        }

        return coinTypeToCoins;
    }

    static async printCoins(client, coinTypeToCoins) {
        for (const coinType in coinTypeToCoins) {
            const coins = coinTypeToCoins[coinType];

            const metadata = await client.getCoinMetadata({
                coinType,
            });

            if (!metadata) {
                printError('No metadata found for', coinType);
                process.exit(0);
            }

            printInfo('Coin Type', coinType);
            printInfo('Total Balance', `${formatUnits(coins.totalBalance.toString(), metadata.decimals).toString()}`);
            printInfo('Total Objects', coins.data.length);

            for (const coin of coins.data) {
                printInfo(`- ${formatUnits(coin.balance, metadata.decimals)}`);
            }
        }
    }

    static async splitCoins(tx, client, coinTypeToCoins, walletAddress, args, options) {
        const coinType = options.coinType || suiCoinId;
        const splitAmount = args.splitAmount;

        const metadata = await client.getCoinMetadata({
            coinType,
        });

        if (!metadata) {
            printError('No metadata found for', coinType);
            process.exit(0);
        }

        const objectToSplit = isGasToken(coinType)
            ? tx.gas
            : coinTypeToCoins[coinType].data.find((coinObject) => BigInt(coinObject.balance) >= splitAmount)?.coinObjectId;

        if (!objectToSplit) {
            printError('No coin object found with enough balance to split');
            process.exit(0);
        }

        const [coin] = tx.splitCoins(objectToSplit, [splitAmount]);

        printInfo('Split Coins', coinType);
        printInfo('Split Amount', `${formatUnits(splitAmount, metadata.decimals).toString()}`);

        if (options.transfer) {
            tx.transferObjects([coin], options.transfer);
            printInfo('Transfer Coins to', options.transfer);
        } else {
            tx.transferObjects([coin], walletAddress);
        }

        // The transaction will fail if the gas budget is not set for splitting coins transaction
        tx.setGasBudget(1e7);
    }

    static async mergeCoins(tx, coinTypeToCoins, options) {
        const coinTypes = options.coinType ? [options.coinType] : Object.keys(coinTypeToCoins);

        let merged = false;

        for (const coinType of coinTypes) {
            const coins = coinTypeToCoins[coinType];

            if (!coins) {
                throw new Error(`No coins found for coin type ${coinType}`);
            }

            const coinObjectIds = coins.data.map((coin) => coin.coinObjectId);

            // If the first coin is a gas token, remove it from the list. Otherwise, the merge will fail.
            if (isGasToken(coins.data[0].coinType)) {
                coinObjectIds.shift();
            }

            if (coinObjectIds.length < 2) {
                // Need at least 2 coins to merge
                continue;
            }

            const firstCoin = coinObjectIds.shift();
            const remainingCoins = coinObjectIds.map((id) => tx.object(id));

            tx.mergeCoins(firstCoin, remainingCoins);
            merged = true;

            printInfo('Merge Coins', coins.data[0].coinType);
        }

        return merged;
    }
}

async function processSplitCommand(keypair, client, args, options) {
    printInfo('Action', 'Split Coins');

    const coinTypeToCoins = await CoinManager.getAllCoins(client, keypair.toSuiAddress());

    const tx = new Transaction();

    await CoinManager.splitCoins(tx, client, coinTypeToCoins, keypair.toSuiAddress(), args, options);

    await broadcast(client, keypair, tx, 'Splitted Coins', options);
}

async function processMergeCommand(keypair, client, args, options) {
    printInfo('Action', 'Merge Coins');
    const coinTypeToCoins = await CoinManager.getAllCoins(client, keypair.toSuiAddress());

    const tx = new Transaction();
    const hasMerged = await CoinManager.mergeCoins(tx, coinTypeToCoins, options);

    if (!hasMerged) {
        printInfo('No coins to merge');
        return;
    }

    await broadcast(client, keypair, tx, 'Merged Coins', options);
}

async function processListCommand(keypair, client, args, options) {
    printInfo('Action', 'List Coins');
    printInfo('Wallet Address', keypair.toSuiAddress());

    const coinTypeToCoins = await CoinManager.getAllCoins(client, keypair.toSuiAddress());
    await CoinManager.printCoins(client, coinTypeToCoins);
}

async function publishCoinCommand(keypair, client, args, options, contracts) {
    const [symbol, name, decimals] = args;

    validateParameters({
        isNonEmptyString: {
            symbol: symbol,
            name: name,
            decimals: decimals,
        },
    });

    const walletAddress = keypair.toSuiAddress();

    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await printWalletInfo(keypair, client, chain, options);

    const deployConfig = { client, keypair, options, walletAddress };

    // Deploy token on Sui
    const [metadata, packageId, tokenType, treasuryCap] = await deployTokenFromInfo(deployConfig, symbol, name, decimals);

    // Save the deployed token
    saveTokenDeployment(packageId, tokenType, contracts, symbol, decimals, null, treasuryCap, metadata);
}

async function legacyCoinsCommand(keypair, client, args, options, contracts) {
    const { InterchainTokenService: itsConfig } = contracts;
    const { InterchainTokenService, InterchainTokenServicev0 } = itsConfig.objects;

    if (options.createCoin) {
        validateParameters({
            isNonEmptyString: {
                symbol: options.createCoin,
                decimals: options.decimals,
                name: options.name,
            },
        });

        const config = loadConfig(options.env);
        const chain = getChainConfig(config.chains, options.chainName);
        await printWalletInfo(keypair, client, chain, options);

        const symbol = options.createCoin;
        const name = options.name;
        const decimals = options.decimals;
        const walletAddress = keypair.toSuiAddress();
        const deployConfig = { client, keypair, options, walletAddress };

        // Deploy token on Sui
        const [metadata, packageId, tokenType, treasuryCap] = await deployTokenFromInfo(deployConfig, symbol, name, decimals);

        // New CoinManagement<T>
        const [txBuilder, coinManagement] = await createLockedCoinManagement(deployConfig, itsConfig, tokenType);

        // New CoinInfo<T>
        const coinInfo = await txBuilder.moveCall({
            target: `${itsConfig.address}::coin_info::from_metadata`,
            arguments: [metadata],
            typeArguments: [tokenType],
        });

        // Register legacy coin
        await txBuilder.moveCall({
            target: `${itsConfig.address}::interchain_token_service::register_coin`,
            arguments: [InterchainTokenService, coinInfo, coinManagement],
            typeArguments: [tokenType],
        });

        const result = await broadcastFromTxBuilder(
            txBuilder,
            keypair,
            `Register legacy coin (${symbol}) in InterchainTokenService`,
            options,
            {
                showEvents: true,
            },
        );

        const tokenId = result.events[0].parsedJson.token_id.id;

        // Save the deployed token
        saveTokenDeployment(packageId, tokenType, contracts, symbol, decimals, tokenId, treasuryCap, metadata);

        if (options.createOnly) return;
    }

    printInfo('Action', 'Generate Legacy Coins List');

    const itsObject = await client.getObject({
        id: InterchainTokenServicev0,
        options: { showContent: true },
    });

    const registeredCoinsId = itsObject.data ? itsObject.data.content.fields.value.fields.registered_coins.fields.id.id : null;

    if (!registeredCoinsId) throw new Error(`Unable to query ITS object at id ${InterchainTokenServicev0}`);

    let hasNextPage = true,
        cursor,
        legacyCoins = [];
    while (hasNextPage) {
        try {
            // Paging (batches of 50)
            const params = { parentId: registeredCoinsId };
            if (cursor) params.cursor = cursor;

            // Fetch token data
            const fields = await client.getDynamicFields(params);
            const coinIds = fields.data ? fields.data.map((coin) => coin.objectId) : [];
            const coinData = await client.multiGetObjects({
                ids: coinIds,
                options: { showContent: true },
            });

            // Target effected tokens by selecting only items with metadata !== null
            legacyCoins = [
                ...legacyCoins,
                ...coinData
                    .filter((coin) => {
                        const coinMetadata = coin.data ? coin.data.content.fields.value.fields.coin_info.fields.metadata : null;
                        return coinMetadata ? true : false;
                    })
                    .map((coin) => {
                        return {
                            TokenId: coin.data.content.fields.name.fields.id,
                            TokenType: coin.data.content.fields.value.fields.coin_info.fields.metadata.type,
                            symbol: coin.data.content.fields.value.fields.coin_info.fields.metadata.fields.symbol,
                        };
                    }),
            ];

            hasNextPage = fields.hasNextPage;
            cursor = fields.nextCursor ? fields.nextCursor : null;
        } catch (e) {
            throw new Error(e);
        }
    }
    if (legacyCoins.length) contracts.InterchainTokenService.legacyCoins = legacyCoins;
}

async function mainProcessor(options, processor, args = {}) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    const [keypair, client] = getWallet(chain, options);
    await processor(keypair, client, args, options, chain.contracts);
    saveConfig(config, options.env);
}

if (require.main === module) {
    // Main program
    const program = new Command('tokens').description('Merge, split, and list coins.');

    // Sub-programs
    const mergeProgram = new Command('merge').description('Merge all coins into a single object');
    const splitProgram = new Command('split').description(
        'Split coins into a new object. If no coin type is specified, SUI coins will be used by default.',
    );
    const listProgram = new Command('list').description('List all coins and balances');
    const legacyCoinsProgram = new Command('legacy-coins').description(
        'Save a list of legacy coins to be migrated to public coin metadata; and / or, create a legacy coin using the createCoin flag.',
    );
    const publishCoinProgram = new Command('publish-coin').description(
        'Deploy a coin on Sui by specifying coin symbol, name and decimal precision',
    );

    // Define options, arguments, and actions for each sub-program
    mergeProgram.option('--coin-type <coinType>', 'Coin type to merge').action((options) => {
        mainProcessor(options, processMergeCommand);
    });

    splitProgram
        .argument('<amount>', 'Amount should be in the full coin unit (e.g. 1.5 for 1_500_000_000 coins)', parseSuiUnitAmount)
        .option('--transfer <recipientAddress>', 'Used with split command to transfer the split coins to the recipient address')
        .option('--coin-type <coinType>', 'Coin type to split')
        .action((splitAmount, options) => {
            mainProcessor(options, processSplitCommand, { splitAmount });
        });

    listProgram.action((options) => {
        mainProcessor(options, processListCommand);
    });

    legacyCoinsProgram
        .option('--createCoin <symbol>', 'Create a legacy coin with the given symbol')
        .option('--createOnly', 'Create a legacy coin without generating a list of all legacy coins')
        .option('--decimals <decimals>', 'Decimal precision for creating a coin')
        .option('--name <name>', 'A human readable coin name')
        .action((options) => {
            mainProcessor(options, legacyCoinsCommand);
        });

    publishCoinProgram
        .argument('<symbol>', 'Coin symbol')
        .argument('<name>', 'Coin name')
        .argument('<decimals>', 'Coin decimal precision')
        .action((symbol, name, decimals, options) => {
            mainProcessor(options, publishCoinCommand, [symbol, name, decimals]);
        });

    // Add sub-programs to the main program
    program.addCommand(mergeProgram);
    program.addCommand(splitProgram);
    program.addCommand(listProgram);
    program.addCommand(legacyCoinsProgram);
    program.addCommand(publishCoinProgram);

    // Add base options to all sub-programs
    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
