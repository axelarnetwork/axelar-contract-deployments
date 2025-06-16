'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { parseEther, parseUnits },
} = ethers;
const { printInfo, printError, printWalletInfo, isAddressArray, mainProcessor, isValidDecimal, prompt, getGasOptions, getContractJSON } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
const { storeSignedTx, getWallet, signTransaction } = require('./sign-utils.js');

async function processCommand(_, chain, options) {
    const { privateKey, offline, env, contract } = options;
    let { amount: amountStr, recipients, nonceOffset } = options;

    const chainName = chain.name.toLowerCase();
    const provider = getDefaultProvider(chain.rpc);

    recipients = options.recipients.split(',').map((str) => str.trim());

    if (!isAddressArray(recipients)) {
        throw new Error('Invalid recipient addresses');
    }

    let isERC20 = false;
    let token = null;
    let tokenDecimals = 18;
    let tokenSymbol = chain.tokenSymbol;

    // Check if this is an ERC20 token transfer
    if (contract) {
        isERC20 = true;
        const wallet = await getWallet(privateKey, provider, options);
        token = new ethers.Contract(contract, getContractJSON('ERC20').abi, wallet);
        
        try {
            tokenDecimals = await token.decimals();
            tokenSymbol = await token.symbol();
        } catch (error) {
            printError(`Failed to get token details from contract ${contract}:`, error.message);
            return;
        }
        
        printInfo('ERC20 Token Contract', contract);
        printInfo('Token Symbol', tokenSymbol);
        printInfo('Token Decimals', tokenDecimals);
    }

    if (!amountStr && options.gasUsage) {
        const gasPrice = parseUnits((await provider.getGasPrice()).toString(), 'wei');
        const gas = gasPrice * parseInt(options.gasUsage);
        amountStr = (gas / 1e18).toString();
    }

    if (!isValidDecimal(amountStr)) {
        throw new Error(`Invalid amount ${amountStr}`);
    }

    const amount = parseUnits(amountStr, tokenDecimals);

    const wallet = await getWallet(privateKey, provider, options);

    const { address, balance } = await printWalletInfo(wallet, options);

    if (!offline) {
        let tokenBalance = balance;
        
        if (isERC20) {
            try {
                tokenBalance = await token.balanceOf(address);
                printInfo(`${tokenSymbol} Token Balance`, `${ethers.utils.formatUnits(tokenBalance, tokenDecimals)} ${tokenSymbol}`);
            } catch (error) {
                printError(`Failed to get token balance:`, error.message);
                return;
            }
        }

        if (tokenBalance.lt(amount)) {
            printError(`Wallet balance ${ethers.utils.formatUnits(tokenBalance, tokenDecimals)} ${tokenSymbol} has insufficient funds for ${amountStr} ${tokenSymbol}.`);
            return;
        }
    }

    const gasOptions = await getGasOptions(chain, options);

    if (
        prompt(
            `Proceed with the transfer of ${chalk.green(amountStr)} ${chalk.green(tokenSymbol)} to ${recipients} on ${chain.name}?`,
            options.yes,
        )
    ) {
        printInfo('Operation Cancelled');
        return;
    }

    for (const recipient of recipients) {
        printInfo('Recipient', recipient);
        let tx;

        if (isERC20) {
            // ERC20 token transfer
            const data = token.interface.encodeFunctionData('transfer', [recipient, amount]);
            tx = {
                to: contract,
                data: data,
                value: 0,
                ...gasOptions,
            };
        } else {
            // Native token transfer
            tx = {
                to: recipient,
                value: amount,
                ...gasOptions,
            };
        }

        const { baseTx, signedTx } = await signTransaction(wallet, chain, tx, options);

        if (offline) {
            const tokenInfo = isERC20 ? `${tokenSymbol} tokens` : 'native tokens';
            const filePath = `./tx/signed-tx-${env}-send-tokens-${chainName}-address-${address}-nonce-${baseTx.nonce}.json`;
            printInfo(`Storing signed Tx offline in file ${filePath}`);

            // Storing the fields in the data that will be stored in file
            const data = {
                msg: `This transaction will send ${amountStr} ${tokenInfo} from ${address} to ${recipient} on chain ${chain.name}`,
                unsignedTx: baseTx,
                signedTx,
                status: 'PENDING',
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

    program.name('send-tokens').description('Send native tokens or ERC20 tokens to an address.');

    addBaseOptions(program);

    program.addOption(new Option('-r, --recipients <recipients>', 'comma-separated recipients of tokens').makeOptionMandatory(true));
    program.addOption(new Option('-a, --amount <amount>', 'amount to transfer (in terms of token units)'));
    program.addOption(new Option('-c, --contract <contract>', 'ERC20 token contract address (if not provided, native tokens will be sent)'));
    program.addOption(new Option('--gasUsage <gasUsage>', 'amount to transfer based on gas usage and gas price').default('50000000'));
    program.addOption(new Option('--offline', 'Run in offline mode'));
    program.addOption(new Option('--nonceOffset <nonceOffset>', 'The value to add in local nonce if it deviates from actual wallet nonce'));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = { sendTokens: processCommand };
}
