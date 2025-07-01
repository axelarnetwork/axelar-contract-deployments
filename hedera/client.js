'use strict';

require('dotenv').config();
const { Client, PrivateKey, AccountId } = require('@hashgraph/sdk');

async function getClient(
	hederaId,
	hederaPk,
	hederaNetwork = 'local'
) {

	if(!hederaId || !hederaPk) {
		console.error('Hedera ID and Private Key are required.');
		process.exit(1);
	}

const method = (() => {
	switch (hederaNetwork) {
		case 'mainnet':
			return 'forMainnet';
		case 'testnet':
			return 'forTestnet';
		case 'previewnet':
			return 'forPreviewnet';
		case 'local':
			return 'forLocalNode';
		default:
			console.error(`Unsupported Hedera network: ${hederaNetwork}`);
			process.exit(1);
	}
}) ();

	// Initialize the Hedera client
	const operatorKey = PrivateKey.fromStringECDSA(hederaPk);
	const operatorId = AccountId.fromString(hederaId);

	const client = Client[method]().setOperator(operatorId, operatorKey);

	const balance = await client.balance

	return client;
}

module.exports = {
	getClient,
}
