'use strict';

const { Asset, Operation } = require('@stellar/stellar-sdk');
const { Command, Option } = require('commander');
const { addBaseOptions, broadcast, getWallet, getNativeBalance } = require('./utils');
const { loadConfig, printInfo, printError, getChainConfig, prompt, validateParameters } = require('../common/utils');

async function processCommand(chain, options) {
    // send-tokens is single-key only: the nativePayment path signs with one key and submits, so it
    // can't co-sign a multisig. (A multisig source would also make the balance check below read the
    // wrong account.) For a multisig / offline native payment, use the stellar CLI instead.
    if (options.sourceAccount || options.offline) {
        printError(
            'send-tokens is single-key only. For a multisig / offline XLM payment use the stellar CLI: ' +
                'stellar tx new payment --source-account <MS> --destination <dst> --asset native --amount <n> --build-only, ' +
                'then stellar tx sign / stellar tx send.',
        );
        return;
    }

    const wallet = await getWallet(chain, options);
    let { amount, recipients, yes } = options;
    recipients = options.recipients.split(',').map((str) => str.trim());

    validateParameters({
        isValidDecimal: { amount },
        isValidStellarAddress: recipients,
    });

    const nativeAssetBalance = await getNativeBalance(chain, wallet.publicKey());
    const totalAmount = amount * recipients.length;

    if (nativeAssetBalance < totalAmount) {
        printError(`Wallet balance ${nativeAssetBalance} has insufficient funds for ${totalAmount}.`);
        return;
    }

    if (prompt(`Send ${amount} XLM to ${recipients}?`, yes)) {
        return;
    }

    options.nativePayment = true;

    for (const recipient of recipients) {
        printInfo(`Sending ${amount} XLM to ${recipient}`);

        const operation = Operation.payment({
            destination: recipient,
            asset: Asset.native(),
            amount,
        });

        await broadcast(operation, wallet, chain, 'Send token', options);
    }
}

async function mainProcessor(options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(chain, options);
}

if (require.main === module) {
    const program = new Command();

    program.name('send-tokens').description('Send native tokens to recipients.');

    addBaseOptions(program);

    program.addOption(new Option('-r, --recipients <recipients>', 'comma-separated recipients of tokens').makeOptionMandatory(true));
    program.addOption(new Option('-a, --amount <amount>', 'amount to transfer (in terms of XLM)').makeOptionMandatory(true));

    program.action((options) => {
        mainProcessor(options, processCommand);
    });

    program.parse();
}
