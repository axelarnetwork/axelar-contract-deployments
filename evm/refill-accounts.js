const { loadConfig, printInfo } = require('../common/index.js');
const { Command, Option } = require('commander');
const { addBaseOptions } = require('../common/cli-utils.js');

const { getWallet } = require('./sign-utils.js');
const { getContractJSON, deriveAccounts, printWalletInfo } = require('./utils.js');

const ethers = require('ethers');

const refillAccounts = async (wallet, options, addressesToFund, decimals, token) => {
    const { targetBalance, minThreshold, inDecimalUnits } = options;

    let targetBalanceUnits = targetBalance;
    let minThresholdUnits = minThreshold;

    if (inDecimalUnits) {
        targetBalanceUnits = ethers.BigNumber.from(targetBalance);
        minThresholdUnits = ethers.BigNumber.from(minThreshold);
    } else {
        targetBalanceUnits = ethers.utils.parseUnits(targetBalance, decimals);
        minThresholdUnits = ethers.utils.parseUnits(minThreshold, decimals);
    }

    printInfo('Target balance', ethers.utils.formatUnits(targetBalanceUnits, decimals));
    printInfo('Min threshold', ethers.utils.formatUnits(minThresholdUnits, decimals));

    for (const address of addressesToFund) {
        console.log('='.repeat(20));
        printInfo(`Funding account with ${token ? 'ERC20' : 'native'} tokens`, address);

        const balance = token ? await token.balanceOf(address) : await wallet.provider.getBalance(address);

        printInfo('Current balance', ethers.utils.formatUnits(balance, decimals));

        if (balance.gt(minThresholdUnits)) {
            printInfo('Account has sufficient balance. Skipping...');
            continue;
        }

        const amount = targetBalanceUnits.sub(balance);

        if (token) {
            await token
                .transfer(address, amount)
                .then((tx) => tx.wait())
                .then(() => printInfo(`Funded account with ERC20 tokens`, ethers.utils.formatUnits(amount, decimals)));
        } else {
            await wallet
                .sendTransaction({
                    to: address,
                    value: amount,
                })
                .then((tx) => tx.wait())
                .then(() => printInfo('Funded account with native tokens', ethers.utils.formatUnits(amount, decimals)));
        }
    }
};

const refillNative = async (wallet, options, addressesToFund) => {
    refillAccounts(wallet, options, addressesToFund, 18, false);
};

const refillErc20 = async (wallet, options, addressesToFund) => {
    const { tokenAddress, decimals } = options;
    const token = new ethers.Contract(tokenAddress, getContractJSON('ERC20').abi, wallet);
    const tokenDecimals = decimals || (await token.decimals());

    refillAccounts(wallet, options, addressesToFund, tokenDecimals, token);
};

const mainProcessor = async (processor, options) => {
    const { env, privateKey, chainNames, addressesToDerive, addresses, mnemonic } = options;
    const config = loadConfig(env);

    const rpc = config.chains[chainNames].rpc;
    const provider = new ethers.providers.JsonRpcProvider(rpc);
    const wallet = await getWallet(privateKey, provider);

    await printWalletInfo(wallet, options);

    if (addressesToDerive) {
        result = await deriveAccounts(mnemonic, addressesToDerive);
    }

    const addressesToFund = addresses || result.map((account) => account.address);

    await processor(wallet, options, addressesToFund);
};

const addOptions = (program) => {
    program.addOption(new Option('-t, --target-balance <target-balance>', 'desired balance after refill'));
    program.addOption(new Option('-u, --in-decimal-units', 'amounts are in decimal units'));
    program.addOption(
        new Option(
            '--addresses-to-derive <addresses-to-derive>',
            'quantity of addresses to derive from mnemonic. Cannot be used with --addresses',
        ).env('DERIVE_ACCOUNTS'),
    );
    program.addOption(
        new Option('--addresses <addresses>', 'comma separated list of addresses to fund').argParser((addresses) =>
            addresses.split(',').map((address) => address.trim()),
        ),
    );
    program.addOption(new Option('--min-threshold <min-threshold>', 'refills accounts only if the balance is below this threshold'));
    program.addOption(new Option('-m, --mnemonic <mnemonic>', 'mnemonic').env('MNEMONIC'));

    program.hook('preAction', (command) => {
        const targetBalance = command.opts().targetBalance;
        const minThreshold = command.opts().minThreshold;

        if (minThreshold >= targetBalance) {
            throw new Error('min-threshold must be less than target balance');
        }
    });
};

const programHandler = () => {
    const program = new Command();
    program.name('refill-accounts').description('Refill multiple accounts funds with native tokens or ERC20 tokens from a single wallet');

    const refillNativeCmd = program
        .command('native')
        .description('Refill with native tokens from a single wallet')
        .action((options) => {
            mainProcessor(refillNative, options);
        });
    addBaseOptions(refillNativeCmd, {});
    addOptions(refillNativeCmd);

    const refillErc20Cmd = program
        .command('erc20')
        .description('Refill with ERC20 tokens from a single wallet')
        .option('--token-address <token-address>', 'Token address')
        .option('-d, --decimals <decimals>', 'token decimals')
        .action((options) => {
            mainProcessor(refillErc20, options);
        });
    addBaseOptions(refillErc20Cmd, {});
    addOptions(refillErc20Cmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
