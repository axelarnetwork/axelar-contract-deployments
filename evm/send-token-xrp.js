'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { parseEther, parseUnits },
    Contract,
} = ethers;
const { printInfo, printError, printWalletInfo, isAddressArray, mainProcessor, isValidDecimal, prompt, getGasOptions } = require('./utils.js');
const { addBaseOptions } = require('./cli-utils.js');
const { storeSignedTx, getWallet, signTransaction } = require('./sign-utils.js');

// Standard ERC20 ABI for token operations
const ERC20_ABI = [
    'function transfer(address to, uint256 amount) returns (bool)',
    'function transferFrom(address from, address to, uint256 amount) returns (bool)',
    'function balanceOf(address account) view returns (uint256)',
    'function allowance(address owner, address spender) view returns (uint256)',
    'function approve(address spender, uint256 amount) returns (bool)',
    'function decimals() view returns (uint8)',
    'function symbol() view returns (string)',
    'function name() view returns (string)',
];

async function processCommand(_axelar, chain, _chains, options) {
    const { privateKey, offline, env, tokenAddress, transferType } = options;
    let { amount: amountStr, recipients, nonceOffset } = options;

    const provider = getDefaultProvider(chain.rpc);

    recipients = options.recipients.split(',').map((str) => str.trim());

    if (!isAddressArray(recipients)) {
        throw new Error('Invalid recipient addresses');
    }

    if (!amountStr && options.gasUsage) {
        const gasPrice = parseUnits((await provider.getGasPrice()).toString(), 'wei');
        const gas = gasPrice * parseInt(options.gasUsage);
        amountStr = (gas / 1e18).toString();
    }

    if (!isValidDecimal(amountStr)) {
        throw new Error(`Invalid amount ${amountStr}`);
    }

    const wallet = await getWallet(privateKey, provider, options);
    const { address, balance } = await printWalletInfo(wallet, options);

    let amount, tokenContract, tokenInfo;

    // Determine transfer type and setup accordingly
    if (transferType === 'erc20' || tokenAddress) {
        if (!tokenAddress) {
            throw new Error('Token address is required for ERC20 transfers');
        }

        tokenContract = new Contract(tokenAddress, ERC20_ABI, wallet);
        
        try {
            const [decimals, symbol, name] = await Promise.all([
                tokenContract.decimals(),
                tokenContract.symbol(),
                tokenContract.name()
            ]);
            
            tokenInfo = { decimals, symbol, name };
            amount = parseUnits(amountStr, decimals);
            
            printInfo('Token Info', `${name} (${symbol}) - ${decimals} decimals`);
            printInfo('Token Amount', `${amountStr} ${symbol}`);
            
            if (!offline) {
                const tokenBalance = await tokenContract.balanceOf(address);
                printInfo('Token Balance', `${tokenBalance.toString()} ${symbol}`);
                
                if (tokenBalance.lt(amount)) {
                    printError(`Insufficient token balance. Required: ${amount.toString()}, Available: ${tokenBalance.toString()}`);
                    return;
                }
            }
        } catch (error) {
            throw new Error(`Failed to interact with ERC20 token at ${tokenAddress}: ${error.message}`);
        }
    } else {
        // Native token transfer
        amount = parseEther(amountStr);
        printInfo('Native Amount', `${amountStr} ${chain.tokenSymbol}`);
        
        if (!offline) {
            if (balance.lte(amount)) {
                printError(`Wallet balance ${balance} has insufficient funds for ${amount}.`);
                return;
            }
        }
    }

    const gasOptions = await getGasOptions(chain, options);

    const transferDescription = transferType === 'erc20' || tokenAddress 
        ? `ERC20 transfer of ${chalk.green(amountStr)} ${chalk.green(tokenInfo?.symbol || 'tokens')}`
        : `native transfer of ${chalk.green(amountStr)} ${chalk.green(chain.tokenSymbol)}`;

    if (
        prompt(
            `Proceed with the ${transferDescription} to ${recipients} on ${chain.name}?`,
            options.yes,
        )
    ) {
        printInfo('Operation Cancelled');
        return;
    }

    for (const recipient of recipients) {
        printInfo('Recipient', recipient);
        
        let tx, baseTx, signedTx;

        if (transferType === 'erc20' || tokenAddress) {
            // ERC20 transfer
            tx = {
                to: tokenAddress,
                data: tokenContract.interface.encodeFunctionData('transfer', [recipient, amount]),
                value: 0, // No native value for ERC20 transfers
                ...gasOptions,
            };
        } else {
            // Native transfer
            tx = {
                to: recipient,
                value: amount,
                ...gasOptions,
            };
        }

        const signedTxResult = await signTransaction(wallet, chain, tx, options);
        baseTx = signedTxResult.baseTx;
        signedTx = signedTxResult.signedTx;

        if (offline) {
            const transferTypeStr = transferType === 'erc20' || tokenAddress ? 'erc20' : 'native';
            const filePath = `./tx/signed-tx-${env}-send-tokens-${transferTypeStr}-${chain.axelarId.toLowerCase()}-address-${address}-nonce-${baseTx.nonce}.json`;
            printInfo(`Storing signed Tx offline in file ${filePath}`);

            const transferMsg = transferType === 'erc20' || tokenAddress
                ? `This transaction will send ${amount} of ERC20 tokens (${tokenInfo?.symbol}) from ${address} to ${recipient} on chain ${chain.name}`
                : `This transaction will send ${amount} of native tokens from ${address} to ${recipient} on chain ${chain.name}`;

            const data = {
                msg: transferMsg,
                unsignedTx: baseTx,
                signedTx,
                status: 'PENDING',
                transferType: transferType === 'erc20' || tokenAddress ? 'erc20' : 'native',
                tokenAddress: tokenAddress || null,
            };

            storeSignedTx(filePath, data);
            nonceOffset = (parseInt(nonceOffset) || 0) + 1;
        }
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('send-tokens-dual').description('Send native tokens or ERC20 tokens to addresses with automatic detection.');

    addBaseOptions(program);

    program.addOption(new Option('-r, --recipients <recipients>', 'comma-separated recipients of tokens').makeOptionMandatory(true));
    program.addOption(new Option('-a, --amount <amount>', 'amount to transfer (in terms of ETH or token units)'));
    program.addOption(new Option('--gasUsage <gasUsage>', 'amount to transfer based on gas usage and gas price').default('50000000'));
    program.addOption(new Option('--offline', 'Run in offline mode'));
    program.addOption(new Option('--nonceOffset <nonceOffset>', 'The value to add in local nonce if it deviates from actual wallet nonce'));
    
    // New options for dual token support
    program.addOption(new Option('--tokenAddress <tokenAddress>', 'ERC20 token contract address (if not provided, will use native token)'));
    program.addOption(new Option('--transferType <transferType>', 'Type of transfer: native or erc20').choices(['native', 'erc20']).default('auto'));

    program.action((options) => {
        // Auto-detect transfer type if not specified
        if (options.transferType === 'auto') {
            if (options.tokenAddress) {
                options.transferType = 'erc20';
                printInfo('Auto-detected transfer type: ERC20 (token address provided)');
            } else {
                options.transferType = 'native';
                printInfo('Auto-detected transfer type: Native token');
            }
        }
        
        main(options);
    });

    program.parse();
} else {
    module.exports = { sendTokensDual: processCommand };
}
