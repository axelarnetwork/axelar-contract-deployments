'use strict';

require('dotenv').config();
const { Client, PrivateKey, AccountId } = require('@hashgraph/sdk');

function getRpcUrl(hederaNetwork) {
	switch (hederaNetwork) {
		case 'mainnet':
			return 'https://mainnet.hashio.io/api';
		case 'testnet':
			return 'https://testnet.hashio.io/api';
		case 'previewnet':
			return 'https://previewnet.hashio.io/api';
		case 'local': {
			if (!process.env.HEDERA_LOCAL_RPC_URL) {
				console.error('HEDERA_LOCAL_RPC_URL environment variable is not set. It is required for local network.');
				process.exit(1);
			}

			return process.env.HEDERA_LOCAL_RPC_URL;
		}
	}
}

async function getClient(
	hederaId,
	hederaPk,
	hederaNetwork = 'local'
) {

	if (!hederaId || !hederaPk) {
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
	})();

	// Initialize the Hedera client
	const operatorKey = PrivateKey.fromStringECDSA(hederaPk);
	const operatorId = AccountId.fromString(hederaId);

	const client = Client[method]().setOperator(operatorId, operatorKey);


	return client;
}

module.exports = {
	getClient,
	getRpcUrl,
};
