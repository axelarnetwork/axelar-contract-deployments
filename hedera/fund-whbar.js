'use strict';

require('dotenv').config();
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider } = ethers;
const { addBaseOptions, printHederaNetwork, addSkipPromptOption } = require('./cli-utils.js');
const { loadConfig, prompt, printInfo, printError } = require('../common/utils.js');
const { getRpcUrl } = require('./client.js');

// Basic WHBAR ABI for deposit, transfer, and balanceOf functions
const WHBAR_ABI = [
    'function deposit() payable',
    'function transfer(address to, uint256 amount) returns (bool)',
    'function balanceOf(address account) view returns (uint256)',
    'function withdraw(uint256 amount)',
];

async function fundWithWHBAR(whbar, targetAddress, amount, wallet) {
    printInfo(`Funding ${targetAddress} with ${amount / 10 ** 18} WHBAR...`);

    // Deposit HBAR to get WHBAR
    const depositTx = await whbar.connect(wallet).deposit({ value: amount });
    await depositTx.wait();

    printInfo('Deposited funds.');
    const ownBalance = await whbar.balanceOf(wallet.address);
    printInfo(`${wallet.address} WHBAR balance`, `${ethers.utils.formatUnits(ownBalance, 8)} WHBAR`);

    // Transfer WHBAR if target is different from wallet
    if (targetAddress.toLowerCase() !== wallet.address.toLowerCase()) {
        printInfo(`Transferring to: ${targetAddress}`);

        // See https://docs.hedera.com/hedera/core-concepts/smart-contracts/wrapped-hbar-whbar
        // as to why we need to scale down the amount
        const scale = 10 ** 10;
        const transferTx = await whbar.connect(wallet).transfer(targetAddress, amount / scale);
        await transferTx.wait();

        const balance = await whbar.balanceOf(targetAddress);
        printInfo(`${targetAddress} WHBAR balance`, `${ethers.utils.formatUnits(balance, 8)} WHBAR`);
    }
}

async function fundWhbar(_config, receiverAddress, options) {
    printHederaNetwork(options);

    try {
        // Load chain configuration
        const config = loadConfig(options.env);
        const chain = config.chains[options.chainName];

        if (!chain) {
            throw new Error(`Chain ${options.chainName} not found in ${options.env} configuration`);
        }

        // Get WHBAR address from config
        const whbarAddress = chain.whbarAddress;
        if (!whbarAddress) {
            throw new Error(`WHBAR address not found for chain ${options.chainName}`);
        }

        // Get RPC URL and create provider
        const provider = getDefaultProvider(chain.rpc || getRpcUrl(options.hederaNetwork));

        // Create wallet from private key
        const wallet = new Wallet(options.privateKey, provider);
        printInfo(`Using wallet address`, wallet.address);
        printInfo(`Chain`, `${chain.name} (${options.chainName})`);
        printInfo(`WHBAR contract address`, whbarAddress);

        const accountBalance = await wallet.getBalance();
        printInfo(`Account balance`, `${ethers.utils.formatEther(accountBalance)} HBAR`);

        // Create WHBAR contract instance
        const whbar = new ethers.Contract(whbarAddress, WHBAR_ABI, provider);

        // Parse amount
        const amount = ethers.utils.parseEther(options.amount.toString());

        if (accountBalance.lt(amount)) {
            printError(
                `Insufficient balance. Your account has ${ethers.utils.formatEther(accountBalance)} HBAR, but you need ${ethers.utils.formatEther(amount)} HBAR to fund ${receiverAddress}.`,
            );
            process.exit(1);
        }

        if (prompt(`Proceed with funding ${receiverAddress} with ${options.amount.toFixed(8)} WHBAR?`, options.yes)) {
            return;
        }

        // Call the funding function
        await fundWithWHBAR(whbar, receiverAddress, amount, wallet);

        printInfo('Funding completed successfully!');
    } catch (error) {
        printError('Funding failed', error.message);
        process.exit(1);
    }
}

if (require.main === module) {
    const program = new Command();

    addBaseOptions(program);
    addSkipPromptOption(program);

    program
        .name('fund-whbar')
        .description('Fund an address with WHBAR by depositing HBAR')
        .argument('<receiverAddress>', 'Address to fund with WHBAR')
        .addOption(new Option('-n, --chainName <chainName>', 'Chain name to get WHBAR address from').env('CHAIN').makeOptionMandatory(true))
        .addOption(
            new Option('-e, --env <env>', 'Environment configuration to use')
                .choices(['mainnet', 'stagenet', 'testnet', 'devnet-amplifier'])
                .default('devnet-amplifier')
                .env('ENV'),
        )
        .addOption(
            new Option('--amount <amount>', 'Amount of HBAR to deposit (will be converted to WHBAR)')
                .makeOptionMandatory(true)
                .env('WHBAR_AMOUNT')
                .argParser((value) => parseFloat(value)),
        )
        .action((receiverAddress, options) => {
            fundWhbar(null, receiverAddress, options);
        });

    program.parse();
}

module.exports = {
    WHBAR_ABI,
    fundWithWHBAR,
};
