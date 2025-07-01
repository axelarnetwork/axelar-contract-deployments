'use strict';

const { Command, Option } = require('commander');
const {
    AccountId,
    PrivateKey,
    TokenAssociateTransaction,
    TokenId,
} = require("@hashgraph/sdk");
const { addBaseOptions } = require('./cli-utils');
const { getClient } = require('./client.js');

function evmAddressToTokenId(evmAddress) {
    return TokenId.fromSolidityAddress(evmAddress);
}

function tokenIdToEvmAddress(tokenId) {
	return TokenId.fromString(tokenId).toSolidityAddress();
}

async function associateToken(_config, tokenId, options) {
    const client = await getClient(
        options.accountId,
        options.privateKey,
        options.hederaNetwork,
    );

    const accountId = AccountId.fromString(options.accountId);
    const privateKey = PrivateKey.fromStringECDSA(options.privateKey);

    console.log("Account ID: ", accountId.toString());
    console.log("Account EVM Address: ", accountId.toSolidityAddress());
    console.log("Token ID: ", tokenId);
    console.log("Token EVM Address: ", tokenIdToEvmAddress(tokenId));

    try {
        const associateTx = new TokenAssociateTransaction()
            .setAccountId(accountId)
            .setTokenIds([tokenId])
            .freezeWith(client);

        const signTx = await associateTx.sign(privateKey);
        const submitTx = await signTx.execute(client);
        const receipt = await submitTx.getReceipt(client);

        console.log("Token associated with account successfully");
        console.log("Transaction ID:", submitTx.transactionId.toString());
        console.log("Receipt status:", receipt.status.toString());

    } catch (error) {
        console.error('Token association failed:', error.message);
        process.exit(1);
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
    associateToken
};
