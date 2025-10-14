'use strict';

require('dotenv').config();
const { ethers } = require('hardhat');
const { getDefaultProvider, Wallet, Contract, utils } = ethers;
const { Command, Option } = require('commander');
const { printInfo, printWarn, printError, prompt, printWalletInfo, mainProcessor, validateParameters } = require('../utils');
const { addEvmOptions } = require('../cli-utils');
const { executeSafeTransaction } = require('./utils');
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
        //not needed if using memento as gnosis not deployed there yet.
        case 'setup-burn': {
            const [multisigAddress] = args;
            validateParameters({ isValidAddress: { multisigAddress } });
            printInfo('Transferring ownership to multisig', multisigAddress);
            const tx = await token.transferOwnership(multisigAddress);
            printInfo('Transaction hash', tx.hash);
            const receipt = await tx.wait();
            printInfo('Transaction confirmed', `Block ${receipt.blockNumber}`);
            break;
        }
        case 'cross-chain-burn': {
            const [targetAccount, amount, multisigAddress] = args;

            const account = targetAccount || wallet.address;

            validateParameters({
                isValidAddress: { account },
                isNonEmptyString: { amount },
            });

            const decimals = await token.decimals();
            const burnAmount = ethers.utils.parseUnits(amount, decimals);

            // Estimate gas for cross-chain call (typically 0.1-1 native token for testnet)
            const gasPayment = ethers.utils.parseEther('0.3');

            const burnFromCrossChainFunctionCall = token.interface.encodeFunctionData('burnFromCrossChain', [
                account, //was accountBytes before
                burnAmount,
                destinationChain,
                destinationChainTokenAddress,
            ]);

            let result;

            if (chain.name === 'memento-demo' || chain.name === 'Memento') {
                //memento does not have gnosis safe deployed yet.

                const tx = await token.burnFromCrossChain(account, burnAmount, destinationChain, destinationChainTokenAddress, {
                    value: gasPayment,
                });
                const receipt = await tx.wait();
                result = {
                    hash: tx.hash,
                    blockNumber: receipt.blockNumber,
                };
            } else {
                //gnosis safe deployed on other chains (eventually use this for memento too).

                validateParameters({ isValidAddress: { multisigAddress } });

                result = await executeSafeTransaction({
                    rpc: chain.rpc,
                    multisigAddress,
                    tokenAddress,
                    functionCall: burnFromCrossChainFunctionCall,
                    gasPayment: gasPayment.toString(),
                    privateKey1: process.env.PRIVATE_KEY,
                    privateKey2: process.env.PRIVATE_KEY_SIGNER_TWO,
                });
            }

            printInfo('Transaction hash', result.hash);
            printInfo('Transaction confirmed', `Block ${result.blockNumber}`);

            break;
        }
        case 'cross-chain-freeze': {
            const [targetAccount, multisigAddress] = args;

            const account = targetAccount || wallet.address;

            validateParameters({
                isValidAddress: { account },
            });

            const gasPayment = ethers.utils.parseEther('0.3');

            const freezeAccountCrossChainFunctionCall = token.interface.encodeFunctionData('freezeAccountCrossChain', [
                account,
                destinationChain,
                destinationChainTokenAddress,
            ]);

            let result;

            if (chain.name === 'memento-demo' || chain.name === 'Memento') {
                //memento does not have gnosis safe deployed yet.

                const tx = await token.freezeAccountCrossChain(account, destinationChain, destinationChainTokenAddress, {
                    value: gasPayment,
                });
                const receipt = await tx.wait();
                result = {
                    hash: tx.hash,
                    blockNumber: receipt.blockNumber,
                };
            } else {
                //gnosis safe deployed on other chains (eventually use this for memento too).

                validateParameters({ isValidAddress: { multisigAddress } });

                result = await executeSafeTransaction({
                    rpc: chain.rpc,
                    multisigAddress,
                    tokenAddress,
                    functionCall: freezeAccountCrossChainFunctionCall,
                    gasPayment: gasPayment.toString(),
                    privateKey1: process.env.PRIVATE_KEY,
                    privateKey2: process.env.PRIVATE_KEY_SIGNER_TWO,
                });
            }

            printInfo('Transaction hash', result.hash);
            printInfo('Transaction confirmed', `Block ${result.blockNumber}`);

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

    // Setup cross-chain burn command
    program
        .command('setup-burn')
        .description('Setup cross-chain burn')
        .argument('<multisigAddress>', 'Multisig address')
        .action((multisigAddress, options, cmd) => {
            main(cmd.name(), [multisigAddress].filter(Boolean), options);
        });

    // Cross-chain burn command
    program
        .command('cross-chain-burn')
        .description('Execute cross-chain burn')
        .argument('[account]', 'Account whose tokens to burn (defaults to wallet address)')
        .argument('<amount>', 'Amount to burn')
        .argument('<multisigAddress>', 'Multisig address')
        .action((account, amount, multisigAddress, options, cmd) => {
            main(cmd.name(), [account, amount, multisigAddress].filter(Boolean), options);
        });

    // Cross-chain freeze command
    program
        .command('cross-chain-freeze')
        .description('Execute cross-chain freeze')
        .argument('[account]', 'Account to freeze (defaults to wallet address)')
        .argument('<multisigAddress>', 'Multisig address')
        .action((account, multisigAddress, options, cmd) => {
            main(cmd.name(), [account, multisigAddress].filter(Boolean), options);
        });

    // Add options to each command
    program.commands.forEach((cmd) => {
        // Token address is always required
        cmd.addOption(new Option('--tokenAddress <address>', 'Token contract address').makeOptionMandatory(true));

        // Only add cross-chain specific options to commands that need them
        if (cmd.name() === 'cross-chain-burn' || cmd.name() === 'cross-chain-freeze') {
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
