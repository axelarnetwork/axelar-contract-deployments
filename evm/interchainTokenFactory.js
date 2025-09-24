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
    loadConfig,
} = require('./utils');
const { validateChain, estimateITSFee, addOptionsToCommands } = require('../common');
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
        // EX: node evm/interchainTokenFactory.js contract-id --chainNames avalanche --env testnet --yes
        case 'contract-id': {
            const contractId = await interchainTokenFactory.contractId();
            printInfo('InterchainTokenFactory contract ID', contractId);

            break;
        }

        // TODO: review
        case 'interchain-token-deploy-salt': {
            const [deployer] = args;

            const deploymentSalt = getDeploymentSalt(options);

            validateParameters({ isValidAddress: { deployer }, isValidString: { deploymentSalt } });

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
            // const { destinationChain, env } = options;
            const [destinationChain, manualGasValue] = args;

            const deploymentSalt = getDeploymentSalt(options);

            const gasValue = await estimateITSFee(chain, destinationChain, env, 'InterchainTokenDeployment', manualGasValue, _axelar);

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
                    value: gasValue,
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
            const [tokenAddress, destinationChain, manualGasValue] = args;

            const { /*gasValue,*/ env } = options;
            const gasValue = await estimateITSFee(chain, destinationChain, env, 'InterchainTokenDeployment', manualGasValue, _axelar);

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
                { value: gasValue, ...gasOptions },
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
            const [destinationChain, destinationTokenAddress, tokenManagerType, linkParams, manualGasValue] = args;

            // const { destinationChain, destinationTokenAddress, tokenManagerType, linkParams, env } = options;
            const { env } = options;

            const gasValue = await estimateITSFee(chain, destinationChain, env, 'LinkToken', manualGasValue, _axelar);

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
                { value: gasValue, ...gasOptions },
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
        .command('interchain-token-deploy-salt <deployer>')
        .description('Get interchain token deploy')
        .action((deployer, options, cmd) => {
            main(cmd.name(), [deployer], options);
        });

    // node evm/interchainTokenFactory.js canonical-interchain-token-deploy-salt 0x8A80b16621e4a14Cb98B64Fd2504b8CFe0Bf5AF1 --chainNames ethereum-sepolia  --env testnet --yes
    program
        .command('canonical-interchain-token-deploy-salt <tokenAddress>')
        .description('Get canonical interchain token deploy salt')
        .action((tokenAddress, options, cmd) => {
            main(cmd.name(), [tokenAddress], options);
        });

    // node evm/interchainTokenFactory.js canonical-interchain-token-id 0x8A80b16621e4a14Cb98B64Fd2504b8CFe0Bf5AF1 --chainNames ethereum-sepolia  --env testnet --yes
    program
        .command('canonical-interchain-token-id <tokenAddress>')
        .description('Get canonical interchain token id')
        .action((tokenAddress, options, cmd) => {
            main(cmd.name(), [tokenAddress], options);
        });

    // node evm/interchainTokenFactory.js interchain-token-id 0x312dba807EAE77f01EF3dd21E885052f8F617c5B 0x48d1c8f6106b661dfe16d1ccc0624c463e11e44a838e6b1f00117c5c74a2cd82 --chainNames avalanche --env testnet --yes --salt 0x48d1c8f6106b661dfe16d1ccc0624c463e11e44a838e6b1f00117c5c74a2cd82
    program
        .command('interchain-token-id <deployer>')
        .description('Get interchain token id')
        .action((deployer, options, cmd) => {
            main(cmd.name(), [deployer], options);
        });

    program
        .command('deploy-interchain-token <name> <symbol> <decimals> <initialSupply> <minter>')
        .description('Deploy interchain token')
        .action((name, symbol, decimals, initialSupply, minter, options, cmd) => {
            main(cmd.name(), [name, symbol, decimals, initialSupply, minter], options);
        });

    //  node evm/interchainTokenFactory.js deploy-remote-interchain-token 0x01bc86881a7ce41ac90eaa6eca5e0e63d9a5a218bdafd9996aa2d5aea947ff39 Avalanche 10000000000000000  --chainNames ethereum-sepolia  --env testnet --yes
    // the salt was what i had passed into deployInterchainToken() on etherscan
    program
        .command('deploy-remote-interchain-token <salt> <destinationChain> <gasValue>')
        .description('Deploy remote interchain token')
        .action((salt, destinationChain, gasValue, options, cmd) => {
            main(cmd.name(), [salt, destinationChain, gasValue], options);
        });

    //  node evm/interchainTokenFactory.js register-canonical-interchain-token 0x4a895FB659aAD3082535Aa193886D7501650685b --chainNames ethereum-sepolia --env testnet --yes
    program
        .command('register-canonical-interchain-token <tokenAddress>')
        .description('Register canonical interchain token')
        .action((tokenAddress, options, cmd) => {
            main(cmd.name(), [tokenAddress], options);
        });

    // node evm/interchainTokenFactory.js deploy-remote-canonical-interchain-token 0x4a895FB659aAD3082535Aa193886D7501650685b Avalanche 100000000000 --chainNames ethereum-sepolia --env testnet --yes
    program
        .command('deploy-remote-canonical-interchain-token <tokenAddress> <destinationChain> <gasValue>')
        .description('Deploy remote canonical interchain token')
        .action((tokenAddress, destinationChain, gasValue, options, cmd) => {
            main(cmd.name(), [tokenAddress, destinationChain, gasValue], options);
        });

    //  node evm/interchainTokenFactory.js register-custom-token  0xB98cF318A3cB1DEBA42a5c50c365B887cA00133C 4 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f --chainNames ethereum-sepolia --env testnet --yes --salt 0x3c39e5b65a730b26afa28238de20f2302c2cdb00f614f652274df74c88d4bb40
    program
        .command('register-custom-token <tokenAddress> <tokenManagerType> <operator>')
        .description('Register custom token')
        .action((tokenAddress, tokenManagerType, operator, options, cmd) => {
            main(cmd.name(), [tokenAddress, tokenManagerType, operator], options);
        });

    // node evm/interchainTokenFactory.js link-token Avalanche 0xB98cF318A3cB1DEBA42a5c50c365B887cA00133C 4 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 1000000  --chainNames ethereum-sepolia --env testnet --yes --salt 0x3c39e5b65a730b26afa28238de20f2302c2cdb00f614f652274df74c88d4bb40
    program
        .command('link-token <destinationChain> <destinationTokenAddress> <tokenManagerType> <linkParams> <gasValue>')
        .description('Link token to token on destination chain')
        .action((destinationChain, destinationTokenAddress, tokenManagerType, linkParams, gasValue, options, cmd) => {
            main(cmd.name(), [destinationChain, destinationTokenAddress, tokenManagerType, linkParams, gasValue], options);
        });

    addOptionsToCommands(program, addEvmOptions, { address: true, salt: true });

    program.parse();
}

module.exports = { interchainTokenFactory: main };
