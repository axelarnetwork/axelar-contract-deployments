'use strict';

require('dotenv').config();
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider, constants } = ethers;
const { addBaseOptions, printHederaNetwork, addSkipPromptOption } = require('./cli-utils.js');
const { loadConfig, prompt, printInfo, printError } = require('../common/utils.js');
const { getRpcUrl } = require('./client.js');

// Basic ERC20 ABI for approve function
const ERC20_ABI = [
    'function approve(address spender, uint256 amount) returns (bool)',
    'function allowance(address owner, address spender) view returns (uint256)',
    'function balanceOf(address account) view returns (uint256)',
    'function symbol() view returns (string)',
    'function decimals() view returns (uint8)',
];

async function approveFactoryWhbar(_config, options) {
    printHederaNetwork(options);

    try {
        // Load chain configuration
        const config = loadConfig(options.env);
        const chain = config.chains[options.chainName];

        if (!chain) {
            throw new Error(`Chain ${options.chainName} not found in ${options.env} configuration`);
        }

        // Get InterchainTokenFactory address
        const factoryAddress = chain.contracts?.InterchainTokenFactory?.address;
        if (!factoryAddress) {
            throw new Error(`InterchainTokenFactory address not found for chain ${options.chainName}`);
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
        printInfo(`WHBAR token address`, whbarAddress);
        printInfo(`InterchainTokenFactory address`, factoryAddress);

        // Create WHBAR contract instance
        const whbar = new ethers.Contract(whbarAddress, ERC20_ABI, wallet);

        // Get token info
        let tokenSymbol = 'WHBAR';
        let decimals = 8;
        try {
            tokenSymbol = await whbar.symbol();
            decimals = await whbar.decimals();
        } catch (error) {
            printInfo('Could not fetch token details, using WHBAR defaults');
        }

        // Check current balance
        const balance = await whbar.balanceOf(wallet.address);
        printInfo(`Current ${tokenSymbol} balance`, `${ethers.utils.formatUnits(balance, decimals)} ${tokenSymbol}`);

        // Check current allowance
        const currentAllowance = await whbar.allowance(wallet.address, factoryAddress);
        const currentAllowanceDisplay = currentAllowance.eq(constants.MaxUint256)
            ? 'MAX (2^256 - 1)'
            : `${ethers.utils.formatUnits(currentAllowance, decimals)} ${tokenSymbol}`;
        printInfo(`Current allowance`, currentAllowanceDisplay);

        // Parse amount (default to max uint256)
        const amount = options.amount === 'max' ? constants.MaxUint256 : ethers.utils.parseUnits(options.amount.toString(), decimals);

        const amountDisplay = amount.eq(constants.MaxUint256)
            ? 'MAX (2^256 - 1)'
            : `${ethers.utils.formatUnits(amount, decimals)} ${tokenSymbol}`;

        printInfo(`Approval amount`, amountDisplay);

        // Check if approval is needed
        if (currentAllowance.eq(constants.MaxUint256) && amount.eq(constants.MaxUint256)) {
            printInfo('Current allowance is already set to MAX - no approval needed');
            return;
        } else if (currentAllowance.gte(amount) && !amount.eq(constants.MaxUint256)) {
            printInfo('Current allowance is already sufficient');

            if (prompt('Do you still want to proceed with the approval?', options.yes)) {
                return;
            }
        }

        if (prompt(`Proceed with approving ${amountDisplay} ${tokenSymbol} for InterchainTokenFactory on ${chain.name}?`, options.yes)) {
            return;
        }

        // Execute approval
        printInfo('Executing approval transaction...');
        const tx = await whbar.approve(factoryAddress, amount);
        printInfo('Transaction submitted', tx.hash);

        // Wait for confirmation
        const receipt = await tx.wait(chain.confirmations || 1);
        printInfo('Transaction confirmed', `Block: ${receipt.blockNumber}`);

        // Verify new allowance
        const newAllowance = await whbar.allowance(wallet.address, factoryAddress);
        const newAllowanceDisplay = newAllowance.eq(constants.MaxUint256)
            ? 'MAX (2^256 - 1)'
            : `${ethers.utils.formatUnits(newAllowance, decimals)} ${tokenSymbol}`;

        printInfo('New allowance', newAllowanceDisplay);
        printInfo('Approval completed successfully!');
    } catch (error) {
        printError('Approval failed', error.message);
        process.exit(1);
    }
}

if (require.main === module) {
    const program = new Command();

    addBaseOptions(program);
    addSkipPromptOption(program);

    program
        .name('approve-factory-whbar')
        .description('Approve WHBAR spending for InterchainTokenFactory')

        .addOption(
            new Option('-n, --chainName <chainName>', 'Chain name to get InterchainTokenFactory address from')
                .env('CHAIN')
                .makeOptionMandatory(true),
        )
        .addOption(
            new Option('-e, --env <env>', 'Environment configuration to use')
                .choices(['mainnet', 'stagenet', 'testnet', 'devnet-amplifier'])
                .default('devnet-amplifier')
                .env('ENV'),
        )
        .addOption(
            new Option('--amount <amount>', 'Amount to approve (use "max" for maximum uint256)').default('max').argParser((value) => {
                if (value === 'max') return 'max';
                const parsed = parseFloat(value);
                if (isNaN(parsed) || parsed < 0) {
                    throw new Error('Amount must be a positive number or "max"');
                }
                return parsed;
            }),
        )
        .action((options) => {
            approveFactoryWhbar(null, options);
        });

    program.parse();
}

module.exports = {
    approveFactoryWhbar,
};
