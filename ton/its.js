#!/usr/bin/env node
const { Command } = require('commander');
const { toNano, Address, beginCell, Cell } = require('@ton/core');
const {
    getTonClient,
    loadWallet,
    waitForTransaction,
    sendTransactionWithCost,
    getJettonCodes,
    getJettonDataComplete,
    sendMultipleTransactionWithCost,
} = require('./common');
const { JettonWallet, JettonMinter, hexStringToCell } = require('axelar-cgp-ton');
const {
    buildDeployInterchainTokenMessage,
    buildAddTrustedChainMessage,
    buildRemoveTrustedChainMessage,
    buildRegisterTokenMetadataMessage,
    buildDeployRemoteInterchainTokenMessage,
    buildLinkTokenMessage,
    buildRegisterCanonicalTokenMessage,
    buildRegisterCustomTokenMessage,
    buildDeployRemoteCanonicalInterchainTokenMessage,
    buildApproveRemoteDeploymentMessage,
    buildRevokeRemoteDeploymentMessage,
    buildInterchainTokenTransferPayload,
    buildChangeOperatorMessage,
    buildPauseMessage,
    buildUnpauseMessage,
    InterchainTokenService,
    buildPayNativeGasForContractCallMessage,
    TokenManager,
    MessageType
} = require('axelar-cgp-ton');
const ethers = require('ethers');

const ITS_ADDRESS = process.env.TON_ITS_ADDRESS;
const TON_CHAIN_NAME = 'ton2';
const AXELAR_HUB_CHAIN_NAME = 'axelar';
const AXELAR_HUB_ADDRESS = 'axelar157hl7gpuknjmhtac2qnphuazv2yerfagva7lsu9vuj2pgn32z22qa26dk4'

if (!ITS_ADDRESS) {
    throw new Error('Please set TON_ITS_ADDRESS in your .env file');
}

const OP_REGISTER_CANONICAL_INTERCHAIN_TOKEN_PERMISSIONED = 0x00000116;
const OP_CHANGE_OWNER = 0x00000117;

function buildRegisterCanonicalTokenPermissionedMessage(name, symbol, decimals, jettonMinterAddress, jettonWalletCode) {
    return beginCell()
        .storeUint(OP_REGISTER_CANONICAL_INTERCHAIN_TOKEN_PERMISSIONED, 32)
        .storeRef(beginCell().storeStringTail(name).endCell())
        .storeRef(beginCell().storeStringTail(symbol).endCell())
        .storeUint(decimals, 8)
        .storeAddress(jettonMinterAddress)
        .storeRef(jettonWalletCode)
        .endCell();
}

function encodeInterchainTransferHubMessage(
    originalSourceChain,
    params,
) {
    const abiCoder = new ethers.utils.AbiCoder();

    // First encode the inner payload (interchain transfer message)
    const innerPayload = abiCoder.encode(
        ['uint256', 'bytes32', 'bytes', 'bytes', 'uint256', 'bytes'],
        [
            MessageType.INTERCHAIN_TRANSFER,
            params.tokenId,
            params.sourceAddress,
            params.destinationAddress,
            params.amount,
            params.data,
        ],
    );

    // Then wrap it in the hub message format
    const hubMessage = abiCoder.encode(
        ['uint256', 'string', 'bytes'],
        [MessageType.SEND_TO_HUB, originalSourceChain, innerPayload],
    );

    return hubMessage.slice(2); // remove the "0x" prefix
}

function encodeDeployInterchainTokenHubMessage(
    originalSourceChain,
    params,
) {
    const abiCoder = new ethers.utils.AbiCoder();

    // First encode the inner payload (deploy interchain token message)
    const innerPayload = abiCoder.encode(
        ["uint256", "bytes32", "string", "string", "uint8", "bytes"],
        [
            MessageType.DEPLOY_INTERCHAIN_TOKEN,
            params.tokenId,
            params.name,
            params.symbol,
            params.decimals,
            params.minter,
        ],
    );

    // Then wrap it in the hub message format
    const hubMessage = abiCoder.encode(
        ["uint256", "string", "bytes"],
        [MessageType.SEND_TO_HUB, originalSourceChain, innerPayload],
    );

    return hubMessage.slice(2); // remove the "0x" prefix
}

function encodeLinkTokenHubMessage(originalSourceChain, params) {
    const abiCoder = new ethers.utils.AbiCoder();

    // First encode the inner payload (link token message)
    const innerPayload = abiCoder.encode(
        ['uint256', 'bytes32', 'uint256', 'bytes', 'bytes', 'bytes'],
        [
            MessageType.LINK_TOKEN,
            params.tokenId,
            params.tokenManagerType,
            params.sourceAddress,
            params.destinationAddress,
            params.linkParams,
        ],
    );

    // Then wrap it in the hub message format
    const hubMessage = abiCoder.encode(
        ['uint256', 'string', 'bytes'],
        [MessageType.SEND_TO_HUB, originalSourceChain, innerPayload],
    );

    return hubMessage.slice(2); // remove the "0x" prefix
}

