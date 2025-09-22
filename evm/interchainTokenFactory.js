'use strict';

const { ethers } = require('hardhat');
const {
    getDefaultProvider,
    Contract,
    constants: { AddressZero },
    BigNumber,
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    prompt,
    mainProcessor,
    validateParameters,
    getContractJSON,
    getGasOptions,
    printWalletInfo,
    printTokenInfo,
    isTrustedChain,
    encodeITSDestination,
    scaleGasValue,
    loadConfig,
} = require('./utils');
const { addOptionsToCommands } = require('../common');
const { validateChain, estimateITSFee } = require('../common/utils');
const { addEvmOptions } = require('./cli-utils');
const { getDeploymentSalt, handleTx } = require('./its');
const { getWallet } = require('./sign-utils');
const IInterchainTokenFactory = getContractJSON('IInterchainTokenFactory');
const IInterchainTokenService = getContractJSON('IInterchainTokenService');

// For version 2.1.1, use the contracts from the specific package
const IInterchainTokenFactoryV211 = getContractJSON(
    'IInterchainTokenFactory',
    '@axelar-network/interchain-token-service-v2.1.1/artifacts/contracts/interfaces/IInterchainTokenFactory.sol/IInterchainTokenFactory.json',
);
const IInterchainTokenServiceV211 = getContractJSON(
    'IInterchainTokenService',
    '@axelar-network/interchain-token-service-v2.1.1/artifacts/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json',
);

