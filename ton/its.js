#!/usr/bin/env node
const { Command } = require('commander');
const { toNano, Address, beginCell } = require('@ton/ton');
const { getTonClient, loadWallet, waitForTransaction, sendTransactionWithCost } = require('./common');

// ITS contract address from environment
const ITS_ADDRESS = process.env.TON_ITS_ADDRESS;

if (!ITS_ADDRESS) {
    throw new Error('Please set TON_ITS_ADDRESS in your .env file');
}

// Operation codes for ITS
const ITS_OPS = {
    DEPLOY_INTERCHAIN_TOKEN: 0x0000006b,
    DEPLOY_REMOTE_INTERCHAIN_TOKEN: 0x00000069,
    DEPLOY_REMOTE_INTERCHAIN_TOKEN_LOG: 0x0000006a,
    CHANGE_OPERATOR: 0x0000006c,
    ADD_TRUSTED_CHAIN: 0x0000006d,
    REMOVE_TRUSTED_CHAIN: 0x0000006e,
    REGISTER_CANONICAL_INTERCHAIN_TOKEN: 0x0000006f,
    DEPLOY_REMOTE_CANONICAL_INTERCHAIN_TOKEN: 0x00000070,
    INTERCHAIN_TOKEN_TRANSFER: 0x00000071,
    REGISTER_CUSTOM_TOKEN: 0x00000072,
    LINK_TOKEN: 0x00000073,
    INTERCHAIN_TOKEN_ID_CLAIMED_LOG: 0x00000074,
    INTERCHAIN_TOKEN_TRANSFER_SEND: 0x00000075,
    INTERCHAIN_TOKEN_TRANSFER_TAKE: 0x00000076,
    APPROVE_REMOTE_DEPLOYMENT: 0x00000077,
    DEPLOY_REMOTE_INTERCHAIN_TOKEN_FROM_TOKEN_MANAGER: 0x00000100,
    REGISTER_TOKEN_METADATA: 0x00000101,
    INTERCHAIN_TOKEN_TRANSFER_FROM: 0x00000102,
    INTERCHAIN_TOKEN_TRANSFER_TAKE_FROM: 0x00000103,
    TOKEN_METADATA_REGISTERED_LOG: 0x00000104,
    LINK_TOKEN_STARTED_LOG: 0x00000105,
    INTERCHAIN_TRANSFER_RECEIVED_LOG: 0x00000106,
    INTERCHAIN_TOKEN_DEPLOYMENT_STARTED_LOG: 0x00000107,
    TOKEN_MANAGER_DEPLOYED_LOG: 0x00000108,
    INTERCHAIN_TOKEN_DEPLOYED_LOG: 0x00000109,
    INTERCHAIN_TRANSFER_LOG: 0x00000110,
};

const program = new Command();
program.name('its').description('Axelar TON Interchain Token Service CLI').version('1.0.0');

async function executeITSOperation(operationName, messageBody, cost) {
    try {
        const client = getTonClient();
        const { contract, key } = await loadWallet(client);
        const itsAddress = Address.parse(ITS_ADDRESS);

        const { transfer, seqno } = await sendTransactionWithCost(contract, key, itsAddress, messageBody, cost);

        console.log(`✅ ${operationName} transaction sent successfully!`);

        await waitForTransaction(contract, seqno);
    } catch (error) {
        console.error(`❌ Error in ${operationName}:`, error.message);
        process.exit(1);
    }
}

function buildDeployInterchainTokenMessage(salt, name, symbol, decimals, initialSupply, minter) {
    const nameCell = beginCell().storeStringTail(name).endCell();

    const symbolCell = beginCell().storeStringTail(symbol).endCell();

    const minterAddress = minter ? Address.parse(minter) : null;
    const minterCell = beginCell();

    if (minterAddress) {
        minterCell.storeAddress(minterAddress);
    } else {
        minterCell.storeBit(0);
    }

    const minterRefCell = minterCell.endCell();

    const message = beginCell()
        .storeUint(ITS_OPS.DEPLOY_INTERCHAIN_TOKEN, 32)
        .storeUint(BigInt(salt), 256)
        .storeRef(nameCell)
        .storeRef(symbolCell)
        .storeUint(decimals, 8)
        .storeUint(BigInt(initialSupply), 256)
        .storeRef(minterRefCell)
        .endCell();

    return message;
}

// Deploy Interchain Token command
program
    .command('deploy-interchain-token')
    .description('Deploy a new interchain token')
    .argument('<salt>', 'Salt value for token deployment (256-bit number or hex string)')
    .argument('<name>', 'Token name')
    .argument('<symbol>', 'Token symbol')
    .argument('<decimals>', 'Token decimals (0-255)')
    .argument('<initial-supply>', 'Initial token supply')
    .argument('[minter]', 'Optional minter address (TON address format)')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.1')
    .action(async (salt, name, symbol, decimals, initialSupply, minter, options) => {
        try {
            // Validate and parse inputs
            const saltBigInt = salt.startsWith('0x') ? salt.slice(2) : salt;

            const decimalsParsed = parseInt(decimals, 10);
            if (isNaN(decimalsParsed) || decimalsParsed < 0 || decimalsParsed > 255) {
                throw new Error('Decimals must be a number between 0 and 255');
            }

            const initialSupplyBigInt = initialSupply;

            console.log('Deploying Interchain Token with parameters:');
            console.log('  Salt:', saltBigInt);
            console.log('  Name:', name);
            console.log('  Symbol:', symbol);
            console.log('  Decimals:', decimalsParsed);
            console.log('  Initial Supply:', initialSupplyBigInt);
            console.log('  Minter:', minter || 'None');
            console.log('  Gas:', options.gas, 'TON');

            const messageBody = buildDeployInterchainTokenMessage(saltBigInt, name, symbol, decimalsParsed, initialSupplyBigInt, minter);

            const cost = toNano(options.gas);
            await executeITSOperation('Deploy Interchain Token', messageBody, cost);
        } catch (error) {
            console.error('❌ Error deploying interchain token:', error.message);
            process.exit(1);
        }
    });

// TODO: Add more commands here as needed
// Examples:
// - deploy-remote-interchain-token
// - register-canonical-token
// - transfer-interchain-token
// - add-trusted-chain
// - remove-trusted-chain
// - change-operator
// etc.

// Parse command line arguments
program.parse();
