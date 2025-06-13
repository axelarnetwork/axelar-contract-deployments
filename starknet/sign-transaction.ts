'use strict';

import { Command } from 'commander';
import { hash, stark, constants, RpcProvider, Account, ec } from 'starknet';
import * as fs from 'fs';
import * as path from 'path';
import { loadConfig } from '../common';
import { getStarknetProvider } from './utils';
import { UnsignedTransaction } from './types';

import TransportNodeHid from '@ledgerhq/hw-transport-node-hid';
import { StarknetClient } from '@ledgerhq/hw-app-starknet';

/**
 * Compute transaction hash for Starknet v3 transaction
 */
function computeTransactionHash(
    transaction: UnsignedTransaction,
    chainId: constants.StarknetChainId
): string {
    // For INVOKE transactions
    if (transaction.type === 'INVOKE') {
        const calldata = transaction.calls.flatMap(call => [
            call.contract_address,
            hash.getSelectorFromName(call.entry_point),
            ...call.calldata
        ]);

        return hash.calculateInvokeTransactionHash({
            senderAddress: transaction.sender_address,
            version: transaction.version as any,
            compiledCalldata: calldata,
            maxFee: '0x0', // v3 doesn't use maxFee
            chainId,
            nonce: transaction.nonce,
            accountDeploymentData: transaction.account_deployment_data || [],
            paymasterData: transaction.paymaster_data || [],
            nonceDataAvailabilityMode: transaction.nonce_data_availability_mode === 'L1' ? 0 : 1,
            feeDataAvailabilityMode: transaction.fee_data_availability_mode === 'L1' ? 0 : 1,
            resourceBounds: transaction.resource_bounds,
            tip: transaction.tip || '0x0',
        });
    }

    throw new Error(`Unsupported transaction type: ${transaction.type}`);
}

/**
 * Sign transaction with Ledger
 */