async function processCommand(_axelar, chain, chains, action, options) {
    const { privateKey, address, yes, args } = options;
    const config = loadConfig(options.env);

    const contracts = chain.contracts;
    const contractName = 'InterchainTokenFactory';
    const interchainTokenFactoryAddress = address || contracts.InterchainTokenFactory?.address;
    const interchainTokenServiceAddress = contracts.InterchainTokenService?.address;
    const itsVersion = contracts.InterchainTokenService?.version;

    validateParameters({ isValidAddress: { interchainTokenFactoryAddress, interchainTokenServiceAddress } });

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = await getWallet(privateKey, provider, options);

    await printWalletInfo(wallet, options);

    printInfo('Contract name', contractName);
    printInfo('Contract address', interchainTokenFactoryAddress);

    let interchainTokenFactory;
    let interchainTokenService;
    if (itsVersion === '2.1.1') {
        interchainTokenFactory = new Contract(interchainTokenFactoryAddress, IInterchainTokenFactoryV211.abi, wallet);
        interchainTokenService = new Contract(interchainTokenServiceAddress, IInterchainTokenServiceV211.abi, wallet);
    } else {
        interchainTokenFactory = new Contract(interchainTokenFactoryAddress, IInterchainTokenFactory.abi, wallet);
        interchainTokenService = new Contract(interchainTokenServiceAddress, IInterchainTokenService.abi, wallet);
    }

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('Action', action);

    if (prompt(`Proceed with action ${action}`, yes)) {
        return;
    }

    switch (action) {
        // EX: node evm/interchainTokenFactory.js contract-id --chainNames avalanche --env testnet --yes
        case 'contract-id': {
            const contractId = await interchainTokenFactory.contractId();
            printInfo('InterchainTokenFactory contract ID', contractId);

            break;
        }

        case 'interchain-token-deploy-salt': {
            const [deployer] = args;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({ isValidAddress: { deployer }, isValidString: { salt } });

            const interchainTokenDeploySalt = await interchainTokenFactory.interchainTokenDeploySalt(deployer, deploymentSalt);

            printInfo(
                `interchainTokenDeploySalt for deployer ${deployer} and deployment salt: ${deploymentSalt}`,
                interchainTokenDeploySalt,
            );

            break;
        }

        case 'canonical-interchain-token-deploy-salt': {
            const [tokenAddress] = args;

            validateParameters({ isValidAddress: { tokenAddress } });
            const canonicalInterchainTokenDeploySalt = await interchainTokenFactory.canonicalInterchainTokenDeploySalt(tokenAddress);
            printInfo(`canonicalInterchainTokenDeploySalt for token address: ${tokenAddress}`, canonicalInterchainTokenDeploySalt);

            break;
        }

        case 'interchain-token-id': {
            const [deployer] = args;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({ isValidAddress: { deployer } });

            const interchainTokenId = await interchainTokenFactory.interchainTokenId(deployer, deploymentSalt);
            printInfo(`InterchainTokenId for deployer ${deployer} and deployment salt: ${deploymentSalt}`, interchainTokenId);

            break;
        }

        case 'canonical-interchain-token-id': {
            const [tokenAddress] = args;

            validateParameters({ isValidAddress: { tokenAddress } });

            const canonicalInterchainTokenId = await interchainTokenFactory.canonicalInterchainTokenId(tokenAddress);
            printInfo(`canonicalInterchainTokenId for token address: ${tokenAddress}`, canonicalInterchainTokenId);

            break;
        }
        case 'deploy-interchain-token': {
            const [name, symbol, decimals, initialSupply, minter] = args;

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

            await handleTx(tx, chain, interchainTokenService, action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            printInfo('Token address', await interchainTokenService.registeredTokenAddress(tokenId));
            break;
        }

        case 'deployRemoteInterchainToken': {
            const { destinationChain, env } = options;

            const deploymentSalt = getDeploymentSalt(options);

            const { gasValue, gasFeeValue } = await estimateITSFee(
                chain,
                destinationChain,
                env,
                'InterchainTokenDeployment',
                options.gasValue,
                _axelar,
            );

            validateParameters({
                isNonEmptyString: { destinationChain },
                isValidNumber: { gasValue },
            });

            if (!(await isTrustedChain(destinationChain, interchainTokenService, itsVersion))) {
                throw new Error(`Destination chain ${destinationChain} is not trusted by ITS`);
            }

            const tx = await interchainTokenFactory['deployRemoteInterchainToken(bytes32,string,uint256)'](
                deploymentSalt,
                destinationChain,
                gasValue,
                {
                    value: gasFeeValue,
                    ...gasOptions,
                },
            );
            const tokenId = await interchainTokenFactory.interchainTokenId(wallet.address, deploymentSalt);
            printInfo('tokenId', tokenId);

            await handleTx(tx, chain, interchainTokenService, action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'register-canonical-interchain-token': {
            const [tokenAddress] = args;

            validateParameters({ isValidAddress: { tokenAddress } });

            const tx = await interchainTokenFactory.registerCanonicalInterchainToken(tokenAddress, gasOptions);

            const tokenId = await interchainTokenFactory.canonicalInterchainTokenId(tokenAddress);
            printInfo('tokenId', tokenId);
            await printTokenInfo(tokenAddress, provider);

            await handleTx(tx, chain, interchainTokenService, action, 'TokenManagerDeployed', 'TokenManagerDeploymentStarted');

            break;
        }

        case 'deployRemoteCanonicalInterchainToken': {
            const { tokenAddress, destinationChain, env } = options;

            const { gasValue, gasFeeValue } = await estimateITSFee(
                chain,
                destinationChain,
                env,
                'InterchainTokenDeployment',
                options.gasValue,
                _axelar,
            );

            validateParameters({
                isValidAddress: { tokenAddress },
                isNonEmptyString: { destinationChain },
                isValidNumber: { gasValue },
            });

            validateChain(chains, destinationChain);

            const tx = await interchainTokenFactory['deployRemoteCanonicalInterchainToken(address,string,uint256)'](
                tokenAddress,
                destinationChain,
                gasValue,
                { value: gasFeeValue, ...gasOptions },
            );

            const tokenId = await interchainTokenFactory.canonicalInterchainTokenId(tokenAddress);
            printInfo('tokenId', tokenId);
            await printTokenInfo(tokenAddress, provider);

            await handleTx(tx, chain, interchainTokenService, action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'register-custom-token': {
            const [tokenAddress, tokenManagerType, operator] = args;

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
            await printTokenInfo(tokenAddress, provider);

            await handleTx(tx, chain, interchainTokenService, action, 'TokenManagerDeployed', 'InterchainTokenDeploymentStarted');

            break;
        }

        case 'linkToken': {
            const { destinationChain, destinationTokenAddress, tokenManagerType, linkParams, env } = options;

            const { gasValue, gasFeeValue } = await estimateITSFee(chain, destinationChain, env, 'LinkToken', options.gasValue, _axelar);

            const deploymentSalt = getDeploymentSalt(options);

            if (!(await isTrustedChain(destinationChain, interchainTokenService, itsVersion))) {
                throw new Error(`Destination chain ${destinationChain} is not trusted by ITS`);
            }
            const itsDestinationTokenAddress = encodeITSDestination(chains, destinationChain, destinationTokenAddress);
            printInfo('Human-readable destination token address', destinationTokenAddress);

            validateParameters({
                isNonEmptyString: { destinationChain, destinationTokenAddress },
                isValidNumber: { tokenManagerType, gasValue },
                isValidBytesArray: { linkParams, itsDestinationTokenAddress },
            });

            const tx = await interchainTokenFactory.linkToken(
                deploymentSalt,
                destinationChain,
                itsDestinationTokenAddress,
                tokenManagerType,
                linkParams,
                gasValue,
                { value: gasFeeValue, ...gasOptions },
            );

            const tokenId = await interchainTokenFactory.linkedTokenId(wallet.address, deploymentSalt);
            printInfo('tokenId', tokenId);

            await handleTx(tx, chain, interchainTokenService, action, 'LinkTokenStarted');

            break;
        }

        default: {
            throw new Error(`Unknown action ${action}`);
        }
    }
}

// async function main(options) {
//     await mainProcessor(options, processCommand);
// }

async function main(action, args, options) {
    options.args = args;
    return mainProcessor(options, (axelar, chain, chains, options) => processCommand(axelar, chain, chains, action, options));
}

if (require.main === module) {
    const program = new Command();

    program.name('InterchainTokenFactory').description('Script to perform interchain token factory commands');

    // program.addOption(
    //     new Option('--action <action>', 'interchain token factory action')
    //         .choices([
    //             'contractId',
    //             'interchainTokenDeploySalt',
    //             'canonicalinterchainTokenDeploySalt',
    //             'interchainTokenId',
    //             'canonicalInterchainTokenId',
    //             'interchainTokenAddress',
    //             'deployInterchainToken',
    //             'deployRemoteInterchainToken',
    //             'registerCanonicalInterchainToken',
    //             'deployRemoteCanonicalInterchainToken',
    //             'registerCustomToken',
    //             'linkToken',
    //         ])
    //         .makeOptionMandatory(true),
    // );

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
    program.addOption(new Option('--gasValue <gasValue>', 'gas value').default('auto'));
    program.addOption(new Option('--rawSalt <rawSalt>', 'raw deployment salt').env('RAW_SALT'));
    program.addOption(new Option('--destinationTokenAddress <destinationTokenAddress>', 'destination token address'));
    program.addOption(new Option('--linkParams <linkParams>', 'parameters to use for linking'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

module.exports = { interchainTokenFactory: main };