function encodeRegisterTokenMetadataAbi(
    message,
) {
    const abiCoder = new ethers.utils.AbiCoder();

    // Encode inner payload: uint256, bytes, uint256
    const encoded = abiCoder.encode(
        ["uint256", "bytes", "uint256"],
        [
            MessageType.REGISTER_TOKEN_METADATA, // uint256 - MessageType.REGISTER_TOKEN_METADATA
            message.tokenAddress, // bytes - token address
            message.decimals, // uint256 - decimals
        ],
    );

    return encoded;
}

function parseTokenManagerInfo(tokenManagerInfo) {
    // Begin parsing the cell
    const slice = tokenManagerInfo.beginParse();
    const tokenId = slice.loadUintBig(256);
    const tokenManagerType = slice.loadUint(8);
    const decimals = slice.loadUint(8);
    const name = slice.loadStringRefTail();
    const symbol = slice.loadStringRefTail();
    const jettonMinterAddressCell = slice.loadRef();
    const jettonMinterAddress = jettonMinterAddressCell.beginParse().loadAddress();
    const itsJettonWallet = slice.loadAddress();

    return {
        tokenId,
        tokenManagerType,
        decimals,
        name,
        symbol,
        jettonMinterAddress,
        itsJettonWallet,
    };
}

function prettyPrintTokenManagerInfo(tokenManagerInfo) {
    console.log('🪙 Token Manager Info');
    console.log(`Token ID              : ${tokenManagerInfo.tokenId}`);
    console.log(`Token Manager Type    : ${tokenManagerInfo.tokenManagerType}`);
    console.log(`Decimals              : ${tokenManagerInfo.decimals}`);
    console.log(`Name                  : ${tokenManagerInfo.name}`);
    console.log(`Symbol                : ${tokenManagerInfo.symbol}`);
    console.log(`Jetton Minter Address : ${tokenManagerInfo.jettonMinterAddress}`);
    console.log(`ITS Jetton Wallet     : ${tokenManagerInfo.itsJettonWallet}`);
    console.log('─'.repeat(30));
}

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
            const saltBigInt = salt.startsWith('0x') ? BigInt(salt) : BigInt(salt);

            const decimalsParsed = parseInt(decimals, 10);
            if (isNaN(decimalsParsed) || decimalsParsed < 0 || decimalsParsed > 255) {
                throw new Error('Decimals must be a number between 0 and 255');
            }

            const initialSupplyBigInt = BigInt(initialSupply);

            const client = getTonClient();
            const { contract, _ } = await loadWallet(client);

            // Create the contract instance directly - no need for openContract with real provider
            const itsAddress = Address.parse(ITS_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            // Use the client's provider when calling contract methods
            const tokenId = await interchainTokenService.getInterchainTokenId(client.provider(itsAddress), salt, contract.address);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const jettonMinterAddress = await interchainTokenService.getJettonMinterAddressForInterchainToken(
                client.provider(itsAddress),
                name,
                symbol,
                decimals,
            );

            console.log('🏗️ Deploying Interchain Token');
            console.log('─'.repeat(45));
            console.log(`  Salt:           ${saltBigInt}`);
            console.log(`  Name:           ${name}`);
            console.log(`  Symbol:         ${symbol}`);
            console.log(`  Decimals:       ${decimalsParsed}`);
            console.log(`  Initial Supply: ${initialSupplyBigInt}`);
            console.log(`  Minter:         ${minter || 'None'}`);
            console.log(`  Gas:            ${options.gas} TON`);
            console.log();
            console.log('🎯 Deployment Result:');
            console.log('─'.repeat(45));
            console.log(`  Token ID:       ${tokenId}`);
            console.log(`  Token Manager:  ${tokenManagerAddress.toString()}`);
            console.log(`  Jetton Minter:  ${jettonMinterAddress.toString()}`);


            const minterAddress = minter ? Address.parse(minter) : undefined;
            const messageBody = buildDeployInterchainTokenMessage(
                saltBigInt,
                name,
                symbol,
                BigInt(decimalsParsed),
                initialSupplyBigInt,
                minterAddress,
            );

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
            console.log('🔧 Adding Trusted Chain');
            console.log('─'.repeat(45));
            console.log(`  Chain Name:     ${chainName}`);
            console.log(`  Chain Address:  ${chainAddress}`);
            console.log(`  Gas:            ${options.gas} TON`);

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
            console.log('🔧 Removing Trusted Chain');
            console.log('─'.repeat(45));
            console.log(`  Chain Name:     ${chainName}`);
            console.log(`  Gas:            ${options.gas} TON`);

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
    .description('Register token metadata for a token (TEP-64 standard) - automatically extracts admin and content from jetton minter')
    .argument('<jetton-minter-address>', 'Jetton minter address to extract admin, content, and codes from')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.2')
    .action(async (jettonMinterAddress, options) => {
        try {
            const client = getTonClient();
            const { contract, key } = await loadWallet(client);
            const sender = contract.address;

            const itsAddress = Address.parse(ITS_ADDRESS);
            const gasServiceAddress = Address.parse(process.env.TON_GAS_SERVICE_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            console.log('🔍 Extracting jetton data...');

            // Get all jetton data from the minter
            const { adminAddress, content, jettonMinterCode, jettonWalletCode } = await getJettonDataComplete(jettonMinterAddress);

            // Convert content cell to hex for display
            const contentHex = content.toBoc().toString('hex');

            const registerTokenMetadataMeessage = buildRegisterTokenMetadataMessage(adminAddress, content, jettonMinterCode, jettonWalletCode);
            const { name, symbol, decimals } = await interchainTokenService.getJettonMetadata(client.provider(itsAddress), content);

            console.log('📋 Extracted Jetton Information:');
            console.log('─'.repeat(45));
            console.log(`  Jetton Minter:         ${jettonMinterAddress}`);
            console.log(`  Admin Address:         ${adminAddress.toString()}`);
            console.log(`  Content (hex):         ${contentHex.substring(0, 50)}...`);
            console.log(`  Gas:                   ${options.gas} TON`);
            console.log();
            console.log('📦 Token Metadata:');
            console.log('─'.repeat(45));
            console.log(`  Name:                  ${name}`);
            console.log(`  Symbol:                ${symbol}`);
            console.log(`  Decimals:              ${decimals}`);
            console.log();

            const jettonMinterAddr = Address.parse(jettonMinterAddress);

            let hubPayload = encodeRegisterTokenMetadataAbi({
              tokenAddress: '0x' + jettonMinterAddr.toRawString().slice(2),
              decimals,
            });

            const gasMessage = buildPayNativeGasForContractCallMessage(sender, AXELAR_HUB_CHAIN_NAME, AXELAR_HUB_ADDRESS, hubPayload.slice(2), sender);

            const { transfer, seqno } = await sendMultipleTransactionWithCost(
                contract,
                key,
                itsAddress,
                registerTokenMetadataMeessage,
                toNano('0.4'),
                gasServiceAddress,
                gasMessage,
                options.gas,
            );

            console.log('💸 Transaction sent successfully!');
            await waitForTransaction(contract, seqno);
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
    .option('-g, --gas <amount>', 'Gas amount in TON for this transaction', '0.4')
    .action(async (salt, chainName, remoteMinter, options) => {
        try {
            const client = getTonClient();
            const { contract, key } = await loadWallet(client);
            const sender = contract.address;

            const saltBigInt = salt.startsWith('0x') ? BigInt(salt) : BigInt(salt);

            const gasServiceAddress = Address.parse(process.env.TON_GAS_SERVICE_ADDRESS);
            const itsAddress = Address.parse(ITS_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            const tokenId = await interchainTokenService.getInterchainTokenId(client.provider(itsAddress), salt, contract.address);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const tokenManager = TokenManager.createFromAddress(tokenManagerAddress);
            const { data } = await tokenManager.getTokenManagerData(client.provider(tokenManagerAddress));
            const tokenManagerInfo = parseTokenManagerInfo(data);

            console.log('📦 Deploying Remote Interchain Token');
            console.log('─'.repeat(45));
            console.log(`Salt                  : ${saltBigInt}`);
            console.log(`Chain Name            : ${chainName}`);
            console.log(`Remote Minter         : ${remoteMinter || 'None'}`);
            console.log(`Transaction Gas       : ${options.gas} TON`);
            console.log(`Token ID              : ${tokenManagerInfo.tokenId}`);
            console.log(`Token Manager Type    : ${tokenManagerInfo.tokenManagerType}`);
            console.log(`Decimals              : ${tokenManagerInfo.decimals}`);
            console.log(`Name                  : ${tokenManagerInfo.name}`);
            console.log(`Symbol                : ${tokenManagerInfo.symbol}`);
            console.log(`Jetton Minter Address : ${tokenManagerInfo.jettonMinterAddress}`);
            console.log(`ITS Jetton Wallet     : ${tokenManagerInfo.itsJettonWallet}`);
            console.log('─'.repeat(45));

            const deployRemoteInterchainTokenMessage = buildDeployRemoteInterchainTokenMessage(saltBigInt, chainName, remoteMinter);

            const hubPayload = encodeDeployInterchainTokenHubMessage(chainName, {
                tokenId: '0x' + tokenId.toString(16).padStart(64, '0'),
                name: tokenManagerInfo.name,
                symbol: tokenManagerInfo.symbol,
                decimals: tokenManagerInfo.decimals,
                minter: remoteMinter || '0x',
            });

            const gasMessage = buildPayNativeGasForContractCallMessage(sender, AXELAR_HUB_CHAIN_NAME, AXELAR_HUB_ADDRESS, hubPayload, sender);

            const { transfer, seqno } = await sendMultipleTransactionWithCost(
                contract,
                key,
                itsAddress,
                deployRemoteInterchainTokenMessage,
                toNano('0.4'),
                gasServiceAddress,
                gasMessage,
                options.gas,
            );

            console.log('💸 Transaction sent successfully!');
            await waitForTransaction(contract, seqno);
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
            // Initialize clients and addresses
            const client = getTonClient();
            const { contract, key } = await loadWallet(client);
            const sender = contract.address;

            const gasServiceAddress = Address.parse(process.env.TON_GAS_SERVICE_ADDRESS);
            const itsAddress = Address.parse(process.env.TON_ITS_ADDRESS);
            const jettonMinterAddress = Address.parse(jettonMinter);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            // Parse and validate inputs
            const tokenIdBigInt = tokenId.startsWith('0x') ? BigInt(tokenId) : BigInt(tokenId);
            const spendAmount = BigInt(amount);
            const tokenIdBytes32 = '0x' + tokenIdBigInt.toString(16).padStart(64, '0');
            const destAddrBuffer = Buffer.from(destinationAddress.slice(2), 'hex');

            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenIdBigInt);
            const tokenManager = TokenManager.createFromAddress(tokenManagerAddress);
            const { data } = await tokenManager.getTokenManagerData(client.provider(tokenManagerAddress));
            const tokenManagerInfo = parseTokenManagerInfo(data);

            console.log('💸 Transferring Interchain Token');
            console.log('─'.repeat(45));
            console.log(`Token ID              : ${tokenIdBigInt.toString()}`);
            console.log(`Chain                 : ${chainName} → ${destinationAddress}`);
            console.log(`Amount                : ${spendAmount.toString()}`);
            console.log(`Token Manager Type    : ${tokenManagerInfo.tokenManagerType}`);
            console.log(`Decimals              : ${tokenManagerInfo.decimals}`);
            console.log(`Name                  : ${tokenManagerInfo.name}`);
            console.log(`Symbol                : ${tokenManagerInfo.symbol}`);
            console.log(`Jetton Minter Address : ${tokenManagerInfo.jettonMinterAddress}`);
            console.log(`ITS Jetton Wallet     : ${tokenManagerInfo.itsJettonWallet}`);
            console.log('─'.repeat(45));

            // Get jetton wallet address
            const minter = JettonMinter.createFromAddress(jettonMinterAddress);
            const jettonWalletAddress = await getJettonWalletAddress(minter, client, jettonMinterAddress, sender);
            const userJettonWallet = JettonWallet.createFromAddress(jettonWalletAddress);

            // Build payloads
            const hubPayload = encodeInterchainTransferHubMessage(chainName, {
                tokenId: tokenIdBytes32,
                sourceAddress: sender.hash,
                destinationAddress: destAddrBuffer,
                amount: amount,
                data: Buffer.from('', 'hex'), // data is always empty
            });

            const gasMessage = buildPayNativeGasForContractCallMessage(sender, AXELAR_HUB_CHAIN_NAME, AXELAR_HUB_ADDRESS, hubPayload, sender);

            const transferPayload = buildInterchainTokenTransferPayload(jettonMinterAddress, tokenIdBigInt, chainName, destAddrBuffer);

            // Create transaction messages
            const jettonTransferMessage = JettonWallet.transferMessage(
                spendAmount,
                itsAddress,
                sender,
                null,
                toNano('0.065'),
                transferPayload,
            );

            const { transfer, seqno } = await sendMultipleTransactionWithCost(
                contract,
                key,
                userJettonWallet.address,
                jettonTransferMessage,
                toNano('0.1'),
                gasServiceAddress,
                gasMessage,
                options.gas,
            );

            console.log('💸 Transaction sent successfully!');
            await waitForTransaction(contract, seqno);
            console.log('🎉 Transaction confirmed!');
        } catch (error) {
            console.error('❌ Transfer failed:', error.message);
            process.exit(1);
        }
    });

// Helper function for cleaner error handling
async function getJettonWalletAddress(minter, client, jettonMinterAddress, sender) {
    try {
        return await minter.getWalletAddress(client.provider(jettonMinterAddress), sender);
    } catch (error) {
        console.error('❌ Failed to get jetton wallet address:');
        console.error(`   Jetton minter: ${jettonMinterAddress.toString()}`);
        throw new Error(`Jetton wallet lookup failed: ${error.message}`);
    }
}

program
    .command('link-token')
    .description('Link a token to a remote chain token')
    .argument('<salt>', 'Salt value for token linking (256-bit hex string)')
    .argument('<chain-name>', 'Name of the destination chain (e.g., "ethereum", "polygon")')
    .argument('<destination-address>', 'Token address on the destination chain')
    .argument(
        '<token-manager-type>',
        'Token manager type (0=INTERCHAIN_TOKEN, 1=MINT_BURN_FROM, 2=LOCK_UNLOCK, 3=LOCK_UNLOCK_FEE, 4=MINT_BURN)',
    )
    .argument('<link-params>', 'Link parameters as hex string (use "0x" for empty params)')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.2')
    .action(async (salt, chainName, destinationAddress, tokenManagerType, linkParams, options) => {
        try {
            const saltBigInt = salt.startsWith('0x') ? BigInt(salt) : BigInt(salt);
            const tmType = parseInt(tokenManagerType, 10);

            const client = getTonClient();
            const { contract, key } = await loadWallet(client);
            const sender = contract.address;

            // Create the contract instance directly - no need for openContract with real provider
            const itsAddress = Address.parse(ITS_ADDRESS);
            const gasServiceAddress = Address.parse(process.env.TON_GAS_SERVICE_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            const tokenId = await interchainTokenService.getLinkedTokenId(client.provider(itsAddress), contract.address, salt);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const tokenManager = TokenManager.createFromAddress(tokenManagerAddress);
            const { data } = await tokenManager.getTokenManagerData(client.provider(tokenManagerAddress));
            const tokenManagerInfo = parseTokenManagerInfo(data);

            console.log('🔗 Linking Token');
            console.log('─'.repeat(45));
            console.log(`Salt                  : ${saltBigInt}`);
            console.log(`Chain Name            : ${chainName}`);
            console.log(`Destination Address   : ${destinationAddress}`);
            console.log(`Token Manager Type    : ${tmType}`);
            console.log(`Link Params           : ${linkParams}`);
            console.log(`Gas                   : ${options.gas} TON`);
            console.log(`Token ID              : ${tokenManagerInfo.tokenId}`);
            console.log(`Decimals              : ${tokenManagerInfo.decimals}`);
            console.log(`Name                  : ${tokenManagerInfo.name}`);
            console.log(`Symbol                : ${tokenManagerInfo.symbol}`);
            console.log(`Jetton Minter Address : ${tokenManagerInfo.jettonMinterAddress}`);
            console.log(`ITS Jetton Wallet     : ${tokenManagerInfo.itsJettonWallet}`);
            console.log('─'.repeat(45));

            const linkParamsCell =
                linkParams === '0x'
                    ? beginCell().endCell()
                    : hexStringToCell(linkParams.startsWith('0x') ? linkParams.slice(2) : linkParams);

            const hubPayload = encodeLinkTokenHubMessage(chainName, {
                tokenId: '0x' + tokenManagerInfo.tokenId.toString(16),
                tokenManagerType: tmType,
                destinationAddress: destinationAddress,
                sourceAddress: '0x' + tokenManagerInfo.jettonMinterAddress.toRawString().slice(2),
                linkParams,
            });

            const gasMessage = buildPayNativeGasForContractCallMessage(sender, AXELAR_HUB_CHAIN_NAME, AXELAR_HUB_ADDRESS, hubPayload, sender);

            const linkTokenMessage = buildLinkTokenMessage(
                saltBigInt,
                chainName,
                Buffer.from(destinationAddress.slice(2), 'hex'),
                tmType,
                linkParamsCell,
            );

            // Send bundled transaction
            const { transfer, seqno } = await sendMultipleTransactionWithCost(
                contract,
                key,
                itsAddress,
                linkTokenMessage,
                toNano('0.4'),
                gasServiceAddress,
                gasMessage,
                options.gas,
            );

            console.log('💸 Transaction sent successfully!');
            await waitForTransaction(contract, seqno);
        } catch (error) {
            console.error('❌ Error linking token:', error.message);
            process.exit(1);
        }
    });

program
    .command('register-canonical-token')
    .description('Register a canonical interchain token (TEP-64 metadata) with jetton codes')
    .argument('<jetton-minter-address>', 'Existing jetton minter address to extract minter and wallet codes from')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.4')
    .action(async (jettonMinterAddress, options) => {
        try {

            const client = getTonClient();
            const itsAddress = Address.parse(ITS_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);


            const { adminAddress, content, jettonMinterCode, jettonWalletCode } = await getJettonDataComplete(jettonMinterAddress);
            const messageBody = buildRegisterCanonicalTokenMessage(adminAddress, content, jettonMinterCode, jettonWalletCode);

            const canonicalJettonMinterAddress = Address.parse(jettonMinterAddress);

            const { tokenId } = await interchainTokenService.getCanonicalTokenId(client.provider(itsAddress), canonicalJettonMinterAddress);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const { name, symbol, decimals } = await interchainTokenService.getJettonMetadata(client.provider(itsAddress), content);

            console.log('📝 Registering Canonical Token');
            console.log('─'.repeat(45));
            console.log(`  Admin Address:     ${adminAddress}`);
            console.log(`  Content Hex:       ${content.toBoc().toString('hex').substring(0, 50)}...`);
            console.log(`  Jetton Minter:     ${jettonMinterAddress}`);
            console.log(`  Gas:               ${options.gas} TON`);
            console.log();
            console.log('📦 Token Metadata:');
            console.log('─'.repeat(45));
            console.log(`  Name:              ${name}`);
            console.log(`  Symbol:            ${symbol}`);
            console.log(`  Decimals:          ${decimals}`);
            console.log();
            console.log('🎯 Registration Result:');
            console.log('─'.repeat(45));
            console.log(`  Token ID:          ${tokenId}`);
            console.log(`  Token Manager:     ${tokenManagerAddress}`);
            console.log(`  Canonical Minter:  ${canonicalJettonMinterAddress}`);


            const cost = toNano(options.gas);
            await executeITSOperation('Register Canonical Token', messageBody, cost);
        } catch (error) {
            console.error('❌ Error registering canonical token:', error.message);
            process.exit(1);
        }
    });

program
    .command('register-canonical-token-permissioned')
    .description('Register a canonical interchain token with permissioned access using metadata parameters')
    .argument('<name>', 'Token name (e.g., "My Token")')
    .argument('<symbol>', 'Token symbol (e.g., "MTK")')
    .argument('<decimals>', 'Token decimals (e.g., 9, 18)')
    .argument('<jetton-minter-address>', 'Existing jetton minter address to extract wallet code from')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.4')
    .action(async (name, symbol, decimals, jettonMinterAddress, options) => {
        try {
            // Validate decimals
            const decimalNum = parseInt(decimals, 10);
            if (isNaN(decimalNum) || decimalNum < 0 || decimalNum > 255) {
                throw new Error('Decimals must be a number between 0 and 255');
            }

            const client = getTonClient();
            const itsAddress = Address.parse(ITS_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);
            const jettonMinterAddr = Address.parse(jettonMinterAddress);

            // Use the same pattern as other registration commands - get jetton codes from the minter
            const { _, jettonWalletCode } = await getJettonCodes(jettonMinterAddress);

            const messageBody = buildRegisterCanonicalTokenPermissionedMessage(
                name,
                symbol,
                decimalNum,
                jettonMinterAddr,
                jettonWalletCode,
            );

            const { tokenId } = await interchainTokenService.getCanonicalTokenId(client.provider(itsAddress), jettonMinterAddr);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);

            console.log('📝 Registering Canonical Token (Permissioned)');
            console.log('─'.repeat(45));
            console.log(`  Name:                  ${name}`);
            console.log(`  Symbol:                ${symbol}`);
            console.log(`  Decimals:              ${decimalNum}`);
            console.log(`  Jetton Minter:         ${jettonMinterAddress}`);
            console.log(`  Gas:                   ${options.gas} TON`);
            console.log();
            console.log('📦 Token Metadata:');
            console.log('─'.repeat(45));
            console.log(`  Name:                  ${name}`);
            console.log(`  Symbol:                ${symbol}`);
            console.log(`  Decimals:              ${decimalNum}`);
            console.log();
            console.log('🎯 Registration Result:');
            console.log('─'.repeat(45));
            console.log(`  Token ID:              ${tokenId}`);
            console.log(`  Token Manager:         ${tokenManagerAddress}`);
            console.log(`  Canonical Minter:      ${jettonMinterAddr}`);


            const cost = toNano(options.gas);
            await executeITSOperation('Register Canonical Token (Permissioned)', messageBody, cost);
        } catch (error) {
            console.error('❌ Error registering canonical token (permissioned):', error.message);
            process.exit(1);
        }
    });

program
    .command('register-custom-token')
    .description('Register a custom interchain token with specific token manager type and jetton codes')
    .argument('<salt>', 'Salt value for token registration (256-bit number or hex string)')
    .argument(
        '<token-manager-type>',
        'Token manager type (0=INTERCHAIN_TOKEN, 1=MINT_BURN_FROM, 2=LOCK_UNLOCK, 3=LOCK_UNLOCK_FEE, 4=MINT_BURN)',
    )
    .argument('<operator-address>', 'Operator address for the token (TON address format)')
    .argument('<admin-address>', 'Admin address for the token (TON address format)')
    .argument('<content-hex>', 'TEP-64 metadata content as BOC hex string (without 0x prefix)')
    .argument('<jetton-minter-address>', 'Existing jetton minter address to extract minter and wallet codes from')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.4')
    .action(async (salt, tokenManagerType, operatorAddress, adminAddress, contentHex, jettonMinterAddress, options) => {
        try {
            const saltBigInt = salt.startsWith('0x') ? BigInt(salt) : BigInt(salt);
            const tmType = parseInt(tokenManagerType, 10);

            // Validate token manager type
            if (isNaN(tmType) || tmType < 0 || tmType > 4) {
                throw new Error('Token manager type must be a number between 0 and 4');
            }

            // Remove 0x prefix if present
            const cleanContentHex = contentHex.startsWith('0x') ? contentHex.slice(2) : contentHex;

            const client = getTonClient();
            const { contract, key } = await loadWallet(client);
            const sender = contract.address;
            const itsAddress = Address.parse(ITS_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            const operatorAddr = Address.parse(operatorAddress);
            const adminAddr = Address.parse(adminAddress);
            const contentCell = Cell.fromHex(cleanContentHex);

            const { jettonMinterCode, jettonWalletCode } = await getJettonCodes(jettonMinterAddress);
            const messageBody = buildRegisterCustomTokenMessage(
                saltBigInt,
                tmType,
                operatorAddr,
                adminAddr,
                contentCell,
                jettonMinterCode,
                jettonWalletCode,
            );

            const tokenId = await interchainTokenService.getLinkedTokenId(client.provider(itsAddress), sender, saltBigInt);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const { name, symbol, decimals } = await interchainTokenService.getJettonMetadata(client.provider(itsAddress), contentCell);

            console.log('User Parameters:');
            console.log('─'.repeat(40));
            console.log(`  Salt:                  ${saltBigInt}`);
            console.log(`  Token Manager Type:    ${tmType}`);
            console.log(`  Operator Address:      ${operatorAddress}`);
            console.log(`  Admin Address:         ${adminAddress}`);
            console.log(`  Content Hex:           ${cleanContentHex.substring(0, 50)}...`);
            console.log(`  Jetton Minter:         ${jettonMinterAddress}`);
            console.log(`  Gas:                   ${options.gas} TON`);
            console.log();
            console.log('Token Metadata:');
            console.log('─'.repeat(40));
            console.log(`  Name:                  ${name}`);
            console.log(`  Symbol:                ${symbol}`);
            console.log(`  Decimals:              ${decimals}`);
            console.log();
            console.log('Deployment Result:');
            console.log('─'.repeat(40));
            console.log(`  Token ID:              ${tokenId}`);
            console.log(`  Token Manager:         ${tokenManagerAddress}`);
            console.log(`  Custom Minter:         ${jettonMinterAddress}`);

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
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.4')
    .action(async (jettonMinterAddress, chainName, options) => {
        try {
            console.log('Deploying Remote Canonical Token with parameters:');
            console.log('  Jetton Minter Address:', jettonMinterAddress);
            console.log('  Chain Name:', chainName);
            console.log('  Gas:', options.gas, 'TON');

            const jettonMinterAddr = Address.parse(jettonMinterAddress);
            const messageBody = buildDeployRemoteCanonicalInterchainTokenMessage(jettonMinterAddr, chainName);

            const cost = toNano(options.gas);
            await executeITSOperation('Deploy Remote Canonical Token', messageBody, cost);
        } catch (error) {
            console.error('❌ Error deploying remote canonical token:', error.message);
            process.exit(1);
        }
    });

program
    .command('approve-remote-deployment')
    .description('Approve remote deployment of an interchain token (must be called by the minter of the local token)')
    .argument('<salt>', 'Salt value used for token deployment (256-bit number or hex string)')
    .argument('<deployer-address>', 'Address of the deployer (TON address format)')
    .argument('<destination-chain>', 'Name of the destination chain (e.g., "ethereum", "polygon")')
    .argument('<minter-to-approve>', 'Address of the minter to be approved on the destination chain (TON address format)')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.4')
    .action(async (salt, deployerAddress, destinationChain, minterToBeApproved, options) => {
        try {
            const saltBigInt = salt.startsWith('0x') ? BigInt(salt) : BigInt(salt);

            console.log('Approving Remote Deployment with parameters:');
            console.log('  Salt:', saltBigInt.toString());
            console.log('  Deployer Address:', deployerAddress);
            console.log('  Destination Chain:', destinationChain);
            console.log('  Minter to Approve:', minterToBeApproved);
            console.log('  Gas:', options.gas, 'TON');
            console.log('⚠️  Note: This transaction must be sent from the minter of the local token');

            const deployerAddr = Address.parse(deployerAddress);
            const messageBody = buildApproveRemoteDeploymentMessage(saltBigInt, deployerAddr, minterToBeApproved, destinationChain);

            const cost = toNano(options.gas);
            await executeITSOperation('Approve Remote Deployment', messageBody, cost);
        } catch (error) {
            console.error('❌ Error approving remote deployment:', error.message);
            process.exit(1);
        }
    });

program
    .command('revoke-remote-deployment')
    .description('Revoke remote deployment of an interchain token')
    .argument('<salt>', 'Salt value used for token deployment (256-bit number or hex string)')
    .argument('<deployer-address>', 'Address of the deployer (TON address format)')
    .argument('<destination-chain>', 'Name of the destination chain (e.g., "ethereum", "polygon")')
    .argument('<minter-to-revoke>', 'Address of the minter to be revoked on the destination chain')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.4')
    .action(async (salt, deployerAddress, destinationChain, minterToRevoke, options) => {
        try {
            const saltBigInt = salt.startsWith('0x') ? BigInt(salt) : BigInt(salt);

            console.log('Revoking Remote Deployment with parameters:');
            console.log('  Salt:', saltBigInt.toString());
            console.log('  Deployer Address:', deployerAddress);
            console.log('  Destination Chain:', destinationChain);
            console.log('  Minter to Revoke:', minterToRevoke);
            console.log('  Gas:', options.gas, 'TON');

            const deployerAddr = Address.parse(deployerAddress);
            const messageBody = buildRevokeRemoteDeploymentMessage(saltBigInt, deployerAddr, minterToRevoke, destinationChain);

            const cost = toNano(options.gas);
            await executeITSOperation('Revoke Remote Deployment', messageBody, cost);
        } catch (error) {
            console.error('❌ Error revoking remote deployment:', error.message);
            process.exit(1);
        }
    });

program
    .command('change-operator')
    .description('Change the operator of the Interchain Token Service (current operator only)')
    .argument('<new-operator>', 'Address of the new operator (TON address format)')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.05')
    .action(async (newOperator, options) => {
        try {
            console.log('Changing ITS Operator with parameters:');
            console.log('  New Operator:', newOperator);
            console.log('  Gas:', options.gas, 'TON');
            console.log('⚠️  Note: Only the current operator can change the operator');

            const newOperatorAddr = Address.parse(newOperator);
            const messageBody = buildChangeOperatorMessage(newOperatorAddr);

            const cost = toNano(options.gas);
            await executeITSOperation('Change Operator', messageBody, cost);
        } catch (error) {
            console.error('❌ Error changing operator:', error.message);
            process.exit(1);
        }
    });

program
    .command('change-owner')
    .description('Change the owner of the Interchain Token Service (current owner only)')
    .argument('<new-owner>', 'Address of the new owner (TON address format)')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.05')
    .action(async (newOwner, options) => {
        try {
            console.log('Changing ITS Operator with parameters:');
            console.log('  New Operator:', newOwner);
            console.log('  Gas:', options.gas, 'TON');
            console.log('⚠️  Note: Only the current operator can change the operator');

            const newOwnerAddr = Address.parse(newOwner);
            const messageBody = beginCell().storeUint(OP_CHANGE_OWNER, 32).storeAddress(newOwnerAddr).endCell();

            const cost = toNano(options.gas);
            await executeITSOperation('Change Owner', messageBody, cost);
        } catch (error) {
            console.error('❌ Error changing owner:', error.message);
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

program
    .command('get-full-state')
    .description('Get complete ITS contract state')
    .argument('[its-address]', 'ITS contract address (defaults to TON_ITS_ADDRESS env var)')
    .action(async (itsAddress) => {
        try {
            const addressToUse = itsAddress || ITS_ADDRESS;
            if (!addressToUse) {
                console.error('❌ Please provide ITS address or set TON_ITS_ADDRESS env var');
                process.exit(1);
            }

            const client = getTonClient();
            const itsAddr = Address.parse(addressToUse);
            const its = InterchainTokenService.createFromAddress(itsAddr);
            const provider = client.provider(itsAddr);

            console.log('🔍 Retrieving ITS state...\n');

            const [itsData, hubConfig, chainNameHash, saltPrefixes, codeHashes] = await Promise.all([
                its.getInterchainTokenServiceData(provider),
                its.getHubConfig(provider),
                its.getChainNameHash(provider),
                its.getSaltPrefixes(provider),
                its.getContractCodeHashes(provider),
            ]);

            // Extract hub info
            let hubAddress, hubChainName;
            try {
                hubAddress = hubConfig.hubAddress.beginParse().loadStringTail();
                hubChainName = hubConfig.hubChainName.beginParse().loadStringTail();
            } catch {
                hubAddress = formatCellOutput(hubConfig.hubAddress);
                hubChainName = formatCellOutput(hubConfig.hubChainName);
            }

            console.log('ContractState {');
            console.log(`    id: ${itsData.id},`);
            console.log(`    gateway_address: ${itsData.axelarGateway},`);
            console.log(`    state: 0,`);
            console.log(`    its_operator: ${itsData.operator},`);
            console.log(`    its_owner: ${itsData.owner},`);
            console.log(`    chain_name_hash: "${chainNameHash.toBoc().toString('hex').toUpperCase()}",`);
            console.log(`    prefix_interchain_token_salt: "${saltPrefixes.interchainTokenPrefix.toBoc().toString('hex').toUpperCase()}",`);
            console.log(`    prefix_canonical_token_salt: "${saltPrefixes.canonicalTokenPrefix.toBoc().toString('hex').toUpperCase()}",`);
            console.log(`    prefix_custom_token_salt: "${saltPrefixes.customTokenPrefix.toBoc().toString('hex').toUpperCase()}",`);
            console.log(`    its_hub_address: "${hubAddress}",`);
            console.log(`    its_hub_chain_name: "${hubChainName}",`);
            console.log(`    jetton_wallet_code_hash: "${codeHashes.jettonWalletCodeHash.toString(16)}",`);
            console.log(`    jetton_minter_code_hash: "${codeHashes.jettonMinterCodeHash.toString(16)}",`);
            console.log(`    token_manager_code_hash: "${codeHashes.tokenManagerCodeHash.toString(16)}",`);
            console.log(`    minter_proxy_code_hash: "${codeHashes.minterProxyCodeHash.toString(16)}",`);
            console.log(`    minter_approval_code_hash: "${codeHashes.minterApprovalCodeHash.toString(16)}",`);

            // Check trusted chains
            const chains = ['ethereum', 'polygon', 'avalanche', 'arbitrum', 'base', 'optimism', 'avalanche-fuji'];
            const trustedChains = {};
            for (const chain of chains) {
                try {
                    const result = await its.getTrustedChainAddress(provider, chain);
                    if (result.found === -1 && result.chainAddress) {
                        trustedChains[chain] = result.chainAddress;
                    }
                } catch {}
            }

            console.log(`    trusted_chains: ${JSON.stringify(trustedChains, null, 8).replace(/\n/g, '\n    ')},`);
            console.log('}');
        } catch (error) {
            console.error('❌ Error getting full state:', error.message);
            process.exit(1);
        }
    });

program.parse();
