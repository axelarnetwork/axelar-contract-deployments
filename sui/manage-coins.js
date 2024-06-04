const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { saveConfig } = require('../evm/utils');
const { Command } = require('commander');
const { addBaseOptions } = require('./cli-utils');
const { getWallet, printWalletInfo } = require('./sign-utils');
const { loadSuiConfig } = require('./utils');

async function getAllCoins(client, account) {
    let cursor;
    const coinTypeToCoins = {};

    do {
        const coinsAtCursor = await client.getCoins({
            owner: account,
            limit: 100,
            cursor,
        });

        for (const coin of coinsAtCursor.data) {
            if (!coinTypeToCoins[coin.coinType]) {
                coinTypeToCoins[coin.coinType] = {
                    data: [],
                    totalBalance: 0n,
                };
            }

            coinTypeToCoins[coin.coinType].data.push(coin);
            coinTypeToCoins[coin.coinType].totalBalance += BigInt(coin.balance);
        }

        if (coinsAtCursor.hasNextPage) {
            cursor = coinsAtCursor.nextCursor;
        }
    } while (cursor);

    return coinTypeToCoins;
}

function checkCoinType(coinType, coinTypeToCoins) {
    if (coinType && !coinTypeToCoins[coinType]) {
        console.error(`No coins found for coin type ${coinType}`);
        process.exit(0);
    }
}

async function splitCoins(tx, coinTypeToCoins, options) {
    checkSplitAmount(options);
    const splitAmount = BigInt(options.split);

    console.log('\n==== Splitting Coins ====');

    const coinType = options.coinType;

    // Throw an error if the coin type is specified but no coins are found
    checkCoinType(coinType, coinTypeToCoins);

    if (coinType) {
        const coins = coinTypeToCoins[coinType];
        doSplitCoins(tx, coins, splitAmount);
    } else {
        for (const coinType in coinTypeToCoins) {
            doSplitCoins(tx, coinTypeToCoins[coinType], splitAmount);
        }
    }

    // The transaction will fail if the gas budget is not set for splitting coins transaction
    tx.setGasBudget(1e8);
}

function doSplitCoins(tx, coins, splitAmount) {
    const firstObjectId = coins.data[0].coinObjectId;
    tx.splitCoins(firstObjectId, [splitAmount]);
    console.log(`Split coins of type '${coins.data[0].coinType}' with amount ${splitAmount}`);
}

async function mergeCoin(tx, coinTypeToCoins, options) {
    const coinType = options.coinType;
    console.log('\n==== Merging Coins ====');

    // Throw an error if the coin type is specified but no coins are found
    checkCoinType(coinType, coinTypeToCoins);

    if (coinType) {
        const coins = coinTypeToCoins[coinType];
        await doMergeCoin(tx, coins);
    } else {
        for (const coinType in coinTypeToCoins) {
            const coins = coinTypeToCoins[coinType];
            await doMergeCoin(tx, coins);
        }
    }
}

function isGasToken(coin) {
    return coin.coinType === '0x2::sui::SUI';
}

function checkSplitAmount(options) {
    try {
        parseInt(options.split);
    } catch (e) {
        console.error('\nError: Please specify a valid split amount');
        process.exit(0);
    }
}

async function doMergeCoin(tx, coins) {
    const coinObjectIds = coins.data.map((coin) => coin.coinObjectId);

    // If the first coin is a gas token, remove it from the list. Otherwise, the merge will fail.
    if (isGasToken(coins[0])) {
        coinObjectIds.shift();
    }

    if (coinObjectIds.length < 2) {
        console.error('\nError: Need at least 2 coins to merge');
        process.exit(0);
    }

    const firstCoin = coinObjectIds.shift();
    const remainingCoins = coinObjectIds.map((id) => tx.object(id));

    tx.mergeCoins(firstCoin, remainingCoins);
    console.log(`Merged ${coins.data.length} coins of type '${coins.data[0].coinType}'`);
}

function printAllCoins(coinTypeToCoins) {
    console.log('==== Coins Info ====');

    for (const coinType in coinTypeToCoins) {
        const coins = coinTypeToCoins[coinType];
        console.log(`Coin Type: ${coinType}`);
        console.log(`Total Balance: ${coins.totalBalance}`);
        console.log(`Total Objects: ${coins.data.length}`);
    }
}

async function processCommand(config, chain, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    const coinTypeToCoins = await getAllCoins(client, keypair.toSuiAddress());
    printAllCoins(coinTypeToCoins);

    const tx = new TransactionBlock();

    if (options.merge) {
        await mergeCoin(tx, coinTypeToCoins, options);
    } else if (options.split) {
        await splitCoins(tx, coinTypeToCoins, options);
    }

    const requireBroadcast = options.merge || options.split;

    if (requireBroadcast) {
        await client.signAndExecuteTransactionBlock({
            transactionBlock: tx,
            signer: keypair,

            options: {
                showEffects: true,
                showObjectChanges: true,
                showContent: true,
            },
        });

        console.log(`\nDone`);
    }
}

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);
    await processor(config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('manage-coins').description('Merge or split coins for an account');

    addBaseOptions(program);

    program
        .option('--merge', 'Merge all coins')
        .option('--split <amount>', 'Split coins')
        .option('--coin-type <coinType>', 'Coin type to merge/split')
        .action((options) => {
            mainProcessor(options, processCommand);
        })
        .parse();
}
