'use strict';

const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider } = ethers;
const { addBaseOptions, printHederaNetwork } = require('./cli-utils');
const { prompt } = require('../common/utils.js');
const { getRpcUrl } = require('./client.js');

// Basic WHBAR ABI for deposit, transfer, and balanceOf functions
const WHBAR_ABI = [
    'function deposit() payable',
    'function transfer(address to, uint256 amount) returns (bool)',
    'function balanceOf(address account) view returns (uint256)',
    'function withdraw(uint256 amount)'
];

async function fundWithWHBAR(whbar, targetAddress, amount, wallet) {
    console.log(`Funding ${targetAddress} with ${amount} HBAR worth of WHBAR...`);

    // Deposit HBAR to get WHBAR
    const depositTx = await whbar.connect(wallet).deposit({ value: amount });
    await depositTx.wait();

    console.log('Deposited funds.');
    const ownBalance = await whbar.balanceOf(wallet.address);
    console.log(`${wallet.address} WHBAR balance: ${ethers.utils.formatUnits(ownBalance, 8)} WHBAR`);

    // Transfer WHBAR if target is different from wallet
    if (targetAddress.toLowerCase() !== wallet.address.toLowerCase()) {
        // See https://docs.hedera.com/hedera/core-concepts/smart-contracts/wrapped-hbar-whbar
        // as to why we need to scale down the amount
        const scale = 10 ** 10;
        const transferTx = await whbar.connect(wallet).transfer(targetAddress, amount / scale);
        await transferTx.wait();
    }

    const balance = await whbar.balanceOf(targetAddress);
    console.log(`${targetAddress} WHBAR balance: ${ethers.utils.formatUnits(balance, 8)} WHBAR`);
}

async function fundWhbar(_config, options) {
		printHederaNetwork(options);

    try {
        // Get RPC URL from environment
        const provider = getDefaultProvider(getRpcUrl(options.hederaNetwork));

        // Create wallet from private key
        const wallet = new Wallet(options.privateKey, provider);
        console.log(`Using wallet address: ${wallet.address}`);

        const accountBalance = await wallet.getBalance();
        console.log(`Account balance: ${ethers.utils.formatEther(accountBalance)} HBAR`);

        // Create WHBAR contract instance
        const whbar = new ethers.Contract(options.whbarAddress, WHBAR_ABI, provider);
        console.log(`Using WHBAR contract at: ${options.whbarAddress}`);

        // Parse amount
        const amount = ethers.utils.parseEther(options.amount.toString());

        if (accountBalance.lt(amount)) {
        		console.error(`Insufficient balance. Your account has ${ethers.utils.formatEther(accountBalance)} HBAR, but you need ${ethers.utils.formatEther(amount)} HBAR to fund ${options.to}.`);
						process.exit(1);
				}

       	if (prompt(`Proceed with funding ${options.to} with ${options.amount.toFixed(8)} WHBAR?`, options.yes)) {
          return;
        }

        // Call the funding function
        await fundWithWHBAR(whbar, options.to, amount, wallet);

        console.log('Funding completed successfully!');

    } catch (error) {
        console.error('Funding failed:', error.message);
        process.exit(1);
    }
}

if (require.main === module) {
    const program = new Command();

    program
        .name('fund-whbar')
        .description('Fund an address with WHBAR by depositing HBAR')
        .addOption(
            new Option('--to <address>', 'address to fund with WHBAR')
                .makeOptionMandatory(true)
        )
        .addOption(
            new Option('--whbarAddress <address>', 'address of the WHBAR contract')
            		.env('WHBAR_ADDRESS')
                .makeOptionMandatory(true)
        )
        .addOption(
            new Option('--amount <amount>', 'amount of HBAR to deposit (will be converted to WHBAR)')
                .makeOptionMandatory(true)
                .env('WHBAR_AMOUNT')
                .argParser((value) => parseFloat(value))
        )
        .action((options) => {
            fundWhbar(null, options);
        });

    addBaseOptions(program);

    program.parse();
}

module.exports = {
	WHBAR_ABI,
  fundWithWHBAR,
};
