'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    getContractAt,
    getContractFactoryFromArtifact,
    utils: { defaultAbiCoder },
} = ethers;
const { Command, Option } = require('commander');
const { verifyContract, getEVMAddresses, printInfo, printError, mainProcessor } = require('./utils');

async function processCommand(config, chain, options) {
    const { env, contractName, dir } = options;
    const provider = getDefaultProvider(chain.rpc);
    const wallet = Wallet.createRandom().connect(provider);
    const verifyOptions = {};

    printInfo('Chain', chain.name);

    if (dir) {
        verifyOptions.dir = dir;
    }

    if (!chain.explorer?.api) {
        printError('Explorer API not found for chain', chain.name);
        return;
    }

    printInfo('Verifying contract', contractName);
    printInfo('Contract address', options.address || chain.contracts[contractName]?.address);

    switch (contractName) {
        case 'Create3Deployer': {
            const Create3Deployer = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/deploy/Create3Deployer.sol/Create3Deployer.json');

            const contractFactory = await getContractAt(Create3Deployer.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.Create3Deployer.address);

            await verifyContract(env, chain.name, contract.address, [], verifyOptions);
            break;
        }

        case 'InterchainGovernance': {
            const InterchainGovernance = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/governance/InterchainGovernance.sol/InterchainGovernance.json');

            const contractFactory = await getContractAt(InterchainGovernance.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.InterchainGovernance.address);

            const contractConfig = chain.contracts[contractName];

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
            const Multisig = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/governance/Multisig.sol/Multisig.json');

            const contractFactory = await getContractAt(Multisig.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.Multisig.address);

            const contractConfig = chain.contracts[contractName];

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
            const ConstAddressDeployer = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/deploy/ConstAddressDeployer.sol/ConstAddressDeployer.json');

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
            const Operators = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/utils/Operators.sol/Operators.json');

            const contractFactory = await getContractAt(Operators.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.Operators.address);

            await verifyContract(env, chain.name, contract.address, [chain.contracts.Operators.owner], verifyOptions);
            break;
        }

        case 'AxelarGateway': {
            const AxelarGateway = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/AxelarGateway.sol/AxelarGateway.json');
            const gatewayFactory = await getContractFactoryFromArtifact(AxelarGateway, wallet);
            const gateway = gatewayFactory.attach(options.address || chain.contracts.AxelarGateway.address);

            const implementation = await gateway.implementation();

            const auth = await gateway.authModule();
            const tokenDeployer = await gateway.tokenDeployer();

            const { addresses, weights, threshold } = await getEVMAddresses(config, chain.id, { keyID: options.args || `evm-${chain.id.toLowerCase()}-genesis` });
            const authParams = [defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [addresses, weights, threshold])];

            await verifyContract(env, chain.name, auth, [authParams], verifyOptions);
            await verifyContract(env, chain.name, tokenDeployer, [], verifyOptions);
            await verifyContract(env, chain.name, implementation, [auth, tokenDeployer], verifyOptions);
            await verifyContract(env, chain.name, gateway.address, [implementation, options.constructorArgs], verifyOptions);

            break;
        }

        case 'AxelarGasService': {
            const AxelarGasService = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/gas-service/AxelarGasService.sol/AxelarGasService.json');
            const gasServiceFactory = await getContractFactoryFromArtifact(AxelarGasService, wallet);
            const contractConfig = chain.contracts[contractName];
            const gasService = gasServiceFactory.attach(options.address || contractConfig.address);

            const implementation = await gasService.implementation();
            await verifyContract(env, chain.name, implementation, [contractConfig.collector], verifyOptions);
            await verifyContract(env, chain.name, gasService.address, [], verifyOptions);
            break;
        }

        case 'AxelarDepositService': {
            const AxelarDepositService = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/deposit-service/AxelarDepositService.sol/AxelarDepositService.json');
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
            const BurnableMintableCappedERC20 = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/BurnableMintableCappedERC20.sol/BurnableMintableCappedERC20.json');
            const token = await getContractFactoryFromArtifact(BurnableMintableCappedERC20, wallet);
            const symbol = options.args;

            console.log(`Verifying ${symbol}...`);

            const AxelarGateway = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/AxelarGateway.sol/AxelarGateway.json');
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

    program.name('balances').description('Display balance of the wallet on specified chains.');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
    program.addOption(new Option('--skipChains <skipChains>', 'skip chains'));
    program.addOption(new Option('-c, --contractName <contractName>', 'contract name'));
    program.addOption(new Option('-a, --address <address>', 'contract address'));
    program.addOption(new Option('-d, --dir <dir>', 'contract artifacts dir'));
    program.addOption(new Option('--args <args>', 'contract args'));
    program.addOption(new Option('--constructorArgs <constructorArgs>', 'contract constructor args'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
