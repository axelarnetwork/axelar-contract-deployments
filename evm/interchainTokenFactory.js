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
                throw new Error(`Destination chain ${destinationChain} is not trusted by InterchainTokenService`);
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
            const { originalChain, tokenAddress, destinationChain, gasValue } = options;

            validateParameters({
                isValidAddress: { tokenAddress },
                isString: { originalChain },
                isNonEmptyString: { destinationChain },
                isValidNumber: { gasValue },
            });

            isValidDestinationChain(config, destinationChain);

            const tx = await interchainTokenFactory.deployRemoteCanonicalInterchainToken(
                originalChain,
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
                throw new Error(`Destination chain ${destinationChain} is not trusted by InterchainTokenService`);
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
                'deployRemoteInterchainToken',
                'registerCanonicalInterchainToken',
                'deployRemoteCanonicalInterchainToken',
                'registerCustomToken',
                'linkToken',
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
