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
const { JettonWallet, JettonMinter, hexStringToCell } = require('@commonprefix/axelar-cgp-ton');
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
    OP_SET_FLOW_LIMIT,
} = require('@commonprefix/axelar-cgp-ton');

const {
    encodeInterchainTransferHubMessage,
    encodeDeployInterchainTokenHubMessage,
    encodeLinkTokenHubMessage,
    encodeRegisterTokenMetadataAbi,
} = require('./abi');

const ITS_ADDRESS = process.env.TON_ITS_ADDRESS;
const AXELAR_HUB_CHAIN_NAME = process.env.AXELAR_HUB_CHAIN_NAME || 'axelar';
const AXELAR_HUB_ADDRESS = process.env.AXELAR_HUB_ADDRESS || 'axelar157hl7gpuknjmhtac2qnphuazv2yerfagva7lsu9vuj2pgn32z22qa26dk4';

if (!ITS_ADDRESS) {
    throw new Error('Please set TON_ITS_ADDRESS in your .env file');
}

const USER_OPERATION_COST = toNano('0.4');
const SIMPLE_OPERATION_COST = toNano('0.02');
const JETTON_TRANSFER_COST = toNano('0.1');
const JETTON_FORWARD_COST = toNano('0.065');

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

function parseTokenManagerInfo(tokenManagerInfo) {
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

const program = new Command();
program.name('its').description('Axelar TON Interchain Token Service CLI').version('1.0.0');

function printSectionHeader(title, icon = '🔧') {
    console.log(`${icon} ${title}`);
    console.log('─'.repeat(45));
}

function printSectionSeparator() {
    console.log('─'.repeat(45));
}

function handleCommandError(operationName, error) {
    console.error(`❌ Error ${operationName}:`, error.message);
    process.exit(1);
}

// Helper function for common ITS environment setup
async function setupITSEnvironment() {
    const client = getTonClient();
    const { contract, key } = await loadWallet(client);
    const sender = contract.address;
    const itsAddress = Address.parse(ITS_ADDRESS);
    const gasServiceAddress = Address.parse(process.env.TON_GAS_SERVICE_ADDRESS);

    return { client, contract, key, sender, itsAddress, gasServiceAddress };
}

// Helper function for creating gas service messages
function createGasServiceMessage(sender, hubPayload) {
    return buildPayNativeGasForContractCallMessage(sender, AXELAR_HUB_CHAIN_NAME, AXELAR_HUB_ADDRESS, hubPayload, sender);
}

function formatTonAmount(amount) {
    if (typeof amount === 'string') {
        return `${amount} TON`;
    }
    if (typeof amount === 'bigint') {
        const tonValue = Number(amount) / 1000000000;
        return `${tonValue} TON`;
    }
    return `${amount} TON`;
}

async function executeWithGasService(contract, key, itsAddress, itsMessage, gasServiceAddress, gasMessage, gasServiceGas, dryRun = false) {
    const ITS_COST = '0.4';

    if (dryRun) {
        console.log('🔍 DRY RUN: Would send dual transaction');
        console.log(`  ITS Transaction:`);
        console.log(`    To: ${itsAddress.toString()}`);
        console.log(`    Cost: ${formatTonAmount(ITS_COST)}`);
        console.log(`  Gas Service Transaction:`);
        console.log(`    To: ${gasServiceAddress.toString()}`);
        console.log(`    Cost: ${formatTonAmount(gasServiceGas)}`);
        console.log('✅ Dry run completed - no transactions sent');
        return;
    }

    const { transfer, seqno } = await sendMultipleTransactionWithCost(
        contract,
        key,
        itsAddress,
        itsMessage,
        toNano(ITS_COST),
        gasServiceAddress,
        gasMessage,
        gasServiceGas,
    );

    console.log('💸 Transaction sent successfully!');
    await waitForTransaction(contract, seqno);
}

// Helper function for displaying token manager info
function displayTokenManagerInfo(tokenManagerInfo) {
    console.log(`Token Manager Type    : ${tokenManagerInfo.tokenManagerType}`);
    console.log(`Decimals              : ${tokenManagerInfo.decimals}`);
    console.log(`Name                  : ${tokenManagerInfo.name}`);
    console.log(`Symbol                : ${tokenManagerInfo.symbol}`);
    console.log(`Jetton Minter Address : ${tokenManagerInfo.jettonMinterAddress}`);
    console.log(`ITS Jetton Wallet     : ${tokenManagerInfo.itsJettonWallet}`);
}

async function executeITSOperation(operationName, messageBody, cost, dryRun = false) {
    try {
        const client = getTonClient();
        const { contract, key } = await loadWallet(client);
        const itsAddress = Address.parse(ITS_ADDRESS);

        if (dryRun) {
            console.log(`🔍 DRY RUN: Would send ${operationName} transaction`);
            console.log(`  To: ${itsAddress.toString()}`);
            console.log(`  Cost: ${formatTonAmount(cost)}`);
            console.log(`  Message body size: ${messageBody.toBoc().length} bytes`);
            console.log('✅ Dry run completed - no transaction sent');
            return;
        }

        const { transfer, seqno } = await sendTransactionWithCost(contract, key, itsAddress, messageBody, cost);

        console.log(`✅ ${operationName} transaction sent successfully!`);

        await waitForTransaction(contract, seqno);
    } catch (error) {
        handleCommandError(`in ${operationName}`, error);
    }
}

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
    .command('deploy-interchain-token')
    .description('Deploy a new interchain token')
    .argument('<salt>', 'Salt value for token deployment (256-bit number or hex string)')
    .argument('<name>', 'Token name')
    .argument('<symbol>', 'Token symbol')
    .argument('<decimals>', 'Token decimals (0-255)')
    .argument('<initial-supply>', 'Initial token supply')
    .argument('[minter]', 'Optional minter address (TON address format)')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (salt, name, symbol, decimals, initialSupply, minter, options) => {
        try {
            const saltBigInt = BigInt(salt);

            const decimalsParsed = parseInt(decimals, 10);
            if (isNaN(decimalsParsed) || decimalsParsed < 0 || decimalsParsed > 255) {
                throw new Error('Decimals must be a number between 0 and 255');
            }

            const initialSupplyBigInt = BigInt(initialSupply);

            const client = getTonClient();
            const { contract, _ } = await loadWallet(client);

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

            printSectionHeader('Deploying Interchain Token', '🏗️');
            console.log(`  Salt:           ${saltBigInt}`);
            console.log(`  Name:           ${name}`);
            console.log(`  Symbol:         ${symbol}`);
            console.log(`  Decimals:       ${decimalsParsed}`);
            console.log(`  Initial Supply: ${initialSupplyBigInt}`);
            console.log(`  Minter:         ${minter || 'None'}`);
            console.log();
            printSectionHeader('Deployment Result', '🎯');
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

            await executeITSOperation('Deploy Interchain Token', messageBody, USER_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('deploying interchain token', error);
        }
    });

program
    .command('add-trusted-chain')
    .description('Add a trusted chain to the ITS')
    .argument('<chain-name>', 'Name of the chain to add (e.g., "ethereum", "polygon")')
    .argument('<chain-address>', 'ITS address on the remote chain')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (chainName, chainAddress, options) => {
        try {
            printSectionHeader('Adding Trusted Chain');
            console.log(`  Chain Name:     ${chainName}`);
            console.log(`  Chain Address:  ${chainAddress}`);

            const messageBody = buildAddTrustedChainMessage(chainName, chainAddress);

            await executeITSOperation('Add Trusted Chain', messageBody, SIMPLE_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('adding trusted chain', error);
        }
    });

program
    .command('remove-trusted-chain')
    .description('Remove a trusted chain from the ITS')
    .argument('<chain-name>', 'Name of the chain to remove (e.g., "ethereum", "polygon")')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (chainName, options) => {
        try {
            printSectionHeader('Removing Trusted Chain');
            console.log(`  Chain Name:     ${chainName}`);

            const messageBody = buildRemoveTrustedChainMessage(chainName);

            await executeITSOperation('Remove Trusted Chain', messageBody, SIMPLE_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('removing trusted chain', error);
        }
    });

program
    .command('register-token-metadata')
    .description('Register token metadata for a token (TEP-64 standard) - automatically extracts admin and content from jetton minter')
    .argument('<jetton-minter-address>', 'Jetton minter address to extract admin, content, and codes from')
    .option('-g, --gas <amount>', 'Gas service payment amount in TON', '0.01')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (jettonMinterAddress, options) => {
        try {
            const { client, contract, key, sender, itsAddress, gasServiceAddress } = await setupITSEnvironment();
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            console.log('🔍 Extracting jetton data...');

            // Get all jetton data from the minter
            const { adminAddress, content, jettonMinterCode, jettonWalletCode } = await getJettonDataComplete(jettonMinterAddress);

            // Convert content cell to hex for display
            const contentHex = content.toBoc().toString('hex');

            const registerTokenMetadataMeessage = buildRegisterTokenMetadataMessage(
                adminAddress,
                content,
                jettonMinterCode,
                jettonWalletCode,
            );
            const { name, symbol, decimals } = await interchainTokenService.getJettonMetadata(client.provider(itsAddress), content);

            printSectionHeader('Extracted Jetton Information', '📋');
            console.log(`  Jetton Minter:         ${jettonMinterAddress}`);
            console.log(`  Admin Address:         ${adminAddress.toString()}`);
            console.log(`  Content (hex):         ${contentHex.substring(0, 50)}...`);
            console.log(`  Gas:                   ${options.gas} TON`);
            console.log();
            printSectionHeader('Token Metadata', '📦');
            console.log(`  Name:                  ${name}`);
            console.log(`  Symbol:                ${symbol}`);
            console.log(`  Decimals:              ${decimals}`);
            console.log();

            const jettonMinterAddr = Address.parse(jettonMinterAddress);

            let hubPayload = encodeRegisterTokenMetadataAbi({
                tokenAddress: '0x' + jettonMinterAddr.toRawString().slice(2),
                decimals,
            });

            const gasMessage = createGasServiceMessage(sender, hubPayload.slice(2));

            await executeWithGasService(
                contract,
                key,
                itsAddress,
                registerTokenMetadataMeessage,
                gasServiceAddress,
                gasMessage,
                options.gas,
                options.dryRun,
            );
        } catch (error) {
            handleCommandError('registering token metadata', error);
        }
    });

