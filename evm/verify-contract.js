'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    getContractAt,
    getContractFactory,
    utils: { defaultAbiCoder },
} = ethers;
const { Command, Option } = require('commander');
const { verifyContract, getEVMAddresses, loadConfig } = require('./utils');

async function verifyContracts(config, chain, options) {
    const { env, contractName, dir } = options;
    const provider = getDefaultProvider(chain.rpc);
    const wallet = Wallet.createRandom().connect(provider);
    const verifyOptions = {};

    if (dir) {
        verifyOptions.dir = dir;
    }

    switch (contractName) {
        case 'Create3Deployer': {
            const Create3Deployer = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/deploy/Create3Deployer.sol/Create3Deployer.json');

            const contractFactory = await getContractAt(Create3Deployer.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.Create3Deployer.address);

            console.log(`Verifying ${contractName} on ${chain.name} at address ${contract.address}...`);

            await verifyContract(env, chain.name, contract.address, [], verifyOptions);
            break;
        }

        case 'ConstAddressDeployer': {
            const ConstAddressDeployer = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/deploy/ConstAddressDeployer.sol/ConstAddressDeployer.json');

            const contractFactory = await getContractAt(ConstAddressDeployer.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.ConstAddressDeployer.address);

            console.log(`Verifying ${contractName} on ${chain.name} at address ${contract.address}...`);

            await verifyContract(env, chain.name, contract.address, [], verifyOptions);
            break;
        }

        case 'CreateDeployer': {
            const CreateDeployer = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/deploy/Create3.sol/CreateDeployer.json');

            const contractFactory = await getContractAt(CreateDeployer.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.CreateDeployer.address);

            console.log(`Verifying ${contractName} on ${chain.name} at address ${contract.address}...`);

            await verifyContract(env, chain.name, contract.address, [], verifyOptions);
            break;
        }

        case 'Operators': {
            const Operators = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/utils/Operators.sol/Operators.json');

            const contractFactory = await getContractAt(Operators.abi, wallet);

            const contract = contractFactory.attach(options.address || chain.contracts.Operators.address);

            console.log(`Verifying ${contractName} on ${chain.name} at address ${contract.address}...`);

            await verifyContract(env, chain.name, contract.address, [chain.contracts.Operators.owner], verifyOptions);
            break;
        }

        case 'AxelarGateway': {
            const gatewayFactory = await getContractFactory('AxelarGateway', wallet);
            const gateway = gatewayFactory.attach(options.address || chain.contracts.AxelarGateway.address);

            const implementation = await gateway.implementation();

            const auth = await gateway.authModule();
            const tokenDeployer = await gateway.tokenDeployer();

            // Assume setup params corresponds to epoch 1
            const admins = await gateway.admins(1);
            const adminThreshold = await gateway.adminThreshold(1);
            const setupParams = defaultAbiCoder.encode(['address[]', 'uint8', 'bytes'], [admins, adminThreshold, '0x']);

            const { addresses, weights, threshold } = await getEVMAddresses(config, chain.id, { keyID: `evm-${chain.id}-genesis` });
            const authParams = [defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [addresses, weights, threshold])];

            console.log(`Verifying ${contractName} on ${chain.name} at address ${gateway.address}...`);

            await verifyContract(env, chain.name, auth, [authParams], verifyOptions);
            await verifyContract(env, chain.name, tokenDeployer, [], verifyOptions);
            await verifyContract(env, chain.name, implementation, [auth, tokenDeployer], verifyOptions);
            await verifyContract(env, chain.name, gateway.address, [implementation, setupParams], verifyOptions);

            break;
        }

        case 'AxelarGasService': {
            const gasServiceFactory = await getContractFactory(contractName, wallet);
            const contractConfig = chain.contracts[contractName];
            const gasService = gasServiceFactory.attach(options.address || contractConfig.address);

            const implementation = await gasService.implementation();
            await verifyContract(env, chain.name, implementation, [contractConfig.collector], verifyOptions);
            await verifyContract(env, chain.name, gasService.address, [], verifyOptions);
            break;
        }

        case 'AxelarDepositService': {
            const depositServiceFactory = await getContractFactory(contractName, wallet);
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
            const token = await getContractFactory('BurnableMintableCappedERC20', wallet);
            const symbol = options.args;

            console.log(`Verifying ${symbol}...`);

            const gatewayFactory = await getContractFactory('AxelarGateway', wallet);
            const gateway = gatewayFactory.attach(chain.contracts.AxelarGateway.address);

            const tokenAddress = await gateway.tokenAddresses(symbol);
            const tokenContract = token.attach(tokenAddress);
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
    const config = loadConfig(options.env);

    let chains = options.chainNames.split(',').map((str) => str.trim());

    if (options.chainNames === 'all') {
        chains = Object.keys(config.chains);
    }

    for (const chainName of chains) {
        if (config.chains[chainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chainName} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];

        try {
            await verifyContracts(config, chain, options);
        } catch (e) {
            console.log(`FAILED VERIFICATION: ${e}`);
        }
    }
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
    program.addOption(new Option('-c, --contractName <contractName>', 'contract name'));
    program.addOption(new Option('-a, --address <address>', 'contract address'));
    program.addOption(new Option('-d, --dir <dir>', 'contract artifacts dir'));
    program.addOption(new Option('--args <args>', 'contract args'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
