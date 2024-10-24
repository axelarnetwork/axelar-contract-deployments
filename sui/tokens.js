const { Transaction } = require('@mysten/sui/transactions');
const { Command } = require('commander');
const { loadConfig, saveConfig, printInfo, printError } = require('../common/');
const {
    addBaseOptions,
    parseSuiUnitAmount,
    addOptionsToCommands,
    broadcast,
    getWallet,
    suiCoinId,
    isGasToken,
    paginateAll,
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

    await broadcast(client, keypair, tx, 'Splitted Coins');
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

    await broadcast(client, keypair, tx, 'Merged Coins');
}

async function processListCommand(keypair, client, args, options) {
    printInfo('Action', 'List Coins');
    printInfo('Wallet Address', keypair.toSuiAddress());

    const coinTypeToCoins = await CoinManager.getAllCoins(client, keypair.toSuiAddress());
    await CoinManager.printCoins(client, coinTypeToCoins);
}

async function mainProcessor(options, processor, args = {}) {
    const config = loadConfig(options.env);
    const [keypair, client] = getWallet(config.chains.sui, options);
    await processor(keypair, client, args, options);
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
        .argument('<amount>', 'Amount should be in the full coin unit (e.g. 1.5 for 1_500_000_000 coins)', parseSuiUnitAmount)
        .option('--transfer <recipientAddress>', 'Used with split command to transfer the split coins to the recipient address')
        .option('--coin-type <coinType>', 'Coin type to split')
        .action((splitAmount, options) => {
            mainProcessor(options, processSplitCommand, { splitAmount });
        });

    listProgram.action((options) => {
        mainProcessor(options, processListCommand);
    });

    // Add sub-programs to the main program
    program.addCommand(mergeProgram);
    program.addCommand(splitProgram);
    program.addCommand(listProgram);

    // Add base options to all sub-programs
    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