program
    .command('deploy-remote-interchain-token')
    .description('Deploy an interchain token on a remote chain')
    .argument('<salt>', 'Salt value for token deployment (256-bit number or hex string)')
    .argument('<chain-name>', 'Name of the remote chain (e.g., "ethereum", "polygon")')
    .argument('[remote-minter]', 'Optional minter address on the remote chain')
    .option('-g, --gas <amount>', 'Gas service payment amount in TON', '0.01')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (salt, chainName, remoteMinter, options) => {
        try {
            const { client, contract, key, sender, itsAddress, gasServiceAddress } = await setupITSEnvironment();
            const saltBigInt = BigInt(salt);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            const tokenId = await interchainTokenService.getInterchainTokenId(client.provider(itsAddress), salt, contract.address);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const tokenManager = TokenManager.createFromAddress(tokenManagerAddress);
            const { data } = await tokenManager.getTokenManagerData(client.provider(tokenManagerAddress));
            const tokenManagerInfo = parseTokenManagerInfo(data);

            printSectionHeader('Deploying Remote Interchain Token', '📦');
            console.log(`Salt                  : ${saltBigInt}`);
            console.log(`Chain Name            : ${chainName}`);
            console.log(`Remote Minter         : ${remoteMinter || 'None'}`);
            console.log(`Transaction Gas       : ${options.gas} TON`);
            console.log(`Token ID              : ${tokenManagerInfo.tokenId}`);
            displayTokenManagerInfo(tokenManagerInfo);
            printSectionSeparator();

            const deployRemoteInterchainTokenMessage = buildDeployRemoteInterchainTokenMessage(saltBigInt, chainName, remoteMinter);

            const hubPayload = encodeDeployInterchainTokenHubMessage(chainName, {
                tokenId: '0x' + tokenId.toString(16).padStart(64, '0'),
                name: tokenManagerInfo.name,
                symbol: tokenManagerInfo.symbol,
                decimals: tokenManagerInfo.decimals,
                minter: remoteMinter || '0x',
            });

            const gasMessage = createGasServiceMessage(sender, hubPayload);

            await executeWithGasService(
                contract,
                key,
                itsAddress,
                deployRemoteInterchainTokenMessage,
                gasServiceAddress,
                gasMessage,
                options.gas,
                options.dryRun,
            );
        } catch (error) {
            handleCommandError('deploying remote interchain token', error);
        }
    });

