'use strict';

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    getContractFactoryFromArtifact,
    utils: { defaultAbiCoder },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const { verifyContract, getEVMAddresses, printInfo, printError, mainProcessor, getContractJSON, validateParameters } = require('./utils');
const { addBaseOptions } = require('../common');
const { getTrustedChainsAndAddresses } = require('./its');

async function processCommand(config, chain, options) {
    const { env, contractName, dir } = options;
    const provider = getDefaultProvider(chain.rpc);
    const wallet = Wallet.createRandom().connect(provider);
    const verifyOptions = {};

    if (dir) {
        verifyOptions.dir = dir;
    }

    if (!chain.explorer?.api) {
        printError('Explorer API not found for chain', chain.name);
        return;
    }

    printInfo('Verifying contract', contractName);
    printInfo('Contract address', options.address || chain.contracts[contractName]?.address);

    const contractConfig = chain.contracts[contractName];
    const contractAddress = options.address || contractConfig.address;
    const contractJson = getContractJSON(contractName);
    const contractFactory = await getContractFactoryFromArtifact(contractJson, wallet);

    switch (contractName) {
        case 'Create3Deployer': {
            await verifyContract(env, chain.name, contractAddress, [], verifyOptions);
            break;
        }

        case 'InterchainGovernance': {
            await verifyContract(
                env,
                chain.name,
                contractAddress,
                [
                    chain.contracts.AxelarGateway.address,
                    contractConfig.governanceChain,
                    contractConfig.governanceAddress,
                    contractConfig.minimumTimeDelay,
                ],
                verifyOptions,
            );
            break;
        }

        case 'Multisig': {
            await verifyContract(env, chain.name, contractAddress, [contractConfig.signers, contractConfig.threshold], verifyOptions);
            break;
        }

        case 'InterchainProposalSender': {
            await verifyContract(
                env,
                chain.name,
                contractAddress,
                [chain.contracts.AxelarGateway.address, chain.contracts.AxelarGasService.address],
                verifyOptions,
            );
            break;
        }

        case 'ConstAddressDeployer': {
            await verifyContract(env, chain.name, contractAddress, [], verifyOptions);
            break;
        }

        case 'CreateDeployer': {
            await verifyContract(env, chain.name, contractAddress, [], verifyOptions);
            break;
        }

        case 'Operators': {
            await verifyContract(env, chain.name, contractAddress, [contractConfig.owner], verifyOptions);
            break;
        }

        case 'AxelarGateway': {
            const gateway = contractFactory.attach(contractAddress);
            const implementation = await gateway.implementation();
            const auth = await gateway.authModule();
            const tokenDeployer = await gateway.tokenDeployer();

            const { addresses, weights, threshold } = await getEVMAddresses(config, chain.axelarId, {
                keyID: chain.contracts.AxelarGateway.startingKeyIDs[0] || options.args || `evm-${chain.axelarId.toLowerCase()}-genesis`,
            });
            const authParams = [defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [addresses, weights, threshold])];
            const setupParams = defaultAbiCoder.encode(
                ['address', 'address', 'bytes'],
                [contractConfig.deployer, contractConfig.deployer, '0x'],
            );

            await verifyContract(env, chain.name, auth, [authParams], verifyOptions);
            await verifyContract(env, chain.name, tokenDeployer, [], verifyOptions);
            await verifyContract(env, chain.name, implementation, [auth, tokenDeployer], verifyOptions);
            await verifyContract(env, chain.name, contractAddress, [implementation, setupParams], verifyOptions);

            break;
        }

        case 'AxelarGasService': {
            const gasService = contractFactory.attach(contractAddress);
            const implementation = await gasService.implementation();

            await verifyContract(env, chain.name, implementation, [contractConfig.collector], verifyOptions);
            await verifyContract(env, chain.name, contractAddress, [], verifyOptions);
            break;
        }

        case 'AxelarDepositService': {
            const depositService = contractFactory.attach(contractAddress);
            const implementation = await depositService.implementation();

            await verifyContract(env, chain.name, implementation, [
                chain.contracts.AxelarGateway.address,
                contractConfig.wrappedSymbol,
                contractConfig.refundIssuer,
            ]);
            await verifyContract(env, chain.name, contractAddress, [], verifyOptions);
            break;
        }

        case 'BurnableMintableCappedERC20': {
            const symbol = options.args;

            printInfo(`Verifying ${symbol}...`);

            const AxelarGateway = getContractJSON('AxelarGateway');
            const gatewayFactory = await getContractFactoryFromArtifact(AxelarGateway, wallet);
            const gateway = gatewayFactory.attach(chain.contracts.AxelarGateway.address);

            const tokenAddress = await gateway.tokenAddresses(symbol);
            const tokenContract = contractFactory.attach(options.address || tokenAddress);
            const name = await tokenContract.name();
            const decimals = await tokenContract.decimals();
            const cap = await tokenContract.cap();

            printInfo(defaultAbiCoder.encode(['string', 'string', 'uint8', 'uint256'], [name, symbol, decimals, cap]));

            printInfo(`Verifying ${name} (${symbol}) decimals ${decimals} on ${chain.name}...`);

            await verifyContract(env, chain.name, tokenContract.address, [name, symbol, decimals, cap], verifyOptions);
            break;
        }

        case 'InterchainTokenService': {
            const its = contractFactory.attach(contractAddress);
            const implementation = await its.implementation();
            const tokenManagerDeployer = await its.tokenManagerDeployer();
            const interchainTokenDeployer = await its.interchainTokenDeployer();
            const interchainTokenDeployerContract = new Contract(
                interchainTokenDeployer,
                getContractJSON('InterchainTokenDeployer').abi,
                wallet,
            );
            const interchainToken = await interchainTokenDeployerContract.implementationAddress();
            const interchainTokenFactory = await its.interchainTokenFactory();
            const interchainTokenFactoryContract = new Contract(
                interchainTokenFactory,
                getContractJSON('InterchainTokenFactory').abi,
                wallet,
            );
            const interchainTokenFactoryImplementation = await interchainTokenFactoryContract.implementation();

            const tokenManager = await its.tokenManager();
            const tokenHandler = await its.tokenHandler();

            const [trustedChains, trustedAddresses] = await getTrustedChainsAndAddresses(config, its);

            const setupParams = defaultAbiCoder.encode(
                ['address', 'string', 'string[]', 'string[]'],
                [contractConfig.deployer, chain.axelarId, trustedChains, trustedAddresses],
            );

            await verifyContract(env, chain.name, tokenManagerDeployer, [], verifyOptions);
            await verifyContract(env, chain.name, interchainToken, [contractAddress], verifyOptions);
            await verifyContract(env, chain.name, interchainTokenDeployer, [interchainToken], verifyOptions);
            await verifyContract(env, chain.name, tokenManager, [contractAddress], verifyOptions);
            await verifyContract(env, chain.name, tokenHandler, [], verifyOptions);
            await verifyContract(
                env,
                chain.name,
                implementation,
                [
                    tokenManagerDeployer,
                    interchainTokenDeployer,
                    chain.contracts.AxelarGateway.address,
                    chain.contracts.AxelarGasService.address,
                    interchainTokenFactory,
                    chain.axelarId,
                    tokenManager,
                    tokenHandler,
                ],
                verifyOptions,
            );
            await verifyContract(env, chain.name, interchainTokenFactoryImplementation, [contractAddress], verifyOptions);
            await verifyContract(
                env,
                chain.name,
                contractAddress,
                [implementation, chain.contracts.InterchainTokenService.deployer, setupParams],
                {
                    ...verifyOptions,
                    contractPath: 'contracts/proxies/InterchainProxy.sol:InterchainProxy',
                },
            );
            await verifyContract(
                env,
                chain.name,
                interchainTokenFactory,
                [interchainTokenFactoryImplementation, chain.contracts.InterchainTokenFactory.deployer, '0x'],
                {
                    ...verifyOptions,
                    contractPath: 'contracts/proxies/InterchainProxy.sol:InterchainProxy',
                },
            );

            break;
        }

        case 'TokenManagerProxy': {
            const { tokenId } = options;

            const minter = options.minter || '0x';

            validateParameters({ isValidTokenId: { tokenId }, isValidCalldata: { minter } });

            const InterchainTokenService = getContractJSON('InterchainTokenService');
            const interchainTokenServiceFactory = await getContractFactoryFromArtifact(InterchainTokenService, wallet);
            const interchainTokenService = interchainTokenServiceFactory.attach(
                options.address || chain.contracts.InterchainTokenService.address,
            );

            const tokenManagerAddress = await interchainTokenService.tokenManagerAddress(tokenId);
            const tokenManagerProxy = contractFactory.attach(tokenManagerAddress);

            const [implementationType, tokenAddress] = await tokenManagerProxy.getImplementationTypeAndTokenAddress();
            const params = defaultAbiCoder.encode(['bytes', 'address'], [minter, tokenAddress]);

            await verifyContract(
                env,
                chain.name,
                tokenManagerAddress,
                [interchainTokenService.address, implementationType, tokenId, params],
                {
                    ...verifyOptions,
                    contractPath: 'contracts/proxies/TokenManagerProxy.sol:TokenManagerProxy',
                },
            );

            break;
        }

        case 'AxelarAmplifierGateway': {
            const contractConfig = chain.contracts.AxelarGateway;
            const amplifierGateway = contractFactory.attach(options.address || contractConfig.address);

            const implementation = await amplifierGateway.implementation();
            const previousSignersRetention = (await amplifierGateway.previousSignersRetention()).toNumber();
            const domainSeparator = await amplifierGateway.domainSeparator();
            const minimumRotationDelay = (await amplifierGateway.minimumRotationDelay()).toNumber();

            await verifyContract(
                env,
                chain.name,
                implementation,
                [previousSignersRetention, domainSeparator, minimumRotationDelay],
                verifyOptions,
            );

            await verifyContract(env, chain.name, amplifierGateway.address, contractConfig.proxyDeploymentArgs, verifyOptions);

            break;
        }

        default: {
            throw new Error(`Contract ${contractName} is not supported`);
        }
    }
}

async function main(options) {
    await mainProcessor(options, processCommand, false, true);
}

if (require.main === module) {
    const program = new Command();

    program.name('verify-contract').description('Verify selected contracts on specified chains.');

    addBaseOptions(program, { ignorePrivateKey: true, address: true });

    program.addOption(new Option('-c, --contractName <contractName>', 'contract name'));
    program.addOption(new Option('-d, --dir <dir>', 'contract artifacts dir'));
    program.addOption(new Option('--args <args>', 'contract args'));
    program.addOption(new Option('--constructorArgs <constructorArgs>', 'contract constructor args'));
    program.addOption(new Option('--minter <minter>', 'interchain token minter address'));
    program.addOption(new Option('--tokenId <tokenId>', 'interchain token tokenId'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
