const { loadConfig, printInfo } = require('../common/index.js');
const { addBaseOptions } = require('../common/cli-utils.js');
const { addTopUpOptions } = require('./cli-utils.js');

const { getWallet } = require('./sign-utils.js');
const { getContractJSON, deriveAccounts, printWalletInfo } = require('./utils.js');

const { Command } = require('commander');
const ethers = require('ethers');

const topUpAccounts = async (wallet, options, addressesToFund, decimals, token) => {
    const { target, threshold, units } = options;

    let targetUnits;
    let thresholdUnits;

    if (units) {
        targetUnits = ethers.BigNumber.from(target);
        thresholdUnits = ethers.BigNumber.from(threshold);
    } else {
        targetUnits = ethers.utils.parseUnits(target, decimals);
        thresholdUnits = ethers.utils.parseUnits(threshold, decimals);
    }

    printInfo('Target balance', ethers.utils.formatUnits(targetUnits, decimals));
    printInfo('Threshold', ethers.utils.formatUnits(thresholdUnits, decimals));

    if (thresholdUnits.gt(targetUnits)) {
        throw new Error('threshold must be less than or equal to target balance');
    }

    for (const address of addressesToFund) {
        printInfo('='.repeat(20));
        printInfo(`Funding account with ${token ? 'ERC20 tokens' : 'native cryptocurrency'}`, address);

        const balance = token ? await token.balanceOf(address) : await wallet.provider.getBalance(address);

        printInfo('Current balance', ethers.utils.formatUnits(balance, decimals));

        if (balance.gte(thresholdUnits)) {
            printInfo('Account has sufficient balance. Skipping...');
            continue;
        }

        const amount = targetUnits.sub(balance);

        const tx = token ? await token.transfer(address, amount) : await wallet.sendTransaction({ to: address, value: amount });
        await tx.wait();

        printInfo('Amount transferred', ethers.utils.formatUnits(amount, decimals));
    }
};

const topUpNative = async (wallet, options, addressesToFund) => {
    topUpAccounts(wallet, options, addressesToFund, 18, false);
};

const topUpToken = async (wallet, options, addressesToFund) => {
    const { contract, decimals } = options;
    const token = new ethers.Contract(contract, getContractJSON('ERC20').abi, wallet);
    const tokenDecimals = decimals || (await token.decimals());

    topUpAccounts(wallet, options, addressesToFund, tokenDecimals, token);
};

const mainProcessor = async (processor, options) => {
    const { env, privateKey, chainNames, addressesToDerive, addresses, mnemonic } = options;
    const config = loadConfig(env);

    const rpc = config.chains[chainNames].rpc;
    const provider = new ethers.providers.JsonRpcProvider(rpc);
    const wallet = await getWallet(privateKey, provider);

    await printWalletInfo(wallet, options);

    let addressesToFund = addresses;

    if (addressesToDerive) {
        const derivedAccounts = await deriveAccounts(mnemonic, addressesToDerive);
        addressesToFund = addressesToFund.concat(derivedAccounts.map((account) => account.address));
    }

    await processor(wallet, options, addressesToFund);
};

const programHandler = () => {
    const program = new Command();
    program.name('top-up').description('Top up multiple accounts with native cryptocurrency or ERC20 tokens from a single wallet');

    const topUpNativeCmd = program
        .command('native')
        .description('Top up multiple accounts with native cryptocurrency from a single wallet')
        .action((options) => {
            mainProcessor(topUpNative, options);
        });
    addBaseOptions(topUpNativeCmd, {});
    addTopUpOptions(topUpNativeCmd);

    const topUpTokenCmd = program
        .command('token')
        .description('Top up multiple accounts with ERC20 tokens from a single wallet')
        .requiredOption('--contract <contract>', 'ERC20 token contract address')
        .option('-d, --decimals <decimals>', 'token decimals, if not provided, the contract will be queried for the decimals')
        .action((options) => {
            mainProcessor(topUpToken, options);
        });
    addBaseOptions(topUpTokenCmd, {});
    addTopUpOptions(topUpTokenCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