program
    .command('interchain-token-transfer')
    .description('Transfer interchain tokens to another chain')
    .argument('<token-id>', 'Token ID (256-bit number or hex string)')
    .argument('<chain-name>', 'Destination chain name (e.g., "ethereum", "polygon")')
    .argument('<destination-address>', 'Recipient address on the destination chain')
    .argument('<amount>', 'Amount of tokens to transfer')
    .option('-g, --gas <amount>', 'Gas service payment amount in TON', '0.01')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (tokenId, chainName, destinationAddress, amount, options) => {
        try {
            // Initialize clients and addresses
            const { client, contract, key, sender, gasServiceAddress } = await setupITSEnvironment();
            const itsAddress = Address.parse(process.env.TON_ITS_ADDRESS);

            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            // Parse and validate inputs
            const tokenIdBigInt = BigInt(tokenId);
            const spendAmount = BigInt(amount);
            const tokenIdBytes32 = '0x' + tokenIdBigInt.toString(16).padStart(64, '0');
            const destAddrBuffer = Buffer.from(destinationAddress.slice(2), 'hex');

            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenIdBigInt);
            const tokenManager = TokenManager.createFromAddress(tokenManagerAddress);
            const { data } = await tokenManager.getTokenManagerData(client.provider(tokenManagerAddress));
            const tokenManagerInfo = parseTokenManagerInfo(data);

            printSectionHeader('Transferring Interchain Token', '💸');
            console.log(`Token ID              : ${tokenIdBigInt.toString()}`);
            console.log(`Chain                 : ${chainName} → ${destinationAddress}`);
            console.log(`Amount                : ${spendAmount.toString()}`);
            displayTokenManagerInfo(tokenManagerInfo);
            printSectionSeparator();

            const jettonMinterAddress = tokenManagerInfo.jettonMinterAddress;

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

            const gasMessage = createGasServiceMessage(sender, hubPayload);

            const transferPayload = buildInterchainTokenTransferPayload(jettonMinterAddress, tokenIdBigInt, chainName, destAddrBuffer);

            // Create transaction messages
            const jettonTransferMessage = JettonWallet.transferMessage(
                spendAmount,
                itsAddress,
                sender,
                null,
                JETTON_FORWARD_COST,
                transferPayload,
            );

            if (options.dryRun) {
                console.log('🔍 DRY RUN: Would send interchain token transfer');
                console.log(`  Jetton Transfer:`);
                console.log(`    To: ${userJettonWallet.address.toString()}`);
                console.log(`    Cost: ${formatTonAmount('0.1')}`);
                console.log(`    Amount: ${spendAmount.toString()}`);
                console.log(`  Gas Service Transaction:`);
                console.log(`    To: ${gasServiceAddress.toString()}`);
                console.log(`    Cost: ${formatTonAmount(options.gas)}`);
                console.log('✅ Dry run completed - no transactions sent');
            } else {
                const { transfer, seqno } = await sendMultipleTransactionWithCost(
                    contract,
                    key,
                    userJettonWallet.address,
                    jettonTransferMessage,
                    JETTON_TRANSFER_COST,
                    gasServiceAddress,
                    gasMessage,
                    options.gas,
                );

                console.log('💸 Transaction sent successfully!');
                await waitForTransaction(contract, seqno);
                console.log('🎉 Transaction confirmed!');
            }
        } catch (error) {
            handleCommandError('transfer failed', error);
        }
    });

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
    .option('-g, --gas <amount>', 'Gas service payment amount in TON', '0.01')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (salt, chainName, destinationAddress, tokenManagerType, linkParams, options) => {
        try {
            const saltBigInt = BigInt(salt);
            const tmType = parseInt(tokenManagerType, 10);

            const { client, contract, key, sender, itsAddress, gasServiceAddress } = await setupITSEnvironment();
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            const tokenId = await interchainTokenService.getLinkedTokenId(client.provider(itsAddress), contract.address, salt);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const tokenManager = TokenManager.createFromAddress(tokenManagerAddress);
            const { data } = await tokenManager.getTokenManagerData(client.provider(tokenManagerAddress));
            const tokenManagerInfo = parseTokenManagerInfo(data);

            printSectionHeader('Linking Token', '🔗');
            console.log(`Salt                  : ${saltBigInt}`);
            console.log(`Chain Name            : ${chainName}`);
            console.log(`Destination Address   : ${destinationAddress}`);
            console.log(`Token Manager Type    : ${tmType}`);
            console.log(`Link Params           : ${linkParams}`);
            console.log(`Token ID              : ${tokenManagerInfo.tokenId}`);
            displayTokenManagerInfo(tokenManagerInfo);
            printSectionSeparator();

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

            const gasMessage = createGasServiceMessage(sender, hubPayload);

            const linkTokenMessage = buildLinkTokenMessage(
                saltBigInt,
                chainName,
                Buffer.from(destinationAddress.slice(2), 'hex'),
                tmType,
                linkParamsCell,
            );

            await executeWithGasService(
                contract,
                key,
                itsAddress,
                linkTokenMessage,
                gasServiceAddress,
                gasMessage,
                options.gas,
                options.dryRun,
            );
        } catch (error) {
            handleCommandError('linking token', error);
        }
    });

