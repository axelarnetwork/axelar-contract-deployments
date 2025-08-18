#!/usr/bin/env node
const { Command } = require('commander');
const { toNano, Address, beginCell, Cell } = require('@ton/core');
const { getTonClient, loadWallet, waitForTransaction, sendTransactionWithCost, getJettonCodes } = require('./common');
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
} = require('axelar-cgp-ton');

const ITS_ADDRESS = process.env.TON_ITS_ADDRESS;

if (!ITS_ADDRESS) {
    throw new Error('Please set TON_ITS_ADDRESS in your .env file');
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

async function sendJettonsTo(receiver, deployer, deployerJettonWallet, jettonMinter, jettonToSend, forwardPayload) {
    const client = getTonClient();
    const { contract, key } = await loadWallet(client);

    return await deployerJettonWallet.sendTransfer(
        client.provider(deployerJettonWallet.address),
        contract.sender(key.secretKey),
        toNano('0.1'), // transaction fee
        jettonToSend, // amount of jettons to send
        receiver, // the destination address
        receiver, // responseAddress (can be your deployer address)
        beginCell().endCell(), // custom payload
        toNano('0.065'), // forward_ton_amount
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

            console.log('User Parameters:');
            console.log('─'.repeat(40));
            console.log(`  Salt:           ${saltBigInt}`);
            console.log(`  Name:           ${name}`);
            console.log(`  Symbol:         ${symbol}`);
            console.log(`  Decimals:       ${decimalsParsed}`);
            console.log(`  Initial Supply: ${initialSupplyBigInt}`);
            console.log(`  Minter:         ${minter || 'None'}`);
            console.log(`  Gas:            ${options.gas} TON`);
            console.log();
            console.log('Deployment Result:');
            console.log('─'.repeat(40));
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
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.4')
    .action(async (adminAddress, contentHex, options) => {
        try {
            // Remove 0x prefix if present
            const cleanContentHex = contentHex.startsWith('0x') ? contentHex.slice(2) : contentHex;

            const adminAddr = Address.parse(adminAddress);
            const contentCell = Cell.fromHex(cleanContentHex);
            const messageBody = buildRegisterTokenMetadataMessage(adminAddr, contentCell);

            const client = getTonClient();
            const itsAddress = Address.parse(ITS_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            const jettonMinterAddress = await interchainTokenService.getCanonicalJettonMinterAddress(
                client.provider(itsAddress),
                adminAddr,
                contentCell,
            );
            const { tokenId } = await interchainTokenService.getCanonicalTokenId(client.provider(itsAddress), jettonMinterAddress);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const { name, symbol, decimals } = await interchainTokenService.getJettonMetadata(client.provider(itsAddress), contentCell);

            console.log('User Parameters:');
            console.log('─'.repeat(40));
            console.log(`  Admin Address:         ${adminAddress}`);
            console.log(`  Content Hex:           ${cleanContentHex.substring(0, 50)}...`);
            console.log(`  Gas:                   ${options.gas} TON`);
            console.log();
            console.log('Token Metadata:');
            console.log('─'.repeat(40));
            console.log(`  Name:                  ${name}`);
            console.log(`  Symbol:                ${symbol}`);
            console.log(`  Decimals:              ${decimals}`);
            console.log();
            console.log('Registration Result:');
            console.log('─'.repeat(40));
            console.log(`  Token ID:              ${tokenId}`);
            console.log(`  Token Manager:         ${tokenManagerAddress}`);
            console.log(`  Jetton Minter:         ${jettonMinterAddress}`);

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
    .option('-g, --gas <amount>', 'Gas amount in TON for this transaction', '0.4')
    .action(async (salt, chainName, remoteMinter, options) => {
        try {
            const saltBigInt = salt.startsWith('0x') ? BigInt(salt) : BigInt(salt);

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

            const spendAmount = BigInt(amount);
            const userJettonWallet = JettonWallet.createFromAddress(jettonWalletAddress);

            // Create the interchain token transfer forward payload using the builder
            const forwardPayload = buildInterchainTokenTransferPayload(jettonMinterAddress, tokenIdBigInt, chainName, destinationAddress);

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
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.4')
    .action(async (salt, chainName, destinationAddress, tokenManagerType, linkParams, options) => {
        try {
            const saltBigInt = salt.startsWith('0x') ? BigInt(salt) : BigInt(salt);
            const tmType = parseInt(tokenManagerType, 10);

            console.log('Linking Token with parameters:');
            console.log('  Salt:', saltBigInt);
            console.log('  Chain Name:', chainName);
            console.log('  Destination Address:', destinationAddress);
            console.log('  Token Manager Type:', tmType);
            console.log('  Link Params:', linkParams);
            console.log('  Gas:', options.gas, 'TON');

            const linkParamsCell =
                linkParams === '0x'
                    ? beginCell().endCell()
                    : hexStringToCell(linkParams.startsWith('0x') ? linkParams.slice(2) : linkParams);
            const messageBody = buildLinkTokenMessage(saltBigInt, chainName, destinationAddress, tmType, linkParamsCell);

            const cost = toNano(options.gas);
            await executeITSOperation('Link Token', messageBody, cost);
        } catch (error) {
            console.error('❌ Error linking token:', error.message);
            process.exit(1);
        }
    });

program
    .command('register-canonical-token')
    .description('Register a canonical interchain token (TEP-64 metadata) with jetton codes')
    .argument('<admin-address>', 'Admin address for the token (TON address format)')
    .argument('<content-hex>', 'TEP-64 metadata content as BOC hex string (without 0x prefix)')
    .argument('<jetton-minter-address>', 'Existing jetton minter address to extract minter and wallet codes from')
    .option('-g, --gas <amount>', 'Gas amount in TON', '0.4')
    .action(async (adminAddress, contentHex, jettonMinterAddress, options) => {
        try {
            // Remove 0x prefix if present
            const cleanContentHex = contentHex.startsWith('0x') ? contentHex.slice(2) : contentHex;

            const client = getTonClient();
            const itsAddress = Address.parse(ITS_ADDRESS);
            const interchainTokenService = InterchainTokenService.createFromAddress(itsAddress);

            const adminAddr = Address.parse(adminAddress);
            const contentCell = Cell.fromHex(cleanContentHex);

            const { jettonMinterCode, jettonWalletCode } = await getJettonCodes(jettonMinterAddress);
            const messageBody = buildRegisterCanonicalTokenMessage(adminAddr, contentCell, jettonMinterCode, jettonWalletCode);

            const canonicalJettonMinterAddress = await interchainTokenService.getCanonicalJettonMinterAddress(
                client.provider(itsAddress),
                adminAddr,
                contentCell,
            );

            const { tokenId } = await interchainTokenService.getCanonicalTokenId(client.provider(itsAddress), canonicalJettonMinterAddress);
            const tokenManagerAddress = await interchainTokenService.getTokenManagerAddress(client.provider(itsAddress), tokenId);
            const { name, symbol, decimals } = await interchainTokenService.getJettonMetadata(client.provider(itsAddress), contentCell);

            console.log('User Parameters:');
            console.log('─'.repeat(40));
            console.log(`  Admin Address:     ${adminAddress}`);
            console.log(`  Content Hex:       ${cleanContentHex.substring(0, 50)}...`);
            console.log(`  Jetton Minter:     ${jettonMinterAddress}`);
            console.log(`  Gas:               ${options.gas} TON`);
            console.log();
            console.log('Token Metadata:');
            console.log('─'.repeat(40));
            console.log(`  Name:              ${name}`);
            console.log(`  Symbol:            ${symbol}`);
            console.log(`  Decimals:          ${decimals}`);
            console.log();
            console.log('Deployment Result:');
            console.log('─'.repeat(40));
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

            const canonicalJettonMinterAddress = await interchainTokenService.getCanonicalJettonMinterAddress(
                client.provider(itsAddress),
                adminAddr,
                contentCell,
            );

            const { tokenId } = await interchainTokenService.getCanonicalTokenId(client.provider(itsAddress), canonicalJettonMinterAddress);
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
            console.log(`  Custom Minter:         ${canonicalJettonMinterAddress}`);

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
