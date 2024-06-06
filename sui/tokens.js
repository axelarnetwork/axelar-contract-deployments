const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { saveConfig } = require('../evm/utils');
const { Command } = require('commander');
const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');
const { printInfo, printError, validateParameters } = require('../evm/utils');
const chalk = require('chalk');
const { loadSuiConfig, SUI_COIN_ID } = require('./utils');

class CoinManager {
    static async getAllCoins(client, account) {
        let cursor;
        const coinTypeToCoins = {};

        do {
            const coinsAtCursor = await client.getAllCoins({
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

    static async splitCoins(tx, coinTypeToCoins, options) {
        CoinManager.checkSplitAmount(options);

        validateParameters({
          isValidNumber: { split: options.split },
        });

        const splitAmount = BigInt(options.split);

        printInfo('\n==== Splitting Coins ====');

        // Set coin type to given coin type or the first coin type if there's only one if it's a SUI token.
        const hasOnlyGasToken = Object.keys(coinTypeToCoins).length === 1 && coinTypeToCoins[SUI_COIN_ID];
        const coinType = options.coinType ? options.coinType : hasOnlyGasToken ? SUI_COIN_ID : undefined;

        if (coinType) {
            // Throw an error if the coin type is specified but no coins are found
            CoinManager.checkCoinType(coinType, coinTypeToCoins);
            const coins = coinTypeToCoins[coinType];
            const [coin] = CoinManager.doSplitCoins(tx, coins, splitAmount);

            if (options.transfer) {
                CoinManager.doTransfer(tx, coin, options.transfer);
            }
        } else {
            for (const coinType in coinTypeToCoins) {
                const coins = coinTypeToCoins[coinType];
                if (this.isGasToken(coins.data[0])) continue;
                const [coin] = CoinManager.doSplitCoins(tx, coins, splitAmount);

                if (options.transfer) {
                    CoinManager.doTransfer(tx, coin, options.transfer);
                }
            }
        }

        if (options.transfer) {
            printInfo(`\nTransfer ${splitAmount} coins for every split coin to ${chalk.green(options.transfer)}`);
        }

        // The transaction will fail if the gas budget is not set for splitting coins transaction
        tx.setGasBudget(1e8);
    }

    static doTransfer(tx, coin, recipient) {
        tx.transferObjects([coin], recipient);
    }

    static doSplitCoins(tx, coins, splitAmount) {
        const firstObjectId = this.isGasToken(coins.data[0]) ? tx.gas : coins.data[0].coinObjectId;
        const response = tx.splitCoins(firstObjectId, [splitAmount]);
        console.log(`Split coins of type '${chalk.green(coins.data[0].coinType)}' with amount ${splitAmount}`);
        return response;
    }

    static async mergeCoin(tx, coinTypeToCoins, options) {
        const coinTypes = options.coinType ? [options.coinType] : Object.keys(coinTypeToCoins);
        printInfo('\n==== Merging Coins ====');

        for (const coinType of coinTypes) {
            const coins = coinTypeToCoins[coinType];

            if(!coins) {
              throw new Error(`No coins found for coin type ${coinType}`);
            }

            await CoinManager.doMergeCoin(tx, coins);
        }
    }

    static async doMergeCoin(tx, coins) {
        const coinObjectIds = coins.data.map((coin) => coin.coinObjectId);

        // If the first coin is a gas token, remove it from the list. Otherwise, the merge will fail.
        if (CoinManager.isGasToken(coins.data[0])) {
            coinObjectIds.shift();
        }

        if (coinObjectIds.length < 2) {
            // Need at least 2 coins to merge
            return;
        }

        const firstCoin = coinObjectIds.shift();
        const remainingCoins = coinObjectIds.map((id) => tx.object(id));

        tx.mergeCoins(firstCoin, remainingCoins);
        console.log(`Merge ${coins.data.length} coins of type '${chalk.green(coins.data[0].coinType)}'`);
    }

    static async processCommand(config, chain, options) {
        const [keypair, client] = getWallet(chain, options);
        printInfo('Wallet Address', keypair.toSuiAddress());

        const coinTypeToCoins = await CoinManager.getAllCoins(client, keypair.toSuiAddress());
        CoinManager.printAllCoins(coinTypeToCoins);

        const tx = new TransactionBlock();

        if (options.merge) {
            await CoinManager.mergeCoin(tx, coinTypeToCoins, options);
        } else if (options.split) {
            await CoinManager.splitCoins(tx, coinTypeToCoins, options);
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

    static printAllCoins(coinTypeToCoins) {
        console.log('==== Coins Info ====');

        for (const coinType in coinTypeToCoins) {
            const coins = coinTypeToCoins[coinType];
            printInfo(`Coin Type`, coinType);
            printInfo(`Total Balance`, coins.totalBalance);
            printInfo(`Total Objects`, coins.data.length);
        }
    }

    static checkCoinType(coinType, coinTypeToCoins) {
        if (coinType && !coinTypeToCoins[coinType]) {
            printError(`No coins found for coin type ${coinType}`);
            process.exit(0);
        }
    }

    static isGasToken(coin) {
        return coin.coinType === '0x2::sui::SUI';
    }
}

async function mainProcessor(options, processor) {
    const config = loadSuiConfig(options.env);
    await processor(config, config.sui, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program.name('tokens').description('Token management tool (e.g. show balance, merge, split etc.)');

    addBaseOptions(program);

    program
        .option('--merge', 'Merge all coins')
        .option('--split <amount>', 'Split coins')
        .option('--coin-type <coinType>', 'Coin type to merge/split')
        .option('--transfer <recipientAddress>', 'Used with --split to transfer the split coins to the recipient address')
        .action((options) => {
            mainProcessor(options, CoinManager.processCommand);
        })
        .parse();
}
