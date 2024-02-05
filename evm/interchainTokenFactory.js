'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    Contract,
    constants: { AddressZero },
    utils: { keccak256, toUtf8Bytes, hexlify },
} = ethers;
const { Command, Option } = require('commander');
const { printInfo, prompt, mainProcessor, validateParameters, getContractJSON, getGasOptions, printWalletInfo } = require('./utils');
const { addExtendedOptions } = require('./cli-utils');
const { getDeploymentSalt, handleTx, isValidDestinationChain } = require('./its');
const { getWallet } = require('./sign-utils');
const IInterchainTokenFactory = getContractJSON('IInterchainTokenFactory');
const IInterchainTokenService = getContractJSON('IInterchainTokenService');
const axios = require('axios');

async function processCommand(config, chain, options) {
    const { privateKey, address, action, yes } = options;

    const contracts = chain.contracts;
    const contractName = 'InterchainTokenFactory';

    const interchainTokenFactoryAddress = address || contracts.InterchainTokenService?.interchainTokenFactory;
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
                parseInt(initialSupply * 10 ** decimals),
                minter,
                gasOptions,
            );

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'deployRemoteInterchainToken': {
            const { originalChain, minter, destinationChain, gasValue } = options;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({
                isString: { originalChain },
                isNonEmptyString: { destinationChain },
                isAddress: { minter },
                isValidNumber: { gasValue },
            });

            isValidDestinationChain(config, destinationChain);

            const tx = await interchainTokenFactory.deployRemoteInterchainToken(
                originalChain,
                deploymentSalt,
                minter,
                destinationChain,
                gasValue,
                { value: gasValue, ...gasOptions },
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

            await handleTx(tx, chain, interchainTokenService, options.action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'registerAllGatewayTokens': {
            let { assetApi, tokenInfoApi, batchSize } = options;

            assetApi = assetApi || `${config.axelar.lcd}/axelar/nexus/v1beta1/assets/`;
            tokenInfoApi = tokenInfoApi || `${config.axelar.lcd}/axelar/evm/v1beta1/token_info/`;
            batchSize = batchSize || 10;

            validateParameters({ isString: { assetApi, tokenInfoApi }, isNumber: { batchSize } });

            const { assets } = (await axios.get(`${assetApi}${chain.name}`)).data;
            const unregisteredAssets = [];

            for (const asset of assets) {
                const salt = keccak256(hexlify(toUtf8Bytes(asset)));
                const tokenId = await interchainTokenService.interchainTokenId(AddressZero, salt);
                const tokenManagerAddress = await interchainTokenService.tokenManagerAddress(tokenId);

                if ((await provider.getCode(tokenManagerAddress)).length === 2) {
                    unregisteredAssets.push(asset);
                }
            }

            for (let i = 0; i < unregisteredAssets.length; i += batchSize) {
                const multicallData = [];

                for (let j = i; j < i + batchSize && j < unregisteredAssets.length; j++) {
                    const asset = unregisteredAssets[j];
                    const salt = keccak256(hexlify(toUtf8Bytes(asset)));
                    const { symbol } = (await axios.get(`${tokenInfoApi}${chain.name}?asset=${asset}`)).data.details;
                    const { data } = await interchainTokenFactory.populateTransaction.registerGatewayToken(salt, symbol);
                    multicallData.push(data);
                }

                const tx = await interchainTokenFactory.multicall(multicallData, { gasOptions });
                await handleTx(tx, chain, interchainTokenFactory, options.action, 'TokenManagerDeployed');
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
                'registerAllGatewayTokens',
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
    program.addOption(new Option('--minter <minter>', 'token minter').default(AddressZero));
    program.addOption(new Option('--initialSupply <initialSupply>', 'initial supply').default(1e9));
    program.addOption(new Option('--originalChain <originalChain>', 'original chain').default(''));
    program.addOption(new Option('--destinationChain <destinationChain>', 'destination chain'));
    program.addOption(new Option('--destinationAddress <destinationAddress>', 'destination address'));
    program.addOption(new Option('--gasValue <gasValue>', 'gas value').default(0));
    program.addOption(new Option('--rawSalt <rawSalt>', 'raw deployment salt').env('RAW_SALT'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