program
    .command('register-canonical-token')
    .description('Register a canonical interchain token (TEP-64 metadata) with jetton codes')
    .argument('<jetton-minter-address>', 'Existing jetton minter address to extract minter and wallet codes from')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (jettonMinterAddress, options) => {
        try {
            const { client, itsAddress } = await setupITSEnvironment();
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            const { adminAddress, content, jettonMinterCode, jettonWalletCode } = await getJettonDataComplete(jettonMinterAddress);
            const messageBody = buildRegisterCanonicalTokenMessage(adminAddress, content, jettonMinterCode, jettonWalletCode);

            const canonicalJettonMinterAddress = Address.parse(jettonMinterAddress);

            const { tokenId } = await interchainTokenService.getCanonicalTokenId(client.provider(itsAddress), canonicalJettonMinterAddress);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const { name, symbol, decimals } = await interchainTokenService.getJettonMetadata(client.provider(itsAddress), content);

            printSectionHeader('Registering Canonical Token', '📝');
            console.log(`  Admin Address:     ${adminAddress}`);
            console.log(`  Content Hex:       ${content.toBoc().toString('hex').substring(0, 50)}...`);
            console.log(`  Jetton Minter:     ${jettonMinterAddress}`);
            console.log();
            printSectionHeader('Token Metadata', '📦');
            console.log(`  Name:              ${name}`);
            console.log(`  Symbol:            ${symbol}`);
            console.log(`  Decimals:          ${decimals}`);
            console.log();
            printSectionHeader('Registration Result', '🎯');
            console.log(`  Token ID:          ${tokenId}`);
            console.log(`  Token Manager:     ${tokenManagerAddress}`);
            console.log(`  Canonical Minter:  ${canonicalJettonMinterAddress}`);

            await executeITSOperation('Register Canonical Token', messageBody, USER_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('registering canonical token', error);
        }
    });

