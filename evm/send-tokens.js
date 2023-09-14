'use strict';

require('dotenv').config();

// const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { parseEther, parseUnits },
} = require('ethers');
const { Command, Option } = require('commander');
const chalk = require('chalk');
const { printInfo, printWalletInfo, isValidPrivateKey, printError } = require('./utils');
const { getLedgerWallet, sendTx, ledgerSign, getAllSignersData, getNonceFromProvider, getLatestNonceFromData, updateSignersData, getLatestNonceAndUpdateData, getSignerData, updateTxNonceAndStatus } = require('./offline-sign-utils.js');
const readlineSync = require('readline-sync');

async function sendTokens(chain, options) {
    let wallet;
    const { privateKey, offline, env, ledgerPath} = options;
    let {amount, recipients, directoryPath, fileName} = options;
    const isOffline = (offline === "true") ? true : false;

    const provider = getDefaultProvider(chain.rpc);
    printInfo(`provider: ${provider}`);
    recipients = options.recipients.split(',').map((str) => str.trim());
    amount = parseEther(amount);

    if (privateKey === 'ledger') {
        wallet = getLedgerWallet(provider, ledgerPath? ledgerPath : undefined);
    } else {
        if (!isValidPrivateKey(privateKey)) {
            throw new Error('Private key is missing/ not provided correctly in the user info');
        }

        wallet = new Wallet(privateKey, provider);
    }

    const signerAddress = await wallet.getAddress();
    const balance = await printWalletInfo(wallet);

    if (balance.lte(amount)) {
        throw new Error(`Wallet has insufficient funds.`);
    }

    if (!options.yes) {
        const anwser = readlineSync.question(
            `Proceed with the transfer of ${chalk.green(options.amount)} ${chalk.green(chain.tokenSymbol)} to ${recipients} on ${
                chain.name
            }? ${chalk.green('(y/n)')} `,
        );
        if (anwser !== 'y') return;
    }

    let gasLimit, gasPrice;
    try {
        gasPrice = parseUnits((await provider.getGasPrice()).toString(), 'gwei');
        const block = await provider.getBlock('latest');
        gasLimit = block.gasLimit.toNumber() / 1000;
    
        printInfo('Gas Price:', gasPrice.toString());
        printInfo('Gas Limit:', gasLimit.toString());
      } catch (error) {
        printError(error.message);
      }

    let nonce = await getNonceFromProvider(provider, signerAddress);
    let signersData = undefined;
    let signerData = undefined;
    if(isOffline) {
        directoryPath = directoryPath || './tx';
        fileName = fileName || env.toLowerCase() + '-' +  chain.name.toLowerCase() + '-' + 'signedTransactions';
        nonce = await getLatestNonceAndUpdateData(directoryPath, fileName, wallet);
        signersData = await getAllSignersData(directoryPath, fileName);
        signerData = await getSignerData(directoryPath, fileName, signerAddress);
    }
    for (const recipient of recipients) {
        printInfo('Recipient', recipient);

        if (privateKey === 'ledger') {
            const [unsignedTx, tx] = await ledgerSign(gasLimit, gasPrice, nonce, chain, wallet, recipient, amount);
            if (isOffline) {
                const tx = {};
                tx.nonce = nonce;
                tx.msg = `This transaction will send ${amount} of native tokens to ${recipient} on chain ${chain.name} with chainId ${chain.chainId}`;
                tx.unsignedTx = unsignedTx;
                tx.status = "PENDING";
                signerData.push(tx);
            } else {
                const response = await sendTx(tx, provider);
                printInfo('Transaction hash', response.transactionHash);
            }
        } else {
            const tx = await wallet.sendTransaction({
                to: recipient,
                value: amount,
            });

            printInfo('Transaction hash', tx.hash);

            await tx.wait();
        }
        ++nonce;
    }
    if(signerData) {
        signersData[signerAddress] = signerData;
        await updateSignersData(directoryPath, fileName, signersData);
    }
}

async function main(options) {
    const config = require(`${__dirname}/../axelar-chains-config/info/${options.env === 'local' ? 'testnet' : options.env}.json`);

    const chains = options.chainNames.split(',').map((str) => str.trim());

    for (const chainName of chains) {
        if (config.chains[chainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chainName} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];

        await sendTokens(chain, options);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('send-tokens').description('Send native tokens to an address.');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(new Option('-r, --recipients <recipients>', 'comma-separated recipients of tokens').makeOptionMandatory(true));
    program.addOption(new Option('-a, --amount <amount>', 'amount to transfer (in terms of ETH)').makeOptionMandatory(true));
    program.addOption(
        new Option('-o, --offline <offline>', 'If this option is set as true, then ').choices(['true', 'false']).makeOptionMandatory(false),
    );
    program.addOption(new Option('-l, --ledgerPath <ledgerPath>', 'The path to identify the account in ledger').makeOptionMandatory(false));
    program.addOption(new Option('-d, --directoryPath <directoryPath>', 'The folder where all the signed tx files are stored').makeOptionMandatory(false));
    program.addOption(new Option('-f, --fileName <fileName>', 'The fileName where the signed tx will be stored').makeOptionMandatory(false));
    program.addOption(new Option('-y, --yes', 'skip prompts'));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = { sendTokens };
}