async function signWithLedger(
    transactionFile: string,
    ledgerPath: string,
    chainId: constants.StarknetChainId
): Promise<void> {
    console.log('📱 Initializing Ledger connection...');

    // Read unsigned transaction
    const transactionData = fs.readFileSync(transactionFile, 'utf8');
    const transaction = JSON.parse(transactionData) as UnsignedTransaction;

    console.log('\n📄 Transaction Details:');
    console.log(`  Type: ${transaction.type}`);
    console.log(`  Sender: ${transaction.sender_address}`);
    console.log(`  Nonce: ${transaction.nonce}`);

    if (transaction.type === 'INVOKE') {
        console.log(`  Calls: ${transaction.calls.length}`);
        transaction.calls.forEach((call, i) => {
            console.log(`    Call ${i + 1}:`);
            console.log(`      Contract: ${call.contract_address}`);
            console.log(`      Entrypoint: ${call.entry_point}`);
            console.log(`      Calldata length: ${call.calldata.length}`);
        });
    }

    // Compute transaction hash
    console.log('\n🔐 Computing transaction hash...');
    const txHash = computeTransactionHash(transaction, chainId);
    console.log(`Transaction hash: ${txHash}`);

    let transport;
    let app;

    try {
        // Connect to Ledger
        console.log('\n🔌 Connecting to Ledger device...');
        transport = await TransportNodeHid.create();
        app = new StarknetClient(transport);

        // Get app version to verify connection
        const version = await app.getAppVersion();
        console.log(`✅ Connected to Starknet app v${version.major}.${version.minor}.${version.patch}`);

        // Get public key for verification
        console.log(`\n🔑 Using derivation path: ${ledgerPath}`);
        const pubKey = await app.getPubKey(ledgerPath);
        console.log(`Public key: 0x${Buffer.from(pubKey.publicKey).toString('hex')}`);

        // Sign the transaction hash
        console.log('\n✍️  Please review and sign the transaction on your Ledger device...');
        console.log('⚠️  Note: You will see the transaction hash on your device screen.');

        console.log({
            accountAddress: transaction.sender_address,
            tip: transaction.tip,
            resourceBounds: transaction.resource_bounds,
            paymaster_data: transaction.paymaster_data,
            chainId: chainId,
            nonce: transaction.nonce, // TODO: Set correct one by querying contract. Will it work if a lot of calls are made to the contract?
            nonceDataAvailabilityMode: transaction.nonce_data_availability_mode,
            feeDataAvailabilityMode: transaction.fee_data_availability_mode,
            account_deployment_data: transaction.account_deployment_data
        });

        const signature = await app.signTx(ledgerPath, transaction.calls, {
            accountAddress: transaction.sender_address,
            tip: transaction.tip,
            resourceBounds: transaction.resource_bounds,
            paymaster_data: transaction.paymaster_data,
            chainId: chainId,
            nonce: transaction.nonce, // TODO: Set correct one by querying contract. Will it work if a lot of calls are made to the contract?
            nonceDataAvailabilityMode: transaction.nonce_data_availability_mode,
            feeDataAvailabilityMode: transaction.fee_data_availability_mode,
            account_deployment_data: transaction.account_deployment_data
        });

        // Check if signature contains an error
        if (signature.errorMessage || signature.returnCode) {
            throw new Error(`${signature.errorMessage || 'Unknown error'} (return code: ${signature.returnCode})`);
        }

        console.log('\n✅ Transaction signed successfully!');
        console.log(`Signature: ${JSON.stringify(signature)}`);

        // Create signed transaction object
        // Handle different signature formats from Ledger
        let signatureArray: string[];
        if (Array.isArray(signature)) {
            signatureArray = signature;
        } else if (signature.r && signature.s) {
            signatureArray = [signature.r, signature.s];
        } else {
            throw new Error(`Unexpected signature format: ${JSON.stringify(signature)}`);
        }

        const signedTransaction = {
            ...transaction,
            signature: signatureArray
        };

        // Save signed transaction
        const dir = path.dirname(transactionFile);
        const basename = path.basename(transactionFile, '.json');
        const signedFile = path.join(dir, `${basename}_signed.json`);

        fs.writeFileSync(signedFile, JSON.stringify(signedTransaction, null, 2));
        console.log(`\n💾 Signed transaction saved to: ${signedFile}`);

        console.log('\n📋 Next steps:');
        console.log('1. For single-signature accounts: Use broadcast-transaction.ts to submit');
        console.log('2. For multisig accounts: Collect more signatures with combine-signatures.ts');

    } catch (error: any) {
        if (error.message?.includes('0x6985')) {
            console.error('❌ Transaction rejected on device');
        } else if (error.message?.includes('0x6e00')) {
            console.error('❌ Starknet app not open on device');
        } else {
            console.error('❌ Error signing transaction:', error.message);
        }
        throw error;
    } finally {
        if (transport) {
            await transport.close();
        }
    }
}

async function main(): Promise<void> {
    const program = new Command();

    program
        .name('sign-transaction')
        .description('Sign Starknet transaction with Ledger hardware wallet')
        .version('1.0.0')
        .argument('<transactionFile>', 'path to unsigned transaction JSON file')
        .option('-p, --ledger-path <path>', 'Ledger derivation path', "m/44'/9004'/0'/0/0")
        .option('-e, --env <env>', 'environment (mainnet, testnet, devnet)', 'mainnet')
        .parse();

    const [transactionFile] = program.args;
    const options = program.opts();

    if (!fs.existsSync(transactionFile)) {
        console.error(`❌ Transaction file not found: ${transactionFile}`);
        process.exit(1);
    }

    // Get chain ID from config
    const config = loadConfig(options.env);
    const chain = config.chains.starknet;

    if (!chain) {
        console.error(`❌ Starknet chain not found in ${options.env} configuration`);
        process.exit(1);
    }

    // Determine chain ID based on environment
    let chainId: constants.StarknetChainId;
    switch (options.env) {
        case 'mainnet':
            chainId = constants.StarknetChainId.SN_MAIN;
            break;
        case 'testnet':
            chainId = constants.StarknetChainId.SN_SEPOLIA;
            break;
        default:
            // For devnet/stagenet, we might need to get it from the provider
            const provider = getStarknetProvider(chain);
            const chainIdHex = await provider.getChainId();
            chainId = chainIdHex as constants.StarknetChainId;
    }

    try {
        await signWithLedger(transactionFile, options.ledgerPath, chainId);
    } catch (error: any) {
        console.error('\n❌ Signing failed:', error.message);
        process.exit(1);
    }
}

if (require.main === module) {
    main().catch((error) => {
        console.error('Script failed:', error);
        process.exit(1);
    });
}

export { signWithLedger };
