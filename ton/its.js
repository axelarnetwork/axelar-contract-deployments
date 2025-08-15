#!/usr/bin/env node
const { Command } = require('commander');
const { toNano, Address, beginCell, Cell } = require('@ton/ton');
const { getTonClient, loadWallet, waitForTransaction, sendTransactionWithCost } = require('./common');
const crypto = require('crypto');
const { JettonWallet, JettonMinter } = require('axelar-cgp-ton');
const ITS_DICT_KEY_LENGTH = 256;

const ITS_ADDRESS = process.env.TON_ITS_ADDRESS;

if (!ITS_ADDRESS) {
    throw new Error('Please set TON_ITS_ADDRESS in your .env file');
}

const OP_PAYMENT = 0x00000200;

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
    PAUSE: 0x00000111,
    UNPAUSE: 0x00000112,
};

const program = new Command();
program.name('its').description('Axelar TON Interchain Token Service CLI').version('1.0.0');

function sleep(seconds) {
    return new Promise((resolve) => setTimeout(resolve, seconds * 1000));
}

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

function buildAddTrustedChainMessage(chainName, chainAddress) {
    const chainNameHash = crypto.createHash('sha256').update(chainName).digest();
    const chainNameBigInt = BigInt('0x' + chainNameHash.toString('hex'));

    const chainAddressCell = beginCell().storeStringTail(chainAddress).endCell();

    const message = beginCell()
        .storeUint(ITS_OPS.ADD_TRUSTED_CHAIN, 32)
        .storeUint(chainNameBigInt, ITS_DICT_KEY_LENGTH)
        .storeRef(chainAddressCell)
        .endCell();

    return message;
}

function buildRemoveTrustedChainMessage(chainName) {
    const chainNameHash = crypto.createHash('sha256').update(chainName).digest();
    const chainNameBigInt = BigInt('0x' + chainNameHash.toString('hex'));
    return beginCell().storeUint(ITS_OPS.REMOVE_TRUSTED_CHAIN, 32).storeUint(chainNameBigInt, ITS_DICT_KEY_LENGTH).endCell();
}

function buildRegisterTokenMetadataMessage(adminAddress, contentHex) {
    const admin = Address.parse(adminAddress);
    const content = Cell.fromHex(contentHex);

    const message = beginCell()
        .storeUint(ITS_OPS.REGISTER_TOKEN_METADATA, 32)
        .storeAddress(admin)
        .storeRef(content) // content cell (indirectly gives token address)
        .endCell();

    return message;
}

function buildDeployRemoteInterchainTokenMessage(salt, chainName, remoteMinter) {
    const chainNameCell = beginCell().storeStringTail(chainName).endCell();

    let minterCell;
    if (remoteMinter) {
        const minterAddress = Address.parse(remoteMinter);
        minterCell = beginCell().storeAddress(minterAddress).endCell();
    } else {
        minterCell = beginCell().endCell();
    }

    const message = beginCell()
        .storeUint(ITS_OPS.DEPLOY_REMOTE_INTERCHAIN_TOKEN, 32)
        .storeUint(BigInt(salt), 256)
        .storeRef(chainNameCell)
        .storeRef(minterCell)
        .endCell();

    return message;
}

function buildInterchainTokenTransferMessage(tokenId, chainName, destinationAddress, amount) {
    const chainNameCell = beginCell().storeStringTail(chainName).endCell();

    const destinationAddressCell = beginCell().storeStringTail(destinationAddress).endCell();

    const message = beginCell()
        .storeUint(ITS_OPS.INTERCHAIN_TOKEN_TRANSFER, 32)
        .storeUint(BigInt(tokenId), 256)
        .storeRef(chainNameCell)
        .storeRef(destinationAddressCell)
        .storeUint(BigInt(amount), 256)
        .endCell();

    return message;
}

function buildLinkTokenMessage(salt, chainName, destinationAddress, tokenManagerType, linkParams) {
    const chainNameCell = beginCell().storeStringTail(chainName).endCell();
    const destinationAddressCell = beginCell().storeStringTail(destinationAddress).endCell();

    // Handle linkParams - if given put it as cell, else empty cell
    const linkParamsCell =
        linkParams && linkParams !== '0x' && linkParams !== ''
            ? Cell.fromHex(linkParams.startsWith('0x') ? linkParams.slice(2) : linkParams)
            : beginCell().endCell();

    const message = beginCell()
        .storeUint(ITS_OPS.LINK_TOKEN, 32)
        .storeUint(BigInt('0x' + salt), 256)
        .storeRef(chainNameCell)
        .storeRef(destinationAddressCell)
        .storeUint(tokenManagerType, 8)
        .storeRef(linkParamsCell)
        .endCell();

    return message;
}

