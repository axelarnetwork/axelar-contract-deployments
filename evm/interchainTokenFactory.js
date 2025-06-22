'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    Contract,
    constants: { AddressZero },
    BigNumber,
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, prompt, mainProcessor, validateParameters, getContractJSON, getGasOptions, printWalletInfo, analyzeStorageSlot } = require('./utils');
const { addEvmOptions } = require('./cli-utils');
const { getDeploymentSalt, handleTx, isValidDestinationChain } = require('./its');
const { getWallet } = require('./sign-utils');
const IInterchainTokenFactory = getContractJSON('IInterchainTokenFactory');
const IInterchainTokenService = getContractJSON('IInterchainTokenService');

async function processCommand(config, chain, options) {
    const { privateKey, address, action, yes } = options;

    const contracts = chain.contracts;
    const contractName = 'InterchainTokenFactory';
    const interchainTokenFactoryAddress = address || contracts.InterchainTokenFactory?.address;
    const interchainTokenServiceAddress = contracts.InterchainTokenService?.address;

    validateParameters({ isValidAddress: { interchainTokenFactoryAddress, interchainTokenServiceAddress } });

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = await getWallet(privateKey, provider, options);

    await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', interchainTokenFactoryAddress);

    const interchainTokenFactory = new Contract(interchainTokenFactoryAddress, IInterchainTokenFactory.abi, wallet);
    const interchainTokenService = new Contract(interchainTokenServiceAddress, IInterchainTokenService.abi, wallet);

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('Action', action);

    if (prompt(`Proceed with action ${action}`, yes)) {
        return;
    }

    switch (action) {
        case 'contractId': {
            const contractId = await interchainTokenFactory.contractId();
            printInfo('InterchainTokenFactory contract ID', contractId);

            break;
        }

        case 'interchainTokenDeploySalt': {
            const { deployer } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({ isValidAddress: { deployer } });

            const interchainTokenDeploySalt = await interchainTokenFactory.interchainTokenDeploySalt(deployer, deploymentSalt);
            printInfo(
                `interchainTokenDeploySalt for deployer ${deployer} and deployment salt: ${deploymentSalt}`,
                interchainTokenDeploySalt,
            );

            break;
        }

        case 'canonicalinterchainTokenDeploySalt': {
            const { tokenAddress } = options;

            validateParameters({ isValidAddress: { tokenAddress } });

            const canonicalinterchainTokenDeploySalt = await interchainTokenFactory.canonicalinterchainTokenDeploySalt(tokenAddress);
            printInfo(`canonicalinterchainTokenDeploySalt for token address: ${tokenAddress}`, canonicalinterchainTokenDeploySalt);

            break;
        }

        case 'interchainTokenId': {
            const { deployer } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({ isValidAddress: { deployer } });

            const interchainTokenId = await interchainTokenFactory.interchainTokenId(deployer, deploymentSalt);
            printInfo(`InterchainTokenId for deployer ${deployer} and deployment salt: ${deploymentSalt}`, interchainTokenId);

            break;
        }

        case 'canonicalInterchainTokenId': {
            const { tokenAddress } = options;

            validateParameters({ isValidAddress: { tokenAddress } });

            const canonicalInterchainTokenId = await interchainTokenFactory.canonicalInterchainTokenId(tokenAddress);
            printInfo(`canonicalInterchainTokenId for token address: ${tokenAddress}`, canonicalInterchainTokenId);

            break;
        }

        case 'interchainTokenAddress': {
            const { deployer } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({ isValidAddress: { deployer } });

            const interchainTokenAddress = await interchainTokenFactory.interchainTokenAddress(deployer, deploymentSalt);
            printInfo(`interchainTokenAddress for deployer ${deployer} and deployment salt: ${deploymentSalt}`, interchainTokenAddress);

            break;
        }

        case 'deployInterchainToken': {
            const { name, symbol, decimals, initialSupply, minter } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isNonEmptyString: { name, symbol },
                isValidNumber: { decimals },
                isValidDecimal: { initialSupply },
                isAddress: { minter },
            });

            const tx = await interchainTokenFactory.deployInterchainToken(
                deploymentSalt,
                name,
                symbol,
                decimals,
                BigNumber.from(10).pow(decimals).mul(parseInt(initialSupply)),
                minter,
                gasOptions,
            );

            const tokenId = await interchainTokenFactory.interchainTokenId(wallet.address, deploymentSalt);
            printInfo('tokenId', tokenId);

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            printInfo('Token address', await interchainTokenService.registeredTokenAddress(tokenId));
            break;
        }

        case 'deployRemoteInterchainToken': {
            const { destinationChain, gasValue } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isNonEmptyString: { destinationChain },
                isValidNumber: { gasValue },
            });

            if ((await interchainTokenService.trustedAddress(destinationChain)) === '') {
                throw new Error(`Destination chain ${destinationChain} is not trusted by ITS`);
            }

            const tx = await interchainTokenFactory['deployRemoteInterchainToken(bytes32,string,uint256)'](
                deploymentSalt,
                destinationChain,
                gasValue,
                {
                    value: gasValue,
                    ...gasOptions,
                },
            );
            const tokenId = await interchainTokenFactory.interchainTokenId(wallet.address, deploymentSalt);
            printInfo('tokenId', tokenId);

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'registerCanonicalInterchainToken': {
            const { tokenAddress } = options;

            validateParameters({ isValidAddress: { tokenAddress } });

            const tx = await interchainTokenFactory.registerCanonicalInterchainToken(tokenAddress, gasOptions);

            const tokenId = await interchainTokenFactory.canonicalInterchainTokenId(tokenAddress);
            printInfo('tokenId', tokenId);

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'TokenManagerDeploymentStarted');

            break;
        }

        case 'deployRemoteCanonicalInterchainToken': {
            const { tokenAddress, destinationChain, gasValue } = options;

            validateParameters({
                isValidAddress: { tokenAddress },
                isNonEmptyString: { destinationChain },
                isValidNumber: { gasValue },
            });

            isValidDestinationChain(config, destinationChain);

            const tx = await interchainTokenFactory['deployRemoteCanonicalInterchainToken(address,string,uint256)'](
                tokenAddress,
                destinationChain,
                gasValue,
                { value: gasValue, ...gasOptions },
            );

            const tokenId = await interchainTokenFactory.canonicalInterchainTokenId(tokenAddress);
            printInfo('tokenId', tokenId);

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'registerCustomToken': {
            const { tokenAddress, tokenManagerType, operator } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isValidAddress: { tokenAddress },
                isAddress: { operator },
                isValidNumber: { tokenManagerType },
            });

            const tx = await interchainTokenFactory.registerCustomToken(
                deploymentSalt,
                tokenAddress,
                tokenManagerType,
                operator,
                gasOptions,
            );
            const tokenId = await interchainTokenFactory.linkedTokenId(wallet.address, deploymentSalt);
            printInfo('tokenId', tokenId);

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'linkToken': {
            const { destinationChain, destinationTokenAddress, tokenManagerType, linkParams, gasValue } = options;

            const deploymentSalt = getDeploymentSalt(options);

            if ((await interchainTokenService.trustedAddress(destinationChain)) === '') {
                throw new Error(`Destination chain ${destinationChain} is not trusted by ITS`);
            }

            validateParameters({
                isNonEmptyString: { destinationChain },
                isValidNumber: { tokenManagerType, gasValue },
                isValidBytesArray: { linkParams, destinationTokenAddress },
            });

            const tx = await interchainTokenFactory.linkToken(
                deploymentSalt,
                destinationChain,
                destinationTokenAddress,
                tokenManagerType,
                linkParams,
                gasValue,
                { value: gasValue, ...gasOptions },
            );

            const tokenId = await interchainTokenFactory.linkedTokenId(wallet.address, deploymentSalt);
            printInfo('tokenId', tokenId);

            await handleTx(tx, chain, interchainTokenService, options.action, 'LinkTokenStarted');

            break;
        }

        case 'debugStorageLayout': {
            const { tokenId } = options;

            validateParameters({ isNonEmptyString: { tokenId } });

            try {
                const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
                printInfo(`Token address`, tokenAddress);

                // Read first 20 storage slots for more comprehensive analysis
                for (let i = 0; i < 20; i++) {
                    const slot = await provider.getStorageAt(tokenAddress, i);
                    const analysis = analyzeStorageSlot(slot, i);

                    printInfo(`Slot ${i}`, `${slot} - ${analysis.description}`);

                    if (analysis.hasConflict) {
                        printInfo(`  ⚠️  CONFLICT DETECTED: ${analysis.conflictType} - ${analysis.description}`);
                    }
                }

                // Also check some common storage patterns
                printInfo(`\n🔍 Checking common storage patterns:`);

                // Check for ERC20-like storage layout
                try {
                    const nameSlot = await provider.getStorageAt(tokenAddress, 3); // Common slot for name
                    const symbolSlot = await provider.getStorageAt(tokenAddress, 4); // Common slot for symbol
                    const decimalsSlot = await provider.getStorageAt(tokenAddress, 5); // Common slot for decimals

                    printInfo(`Name slot (3)`, nameSlot);
                    printInfo(`Symbol slot (4)`, symbolSlot);
                    printInfo(`Decimals slot (5)`, decimalsSlot);

                    // Try to decode as strings if they look like strings
                    if (nameSlot !== '0x0000000000000000000000000000000000000000000000000000000000000000') {
                        try {
                            const nameLength = parseInt(nameSlot.slice(2, 10), 16);
                            if (nameLength > 0 && nameLength < 32) {
                                printInfo(`  Name length`, nameLength);
                            }
                        } catch (e) {
                            // Not a string
                        }
                    }
                } catch (e) {
                    printInfo(`Could not check ERC20 patterns`);
                }
            } catch (error) {
                if (error.errorName === 'TokenManagerDoesNotExist') {
                    printInfo(`❌ Token ${tokenId} does not exist on ${chain.name}`);
                    printInfo(`This could mean:`);
                    printInfo(`  • Token was deployed on a different chain`);
                    printInfo(`  • Token ID is incorrect`);
                    printInfo(`  • Token deployment failed`);

                    // Check if we can find info about this token in the factory
                    try {
                        const deployer = await interchainTokenFactory.getTokenDeployer(tokenId);
                        if (deployer !== AddressZero) {
                            printInfo(`✅ Factory has deployer record:`, deployer);
                            printInfo(`   This suggests token was intended to be deployed but may have failed`);
                        } else {
                            printInfo(`❌ No deployer record in factory`);
                        }
                    } catch (factoryError) {
                        printInfo(`❌ Could not check factory deployer record`);
                    }
                } else {
                    throw error;
                }
            }

            break;
        }

        case 'updateTokenDeployer': {
            const { tokenId, deployer } = options;

            validateParameters({ 
                isNonEmptyString: { tokenId },
                isValidAddress: { deployer }
            });

            try {
                const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
                printInfo(`Token address for ${tokenId}`, tokenAddress);

                // Try HyperliquidInterchainToken ABI first
                try {
                    const HyperliquidInterchainToken = getContractJSON('HyperliquidInterchainToken');
                    const hyperliquidToken = new Contract(tokenAddress, HyperliquidInterchainToken.abi, wallet);

                    const currentDeployer = await hyperliquidToken.getDeployer();
                    printInfo(`Current deployer`, currentDeployer);
                    printInfo(`New deployer`, deployer);

                    const tx = await hyperliquidToken.updateDeployer(deployer, gasOptions);
                    printInfo(`Updating deployer...`);

                    const receipt = await tx.wait();
                    printInfo(`Transaction hash`, receipt.transactionHash);

                    const updatedDeployer = await hyperliquidToken.getDeployer();
                    printInfo(`Updated deployer`, updatedDeployer);
                    printInfo(`Update successful`, updatedDeployer.toLowerCase() === deployer.toLowerCase());

                } catch (hyperliquidError) {
                    if (hyperliquidError.message.includes('getDeployer is not a function') ||
                        hyperliquidError.message.includes('execution reverted')) {

                        // Fall back to standard InterchainToken ABI
                        const InterchainToken = getContractJSON('InterchainToken');
                        const standardToken = new Contract(tokenAddress, InterchainToken.abi, wallet);

                        // Check if standard token has deployer functions
                        const tokenFunctions = Object.keys(standardToken.interface.functions);
                        const hasDeployerFunctions = tokenFunctions.some(fn => fn.includes('deployer'));

                        if (!hasDeployerFunctions) {
                            printInfo(`❌ This token does not support deployer updates`);
                            printInfo(`   - Token type: Standard InterchainToken`);
                            printInfo(`   - Standard InterchainToken does not have getDeployer/updateDeployer functions`);
                            printInfo(`   - Only HyperliquidInterchainToken supports deployer updates`);
                            return;
                        }
                    } else {
                        throw hyperliquidError;
                    }
                }
            } catch (error) {
                if (error.errorName === 'TokenManagerDoesNotExist') {
                    printInfo(`❌ Token ${tokenId} does not exist on ${chain.name}`);
                } else if (error.errorName === 'NotAuthorized') {
                    printInfo(`❌ Not authorized to update deployer. Must be ITS operator.`);
                } else {
                    printInfo(`❌ Error updating deployer:`, error.message);
                }
            }

            break;
        }

        case 'checkStorageConflicts': {
            const { tokenId, startSlot, endSlot } = options;

            validateParameters({ isNonEmptyString: { tokenId } });

            const start = startSlot ? parseInt(startSlot) : 0;
            const end = endSlot ? parseInt(endSlot) : 50; // Check more slots by default

            try {
                const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
                printInfo(`Token address`, tokenAddress);
                printInfo(`Checking slots ${start} to ${end} for conflicts...`);

                let conflicts = [];
                let nonEmptySlots = [];

                for (let i = start; i < end; i++) {
                    const slot = await provider.getStorageAt(tokenAddress, i);
                    const analysis = analyzeStorageSlot(slot, i);

                    if (!analysis.hasConflict && slot !== '0x0000000000000000000000000000000000000000000000000000000000000000') {
                        nonEmptySlots.push(i);
                    }

                    if (analysis.hasConflict) {
                        conflicts.push({
                            slot: i,
                            content: slot,
                            conflictType: analysis.conflictType,
                            description: analysis.description
                        });
                    }

                    printInfo(`Slot ${i}`, `${slot} - ${analysis.description}`);
                }

                printInfo(`\n📊 Summary:`);
                printInfo(`Total slots checked`, end - start);
                printInfo(`Non-empty slots found`, nonEmptySlots.length);
                printInfo(`Potential conflicts detected`, conflicts.length);

                if (conflicts.length > 0) {
                    printInfo(`\n⚠️  CONFLICTS DETECTED:`);
                    conflicts.forEach(conflict => {
                        printInfo(`Slot ${conflict.slot}: ${conflict.conflictType} - ${conflict.description}`);
                    });
                    printInfo(`\n💡 Recommendations:`);
                    printInfo(`• Avoid using slots: ${conflicts.map(c => c.slot).join(', ')}`);
                    printInfo(`• Consider using higher slot numbers for your implementation`);
                    printInfo(`• Check if these values are expected for your token type`);
                } else {
                    printInfo(`✅ No obvious conflicts detected in checked range`);
                }

                // Show available slots
                const availableSlots = [];
                for (let i = start; i < end; i++) {
                    if (!nonEmptySlots.includes(i)) {
                        availableSlots.push(i);
                    }
                }

                if (availableSlots.length > 0) {
                    printInfo(`\n✅ Available slots: ${availableSlots.slice(0, 10).join(', ')}${availableSlots.length > 10 ? '...' : ''}`);
                }
            } catch (error) {
                if (error.errorName === 'TokenManagerDoesNotExist') {
                    printInfo(`❌ Token ${tokenId} does not exist on ${chain.name}`);
                } else {
                    throw error;
                }
            }

            break;
        }

        default: {
            throw new Error(`Unknown action ${action}`);
        }
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('InterchainTokenFactory').description('Script to perform interchain token factory commands');

    addEvmOptions(program, { address: true, salt: true });

    program.addOption(
        new Option('--action <action>', 'interchain token factory action')
            .choices([
                'contractId',
                'interchainTokenDeploySalt',
                'canonicalinterchainTokenDeploySalt',
                'interchainTokenId',
                'canonicalInterchainTokenId',
                'interchainTokenAddress',
                'deployInterchainToken',
                'deployRemoteInterchainToken',
                'registerCanonicalInterchainToken',
                'deployRemoteCanonicalInterchainToken',
                'registerCustomToken',
                'linkToken',
                'debugStorageLayout',
                'updateTokenDeployer',
                'checkStorageConflicts'
            ])
            .makeOptionMandatory(true),
    );

    program.addOption(new Option('--tokenId <tokenId>', 'ID of the token'));
    program.addOption(new Option('--sender <sender>', 'TokenManager deployer address'));
    program.addOption(new Option('--deployer <deployer>', 'deployer address'));
    program.addOption(new Option('--tokenAddress <tokenAddress>', 'token address'));
    program.addOption(new Option('--name <name>', 'token name'));
    program.addOption(new Option('--symbol <symbol>', 'token symbol'));
    program.addOption(new Option('--decimals <decimals>', 'token decimals'));
    program.addOption(new Option('--minter <minter>', 'token minter').default(AddressZero));
    program.addOption(new Option('--operator <operator>', 'token manager operator').default(AddressZero));
    program.addOption(new Option('--tokenManagerType <tokenManagerType>', 'token manager type'));
    program.addOption(new Option('--initialSupply <initialSupply>', 'initial supply').default(1e9));
    program.addOption(new Option('--destinationChain <destinationChain>', 'destination chain'));
    program.addOption(new Option('--destinationAddress <destinationAddress>', 'destination address'));
    program.addOption(new Option('--gasValue <gasValue>', 'gas value').default(0));
    program.addOption(new Option('--rawSalt <rawSalt>', 'raw deployment salt').env('RAW_SALT'));
    program.addOption(new Option('--destinationTokenAddress <destinationTokenAddress>', 'destination token address'));
    program.addOption(new Option('--linkParams <linkParams>', 'parameters to use for linking'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
