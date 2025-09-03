'use strict';

require('dotenv').config();
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider } = ethers;
const { addSkipPromptOption } = require('./cli-utils.js');
const { loadConfig, prompt, printInfo, printError, getContractJSON } = require('../evm/utils.js');

// See https://docs.hedera.com/hedera/core-concepts/smart-contracts/system-smart-contracts#iexchangerate.sol
// for some context on tinycents and tinybars.
const TINY_PARTS_PER_WHOLE = 100_000_000;
const DEFAULT_TOKEN_CREATION_PRICE_USD = 1;
const DEFAULT_TOKEN_CREATION_PRICE_TINY_CENTS = DEFAULT_TOKEN_CREATION_PRICE_USD * 100 * TINY_PARTS_PER_WHOLE;

// Hedera system contract for exchange rate
const EXCHANGE_RATE_PRECOMPILE_ADDRESS = '0x0000000000000000000000000000000000000168';
const EXCHANGE_RATE_ABI = ['function tinycentsToTinybars(uint256 tinycents) external view returns (uint256 tinybars)'];

function formatPrice(price) {
    const priceNum = ethers.BigNumber.from(price);
    const cents = priceNum.div(TINY_PARTS_PER_WHOLE);
    const usd = cents.div(100);

    return {
        tinycents: `${priceNum.toString()} tinycents`,
        cents: `${cents.toString()} cents`,
        usd: `$${usd.toString()}`,
    };
}

async function queryTokenCreationPrice(_config, options) {
    // Get ABIs from contract JSON files
    const TokenCreationPricing = getContractJSON('TokenCreationPricing');

    try {
        // Load chain configuration
        const config = loadConfig(options.env);
        const chain = config.chains[options.chainName];

        if (!chain) {
            throw new Error(`Chain ${options.chainName} not found in ${options.env} configuration`);
        }

        // Get InterchainTokenService address (which inherits from TokenCreationPricing)
        const itsAddress = chain.contracts?.InterchainTokenService?.address;
        if (!itsAddress) {
            throw new Error(`InterchainTokenService address not found for chain ${options.chainName}`);
        }

        // Get RPC URL and create provider
        const provider = getDefaultProvider(chain.rpc);

        printInfo(`Chain`, `${chain.name} (${options.chainName})`);
        printInfo(`InterchainTokenService address`, itsAddress);

        // Create contract instance
        const tokenCreationPricing = new ethers.Contract(itsAddress, TokenCreationPricing.abi, provider);

        // Get price in tinycents
        const priceInTinycents = await tokenCreationPricing.tokenCreationPrice();
        const formatted = formatPrice(priceInTinycents);

        printInfo('Token creation price (tinycents)', formatted.tinycents);
        printInfo('Token creation price (cents)', formatted.cents);
        printInfo('Token creation price (USD)', formatted.usd);

        // Convert to tinybars using Hedera's exchange rate precompile
        const exchangeRate = new ethers.Contract(EXCHANGE_RATE_PRECOMPILE_ADDRESS, EXCHANGE_RATE_ABI, provider);
        const priceInTinybars = await exchangeRate.tinycentsToTinybars(priceInTinycents);

        printInfo('Token creation price (tinybars)', `${priceInTinybars.toString()} tinybars`);
        printInfo('Token creation price (HBAR)', `${ethers.utils.formatUnits(priceInTinybars, 8)} HBAR`);
    } catch (error) {
        printError('Query failed', error.message);
        process.exit(1);
    }
}

async function setTokenCreationPrice(_config, priceStr, options) {
    const InterchainTokenService = getContractJSON('InterchainTokenService');

    try {
        // Load chain configuration
        const config = loadConfig(options.env);
        const chain = config.chains[options.chainName];

        if (!chain) {
            throw new Error(`Chain ${options.chainName} not found in ${options.env} configuration`);
        }

        // Get InterchainTokenService address
        const itsAddress = chain.contracts?.InterchainTokenService?.address;
        if (!itsAddress) {
            throw new Error(`InterchainTokenService address not found for chain ${options.chainName}`);
        }

        // Get RPC URL and create provider
        const provider = getDefaultProvider(chain.rpc);

        // Create wallet from private key
        const wallet = new Wallet(options.privateKey, provider);

        printInfo(`Using wallet address`, wallet.address);
        printInfo(`Chain`, `${chain.name} (${options.chainName})`);
        printInfo(`InterchainTokenService address`, itsAddress);

        // Parse the price in tinycents
        const price = ethers.BigNumber.from(priceStr);
        const formatted = formatPrice(price);

        printInfo(`Setting price to`, formatted.tinycents);
        printInfo(`Equivalent to`, formatted.cents);
        printInfo(`Equivalent to`, formatted.usd);

        // Create contract instance for setting price
        const its = new ethers.Contract(itsAddress, InterchainTokenService.abi, wallet);

        if (prompt(`Proceed with setting token creation price to ${formatted.tinycents}?`, options.yes)) {
            return;
        }

        // Execute price setting
        printInfo('Setting token creation price...');
        const tx = await its.setTokenCreationPrice(price);
        printInfo('Transaction submitted', tx.hash);

        // Wait for confirmation
        const receipt = await tx.wait(chain.confirmations || 1);
        printInfo('Transaction confirmed', `Block: ${receipt.blockNumber}`);
        printInfo('Token creation price set successfully!');
    } catch (error) {
        printError('Setting price failed', error.message);
        process.exit(1);
    }
}

const addCommonOptions = (command) => {
    return command
        .addOption(
            new Option('-n, --chainName <chainName>', 'Chain name to get InterchainTokenService address from')
                .env('CHAIN')
                .makeOptionMandatory(true),
        )
        .addOption(
            new Option('-e, --env <env>', 'Environment configuration to use')
                .choices(['mainnet', 'stagenet', 'testnet', 'devnet-amplifier'])
                .default('devnet-amplifier')
                .env('ENV'),
        );
};

if (require.main === module) {
    const program = new Command();

    addSkipPromptOption(program);

    // Query price
    addCommonOptions(
        program.command('price').description('Get token creation price in all formats (tinycents, cents, USD, tinybars, HBAR)'),
    ).action((options) => {
        queryTokenCreationPrice(null, options);
    });

    // Set price
    addCommonOptions(
        program.command('set-price').description('Set token creation price in tinycents').argument('<price>', 'Price value in tinycents'),
    )
        .addOption(new Option('-p, --privateKey <privateKey>', 'hex encoded private key').makeOptionMandatory(true).env('PRIVATE_KEY'))
        .action((price, options) => {
            setTokenCreationPrice(null, price, options);
        });

    program.parse();
}

module.exports = {
    TINY_PARTS_PER_WHOLE,
    DEFAULT_TOKEN_CREATION_PRICE_USD,
    DEFAULT_TOKEN_CREATION_PRICE_TINY_CENTS,
    queryTokenCreationPrice,
    setTokenCreationPrice,
    formatPrice,
};
