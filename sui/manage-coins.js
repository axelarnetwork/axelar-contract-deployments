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

async function mergeCoin(tx, coinTypeToCoins, options) {
    const coinType = options.coinType;
    console.log('\n==== Merging Coins ====');

    if (coinType) {
        const coins = coinTypeToCoins[coinType];

        if (!coins) {
            console.error(`No coins found for coin type ${coinType}`);
            return;
        }

        await doMergeCoin(tx, coins.data);
    } else {
        for (const coinType in coinTypeToCoins) {
            const coins = coinTypeToCoins[coinType];
            await doMergeCoin(tx, coins.data);
        }
    }
}

function isGasToken(coin) {
    return coin.coinType === '0x2::sui::SUI';
}

async function doMergeCoin(tx, coins) {
    const coinObjectIds = coins.map((coin) => coin.coinObjectId);

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
    console.log(`Merged ${coins.length} coins of type '${coins[0].coinType}'`);
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
