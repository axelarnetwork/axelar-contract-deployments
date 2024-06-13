'use strict';

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    getContractAt,
    getContractFactoryFromArtifact,
    utils: { defaultAbiCoder },
    Contract,
} = ethers;
const { Command, Option } = require('commander');
const { verifyContract, getEVMAddresses, printInfo, printError, mainProcessor, getContractJSON, validateParameters } = require('./utils');
const { addBaseOptions } = require('./cli-utils');
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

    switch (contractName) {
        case 'Create3Deployer': {
            const Create3Deployer = getContractJSON('Create3Deployer');

            const contractFactory = await getContractAt(Create3Deployer.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.Create3Deployer.address);

            await verifyContract(env, chain.name, contract.address, [], verifyOptions);
            break;
        }

        case 'InterchainGovernance': {
            const InterchainGovernance = getContractJSON('InterchainGovernance');

            const contractFactory = await getContractAt(InterchainGovernance.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.InterchainGovernance.address);

            await verifyContract(
                env,
                chain.name,
                contract.address,
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
            const Multisig = getContractJSON('Multisig');

            const contractFactory = await getContractAt(Multisig.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.Multisig.address);

            await verifyContract(env, chain.name, contract.address, [contractConfig.signers, contractConfig.threshold], verifyOptions);
            break;
        }

        case 'InterchainProposalSender': {
            await verifyContract(
                env,
                chain.name,
                options.address || chain.contracts.InterchainProposalSender.address,
                [chain.contracts.AxelarGateway.address, chain.contracts.AxelarGasService.address],
                verifyOptions,
            );
            break;
        }

        case 'ConstAddressDeployer': {
            const ConstAddressDeployer = getContractJSON('ConstAddressDeployer');

            const contractFactory = await getContractAt(ConstAddressDeployer.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.ConstAddressDeployer.address);

            await verifyContract(env, chain.name, contract.address, [], verifyOptions);
            break;
        }

        case 'CreateDeployer': {
            const CreateDeployer = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/deploy/Create3.sol/CreateDeployer.json');

            const contractFactory = await getContractAt(CreateDeployer.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.CreateDeployer.address);

            await verifyContract(env, chain.name, contract.address, [], verifyOptions);
            break;
        }

        case 'Operators': {
            const Operators = getContractJSON('Operators');

            const contractFactory = await getContractAt(Operators.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.Operators.address);

            await verifyContract(env, chain.name, contract.address, [chain.contracts.Operators.owner], verifyOptions);
            break;
        }

        case 'AxelarGateway': {
            const AxelarGateway = getContractJSON('AxelarGateway');
            const gatewayFactory = await getContractFactoryFromArtifact(AxelarGateway, wallet);
            const gateway = gatewayFactory.attach(options.address || chain.contracts.AxelarGateway.address);

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
            await verifyContract(env, chain.name, gateway.address, [implementation, setupParams], verifyOptions);

            break;
        }

        case 'AxelarGasService': {
            const AxelarGasService = getContractJSON('AxelarGasService');
            const gasServiceFactory = await getContractFactoryFromArtifact(AxelarGasService, wallet);
            const contractConfig = chain.contracts[contractName];
            const gasService = gasServiceFactory.attach(options.address || contractConfig.address);

            const implementation = await gasService.implementation();
            await verifyContract(env, chain.name, implementation, [contractConfig.collector], verifyOptions);
            await verifyContract(env, chain.name, gasService.address, [], verifyOptions);
            break;
        }

        case 'AxelarDepositService': {
            const AxelarDepositService = getContractJSON('AxelarDepositService');
            const depositServiceFactory = await getContractFactoryFromArtifact(AxelarDepositService, wallet);
            const contractConfig = chain.contracts[contractName];
            const gasService = depositServiceFactory.attach(options.address || contractConfig.address);

            const implementation = await gasService.implementation();
            await verifyContract(env, chain.name, implementation, [
                chain.contracts.AxelarGateway.address,
                contractConfig.wrappedSymbol,
                contractConfig.refundIssuer,
            ]);
            await verifyContract(env, chain.name, gasService.address, [], verifyOptions);
            break;
        }

        case 'BurnableMintableCappedERC20': {
            const BurnableMintableCappedERC20 = getContractJSON('BurnableMintableCappedERC20');
            const token = await getContractFactoryFromArtifact(BurnableMintableCappedERC20, wallet);
            const symbol = options.args;

            console.log(`Verifying ${symbol}...`);

            const AxelarGateway = getContractJSON('AxelarGateway');
            const gatewayFactory = await getContractFactoryFromArtifact(AxelarGateway, wallet);
            const gateway = gatewayFactory.attach(chain.contracts.AxelarGateway.address);

            const tokenAddress = await gateway.tokenAddresses(symbol);
            const tokenContract = token.attach(options.address || tokenAddress);
            const name = await tokenContract.name();
            const decimals = await tokenContract.decimals();
            const cap = await tokenContract.cap();

            console.log(defaultAbiCoder.encode(['string', 'string', 'uint8', 'uint256'], [name, symbol, decimals, cap]));

            console.log(`Verifying ${name} (${symbol}) decimals ${decimals} on ${chain.name}...`);

            await verifyContract(env, chain.name, tokenContract.address, [name, symbol, decimals, cap], verifyOptions);
            break;
        }

        case 'InterchainTokenService': {
            const InterchainTokenService = getContractJSON('InterchainTokenService');
            const interchainTokenServiceFactory = await getContractFactoryFromArtifact(InterchainTokenService, wallet);
            const interchainTokenService = interchainTokenServiceFactory.attach(
                options.address || chain.contracts.InterchainTokenService.address,
            );
            const contractConfig = chain.contracts[contractName];

            const implementation = await interchainTokenService.implementation();
            const tokenManagerDeployer = await interchainTokenService.tokenManagerDeployer();
            const interchainTokenDeployer = await interchainTokenService.interchainTokenDeployer();
            const interchainTokenDeployerContract = new Contract(
                interchainTokenDeployer,
                getContractJSON('InterchainTokenDeployer').abi,
                wallet,
            );
            const interchainToken = await interchainTokenDeployerContract.implementationAddress();
            const interchainTokenFactory = await interchainTokenService.interchainTokenFactory();
            const interchainTokenFactoryContract = new Contract(
                interchainTokenFactory,
                getContractJSON('InterchainTokenFactory').abi,
                wallet,
            );
            const interchainTokenFactoryImplementation = await interchainTokenFactoryContract.implementation();

            const tokenManager = await interchainTokenService.tokenManager();
            const tokenHandler = await interchainTokenService.tokenHandler();

            const [trustedChains, trustedAddresses] = await getTrustedChainsAndAddresses(config, interchainTokenService);

            const setupParams = defaultAbiCoder.encode(
                ['address', 'string', 'string[]', 'string[]'],
                [contractConfig.deployer, chain.axelarId, trustedChains, trustedAddresses],
            );

            await verifyContract(env, chain.name, tokenManagerDeployer, [], verifyOptions);
            await verifyContract(env, chain.name, interchainToken, [interchainTokenService.address], verifyOptions);
            await verifyContract(env, chain.name, interchainTokenDeployer, [interchainToken], verifyOptions);
            await verifyContract(env, chain.name, tokenManager, [interchainTokenService.address], verifyOptions);
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
            await verifyContract(env, chain.name, interchainTokenFactoryImplementation, [interchainTokenService.address], verifyOptions);
            await verifyContract(
                env,
                chain.name,
                interchainTokenService.address,
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

            const TokenManagerProxy = getContractJSON('TokenManagerProxy');
            const tokenManagerProxyFactory = await getContractFactoryFromArtifact(TokenManagerProxy, wallet);
            const tokenManagerProxy = tokenManagerProxyFactory.attach(tokenManagerAddress);

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
            const AxelarAmplifierGateway = getContractJSON('AxelarAmplifierGateway');
            const amplifierGatewayFactory = await getContractFactoryFromArtifact(AxelarAmplifierGateway, wallet);
            const contractConfig = chain.contracts.AxelarGateway;
            const amplifierGateway = amplifierGatewayFactory.attach(options.address || contractConfig.address);

            const implementation = await amplifierGateway.implementation();
            const previousSignersRetention = (await amplifierGateway.previousSignersRetention()).toNumber();
            const domainSeparator = await amplifierGateway.domainSeparator();
            const minimumRotationDelay = (await amplifierGateway.minimumRotationDelay()).toNumber();

            verifyOptions.contractPath = 'contracts/gateway/AxelarAmplifierGateway.sol:AxelarAmplifierGateway';
            await verifyContract(
                env,
                chain.name,
                implementation,
                [previousSignersRetention, domainSeparator, minimumRotationDelay],
                verifyOptions,
            );

            verifyOptions.contractPath = 'contracts/gateway/AxelarAmplifierGatewayProxy.sol:AxelarAmplifierGatewayProxy';
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