function buildRegisterCanonicalTokenMessage(adminAddress, contentHex) {
    const admin = Address.parse(adminAddress);
    const content = Cell.fromHex(contentHex);

    return beginCell().storeUint(ITS_OPS.REGISTER_CANONICAL_INTERCHAIN_TOKEN, 32).storeAddress(admin).storeRef(content).endCell();
}

function buildRegisterCustomTokenMessage(salt, tokenManagerType, operatorAddress, adminAddress, contentHex) {
    const saltBigInt = BigInt('0x' + salt);
    const operator = Address.parse(operatorAddress);
    const admin = Address.parse(adminAddress);
    const content = Cell.fromHex(contentHex);

    const message = beginCell()
        .storeUint(ITS_OPS.REGISTER_CUSTOM_TOKEN, 32)
        .storeUint(saltBigInt, 256)
        .storeUint(tokenManagerType, 8)
        .storeAddress(operator)
        .storeAddress(admin)
        .storeRef(content)
        .endCell();

    return message;
}

function buildDeployRemoteCanonicalInterchainTokenMessage(jettonMinterAddress, chainName) {
    const jettonMinter = Address.parse(jettonMinterAddress);
    const chainNameCell = beginCell().storeStringTail(chainName).endCell();

    const message = beginCell()
        .storeUint(ITS_OPS.DEPLOY_REMOTE_CANONICAL_INTERCHAIN_TOKEN, 32)
        .storeAddress(jettonMinter)
        .storeRef(chainNameCell)
        .endCell();

    return message;
}

function buildPauseMessage() {
    const message = beginCell().storeUint(ITS_OPS.PAUSE, 32).endCell();
    return message;
}

function buildUnpauseMessage() {
    const message = beginCell().storeUint(ITS_OPS.UNPAUSE, 32).endCell();
    return message;
}

// Helper function to send jettons with bundled operations
async function sendJettonsTo(receiver, deployer, deployerJettonWallet, jettonMinter, jettonToSend, forwardPayload) {
    const client = getTonClient();
    const { contract, key } = await loadWallet(client);

    return await deployerJettonWallet.sendTransfer(
        client.provider(deployerJettonWallet.address),
        contract.sender(key.secretKey),
        toNano('0.12'), // transaction fee
        jettonToSend, // amount of jettons to send
        receiver, // the destination address
        receiver, // responseAddress (can be your deployer address)
        beginCell().endCell(), // custom payload
        toNano('0.08'), // forward_ton_amount
        forwardPayload, // forwardPayload
    );
}

