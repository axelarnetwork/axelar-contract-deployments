'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    Contract,
    constants: { AddressZero },
    BigNumber,
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, prompt, mainProcessor, validateParameters, getContractJSON, getGasOptions, printWalletInfo } = require('./utils');
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
        case 'debugStorageLayout': {
            const { tokenId } = options;
        
            validateParameters({ isNonEmptyString: { tokenId } });
        
            try {
                const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
                printInfo(`Token address`, tokenAddress);
            
                // Read first 10 storage slots
                for (let i = 0; i < 10; i++) {
                    const slot = await provider.getStorageAt(tokenAddress, i);
                    const isEmpty = slot === '0x0000000000000000000000000000000000000000000000000000000000000000';
                    const asAddress = '0x' + slot.slice(-40);
                    const isYourAddress = asAddress.toLowerCase() === wallet.address.toLowerCase();
                    
                    printInfo(`Slot ${i}`, `${slot} ${isEmpty ? '(empty)' : ''} ${isYourAddress ? '← YOUR ADDRESS!' : ''}`);
                    if (!isEmpty && !isYourAddress) {
                        printInfo(`  As address`, asAddress);
                        printInfo(`  As number`, parseInt(slot, 16));
                    }
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
        
        case 'getTokenDeployer': {
            const { tokenId } = options;
        
            validateParameters({ isNonEmptyString: { tokenId } });
        
            try {
                const deployer = await interchainTokenFactory.getTokenDeployer(tokenId);
                printInfo(`Token ${tokenId} was deployed by`, deployer);
                
                // Also show the current wallet address for comparison
                printInfo(`Current wallet address`, wallet.address);
                printInfo(`Deployer matches current wallet`, deployer.toLowerCase() === wallet.address.toLowerCase());
            } catch (error) {
                printInfo(`❌ Error getting token deployer:`, error.message);
                printInfo(`This could mean the token doesn't exist or wasn't deployed via this factory`);
            }
        
            break;
        }
        
        case 'checkTokenSlot0': {
            const { tokenId } = options;

            validateParameters({ isNonEmptyString: { tokenId } });

            try {
                // Get the token address
                const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
                printInfo(`Token address for ${tokenId}`, tokenAddress);

                // Read storage slot 0 directly from the token contract
                const slot0 = await provider.getStorageAt(tokenAddress, 0);
                const deployerFromSlot0 = '0x' + slot0.slice(-40); // Last 20 bytes
                
                printInfo(`Deployer in token slot 0`, deployerFromSlot0);
                printInfo(`Current wallet address`, wallet.address);
                printInfo(`Slot 0 matches wallet`, deployerFromSlot0.toLowerCase() === wallet.address.toLowerCase());
                
                // Also check via factory's mapping for comparison
                const deployerFromFactory = await interchainTokenFactory.getTokenDeployer(tokenId);
                printInfo(`Deployer from factory`, deployerFromFactory);
                printInfo(`Factory and slot 0 match`, deployerFromSlot0.toLowerCase() === deployerFromFactory.toLowerCase());

                // Test the getDeployer function on the token contract itself
                const IInterchainToken = getContractJSON('IInterchainToken');
                const tokenContract = new Contract(tokenAddress, IInterchainToken.abi, wallet);
                
                try {
                    const deployerFromContract = await tokenContract.getDeployer();
                    printInfo(`Deployer from token contract`, deployerFromContract);
                    printInfo(`All methods match`, 
                        deployerFromSlot0.toLowerCase() === deployerFromContract.toLowerCase() &&
                        deployerFromFactory.toLowerCase() === deployerFromContract.toLowerCase()
                    );
                } catch (error) {
                    printInfo(`Token contract getDeployer() failed`, error.message);
                }
            } catch (error) {
                if (error.errorName === 'TokenManagerDoesNotExist') {
                    printInfo(`❌ Token ${tokenId} does not exist on ${chain.name}`);
                    printInfo(`Cannot check storage slots for non-existent token`);
                    
                    // Check if we can find info about this token in the factory
                    try {
                        const deployer = await interchainTokenFactory.getTokenDeployer(tokenId);
                        if (deployer !== AddressZero) {
                            printInfo(`✅ Factory has deployer record:`, deployer);
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
        
        case 'checkTokenExists': {
            const { tokenId } = options;
        
            validateParameters({ isNonEmptyString: { tokenId } });
        
            printInfo(`Checking token ${tokenId} on ${chain.name}:`);
            
            // Check if token exists via ITS
            try {
                const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
                printInfo(`✅ Token exists at address:`, tokenAddress);
                
                // Check token manager
                try {
                    const tokenManagerAddress = await interchainTokenService.deployedTokenManager(tokenId);
                    printInfo(`✅ Token manager at:`, tokenManagerAddress);
                } catch (error) {
                    printInfo(`❌ Token manager not found`);
                }
                
            } catch (error) {
                if (error.errorName === 'TokenManagerDoesNotExist') {
                    printInfo(`❌ Token does not exist on this chain`);
                } else {
                    printInfo(`❌ Error checking token:`, error.message);
                }
            }
            
            // Check factory deployer record
            try {
                const deployer = await interchainTokenFactory.getTokenDeployer(tokenId);
                if (deployer !== AddressZero) {
                    printInfo(`✅ Factory deployer record:`, deployer);
                    printInfo(`   Deployer matches wallet:`, deployer.toLowerCase() === wallet.address.toLowerCase());
                } else {
                    printInfo(`❌ No factory deployer record`);
                }
            } catch (error) {
                printInfo(`❌ Error checking factory record:`, error.message);
            }
        
            break;
        }

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

        case 'deployInterchainTokenWithDeployer': {
            const { name, symbol, decimals, initialSupply, minter, deployer } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isNonEmptyString: { name, symbol },
                isValidNumber: { decimals },
                isValidDecimal: { initialSupply },
                isAddress: { minter },
                isValidAddress: { deployer },
            });

            const tx = await interchainTokenFactory.deployInterchainTokenWithDeployer(
                deploymentSalt,
                name,
                symbol,
                decimals,
                BigNumber.from(10).pow(decimals).mul(parseInt(initialSupply)),
                minter,
                deployer,
                gasOptions,
            );

            const tokenId = await interchainTokenFactory.interchainTokenId(wallet.address, deploymentSalt);
            printInfo('tokenId', tokenId);
            printInfo('Custom deployer', deployer);

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            printInfo('Token address', await interchainTokenService.registeredTokenAddress(tokenId));
            break;
        }

        case 'updateTokenDeployer': {
            const { tokenId, deployer } = options;  // Note: changed from newDeployer to deployer to match your command
        
            validateParameters({ 
                isNonEmptyString: { tokenId },
                isValidAddress: { deployer }
            });
        
            try {
                // Get the token address from ITS
                const tokenAddress = await interchainTokenService.registeredTokenAddress(tokenId);
                printInfo(`Token address for ${tokenId}`, tokenAddress);
        
                // Create token contract instance with the full InterchainToken ABI
                const InterchainToken = getContractJSON('InterchainToken');  // Changed from IInterchainToken
                const tokenContract = new Contract(tokenAddress, InterchainToken.abi, wallet);
        
                // Get current deployer for comparison
                const currentDeployer = await tokenContract.getDeployer();
                printInfo(`Current deployer`, currentDeployer);
                printInfo(`New deployer`, deployer);
        
                // Update the deployer
                const tx = await tokenContract.updateDeployer(deployer, gasOptions);
                printInfo(`Updating deployer...`);
                
                // Wait for transaction
                const receipt = await tx.wait();
                printInfo(`Transaction hash`, receipt.transactionHash);
        
                // Verify the change
                const updatedDeployer = await tokenContract.getDeployer();
                printInfo(`Updated deployer`, updatedDeployer);
                printInfo(`Update successful`, updatedDeployer.toLowerCase() === deployer.toLowerCase());
        
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

        case 'deployRemoteInterchainToken': {
            const { destinationChain, gasValue } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isNonEmptyString: { destinationChain },
                isValidNumber: { gasValue },
            });

            //if ((await interchainTokenService.trustedAddress(destinationChain)) === '') {
            //    throw new Error(`Destination chain ${destinationChain} is not trusted by ITS`);
            //}

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

        case 'deployRemoteInterchainTokenWithDeployer': {
            const { destinationChain, gasValue, deployer } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isNonEmptyString: { destinationChain },
                isValidNumber: { gasValue },
                isValidAddress: { deployer },
            });

            if ((await interchainTokenService.trustedAddress(destinationChain)) === '') {
                throw new Error(`Destination chain ${destinationChain} is not trusted by ITS`);
            }

            const tx = await interchainTokenFactory['deployRemoteInterchainTokenWithDeployer(bytes32,string,uint256,address)'](
                deploymentSalt,
                destinationChain,
                gasValue,
                deployer,
                {
                    value: gasValue,
                    ...gasOptions,
                },
            );

            const tokenId = await interchainTokenFactory.interchainTokenId(wallet.address, deploymentSalt);
            printInfo('tokenId', tokenId);
            printInfo('Custom deployer', deployer);

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }
        case 'deployRemoteInterchainTokenWithMinter': {
            const { destinationChain, gasValue, minter, destinationMinter } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isNonEmptyString: { destinationChain },
                isValidNumber: { gasValue },
                isAddress: { minter },
            });

            if ((await interchainTokenService.trustedAddress(destinationChain)) === '') {
                throw new Error(`Destination chain ${destinationChain} is not trusted by ITS`);
            }

            // Convert destinationMinter to bytes if provided, otherwise use empty bytes
            const destinationMinterBytes = destinationMinter ? ethers.utils.toUtf8Bytes(destinationMinter) : '0x';

            const tx = await interchainTokenFactory.deployRemoteInterchainTokenWithMinter(
                deploymentSalt,
                minter || AddressZero,
                destinationChain,
                destinationMinterBytes,
                gasValue,
                {
                    value: gasValue,
                    ...gasOptions,
                },
            );

            const tokenId = await interchainTokenFactory.interchainTokenId(wallet.address, deploymentSalt);
            printInfo('tokenId', tokenId);
            printInfo('Minter', minter || 'None (address(0))');

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'deployRemoteInterchainTokenWithMinterAndDeployer': {
            const { destinationChain, gasValue, minter, deployer, destinationMinter } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isNonEmptyString: { destinationChain },
                isValidNumber: { gasValue },
                isAddress: { minter },
                isValidAddress: { deployer },
            });

            if ((await interchainTokenService.trustedAddress(destinationChain)) === '') {
                throw new Error(`Destination chain ${destinationChain} is not trusted by ITS`);
            }

            // Convert destinationMinter to bytes if provided, otherwise use empty bytes
            const destinationMinterBytes = destinationMinter ? ethers.utils.toUtf8Bytes(destinationMinter) : '0x';

            const tx = await interchainTokenFactory.deployRemoteInterchainTokenWithMinterAndDeployer(
                deploymentSalt,
                minter || AddressZero,
                destinationChain,
                destinationMinterBytes,
                gasValue,
                deployer,
                {
                    value: gasValue,
                    ...gasOptions,
                },
            );

            const tokenId = await interchainTokenFactory.interchainTokenId(wallet.address, deploymentSalt);
            printInfo('tokenId', tokenId);
            printInfo('Minter', minter || 'None (address(0))');
            printInfo('Custom deployer', deployer);

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

        case 'deployRemoteCanonicalInterchainTokenWithDeployer': {
            const { tokenAddress, destinationChain, gasValue, deployer } = options;

            validateParameters({
                isValidAddress: { tokenAddress },
                isNonEmptyString: { destinationChain },
                isValidNumber: { gasValue },
                isValidAddress: { deployer },
            });

            isValidDestinationChain(config, destinationChain);

            const tx = await interchainTokenFactory['deployRemoteCanonicalInterchainTokenWithDeployer(address,string,uint256,address)'](
                tokenAddress,
                destinationChain,
                gasValue,
                deployer,
                { value: gasValue, ...gasOptions },
            );

            const tokenId = await interchainTokenFactory.canonicalInterchainTokenId(tokenAddress);
            printInfo('tokenId', tokenId);
            printInfo('Custom deployer', deployer);

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
                'deployInterchainTokenWithDeployer',
                'deployRemoteInterchainToken',
                'deployRemoteInterchainTokenWithDeployer',
                'deployRemoteInterchainTokenWithMinter',
                'deployRemoteInterchainTokenWithMinterAndDeployer',
                'registerCanonicalInterchainToken',
                'deployRemoteCanonicalInterchainToken',
                'deployRemoteCanonicalInterchainTokenWithDeployer',
                'registerCustomToken',
                'linkToken',
                'getTokenDeployer',
                'checkTokenExists',
                'checkTokenSlot0',
                'debugStorageLayout',
                'updateTokenDeployer'
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
    program.addOption(new Option('--destinationMinter <destinationMinter>', 'destination minter address'));
    program.addOption(new Option('--gasValue <gasValue>', 'gas value').default(0));
    program.addOption(new Option('--rawSalt <rawSalt>', 'raw deployment salt').env('RAW_SALT'));
    program.addOption(new Option('--destinationTokenAddress <destinationTokenAddress>', 'destination token address'));
    program.addOption(new Option('--linkParams <linkParams>', 'parameters to use for linking'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}