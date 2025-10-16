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
        case 'contract-id': {
            const contractId = await interchainTokenFactory.contractId();
            printInfo('InterchainTokenFactory contract ID', contractId);

            break;
        }

        case 'interchain-token-deploy-salt': {
            const [deployer] = args;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({ isValidAddress: { deployer } });

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

        case 'deploy-remote-interchain-token': {
            const [destinationChain] = args;
            const { env } = options;

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

        case 'deploy-remote-canonical-interchain-token': {
            const [tokenAddress, destinationChain] = args;

            const { env } = options;
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

        case 'link-token': {
            const [destinationChain, destinationTokenAddress, tokenManagerType, linkParams] = args;

            const { env } = options;

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

async function main(action, args, options) {
    options.args = args;
    return mainProcessor(options, (axelar, chain, chains, options) => processCommand(axelar, chain, chains, action, options));
}

if (require.main === module) {
    const program = new Command();

    program.name('InterchainTokenFactory').description('Script to perform interchain token factory commands');

    program
        .command('contract-id')
        .description('Get contract ID')
        .action((options, cmd) => {
            main(cmd.name(), [], options);
        });

    program
        .command('interchain-token-deploy-salt')
        .description('Get interchain token deploy salt')
        .requiredOption('--deployer <deployer>', 'Deployer address')
        .action((options, cmd) => {
            main(cmd.name(), [options.deployer], options);
        });

    program
        .command('canonical-interchain-token-deploy-salt')
        .description('Get canonical interchain token deploy salt')
        .requiredOption('--tokenAddress <tokenAddress>', 'Token address')
        .action((options, cmd) => {
            main(cmd.name(), [options.tokenAddress], options);
        });

    program
        .command('canonical-interchain-token-id')
        .description('Get canonical interchain token id')
        .requiredOption('--tokenAddress <tokenAddress>', 'Token address')
        .action((options, cmd) => {
            main(cmd.name(), [options.tokenAddress], options);
        });

    program
        .command('interchain-token-id')
        .description('Get interchain token id')
        .requiredOption('--deployer <deployer>', 'Deployer')
        .action((options, cmd) => {
            main(cmd.name(), [options.deployer], options);
        });

    program
        .command('deploy-interchain-token')
        .description('Deploy interchain token')
        .requiredOption('--name <name>', 'Name')
        .requiredOption('--symbol <symbol>', 'Symbol')
        .requiredOption('--decimals <decimals>', 'Decimals')
        .requiredOption('--initialSupply <initialSupply>', 'Initial supply')
        .requiredOption('--minter <minter>', 'Minter')
        .action((options, cmd) => {
            main(cmd.name(), [options.name, options.symbol, options.decimals, options.initialSupply, options.minter], options);
        });

    program
        .command('deploy-remote-interchain-token')
        .description('Deploy remote interchain token')
        .requiredOption('--destinationChain <destinationChain>', 'Destination chain')
        .addOption(new Option('--gasValue <gasValue>', 'gas value').default('auto'))
        .action((options, cmd) => {
            main(cmd.name(), [options.destinationChain], options);
        });

    program
        .command('register-canonical-interchain-token')
        .description('Register canonical interchain token')
        .requiredOption('--tokenAddress <tokenAddress>', 'Token address')
        .action((options, cmd) => {
            main(cmd.name(), [options.tokenAddress], options);
        });

    program
        .command('deploy-remote-canonical-interchain-token')
        .description('Deploy remote canonical interchain token')
        .requiredOption('--tokenAddress <tokenAddress>', 'Token address')
        .requiredOption('--destinationChain <destinationChain>', 'Destination chain')
        .addOption(new Option('--gasValue <gasValue>', 'gas value').default('auto'))
        .action((options, cmd) => {
            main(cmd.name(), [options.tokenAddress, options.destinationChain], options);
        });

    program
        .command('register-custom-token')
        .description('Register custom token')
        .requiredOption('--tokenAddress <tokenAddress>', 'Token address')
        .requiredOption('--tokenManagerType <tokenManagerType>', 'Token manager type')
        .requiredOption('--operator <operator>', 'Operator')
        .action((options, cmd) => {
            main(cmd.name(), [options.tokenAddress, options.tokenManagerType, options.operator], options);
        });

    program
        .command('link-token')
        .description('Link token to token on destination chain')
        .requiredOption('--destinationChain <destinationChain>', 'Destination chain')
        .requiredOption('--destinationTokenAddress <destinationTokenAddress>', 'Destination token address')
        .requiredOption('--tokenManagerType <tokenManagerType>', 'Token manager type')
        .requiredOption('--linkParams <linkParams>', 'Link params')
        .addOption(new Option('--gasValue <gasValue>', 'gas value').default('auto'))
        .action((options, cmd) => {
            main(
                cmd.name(),
                [options.destinationChain, options.destinationTokenAddress, options.tokenManagerType, options.linkParams],
                options,
            );
        });

    addOptionsToCommands(program, addEvmOptions, { address: true, salt: true });

    program.parse();
}

module.exports = { interchainTokenFactory: main };
