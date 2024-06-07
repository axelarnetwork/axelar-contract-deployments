const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { saveConfig } = require('../evm/utils');
const { Command } = require('commander');
const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');
const { printInfo, printError, validateParameters } = require('../evm/utils');
const chalk = require('chalk');
const {
    utils: { parseUnits },
} = require('ethers');
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
        const splitAmount = BigInt(options.split);
        const coinType = options.coinType || SUI_COIN_ID;

        const coins = coinTypeToCoins[coinType];
        const firstObjectId = this.isGasToken(coins.data[0]) ? tx.gas : coins.data[0].coinObjectId;
        const [coin] = tx.splitCoins(firstObjectId, [splitAmount]);
        printInfo('Split Coins', coins.data[0].coinType);
        printInfo('Split Amount', splitAmount.toString());

        if (options.transfer) {
            tx.transferObjects([coin], options.transfer);
            printInfo('Transfer Coins to', options.transfer);
        }

        // The transaction will fail if the gas budget is not set for splitting coins transaction
        tx.setGasBudget(1e8);
    }

    static async mergeCoin(tx, coinTypeToCoins, options) {
        const coinTypes = options.coinType ? [options.coinType] : Object.keys(coinTypeToCoins);

        let merged = false;

        for (const coinType of coinTypes) {
            const coins = coinTypeToCoins[coinType];

            if (!coins) {
                throw new Error(`No coins found for coin type ${coinType}`);
            }

            const hasMergedCoins = await CoinManager.doMergeCoin(tx, coins);
            merged = merged || hasMergedCoins;
        }

        return merged;
    }

    static async doMergeCoin(tx, coins) {
        const coinObjectIds = coins.data.map((coin) => coin.coinObjectId);

        // If the first coin is a gas token, remove it from the list. Otherwise, the merge will fail.
        if (CoinManager.isGasToken(coins.data[0])) {
            coinObjectIds.shift();
        }

        if (coinObjectIds.length < 2) {
            // Need at least 2 coins to merge
            return false;
        }

        const firstCoin = coinObjectIds.shift();
        const remainingCoins = coinObjectIds.map((id) => tx.object(id));

        tx.mergeCoins(firstCoin, remainingCoins);
        printInfo('Merge Coins', coins.data[0].coinType);
        return true;
    }

    static async processCommand(config, chain, options) {
        const [keypair, client] = getWallet(chain, options);
        printInfo('Wallet Address', keypair.toSuiAddress());

        const coinTypeToCoins = await CoinManager.getAllCoins(client, keypair.toSuiAddress());
        CoinManager.printAllCoins(coinTypeToCoins);

        const tx = new TransactionBlock();

        if (options.merge) {
            printInfo('Action', 'Merge Coins');
            const hasMerged = await CoinManager.mergeCoin(tx, coinTypeToCoins, options);

            if (!hasMerged) {
                printInfo('No coins to merge');
            }
        } else if (options.split) {
            validateParameters({
                isValidNumber: { split: options.split },
            });

            const coinType = options.coinType || SUI_COIN_ID;
            const metadata = await client.getCoinMetadata({
                coinType,
            });

            if (!metadata) {
                printError('No metadata found for', coinType);
                process.exit(0);
            }

            const splitAmount = parseUnits(options.split, metadata.decimals);

            printInfo('Action', 'Split Coins');
            await CoinManager.splitCoins(tx, coinTypeToCoins, {
                ...options,
                split: splitAmount.toString(),
                coinType,
            });
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

            printInfo(`Done`);
        }
    }

    static printAllCoins(coinTypeToCoins) {
        printInfo('Action', 'List all coins and balances');

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
        .option('--split <amount>', 'Split coins. The amount is expected to be in the full coin unit (e.g. 1.5 for 1_500_000_000 coins)')
        .option('--coin-type <coinType>', 'Coin type to merge/split')
        .option('--transfer <recipientAddress>', 'Used with --split to transfer the split coins to the recipient address')
        .action((options) => {
            mainProcessor(options, CoinManager.processCommand);
        })
        .parse();
}