program
    .command('register-canonical-token-permissioned')
    .description('Register a canonical interchain token with permissioned access using metadata parameters')
    .argument('<name>', 'Token name (e.g., "My Token")')
    .argument('<symbol>', 'Token symbol (e.g., "MTK")')
    .argument('<decimals>', 'Token decimals (e.g., 9, 18)')
    .argument('<jetton-minter-address>', 'Existing jetton minter address to extract wallet code from')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (name, symbol, decimals, jettonMinterAddress, options) => {
        try {
            const decimalNum = parseInt(decimals, 10);
            if (isNaN(decimalNum) || decimalNum < 0 || decimalNum > 255) {
                throw new Error('Decimals must be a number between 0 and 255');
            }

            const { client, itsAddress } = await setupITSEnvironment();
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);
            const jettonMinterAddr = Address.parse(jettonMinterAddress);
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

            printSectionHeader('Registering Canonical Token (Permissioned)', '📝');
            console.log(`  Name:                  ${name}`);
            console.log(`  Symbol:                ${symbol}`);
            console.log(`  Decimals:              ${decimalNum}`);
            console.log(`  Jetton Minter:         ${jettonMinterAddress}`);
            console.log();
            printSectionHeader('Token Metadata', '📦');
            console.log(`  Name:                  ${name}`);
            console.log(`  Symbol:                ${symbol}`);
            console.log(`  Decimals:              ${decimalNum}`);
            console.log();
            printSectionHeader('Registration Result', '🎯');
            console.log(`  Token ID:              ${tokenId}`);
            console.log(`  Token Manager:         ${tokenManagerAddress}`);
            console.log(`  Canonical Minter:      ${jettonMinterAddr}`);

            await executeITSOperation('Register Canonical Token (Permissioned)', messageBody, USER_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('registering canonical token (permissioned)', error);
        }
    });

program
    .command('register-custom-token')
    .description(
        'Register a custom interchain token with specific token manager type - automatically extracts admin and content from jetton minter',
    )
    .argument('<salt>', 'Salt value for token registration (256-bit number or hex string)')
    .argument(
        '<token-manager-type>',
        'Token manager type (0=INTERCHAIN_TOKEN, 1=MINT_BURN_FROM, 2=LOCK_UNLOCK, 3=LOCK_UNLOCK_FEE, 4=MINT_BURN)',
    )
    .argument('<operator-address>', 'Operator address for the token (TON address format)')
    .argument('<jetton-minter-address>', 'Jetton minter address to extract admin, content, and codes from')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (salt, tokenManagerType, operatorAddress, jettonMinterAddress, options) => {
        try {
            const saltBigInt = BigInt(salt);
            const tmType = parseInt(tokenManagerType, 10);

            // Validate token manager type
            if (isNaN(tmType) || tmType < 0 || tmType > 4) {
                throw new Error('Token manager type must be a number between 0 and 4');
            }

            const { client, contract, key, sender, itsAddress } = await setupITSEnvironment();
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);
            const operatorAddr = Address.parse(operatorAddress);

            console.log('🔍 Extracting jetton data...');
            const { adminAddress, content, jettonMinterCode, jettonWalletCode } = await getJettonDataComplete(jettonMinterAddress);

            const contentHex = content.toBoc().toString('hex');

            const messageBody = buildRegisterCustomTokenMessage(
                saltBigInt,
                tmType,
                operatorAddr,
                adminAddress,
                content,
                jettonMinterCode,
                jettonWalletCode,
            );

            const tokenId = await interchainTokenService.getLinkedTokenId(client.provider(itsAddress), contract.address, saltBigInt);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const { name, symbol, decimals } = await interchainTokenService.getJettonMetadata(client.provider(itsAddress), content);

            printSectionHeader('Registering Custom Token', '🏗️');
            console.log(`  Salt:                  ${saltBigInt}`);
            console.log(`  Token Manager Type:    ${tmType}`);
            console.log(`  Operator Address:      ${operatorAddr}`);
            console.log(`  Admin Address:         ${adminAddress}`);
            console.log(`  Content (hex):         ${contentHex.substring(0, 50)}...`);
            console.log(`  Jetton Minter:         ${jettonMinterAddress}`);
            console.log();
            printSectionHeader('Token Metadata', '📦');
            console.log(`  Name:                  ${name}`);
            console.log(`  Symbol:                ${symbol}`);
            console.log(`  Decimals:              ${decimals}`);
            console.log();
            printSectionHeader('Registration Result', '🎯');
            console.log(`  Token ID:              ${tokenId}`);
            console.log(`  Token Manager:         ${tokenManagerAddress}`);

            await executeITSOperation('Register Custom Token', messageBody, USER_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('registering custom token', error);
        }
    });

