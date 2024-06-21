const { TransactionBlock } = require('@mysten/sui.js/transactions');
const { saveConfig } = require('../evm/utils');
const { Command } = require('commander');
const { addBaseOptions } = require('./cli-utils');
const { getWallet } = require('./sign-utils');
const { printInfo, printError, validateParameters } = require('../evm/utils');
const {
    utils: { parseUnits, formatUnits },
} = require('ethers');
const { loadSuiConfig, SUI_COIN_ID, isGasToken, signAndBroadcast, paginateAll } = require('./utils');

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
            printInfo(
                'Total Balance',
                `${coins.totalBalance} (${formatUnits(coins.totalBalance.toString(), metadata.decimals).toString()})`,
            );
            printInfo('Total Objects', coins.data.length);
        }
    }

    static async splitCoins(tx, client, coinTypeToCoins, args, options) {
        const coinType = options.coinType || SUI_COIN_ID;

        const metadata = await client.getCoinMetadata({
            coinType,
        });

        if (!metadata) {
            printError('No metadata found for', coinType);
            process.exit(0);
        }

        const splitAmount = parseUnits(args.splitAmount, metadata.decimals).toBigInt();

        const coins = coinTypeToCoins[coinType];
        const firstObjectId = isGasToken(coinType) ? tx.gas : coins.data[0].coinObjectId;
        const [coin] = tx.splitCoins(firstObjectId, [splitAmount]);

        printInfo('Split Coins', coinType);
        printInfo('Split Amount', splitAmount);

        if (options.transfer) {
            tx.transferObjects([coin], options.transfer);
            printInfo('Transfer Coins to', options.transfer);
        }

        // The transaction will fail if the gas budget is not set for splitting coins transaction
        tx.setGasBudget(1e8);
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

async function processSplitCommand(chain, args, options) {
    printInfo('Action', 'Split Coins');
    const [keypair, client] = getWallet(chain, options);

    const coinTypeToCoins = await CoinManager.getAllCoins(client, keypair.toSuiAddress());

    const tx = new TransactionBlock();
    validateParameters({
        isValidNumber: { splitAmount: args.splitAmount },
    });

    await CoinManager.splitCoins(tx, client, coinTypeToCoins, args, options);

    await signAndBroadcast(client, keypair, tx);
}

async function processMergeCommand(chain, args, options) {
    printInfo('Action', 'Merge Coins');
    const [keypair, client] = getWallet(chain, options);
    const coinTypeToCoins = await CoinManager.getAllCoins(client, keypair.toSuiAddress());

    const tx = new TransactionBlock();
    const hasMerged = await CoinManager.mergeCoins(tx, coinTypeToCoins, options);

    if (!hasMerged) {
        printInfo('No coins to merge');
    }

    await signAndBroadcast(client, keypair, tx);
}

async function processListCommand(chain, args, options) {
    printInfo('Action', 'List Coins');
    const [keypair, client] = getWallet(chain, options);
    printInfo('Wallet Address', keypair.toSuiAddress());

    const coinTypeToCoins = await CoinManager.getAllCoins(client, keypair.toSuiAddress());
    await CoinManager.printCoins(client, coinTypeToCoins);
}

async function mainProcessor(options, processor, args = {}) {
    const config = loadSuiConfig(options.env);
    await processor(config.sui, args, options);
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

    // Define options, arguments, and actions for each sub-program
    mergeProgram.option('--coin-type <coinType>', 'Coin type to merge').action((options) => {
        mainProcessor(options, processMergeCommand);
    });

    splitProgram
        .argument('<amount>', 'Amount should be in the full coin unit (e.g. 1.5 for 1_500_000_000 coins)')
        .option('--transfer <recipientAddress>', 'Used with split command to transfer the split coins to the recipient address')
        .option('--coin-type <coinType>', 'Coin type to split')
        .action((splitAmount, options) => {
            const args = { splitAmount };
            mainProcessor(options, processSplitCommand, args);
        });

    listProgram.action((options) => {
        mainProcessor(options, processListCommand);
    });

    // Add sub-programs to the main program
    program.addCommand(mergeProgram);
    program.addCommand(splitProgram);
    program.addCommand(listProgram);

    // Add base options to all sub-programs
    program.commands.forEach((cmd) => addBaseOptions(cmd));

    program.parse();
}
