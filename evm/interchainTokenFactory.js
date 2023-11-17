'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    utils: { hexZeroPad },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, prompt, mainProcessor, validateParameters, getContractJSON } = require('./utils');
const { getWallet } = require('./sign-utils');
const { addExtendedOptions } = require('./cli-utils');
const { isValidTokenId, handleTx } = require('./its');
const IInterchainTokenFactory = getContractJSON('IInterchainTokenFactory');

async function processCommand(chain, options) {
    const { privateKey, address, action, yes } = options;

    const contracts = chain.contracts;
    const contractName = 'InterchainTokenFactory';
    const contractConfig = contracts.InterchainTokenFactory;

    const interchainTokenFactoryAddress = address || contracts.interchainTokenFactory?.interchainTokenFactory;

    validateParameters({ isValidAddress: { interchainTokenFactoryAddress } });

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    printInfo('Chain', chain.name);

    const wallet = await getWallet(privateKey, provider, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', interchainTokenFactoryAddress);

    const interchainTokenFactory = new Contract(interchainTokenFactoryAddress, IInterchainTokenFactory.abi, wallet);

    const gasOptions = contractConfig?.gasOptions || chain?.gasOptions || {};
    printInfo('Gas options', JSON.stringify(gasOptions, null, 2));

    printInfo('Action', action);

    if (prompt(`Proceed with action ${action}`, yes)) {
        return;
    }

    const tokenId = options.tokenId;

    if (!isValidTokenId(tokenId)) {
        throw new Error(`Invalid tokenId value: ${tokenId}`);
    }

    switch (action) {
        case 'contractId': {
            const contractId = await interchainTokenFactory.contractId();
            printInfo('InterchainTokenFactory contract ID', contractId);

            break;
        }

        case 'interchainTokenSalt': {
            const { chainNameHash, deployer, salt } = options;

            validateParameters({ isValidAddress: { deployer }, isKeccak256Hash: { chainNameHash, salt } });

            const interchainTokenSalt = await interchainTokenFactory.interchainTokenSalt(chainNameHash, deployer, salt);
            printInfo(`interchainTokenSalt for deployer ${deployer} and deployment salt: ${salt}`, interchainTokenSalt);

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
            const { deployer, salt } = options;

            validateParameters({ isValidAddress: { deployer }, isKeccak256Hash: { salt } });

            const interchainTokenId = await interchainTokenFactory.interchainTokenId(deployer, salt);
            printInfo(`InterchainTokenId for deployer ${deployer} and deployment salt: ${salt}`, interchainTokenId);

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
            const { deployer, salt } = options;

            validateParameters({ isValidAddress: { deployer }, isKeccak256Hash: { salt } });

            const interchainTokenAddress = await interchainTokenFactory.interchainTokenAddress(deployer, salt);
            printInfo(`interchainTokenAddress for deployer ${deployer} and deployment salt: ${salt}`, interchainTokenAddress);

            break;
        }

        case 'deployInterchainToken': {
            const { salt, name, symbol, decimals, mintAmount, distributor } = options;

            validateParameters({
                isKeccak256Hash: { salt },
                isString: { name, symbol },
                isValidAddress: { distributor },
                isValidNumber: { decimals, mintAmount },
            });

            const tx = await interchainTokenFactory.deployInterchainToken(salt, name, symbol, decimals, mintAmount, distributor);

            await handleTx(tx, chain, interchainTokenFactory, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'deployRemoteInterchainToken': {
            const { originalChain, salt, distributor, destinationChain, gasValue } = options;

            validateParameters({
                isKeccak256Hash: { salt },
                isString: { originalChain, destinationChain },
                isValidBytesAddress: { distributor },
                isValidNumber: { gasValue },
            });

            const tx = await interchainTokenFactory.deployRemoteInterchainToken(
                originalChain,
                salt,
                distributor,
                destinationChain,
                gasValue,
            );

            await handleTx(tx, chain, interchainTokenFactory, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'registerCanonicalInterchainToken': {
            const { tokenAddress } = options;

            validateParameters({ isValidAddress: { tokenAddress } });

            const tx = await interchainTokenFactory.registerCanonicalInterchainToken(tokenAddress);

            await handleTx(tx, chain, interchainTokenFactory, options.action, 'TokenManagerDeployed', 'TokenManagerDeploymentStarted');

            break;
        }

        case 'deployRemoteCanonicalInterchainToken': {
            const { originalChain, tokenAddress, destinationChain, gasValue } = options;

            validateParameters({
                isValidAddress: { tokenAddress },
                isString: { originalChain, destinationChain },
                isValidNumber: { gasValue },
            });

            const tx = await interchainTokenFactory.deployRemoteCanonicalInterchainToken(
                originalChain,
                tokenAddress,
                destinationChain,
                gasValue,
            );

            await handleTx(tx, chain, interchainTokenFactory, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'interchainTransfer': {
            const { tokenId, destinationChain, destinationAddress, amount, gasValue } = options;

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            validateParameters({
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
            );

            await handleTx(tx, chain, interchainTokenFactory, options.action, 'Transfer', 'InterchainTransferWithData');

            break;
        }

        case 'tokenTransferFrom': {
            const { tokenId, amount } = options;

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            validateParameters({ isValidNumber: { amount } });

            const tx = await interchainTokenFactory.tokenTransferFrom(tokenIdBytes32, amount);

            await handleTx(tx, chain, interchainTokenFactory, options.action, 'Transfer');

            break;
        }

        case 'tokenApprove': {
            const { tokenId, amount } = options;

            const tokenIdBytes32 = hexZeroPad(tokenId.startsWith('0x') ? tokenId : '0x' + tokenId, 32);

            validateParameters({ isValidNumber: { amount } });

            const tx = await interchainTokenFactory.tokenApprove(tokenIdBytes32, amount);

            await handleTx(tx, chain, interchainTokenFactory, options.action, 'Approval');

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
    program.addOption(new Option('--mintAmount <mintAmount>', 'mint amount'));
    program.addOption(new Option('--originalChain <originalChain>', 'original chain'));
    program.addOption(new Option('--destinationChain <destinationChain>', 'destination chain'));
    program.addOption(new Option('--destinationAddress <destinationAddress>', 'destination address'));
    program.addOption(new Option('--gasValue <gasValue>', 'gas value'));
    program.addOption(new Option('--amount <amount>', 'token amount'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