program
    .command('deploy-remote-canonical-token')
    .description('Deploy a canonical interchain token on a remote chain')
    .argument('<jetton-minter-address>', 'Jetton minter address for the canonical token (TON address format)')
    .argument('<chain-name>', 'Name of the remote chain (e.g., "ethereum", "polygon")')
    .option('-g, --gas <amount>', 'Gas service payment amount in TON', '0.01')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (jettonMinterAddress, chainName, options) => {
        try {
            const { client, contract, key, sender, itsAddress, gasServiceAddress } = await setupITSEnvironment();
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);
            const jettonMinterAddr = Address.parse(jettonMinterAddress);
            const deployRemoteCanonicalMessage = buildDeployRemoteCanonicalInterchainTokenMessage(jettonMinterAddr, chainName);

            const { tokenId } = await interchainTokenService.getCanonicalTokenId(client.provider(itsAddress), jettonMinterAddr);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const tokenManager = TokenManager.createFromAddress(tokenManagerAddress);
            const { data } = await tokenManager.getTokenManagerData(client.provider(tokenManagerAddress));
            const tokenManagerInfo = parseTokenManagerInfo(data);

            printSectionHeader('Deploying Remote Canonical Token', '🏗️');
            console.log(`Jetton Minter         : ${jettonMinterAddress}`);
            console.log(`Chain Name            : ${chainName}`);
            console.log(`Transaction Gas       : ${options.gas} TON`);
            console.log(`Token ID              : ${tokenManagerInfo.tokenId}`);
            displayTokenManagerInfo(tokenManagerInfo);
            printSectionSeparator();

            const hubPayload = encodeDeployInterchainTokenHubMessage(chainName, {
                tokenId: '0x' + tokenId.toString(16).padStart(64, '0'),
                name: tokenManagerInfo.name,
                symbol: tokenManagerInfo.symbol,
                decimals: tokenManagerInfo.decimals,
                minter: '0x',
            });

            const gasMessage = createGasServiceMessage(sender, hubPayload);

            await executeWithGasService(
                contract,
                key,
                itsAddress,
                deployRemoteCanonicalMessage,
                gasServiceAddress,
                gasMessage,
                options.gas,
                options.dryRun,
            );
        } catch (error) {
            handleCommandError('deploying remote canonical token', error);
        }
    });

program
    .command('approve-remote-deployment')
    .description('Approve remote deployment of an interchain token (must be called by the minter of the local token)')
    .argument('<salt>', 'Salt value used for token deployment (256-bit number or hex string)')
    .argument('<deployer-address>', 'Address of the deployer (TON address format)')
    .argument('<destination-chain>', 'Name of the destination chain (e.g., "ethereum", "polygon")')
    .argument('<minter-to-approve>', 'Address of the minter to be approved on the destination chain (TON address format)')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (salt, deployerAddress, destinationChain, minterToBeApproved, options) => {
        try {
            const saltBigInt = BigInt(salt);

            printSectionHeader('Approving Remote Deployment');
            console.log(`  Salt:                 ${saltBigInt.toString()}`);
            console.log(`  Deployer Address:     ${deployerAddress}`);
            console.log(`  Destination Chain:    ${destinationChain}`);
            console.log(`  Minter to Approve:    ${minterToBeApproved}`);

            const deployerAddr = Address.parse(deployerAddress);
            const messageBody = buildApproveRemoteDeploymentMessage(saltBigInt, deployerAddr, minterToBeApproved, destinationChain);

            await executeITSOperation('Approve Remote Deployment', messageBody, USER_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('approving remote deployment', error);
        }
    });

program
    .command('revoke-remote-deployment')
    .description('Revoke remote deployment of an interchain token')
    .argument('<salt>', 'Salt value used for token deployment (256-bit number or hex string)')
    .argument('<deployer-address>', 'Address of the deployer (TON address format)')
    .argument('<destination-chain>', 'Name of the destination chain (e.g., "ethereum", "polygon")')
    .argument('<minter-to-revoke>', 'Address of the minter to be revoked on the destination chain')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (salt, deployerAddress, destinationChain, minterToRevoke, options) => {
        try {
            const saltBigInt = BigInt(salt);

            printSectionHeader('Revoking Remote Deployment');
            console.log(`  Salt:                 ${saltBigInt.toString()}`);
            console.log(`  Deployer Address:     ${deployerAddress}`);
            console.log(`  Destination Chain:    ${destinationChain}`);
            console.log(`  Minter to Revoke:     ${minterToRevoke}`);

            const deployerAddr = Address.parse(deployerAddress);
            const messageBody = buildRevokeRemoteDeploymentMessage(saltBigInt, deployerAddr, minterToRevoke, destinationChain);

            await executeITSOperation('Revoke Remote Deployment', messageBody, USER_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('revoking remote deployment', error);
        }
    });

