'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { hexZeroPad },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, prompt, mainProcessor, validateParameters, getContractJSON, getGasOptions } = require('./utils');
const { getWallet } = require('./sign-utils');
const { addExtendedOptions } = require('./cli-utils');
const { getDeploymentSalt, handleTx } = require('./its');
const IInterchainTokenFactory = getContractJSON('IInterchainTokenFactory');
const IInterchainTokenService = getContractJSON('IInterchainTokenService');
const IERC20 = getContractJSON('IERC20');

async function processCommand(_config, chain, options) {
    const { privateKey, address, action, yes } = options;

    const contracts = chain.contracts;
    const contractName = 'InterchainTokenFactory';

    const interchainTokenFactoryAddress = address || contracts.InterchainTokenService?.interchainTokenFactory;
    const interchainTokenServiceAddress = contracts.InterchainTokenService?.address;

    validateParameters({ isValidAddress: { interchainTokenFactoryAddress, interchainTokenServiceAddress } });

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    printInfo('Chain', chain.name);

    const wallet = await getWallet(privateKey, provider, options);

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

        case 'interchainTokenSalt': {
            const { chainNameHash, deployer } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({ isValidAddress: { deployer }, isKeccak256Hash: { chainNameHash } });

            const interchainTokenSalt = await interchainTokenFactory.interchainTokenSalt(chainNameHash, deployer, deploymentSalt);
            printInfo(`interchainTokenSalt for deployer ${deployer} and deployment salt: ${deploymentSalt}`, interchainTokenSalt);

            break;
        }

        case 'canonicalInterchainTokenSalt': {
            const { chainNameHash, tokenAddress } = options;

            validateParameters({ isValidAddress: { tokenAddress }, isKeccak256Hash: { chainNameHash } });

            const canonicalInterchainTokenSalt = await interchainTokenFactory.canonicalInterchainTokenSalt(chainNameHash, tokenAddress);
            printInfo(`canonicalInterchainTokenSalt for token address: ${tokenAddress}`, canonicalInterchainTokenSalt);

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

        case 'deployerTokenBalance': {
            const { tokenId, deployer } = options;

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            validateParameters({ isValidTokenId: { tokenId }, isValidAddress: { deployer } });

            const deployerTokenBalance = await interchainTokenFactory.deployerTokenBalance(tokenIdBytes32, deployer);
            printInfo(`deployerTokenBalance for deployer ${deployer} and token ID: ${tokenId}`, deployerTokenBalance);

            break;
        }

        case 'deployInterchainToken': {
            const { name, symbol, decimals, initialSupply, distributor } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isNonEmptyString: { name, symbol },
                isValidAddress: { distributor },
                isValidNumber: { decimals, initialSupply },
            });

            const tx = await interchainTokenFactory.deployInterchainToken(
                deploymentSalt,
                name,
                symbol,
                decimals,
                initialSupply,
                distributor,
                gasOptions,
            );

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'deployRemoteInterchainToken': {
            const { originalChain, distributor, destinationChain, gasValue } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isNonEmptyString: { originalChain, destinationChain },
                isValidBytesAddress: { distributor },
                isValidNumber: { gasValue },
            });

            const tx = await interchainTokenFactory.deployRemoteInterchainToken(
                originalChain,
                deploymentSalt,
                distributor,
                destinationChain,
                gasValue,
                gasOptions,
            );

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'registerCanonicalInterchainToken': {
            const { tokenAddress } = options;

            validateParameters({ isValidAddress: { tokenAddress } });

            const tx = await interchainTokenFactory.registerCanonicalInterchainToken(tokenAddress, gasOptions);

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'TokenManagerDeploymentStarted');

            break;
        }

        case 'deployRemoteCanonicalInterchainToken': {
            const { originalChain, tokenAddress, destinationChain, gasValue } = options;

            validateParameters({
                isValidAddress: { tokenAddress },
                isNonEmptyString: { originalChain, destinationChain },
                isValidNumber: { gasValue },
            });

            const tx = await interchainTokenFactory.deployRemoteCanonicalInterchainToken(
                originalChain,
                tokenAddress,
                destinationChain,
                gasValue,
                gasOptions,
            );

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'interchainTransfer': {
            const { tokenId, destinationChain, destinationAddress, amount, gasValue } = options;

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            validateParameters({
                isValidTokenId: { tokenId },
                isString: { destinationChain },
                isValidCalldata: { destinationAddress },
                isValidNumber: { amount, gasValue },
            });

            const tx = await interchainTokenFactory.interchainTransfer(
                tokenIdBytes32,
                destinationChain,
                destinationAddress,
                amount,
                gasValue,
                gasOptions,
            );

            if (destinationChain === '') {
                const tokenAddress = await interchainTokenService.interchainTokenAddress(tokenIdBytes32);
                const token = new Contract(tokenAddress, IERC20.abi, wallet);

                await handleTx(tx, chain, token, options.action, 'Transfer');
            } else {
                await handleTx(tx, chain, interchainTokenFactory, options.action, 'InterchainTransferWithData');
            }

            break;
        }

        case 'tokenTransferFrom': {
            const { tokenId, amount } = options;

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            validateParameters({ isValidTokenId: { tokenId }, isValidNumber: { amount } });

            const tokenAddress = await interchainTokenService.interchainTokenAddress(tokenIdBytes32);
            const token = new Contract(tokenAddress, IERC20.abi, wallet);

            const tx = await interchainTokenFactory.tokenTransferFrom(tokenIdBytes32, amount, gasOptions);

            await handleTx(tx, chain, token, options.action, 'Transfer');

            break;
        }

        case 'tokenApprove': {
            const { tokenId, amount } = options;

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            validateParameters({ isValidTokenId: { tokenId }, isValidNumber: { amount } });

            const tokenAddress = await interchainTokenService.interchainTokenAddress(tokenIdBytes32);
            const token = new Contract(tokenAddress, IERC20.abi, wallet);

            const tx = await interchainTokenFactory.tokenApprove(tokenIdBytes32, amount, gasOptions);

            await handleTx(tx, chain, token, options.action, 'Approval');

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

    addExtendedOptions(program, { address: true, salt: true });

    program.addOption(
        new Option('--action <action>', 'interchain token factory action')
            .choices([
                'contractId',
                'interchainTokenSalt',
                'canonicalInterchainTokenSalt',
                'interchainTokenId',
                'canonicalInterchainTokenId',
                'interchainTokenAddress',
                'deployerTokenBalance',
                'deployInterchainToken',
                'deployRemoteInterchainToken',
                'registerCanonicalInterchainToken',
                'deployRemoteCanonicalInterchainToken',
                'interchainTransfer',
                'tokenTransferFrom',
                'tokenApprove',
            ])
            .makeOptionMandatory(true),
    );

    program.addOption(new Option('--tokenId <tokenId>', 'ID of the token'));
    program.addOption(new Option('--sender <sender>', 'TokenManager deployer address'));
    program.addOption(new Option('--chainNameHash <chainNameHash>', 'chain name hash'));
    program.addOption(new Option('--deployer <deployer>', 'deployer address'));
    program.addOption(new Option('--tokenAddress <tokenAddress>', 'token address'));
    program.addOption(new Option('--name <name>', 'token name'));
    program.addOption(new Option('--symbol <symbol>', 'token symbol'));
    program.addOption(new Option('--decimals <decimals>', 'token decimals'));
    program.addOption(new Option('--distributor <distributor>', 'token distributor'));
    program.addOption(new Option('--initialSupply <initialSupply>', 'initial supply').default(0));
    program.addOption(new Option('--originalChain <originalChain>', 'original chain'));
    program.addOption(new Option('--destinationChain <destinationChain>', 'destination chain'));
    program.addOption(new Option('--destinationAddress <destinationAddress>', 'destination address'));
    program.addOption(new Option('--gasValue <gasValue>', 'gas value'));
    program.addOption(new Option('--amount <amount>', 'token amount'));
    program.addOption(new Option('--rawSalt <rawSalt>', 'raw deployment salt').env('RAW_SALT'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
