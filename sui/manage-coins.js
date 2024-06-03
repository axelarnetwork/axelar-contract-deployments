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

async function mergeCoin(tx, coins) {
    console.log(coins);
    const coinObjectIds = coins.map((coin) => coin.coinObjectId);
    const firstCoin = coinObjectIds.shift();
    const remainingCoins = coinObjectIds.map((id) => tx.object(id));
    tx.mergeCoins(firstCoin, remainingCoins);
}

async function processCommand(config, chain, options) {
    const [keypair, client] = getWallet(chain, options);

    await printWalletInfo(keypair, client, chain, options);

    const coinTypeToCoins = await getAllCoins(client, keypair.toSuiAddress());
    const tx = new TransactionBlock();

    if (options.merge) {
        const coinType = options.coinType;

        if (coinType) {
            const coins = coinTypeToCoins[coinType];

            if (coins) {
                await mergeCoin(tx, coins.data);
            } else {
                console.error(`No coins found for coin type ${coinType}`);
            }
        } else {
            for (const coinType in coinTypeToCoins) {
                const coins = coinTypeToCoins[coinType];
                await mergeCoin(tx, coins.data);
            }
        }
    }

    if (options.split) {
    }

    const receipt = await client.signAndExecuteTransactionBlock({
        transactionBlock: tx,
        signer: keypair,
        options: {
            showEffects: true,
            showObjectChanges: true,
            showContent: true,
        },
    });

    console.log(receipt);
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