program
    .command('change-operator')
    .description('Change the operator of the Interchain Token Service (current operator only)')
    .argument('<new-operator>', 'Address of the new operator (TON address format)')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (newOperator, options) => {
        try {
            printSectionHeader('Changing Operator');
            console.log(`  New Operator:      ${newOperator}`);
            const operatorAddress = Address.parse(newOperator);
            const messageBody = buildChangeOperatorMessage(operatorAddress);

            await executeITSOperation('Change Operator', messageBody, SIMPLE_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('changing operator', error);
        }
    });

program
    .command('change-owner')
    .description('Change the owner of the ITS (must be called by the owner)')
    .argument('<new-owner>', 'New owner address (TON address format)')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (newOwner, options) => {
        try {
            printSectionHeader('Changing Owner');
            console.log(`  New Owner:         ${newOwner}`);
            const newOwnerAddr = Address.parse(newOwner);
            const messageBody = beginCell().storeUint(OP_CHANGE_OWNER, 32).storeAddress(newOwnerAddr).endCell();

            await executeITSOperation('Change Owner', messageBody, SIMPLE_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('changing owner', error);
        }
    });

program
    .command('pause')
    .description('Pause the Interchain Token Service (operator only)')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (options) => {
        try {
            printSectionHeader('Pausing ITS');
            const messageBody = buildPauseMessage();

            await executeITSOperation('Pause ITS', messageBody, SIMPLE_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('pausing ITS', error);
        }
    });

program
    .command('unpause')
    .description('Unpause the Interchain Token Service (operator only)')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (options) => {
        try {
            printSectionHeader('Unpausing ITS');
            const messageBody = buildUnpauseMessage();

            await executeITSOperation('Unpause ITS', messageBody, SIMPLE_OPERATION_COST, options.dryRun);
        } catch (error) {
            handleCommandError('unpausing ITS', error);
        }
    });

program
    .command('set-flow-limit')
    .description('Set flow limit for a token manager')
    .argument('<token-id>', 'Token ID (256-bit number or hex string)')
    .argument('<flow-limit>', 'Flow limit value (number)')
    .option('--dry-run', 'Show what would be executed without sending transaction')
    .action(async (tokenId, flowLimit, options) => {
        try {
            const client = getTonClient();
            const { contract, key } = await loadWallet(client);
            const itsAddress = Address.parse(ITS_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            // Parse token ID
            let tokenIdBigInt;
            if (tokenId.startsWith('0x')) {
                tokenIdBigInt = BigInt(tokenId);
            } else {
                tokenIdBigInt = BigInt(tokenId);
            }

            // Parse flow limit
            const flowLimitNum = parseInt(flowLimit, 10);
            if (isNaN(flowLimitNum) || flowLimitNum < 0) {
                throw new Error('Flow limit must be a non-negative number');
            }

            // Get token manager address
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenIdBigInt);

            printSectionHeader('Setting Flow Limit', '⚡');
            console.log(`  Token ID:             ${tokenIdBigInt}`);
            console.log(`  Token Manager:        ${tokenManagerAddress.toString()}`);
            console.log(`  New Flow Limit:       ${flowLimitNum}`);

            // Check if token manager exists
            try {
                const tokenManager = TokenManager.createFromAddress(tokenManagerAddress);
                const provider = client.provider(tokenManagerAddress);
                const { data } = await tokenManager.getTokenManagerData(provider);
                const tokenManagerInfo = parseTokenManagerInfo(data);

                console.log(`  Token Name:           ${tokenManagerInfo.name}`);
                console.log(`  Token Symbol:         ${tokenManagerInfo.symbol}`);
            } catch (tmError) {
                console.log('⚠️  Warning: Could not retrieve token manager info (token may not exist)');
            }

            // Create the set flow limit message
            const messageBody = beginCell().storeUint(OP_SET_FLOW_LIMIT, 32).storeUint(flowLimitNum, 256).endCell();

            if (options.dryRun) {
                console.log('🔍 DRY RUN: Would send set flow limit transaction');
                console.log(`  To: ${tokenManagerAddress.toString()}`);
                console.log(`  Cost: ${formatTonAmount('0.02')}`);
                console.log(`  Message body size: ${messageBody.toBoc().length} bytes`);
                console.log('✅ Dry run completed - no transaction sent');
                return;
            }

            const { transfer, seqno } = await sendTransactionWithCost(
                contract,
                key,
                tokenManagerAddress,
                messageBody,
                SIMPLE_OPERATION_COST,
            );

            console.log('✅ Set Flow Limit transaction sent successfully!');
            await waitForTransaction(contract, seqno);
        } catch (error) {
            handleCommandError('setting flow limit', error);
        }
    });