program
    .command('deploy-interchain-token')
    .description('Deploy a new interchain token')
    .argument('<salt>', 'Salt value for token deployment (256-bit number or hex string)')
    .argument('<name>', 'Token name')
    .argument('<symbol>', 'Token symbol')
    .argument('<decimals>', 'Token decimals (0-255)')
    .argument('<initial-supply>', 'Initial token supply')
    .argument('[minter]', 'Optional minter address (TON address format)')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.4')
    .action(async (salt, name, symbol, decimals, initialSupply, minter, options) => {
        try {
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

program
    .command('add-trusted-chain')
    .description('Add a trusted chain to the ITS')
    .argument('<chain-name>', 'Name of the chain to add (e.g., "ethereum", "polygon")')
    .argument('<chain-address>', 'ITS address on the remote chain')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.05')
    .action(async (chainName, chainAddress, options) => {
        try {
            console.log('Adding Trusted Chain with parameters:');
            console.log('  Chain Name:', chainName);
            console.log('  Chain Address:', chainAddress);
            console.log('  Gas:', options.gas, 'TON');

            const messageBody = buildAddTrustedChainMessage(chainName, chainAddress);

            const cost = toNano(options.gas);
            await executeITSOperation('Add Trusted Chain', messageBody, cost);
        } catch (error) {
            console.error('❌ Error adding trusted chain:', error.message);
            process.exit(1);
        }
    });

program
    .command('remove-trusted-chain')
    .description('Remove a trusted chain from the ITS')
    .argument('<chain-name>', 'Name of the chain to remove (e.g., "ethereum", "polygon")')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.05')
    .action(async (chainName, options) => {
        try {
            console.log('Removing Trusted Chain with parameters:');
            console.log('  Chain Name:', chainName);
            console.log('  Gas:', options.gas, 'TON');

            const messageBody = buildRemoveTrustedChainMessage(chainName);

            const cost = toNano(options.gas);
            await executeITSOperation('Remove Trusted Chain', messageBody, cost);
        } catch (error) {
            console.error('❌ Error removing trusted chain:', error.message);
            process.exit(1);
        }
    });

program
    .command('register-token-metadata')
    .description('Register token metadata for a token (TEP-64 standard)')
    .argument('<admin-address>', 'Admin address for the token (TON address format)')
    .argument('<content-hex>', 'TEP-64 metadata content as BOC hex string (without 0x prefix)')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.3')
    .action(async (adminAddress, contentHex, options) => {
        try {
            // Remove 0x prefix if present
            const cleanContentHex = contentHex.startsWith('0x') ? contentHex.slice(2) : contentHex;

            console.log('Registering Token Metadata with parameters:');
            console.log('  Admin Address:', adminAddress);
            console.log('  Content Hex (first 50 chars):', cleanContentHex.substring(0, 50) + '...');
            console.log('  Gas:', options.gas, 'TON');

            const messageBody = buildRegisterTokenMetadataMessage(adminAddress, cleanContentHex);

            const cost = toNano(options.gas);
            await executeITSOperation('Register Token Metadata', messageBody, cost);
        } catch (error) {
            console.error('❌ Error registering token metadata:', error.message);
            process.exit(1);
        }
    });

program
    .command('deploy-remote-interchain-token')
    .description('Deploy an interchain token on a remote chain')
    .argument('<salt>', 'Salt value for token deployment (256-bit number or hex string)')
    .argument('<chain-name>', 'Name of the remote chain (e.g., "ethereum", "polygon")')
    .argument('[remote-minter]', 'Optional minter address on the remote chain')
    .option('-g, --gas <amount>', 'Gas amount in TON for this transaction', '0.3')
    .action(async (salt, chainName, remoteMinter, options) => {
        try {
            const saltBigInt = salt.startsWith('0x') ? salt.slice(2) : salt;

            console.log('Deploying Remote Interchain Token with parameters:');
            console.log('  Salt:', saltBigInt);
            console.log('  Chain Name:', chainName);
            console.log('  Remote Minter:', remoteMinter || 'None');
            console.log('  Transaction Gas:', options.gas, 'TON');

            const messageBody = buildDeployRemoteInterchainTokenMessage(saltBigInt, chainName, remoteMinter);

            const cost = toNano(options.gas);
            await executeITSOperation('Deploy Remote Interchain Token', messageBody, cost);
        } catch (error) {
            console.error('❌ Error deploying remote interchain token:', error.message);
            process.exit(1);
        }
    });

program
    .command('interchain-token-transfer')
    .description('Transfer interchain tokens to another chain')
    .argument('<token-id>', 'Token ID (256-bit number or hex string)')
    .argument('<chain-name>', 'Destination chain name (e.g., "ethereum", "polygon")')
    .argument('<destination-address>', 'Recipient address on the destination chain')
    .argument('<amount>', 'Amount of tokens to transfer')
    .argument('<jetton-minter>', 'Jetton minter address for gas payment')
    .option('-g, --gas <amount>', 'Gas amount in TON for this transaction', '0.1')
    .action(async (tokenId, chainName, destinationAddress, amount, jettonMinter, options) => {
        try {
            const tokenIdBigInt = tokenId.startsWith('0x') ? BigInt('0x' + tokenId.slice(2)) : BigInt(tokenId);

            console.log('Transferring Interchain Token with bundled operation:');
            console.log('  Token ID:', tokenIdBigInt.toString());
            console.log('  Destination Chain:', chainName);
            console.log('  Destination Address:', destinationAddress);
            console.log('  Amount:', amount);
            console.log('  Jetton Minter:', jettonMinter);
            console.log('  Transaction Gas:', options.gas, 'TON');

            const client = getTonClient();
            const { contract, key } = await loadWallet(client);

            const itsAddress = Address.parse(process.env.TON_ITS_ADDRESS);
            const jettonMinterAddress = Address.parse(jettonMinter);
            const sender = contract.address;

            const minter = JettonMinter.createFromAddress(jettonMinterAddress);

            let jettonWalletAddress;
            try {
                jettonWalletAddress = await minter.getWalletAddress(client.provider(jettonMinterAddress), sender);
            } catch (error) {
                console.error(`❌ Failed to get jetton wallet address:`);
                console.error(`   Jetton minter: ${jettonMinterAddress.toString()}`);
                console.error(`   Original error: ${error.message}`);
                process.exit(1);
            }

            const spendAmount = amount;
            const userJettonWallet = JettonWallet.createFromAddress(jettonWalletAddress);

            // Create the interchain token transfer message (same as buildInterchainTokenTransferMessage)
            const interchainTransferMessage = beginCell()
                .storeUint(ITS_OPS.INTERCHAIN_TOKEN_TRANSFER, 32)
                .storeUint(tokenIdBigInt, 256)
                .storeRef(beginCell().storeStringTail(chainName).endCell())
                .storeRef(beginCell().storeStringTail(destinationAddress).endCell())
                .storeUint(spendAmount, 256)
                .endCell();

            // Create the bundled forward payload (payment + transfer operation)
            const forwardPayload = beginCell().storeAddress(jettonMinterAddress).storeRef(interchainTransferMessage).endCell();

            console.log(`Sending ${spendAmount.toString()} Jettons with bundled interchain transfer`);

            // Use the sendJettonsTo helper function similar to gas service
            const res = await sendJettonsTo(itsAddress, contract, userJettonWallet, minter, spendAmount, forwardPayload);

            console.log('Transaction result:', res);
            console.log('✅ Bundled interchain token transfer sent successfully!');

            // Wait for confirmation
            const seqno = await contract.getSeqno();
            await waitForTransaction(contract, seqno);
        } catch (error) {
            console.error('❌ Error in bundled interchain token transfer:', error);
            console.error('Error details:', error.message);
            process.exit(1);
        }
    });

program
    .command('link-token')
    .description('Link a token to a remote chain token')
    .argument('<salt>', 'Salt value for token linking (256-bit hex string)')
    .argument('<chain-name>', 'Name of the destination chain (e.g., "ethereum", "polygon")')
    .argument('<destination-address>', 'Token address on the destination chain')
    .argument(
        '[token-manager-type]',
        'Token manager type (0=INTERCHAIN_TOKEN, 1=MINT_BURN_FROM, 2=LOCK_UNLOCK, 3=LOCK_UNLOCK_FEE, 4=MINT_BURN)',
        '2',
    )
    .argument('[link-params]', 'Link parameters as hex string (optional)', '0x')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.3')
    .action(async (salt, chainName, destinationAddress, tokenManagerType, linkParams, options) => {
        try {
            const saltBigInt = salt.startsWith('0x') ? salt.slice(2) : salt;
            const tmType = parseInt(tokenManagerType, 10);

            console.log('Linking Token with parameters:');
            console.log('  Salt:', saltBigInt);
            console.log('  Chain Name:', chainName);
            console.log('  Destination Address:', destinationAddress);
            console.log('  Token Manager Type:', tmType);
            console.log('  Link Params:', linkParams);
            console.log('  Gas:', options.gas, 'TON');

            const messageBody = buildLinkTokenMessage(saltBigInt, chainName, destinationAddress, tmType, linkParams);

            const cost = toNano(options.gas);
            await executeITSOperation('Link Token', messageBody, cost);
        } catch (error) {
            console.error('❌ Error linking token:', error.message);
            process.exit(1);
        }
    });

program
    .command('register-canonical-token')
    .description('Register a canonical interchain token (TEP-64 metadata)')
    .argument('<admin-address>', 'Admin address for the token (TON address format)')
    .argument('<content-hex>', 'TEP-64 metadata content as BOC hex string (without 0x prefix)')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.3')
    .action(async (adminAddress, contentHex, options) => {
        try {
            // Remove 0x prefix if present
            const cleanContentHex = contentHex.startsWith('0x') ? contentHex.slice(2) : contentHex;

            console.log('Registering Canonical Token with parameters:');
            console.log('  Admin Address:', adminAddress);
            console.log('  Content Hex (first 50 chars):', cleanContentHex.substring(0, 50) + '...');
            console.log('  Gas:', options.gas, 'TON');

            const messageBody = buildRegisterCanonicalTokenMessage(adminAddress, cleanContentHex);

            const cost = toNano(options.gas);
            await executeITSOperation('Register Canonical Token', messageBody, cost);
        } catch (error) {
            console.error('❌ Error registering canonical token:', error.message);
            process.exit(1);
        }
    });

program
    .command('register-custom-token')
    .description('Register a custom interchain token with specific token manager type')
    .argument('<salt>', 'Salt value for token registration (256-bit number or hex string)')
    .argument(
        '<token-manager-type>',
        'Token manager type (0=INTERCHAIN_TOKEN, 1=MINT_BURN_FROM, 2=LOCK_UNLOCK, 3=LOCK_UNLOCK_FEE, 4=MINT_BURN)',
    )
    .argument('<operator-address>', 'Operator address for the token (TON address format)')
    .argument('<admin-address>', 'Admin address for the token (TON address format)')
    .argument('<content-hex>', 'TEP-64 metadata content as BOC hex string (without 0x prefix)')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.3')
    .action(async (salt, tokenManagerType, operatorAddress, adminAddress, contentHex, options) => {
        try {
            const saltBigInt = salt.startsWith('0x') ? salt.slice(2) : salt;
            const tmType = parseInt(tokenManagerType, 10);

            // Validate token manager type
            if (isNaN(tmType) || tmType < 0 || tmType > 4) {
                throw new Error('Token manager type must be a number between 0 and 4');
            }

            // Remove 0x prefix if present
            const cleanContentHex = contentHex.startsWith('0x') ? contentHex.slice(2) : contentHex;

            console.log('Registering Custom Token with parameters:');
            console.log('  Salt:', saltBigInt);
            console.log('  Token Manager Type:', tmType);
            console.log('  Operator Address:', operatorAddress);
            console.log('  Admin Address:', adminAddress);
            console.log('  Content Hex (first 50 chars):', cleanContentHex.substring(0, 50) + '...');
            console.log('  Gas:', options.gas, 'TON');

            const messageBody = buildRegisterCustomTokenMessage(saltBigInt, tmType, operatorAddress, adminAddress, cleanContentHex);

            const cost = toNano(options.gas);
            await executeITSOperation('Register Custom Token', messageBody, cost);
        } catch (error) {
            console.error('❌ Error registering custom token:', error.message);
            process.exit(1);
        }
    });

program
    .command('deploy-remote-canonical-token')
    .description('Deploy a canonical interchain token on a remote chain')
    .argument('<jetton-minter-address>', 'Jetton minter address for the canonical token (TON address format)')
    .argument('<chain-name>', 'Name of the remote chain (e.g., "ethereum", "polygon")')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.3')
    .action(async (jettonMinterAddress, chainName, options) => {
        try {
            console.log('Deploying Remote Canonical Token with parameters:');
            console.log('  Jetton Minter Address:', jettonMinterAddress);
            console.log('  Chain Name:', chainName);
            console.log('  Gas:', options.gas, 'TON');

            const messageBody = buildDeployRemoteCanonicalInterchainTokenMessage(jettonMinterAddress, chainName);

            const cost = toNano(options.gas);
            await executeITSOperation('Deploy Remote Canonical Token', messageBody, cost);
        } catch (error) {
            console.error('❌ Error deploying remote canonical token:', error.message);
            process.exit(1);
        }
    });

program
    .command('pause')
    .description('Pause the Interchain Token Service (operator only)')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.05')
    .action(async (options) => {
        try {
            console.log('Pausing Interchain Token Service...');
            console.log('  Gas:', options.gas, 'TON');
            console.log('⚠️  Note: Only the operator can pause the service');

            const messageBody = buildPauseMessage();

            const cost = toNano(options.gas);
            await executeITSOperation('Pause ITS', messageBody, cost);
        } catch (error) {
            console.error('❌ Error pausing ITS:', error.message);
            process.exit(1);
        }
    });

program
    .command('unpause')
    .description('Unpause the Interchain Token Service (operator only)')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.05')
    .action(async (options) => {
        try {
            console.log('Unpausing Interchain Token Service...');
            console.log('  Gas:', options.gas, 'TON');
            console.log('⚠️  Note: Only the operator can unpause the service');

            const messageBody = buildUnpauseMessage();

            const cost = toNano(options.gas);
            await executeITSOperation('Unpause ITS', messageBody, cost);
        } catch (error) {
            console.error('❌ Error unpausing ITS:', error.message);
            process.exit(1);
        }
    });

program.parse();
