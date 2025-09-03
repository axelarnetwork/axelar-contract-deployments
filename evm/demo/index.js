'use strict';

require('dotenv').config();
const { ethers } = require('hardhat');
const { getDefaultProvider, Wallet, Contract, utils } = ethers;
const { Command, Option } = require('commander');
const { printInfo, printWarn, printError, prompt, printWalletInfo, mainProcessor, validateParameters } = require('../utils');
const { addEvmOptions } = require('../cli-utils');
const CROSSCHAIN_BURN_ABI = require('../../artifacts/evm/solidity/CrossChainBurn.sol/CrosschainBurn.json').abi;

async function processCommand(chain, action, options) {
    const { privateKey, yes, args, destinationChain, destinationChainTokenAddress, tokenAddress } = options;

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = new Wallet(privateKey, provider);

    await printWalletInfo(wallet);

    const token = new Contract(tokenAddress, CROSSCHAIN_BURN_ABI, wallet);

    printInfo('Chain', chain.name);
    printInfo('Token Address', tokenAddress);

    if (prompt(`Proceed with action ${action} on ${chain.name}?`, yes)) {
        return;
    }

    switch (action) {
        case 'balance': {
            const [address] = args;
            const targetAddress = address || wallet.address;

            validateParameters({ isValidAddress: { targetAddress } });

            const balance = await token.balanceOf(targetAddress);
            const decimals = await token.decimals();
            const symbol = await token.symbol();

            printInfo(`Balance of ${targetAddress}`, `${ethers.utils.formatUnits(balance, decimals)} ${symbol}`);

            break;
        }

        case 'mint': {
            const [to, amount] = args;
            const recipient = to || wallet.address;

            validateParameters({
                isValidAddress: { recipient },
                isNonEmptyString: { amount },
            });

            let decimals = 18;
            let symbol = 'TOKEN';
            try {
                decimals = await token.decimals();
            } catch (e) {
                printWarn('decimals() not found on token; defaulting to 18');
            }
            try {
                symbol = await token.symbol();
            } catch (e) {
                printWarn('symbol() not found on token; defaulting to TOKEN');
            }
            const mintAmount = ethers.utils.parseUnits(amount, decimals);

            printInfo('Minting', `${amount} ${symbol} to ${recipient}`);

            const tx = await token.mint(recipient, mintAmount);
            printInfo('Transaction hash', tx.hash);

            const receipt = await tx.wait();
            printInfo('Transaction confirmed', `Block ${receipt.blockNumber}`);

            const newBalance = await token.balanceOf(recipient);
            printInfo('New balance', `${ethers.utils.formatUnits(newBalance, decimals)} ${symbol}`);

            break;
        }
        case 'cross-chain-burn': {
            const [targetAccount, amount] = args;

            const account = targetAccount || wallet.address;

            validateParameters({
                isValidAddress: { account },
                isNonEmptyString: { amount },
            });

            
            // TODO: Refactor token to include whitelisted address to cross-chain burn
            // const owner = await token.owner();
            // if (owner.toLowerCase() !== wallet.address.toLowerCase()) {
            //     printError(`Only the owner (${owner}) can execute cross-chain burns`);
            //     return;
            // }
            const decimals = await token.decimals();
            const symbol = await token.symbol();
            const burnAmount = ethers.utils.parseUnits(amount, decimals);

            // Convert account address to bytes
            const accountBytes = ethers.utils.arrayify(account);

            // Setup destination
            const destinationChainsAndAddresses = [
                {
                    destinationChain: destinationChain,
                    destinationAddress: destinationChainTokenAddress,
                },
            ];

            // Estimate gas for cross-chain call (typically 0.1-1 native token for testnet)
            const gasPayment = ethers.utils.parseEther('0.5');

            printInfo('Cross-chain Burn Parameters:');
            printInfo('- Account', account);
            printInfo('- Amount', `${amount} ${symbol}`);
            printInfo('- Source Chain', chain.name);
            printInfo('- Destination Chain', destinationChain);
            printInfo('- Destination Token Address', destinationChainTokenAddress);
            printInfo('- Gas Payment', ethers.utils.formatEther(gasPayment));

            const tx = await token.burnFromCrossChain(accountBytes, burnAmount, destinationChainsAndAddresses, {
                value: gasPayment,
            });

            printInfo('Transaction hash', tx.hash);
            printInfo('Waiting for confirmation...');

            const receipt = await tx.wait();
            printInfo('Transaction confirmed', `Block ${receipt.blockNumber}`);

            printInfo('\n=== Cross-chain Burn Initiated ===');
            printInfo('The message has been sent to Axelar.');
            printInfo('It will take 3-5 minutes to be relayed to the destination chain.');
            printInfo(`Monitor at: https://testnet.axelarscan.io/gmp/${tx.hash}`);

            break;
        }

        default: {
            throw new Error(`Unknown action: ${action}`);
        }
    }
}

async function main(action, args, options) {
    options.args = args;

    return mainProcessor(options, async (axelar, chain, chains) => {
        // For cross-chain operations, we might need to setup both chains
        return processCommand(chain, action, options);
    });
}

if (require.main === module) {
    const program = new Command();
    program.name('crosschain-burn-demo').description('Demo script for CrosschainBurn token operations');

    // Balance command
    program
        .command('balance')
        .description('Check token balance')
        .argument('[address]', 'Address to check (defaults to wallet address)')
        .action((address, options, cmd) => {
            main(cmd.name(), [address].filter(Boolean), options);
        });

    // Mint command
    program
        .command('mint')
        .description('Mint tokens (owner only)')
        .argument('[recipient]', 'Recipient address (defaults to wallet address)')
        .argument('<amount>', 'Amount to mint')
        .action((recipient, amount, options, cmd) => {
            main(cmd.name(), [recipient, amount].filter(Boolean), options);
        });

    // Cross-chain burn command
    program
        .command('cross-chain-burn')
        .description('Execute cross-chain burn')
        .argument('[account]', 'Account whose tokens to burn (defaults to wallet address)')
        .argument('<amount>', 'Amount to burn')
        .action((account, amount, options, cmd) => {
            main(cmd.name(), [account, amount].filter(Boolean), options);
        });

    // Add options to each command
    program.commands.forEach((cmd) => {
        // Token address is always required
        cmd.addOption(new Option('--tokenAddress <address>', 'Token contract address').makeOptionMandatory(true));

        // Only add cross-chain specific options to commands that need them
        if (cmd.name() === 'cross-chain-burn') {
            cmd.addOption(
                new Option('--destinationChain <chain>', 'Destination chain for cross-chain operations').default('ethereum-sepolia'),
            );
            cmd.addOption(new Option('--destinationChainTokenAddress <address>', 'Destination chain token contract address'));
        }

        // Add common EVM options to each command
        addEvmOptions(cmd, {
            artifactPath: false,
            contractName: false,
        });
    });

    program.parse();
}

module.exports = { main };