program
    .command('get-minter-proxy')
    .description('Get minter proxy address for a given jetton minter address')
    .argument('<jetton-minter-address>', 'Jetton minter address')
    .action(async (jettonMinterAddress) => {
        try {
            const client = getTonClient();
            const itsAddress = Address.parse(ITS_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            // Parse jetton minter address
            const jettonMinterAddr = Address.parse(jettonMinterAddress);

            printSectionHeader('Minter Proxy Information', '🏭');
            console.log(`Jetton Minter Address: ${jettonMinterAddr.toString()}`);

            try {
                // Get minter proxy address
                const minterProxyAddress = await interchainTokenService.getMinterProxyAddress(
                    client.provider(itsAddress),
                    jettonMinterAddr,
                );

                console.log(`Minter Proxy Address: ${minterProxyAddress.toString()}`);
            } catch (proxyError) {
                console.log('❌ Could not retrieve minter proxy address');
                console.log(`   Error: ${proxyError.message}`);
                console.log('   This jetton minter may not have an associated proxy');
            }
        } catch (error) {
            handleCommandError('getting minter proxy', error);
        }
    });

program
    .command('get-token-manager')
    .description('Get comprehensive token manager information including flow data')
    .argument('<token-id>', 'Token ID (256-bit number or hex string)')
    .action(async (tokenId) => {
        try {
            const client = getTonClient();
            const itsAddress = Address.parse(ITS_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            // Parse token ID
            let tokenIdBigInt = BigInt(tokenId);

            // Get token manager address
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenIdBigInt);

            printSectionHeader('Token Manager Information', '🪙');
            console.log(`Token ID: ${tokenIdBigInt}`);
            console.log(`Token Manager Address: ${tokenManagerAddress.toString()}`);
            console.log();

            try {
                // Create token manager instance and get data
                const tokenManager = TokenManager.createFromAddress(tokenManagerAddress);
                const provider = client.provider(tokenManagerAddress);

                // Get token manager data
                const { data } = await tokenManager.getTokenManagerData(provider);
                const tokenManagerInfo = parseTokenManagerInfo(data);

                // Display basic token manager info
                printSectionHeader('Token Details', '📋');
                console.log(`Name: ${tokenManagerInfo.name}`);
                console.log(`Symbol: ${tokenManagerInfo.symbol}`);
                console.log(`Decimals: ${tokenManagerInfo.decimals}`);
                console.log(`Token Manager Type: ${tokenManagerInfo.tokenManagerType}`);
                console.log(`Jetton Minter Address: ${tokenManagerInfo.jettonMinterAddress}`);
                console.log(`ITS Jetton Wallet: ${tokenManagerInfo.itsJettonWallet}`);
                console.log();

                // Get flow limit data
                try {
                    printSectionHeader('Flow Limit Data', '📊');
                    const flowData = await tokenManager.getFlowLimitData(provider);
                    console.log(`Flow Limit: ${flowData.flowLimit}`);
                    console.log(`Current Epoch: ${flowData.currentEpoch}`);
                    console.log(`Total In Flow: ${flowData.totalInFlow}`);
                    console.log(`Total Out Flow: ${flowData.totalOutFlow}`);
                } catch (flowError) {
                    console.log('❌ Could not retrieve flow limit data');
                    console.log(`   Error: ${flowError.message}`);
                }
            } catch (tmError) {
                console.log('❌ Token Manager does not exist or could not retrieve data');
                console.log(`   Error: ${tmError.message}`);
                console.log(`   This token ID may not have been deployed yet`);
            }
        } catch (error) {
            handleCommandError('getting token manager info', error);
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
            const chains = [
                'ethereum',
                'polygon',
                'avalanche',
                'arbitrum',
                'base',
                'optimism',
                'avalanche-fuji',
                'core-ethereum',
                'core-avalanche',
                'core-optimism',
                'eth-sepolia',
                'optimism-sepolia',
            ];
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
            handleCommandError('getting full state', error);
        }
    });

program
    .command('get-its-jetton-wallet')
    .description('Get the ITS jetton wallet address for a given jetton minter')
    .argument('<jetton-minter-address>', 'Jetton minter address')
    .action(async (jettonMinterAddressStr) => {
        try {
            const client = getTonClient();
            const itsAddress = Address.parse(ITS_ADDRESS);
            const jettonMinterAddress = Address.parse(jettonMinterAddressStr);

            printSectionHeader('ITS Jetton Wallet Lookup', '💰');

            // Create jetton minter instance
            const jettonMinter = JettonMinter.createFromAddress(jettonMinterAddress);

            // Get the ITS jetton wallet address using the minter's getWalletAddress method
            const itsJettonWalletAddress = await getJettonWalletAddress(jettonMinter, client, jettonMinterAddress, itsAddress);
            console.log(`ITS Wallet (linkParams) : 0x${itsJettonWalletAddress.toRawString().slice(2)}`);
            console.log(`Minter                  : 0x${jettonMinter.address.toRawString().slice(2)}`);
        } catch (error) {
            handleCommandError('getting ITS jetton wallet', error);
        }
    });

program.parse();
