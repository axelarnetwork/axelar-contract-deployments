'use strict';

const { Command, Option } = require('commander');
const { AccountId, PrivateKey, TokenAssociateTransaction, TokenId } = require('@hashgraph/sdk');
const { addBaseOptions, printHederaNetwork } = require('./cli-utils');
const { getClient } = require('./client.js');
const { printInfo } = require('../common/utils');

function evmAddressToTokenId(evmAddress) {
    return TokenId.fromSolidityAddress(evmAddress);
}

function tokenIdToEvmAddress(tokenId) {
    return TokenId.fromString(tokenId).toSolidityAddress();
}

async function associateToken(_config, tokenId, options) {
    const client = await getClient(options.accountId, options.privateKey, options.hederaNetwork);

    printHederaNetwork(options);

    const accountId = AccountId.fromString(options.accountId);
    const privateKey = PrivateKey.fromStringECDSA(options.privateKey);

    printInfo('Account EVM Address', accountId.toSolidityAddress());

    if (tokenId.length >= 40) {
        tokenId = evmAddressToTokenId(tokenId);
    }

    printInfo('Token ID', tokenId.toString());
    printInfo('Token EVM Address', tokenIdToEvmAddress(tokenId));

    if (prompt(`Proceed with associating?`, options.yes)) {
        return;
    }

    try {
        const associateTx = new TokenAssociateTransaction().setAccountId(accountId).setTokenIds([tokenId]).freezeWith(client);

        const signTx = await associateTx.sign(privateKey);
        const submitTx = await signTx.execute(client);
        const receipt = await submitTx.getReceipt(client);

        printInfo('Token associated with account successfully');
        printInfo('Transaction ID', submitTx.transactionId.toString());
        printInfo('Receipt status', receipt.status.toString());
    } catch (error) {
        throw new Error(`Token association failed: ${error.messsage}`);
    }
}

if (require.main === module) {
    const program = new Command();

    program
        .name('associate-token')
        .description('Associate a token with a Hedera account')
        .argument('<tokenId>', 'Token ID in Hedera format (0.0.xxxxx)')
        .action((tokenId, options) => {
            associateToken(null, tokenId, options);
        });

    addBaseOptions(program);

    program.parse();
}

module.exports = {
    associateToken,
};
