#!/usr/bin/env node
const { getTonClient, loadWallet } = require('./common');
const { fromNano } = require('@ton/ton');

async function printWalletInfo() {
    try {
        console.log('Loading wallet information...\n');

        const client = getTonClient();
        const { contract, wallet } = await loadWallet(client);

        // Get wallet address in different formats
        const rawAddress = wallet.address.toRawString();
        const friendlyAddress = wallet.address.toString({ urlSafe: true, bounceable: true });
        const nonBounceableAddress = wallet.address.toString({ urlSafe: true, bounceable: false });

        // Get balance
        const balance = await contract.getBalance();
        const balanceInTON = fromNano(balance);

        // Get seqno (transaction count)
        const seqno = await contract.getSeqno();

        console.log('='.repeat(60));
        console.log('WALLET INFORMATION');
        console.log('='.repeat(60));
        console.log('\nRaw Address:');
        console.log(`  ${rawAddress}`);
        console.log('\nUser-Friendly Address (Bounceable):');
        console.log(`  ${friendlyAddress}`);
        console.log('\nUser-Friendly Address (Non-Bounceable):');
        console.log(`  ${nonBounceableAddress}`);
        console.log('\nBalance:');
        console.log(`  ${balanceInTON} TON (${balance.toString()} nanoTON)`);
        console.log('\nTransaction Count (Seqno):');
        console.log(`  ${seqno}`);
        console.log('\n' + '='.repeat(60));
    } catch (error) {
        console.error('❌ Error loading wallet:', error.message);
        process.exit(1);
    }
}

// Run the function
printWalletInfo();
