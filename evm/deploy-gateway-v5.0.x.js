'use strict';

require('dotenv').config();

const {
    saveConfig,
    loadConfig,
    getBytecodeHash,
    verifyContract,
    printInfo,
    getProxy,
    getEVMAddresses,
    httpGet,
    printError,
    printWalletInfo,
    printWarn,
} = require('./utils');
const { ethers } = require('hardhat');
const {
    ContractFactory,
    Contract,
    Wallet,
    utils: { defaultAbiCoder, getContractAddress, AddressZero },
    getDefaultProvider,
} = ethers;
const readlineSync = require('readline-sync');
const { Command, Option } = require('commander');
const chalk = require('chalk');

const AxelarGatewayProxy = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/AxelarGatewayProxy.sol/AxelarGatewayProxy.json');
const AxelarGateway = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/AxelarGateway.sol/AxelarGateway.json');
const AxelarAuthWeighted = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/auth/AxelarAuthWeighted.sol/AxelarAuthWeighted.json');
const TokenDeployer = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/TokenDeployer.sol/TokenDeployer.json');

async function getAuthParams(config, chain, options) {
    printInfo('Retrieving auth key');

    if (!options.amplifier) {
        // check if key rotation is in progress
        try {
            const resp = await httpGet(`${config.axelar.lcd}/axelar/multisig/v1beta1/next_key_id/${chain}`);
            throw new Error(`Key rotation is in progress for ${chain.name}: ${resp}`);
        } catch (err) {}
    }

    const params = [];

    if (options.prevKeyIDs) {
        for (const keyID of options.prevKeyIDs.split(',')) {
            const { addresses, weights, threshold } = await getEVMAddresses(config, chain, { ...options, keyID });
            printInfo(JSON.stringify({ keyID, addresses, weights, threshold }));
            params.push(defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [addresses, weights, threshold]));
        }
    }

    const { addresses, weights, threshold } = await getEVMAddresses(config, chain, options);
    printInfo(JSON.stringify({ keyID: 'latest', addresses, weights, threshold }));
    params.push(defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [addresses, weights, threshold]));

    return params;
}

function getProxyParams(governance, mintLimiter) {
    return defaultAbiCoder.encode(['address', 'address', 'bytes'], [governance, mintLimiter, '0x']);
}

async function deploy(config, options) {
    const { privateKey, reuseProxy, reuseHelpers, verify, yes } = options;
    const chainName = options.chainName.toLowerCase();

    const contractName = 'AxelarGateway';

    const chain = config.chains[chainName] || { contracts: {}, name: chainName, id: chainName, rpc: options.rpc, tokenSymbol: 'ETH' };
    const rpc = options.rpc || chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey).connect(provider);
    await printWalletInfo(wallet);

    if (chain.contracts[contractName] === undefined) {
        chain.contracts[contractName] = {};
    }

    const contractConfig = chain.contracts[contractName];
    const governance = options.governance || contractConfig.governance;
    const mintLimiter = options.mintLimiter || contractConfig.mintLimiter;

    if (governance === undefined) {
        throw new Error('governance address is required');
    }

    if (mintLimiter === undefined) {
        throw new Error('mintLimiter address is required');
    }

    const transactionCount = await wallet.getTransactionCount();
    const proxyAddress = getContractAddress({
        from: wallet.address,
        nonce: transactionCount + 3,
    });
    printInfo('Predicted proxy address', proxyAddress);

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    printInfo('Gas override', JSON.stringify(gasOptions, null, 2));
    printInfo('Is verification enabled?', verify ? 'y' : 'n');

    const gatewayFactory = new ContractFactory(AxelarGateway.abi, AxelarGateway.bytecode, wallet);
    const authFactory = new ContractFactory(AxelarAuthWeighted.abi, AxelarAuthWeighted.bytecode, wallet);
    const tokenDeployerFactory = new ContractFactory(TokenDeployer.abi, TokenDeployer.bytecode, wallet);
    const gatewayProxyFactory = new ContractFactory(AxelarGatewayProxy.abi, AxelarGatewayProxy.bytecode, wallet);

    let gateway;
    let auth;
    let tokenDeployer;
    const contractsToVerify = [];

    if (!yes) {
        console.log('Does this match any existing deployments?');
        const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
        if (anwser !== 'y') return;
    }

    contractConfig.deployer = wallet.address;

    if (reuseProxy) {
        printInfo('Reusing gateway proxy contract');

        const gatewayProxy = chain.contracts.AxelarGateway?.address || (await getProxy(config, chain.id));
        printInfo('Proxy address', gatewayProxy);
        gateway = gatewayFactory.attach(gatewayProxy);
    }

    if (reuseProxy && reuseHelpers) {
        auth = authFactory.attach(await gateway.authModule());
    } else {
        printInfo(`Deploying auth contract`);

        const params = await getAuthParams(config, chain.id, options);
        printInfo('Auth deployment args', params);

        auth = await authFactory.deploy(params, gasOptions);
        await auth.deployTransaction.wait(chain.confirmations);

        contractsToVerify.push({
            address: auth.address,
            params: [params],
        });
    }

    if (reuseProxy && reuseHelpers) {
        tokenDeployer = tokenDeployerFactory.attach(await gateway.tokenDeployer());
    } else {
        printInfo(`Deploying token deployer contract`);

        tokenDeployer = await tokenDeployerFactory.deploy(gasOptions);
        await tokenDeployer.deployTransaction.wait(chain.confirmations);

        contractsToVerify.push({
            address: tokenDeployer.address,
            params: [],
        });
    }

    printInfo('Auth address', auth.address);
    printInfo('Token Deployer address', tokenDeployer.address);

    printInfo(`Deploying gateway implementation contract`);
    printInfo('Gateway Implementation args', `${auth.address},${tokenDeployer.address}`);

    const implementation = await gatewayFactory.deploy(auth.address, tokenDeployer.address);
    await implementation.deployTransaction.wait(chain.confirmations);

    printInfo('Gateway Implementation', implementation.address);

    const implementationCodehash = await getBytecodeHash(implementation, chainName);
    printInfo('Gateway Implementation codehash', implementationCodehash);

    contractsToVerify.push({
        address: implementation.address,
        params: [auth.address, tokenDeployer.address],
    });

    if (!reuseProxy) {
        const params = getProxyParams(governance, mintLimiter);
        printInfo('Deploying gateway proxy contract');
        printInfo('Proxy deployment args', `${implementation.address},${params}`);

        const gatewayProxy = await gatewayProxyFactory.deploy(implementation.address, params, gasOptions);
        await gatewayProxy.deployTransaction.wait(chain.confirmations);

        printInfo('Gateway Proxy', gatewayProxy.address);

        gateway = gatewayFactory.attach(gatewayProxy.address);

        contractsToVerify.push({
            address: gatewayProxy.address,
            params: [implementation.address, params],
        });
    }

    if (!(reuseProxy && reuseHelpers)) {
        printInfo('Transferring auth ownership');
        await auth.transferOwnership(gateway.address, { gasLimit: 5e6, ...gasOptions }).then((tx) => tx.wait(chain.confirmations));
        printInfo('Transferred auth ownership. All done!');
    }

    let error = false;
    let governanceModule;
    let mintLimiterModule;

    try {
        governanceModule = await gateway.governance();
    } catch (e) {
        // this can fail when upgrading from an older version
        printWarn(`WARN: Failed to retrieve governance address`);
    }

    printInfo(`Existing governance`, governanceModule);

    if (!reuseProxy && governanceModule !== governance) {
        printError(`ERROR: Retrieved governance address is different:`);
        printError(`   Actual:   ${governanceModule}`);
        printError(`   Expected: ${governance}`);
        error = true;
    }

    try {
        mintLimiterModule = await gateway.mintLimiter();
    } catch (e) {
        // this can fail when upgrading from an older version
        printWarn(`WARN: Failed to retrieve mint limiter address`);
    }

    printInfo('Existing mintLimiter', mintLimiterModule);

    if (!reuseProxy && mintLimiterModule !== mintLimiter) {
        printError(`ERROR: Retrieved mintLimiter address is different:`);
        printError(`   Actual:   ${mintLimiterModule}`);
        printError(`   Expected: ${mintLimiter}`);
        error = true;
    }

    const authModule = await gateway.authModule();

    if (!reuseProxy && authModule !== auth.address) {
        printError(`ERROR: Auth module retrieved from gateway ${authModule} doesn't match deployed contract ${auth.address}`);
        error = true;
    }

    const tokenDeployerAddress = await gateway.tokenDeployer();

    if (!reuseProxy && tokenDeployerAddress !== tokenDeployer.address) {
        printError(
            `ERROR: Token deployer retrieved from gateway ${tokenDeployerAddress} doesn't match deployed contract ${tokenDeployer.address}`,
        );
        error = true;
    }

    const authOwner = await auth.owner();

    if (authOwner !== gateway.address) {
        printError(`ERROR: Auth module owner is set to ${authOwner} instead of proxy address ${gateway.address}`);
        error = true;
    }

    const gatewayImplementation = await gateway.implementation();

    if (!reuseProxy && gatewayImplementation !== implementation.address) {
        printError(
            `ERROR: Implementation contract retrieved from gateway ${gatewayImplementation} doesn't match deployed contract ${implementation.address}`,
        );
        error = true;
    }

    if (error) {
        printError('Deployment status', 'FAILED');
        return;
    }

    contractConfig.address = gateway.address;
    contractConfig.implementation = implementation.address;
    contractConfig.authModule = auth.address;
    contractConfig.tokenDeployer = tokenDeployer.address;
    contractConfig.governance = governance;
    contractConfig.mintLimiter = mintLimiter;
    contractConfig.deployer = wallet.address;

    printInfo('Deployment status', 'SUCCESS');

    saveConfig(config, options.env);

    if (verify) {
        // Verify contracts at the end to avoid deployment failures in the middle
        for (const contract of contractsToVerify) {
            await verifyContract(options.env, chain.name, contract.address, contract.params);
        }

        printInfo('Verified all contracts!');
    }
}

async function upgrade(config, options) {
    const { chainName, privateKey, yes } = options;

    const contractName = 'AxelarGateway';

    const chain = config.chains[chainName] || { contracts: {}, name: chainName, id: chainName, rpc: options.rpc, tokenSymbol: 'ETH' };
    const rpc = options.rpc || chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey).connect(provider);
    await printWalletInfo(wallet);

    const contractConfig = chain.contracts[contractName];

    const gateway = new Contract(contractConfig.address, AxelarGateway.abi, wallet);
    const implementationCodehash = await getBytecodeHash(contractConfig.implementation, chainName, provider);
    let governance = options.governance || contractConfig.governance;
    let mintLimiter = options.mintLimiter || contractConfig.mintLimiter;
    let setupParams = '0x';

    if (governance || mintLimiter) {
        governance = governance || AddressZero;
        mintLimiter = mintLimiter || AddressZero;
        setupParams = getProxyParams(governance, mintLimiter);
    }

    printInfo('Chain', chain.name);
    printInfo('Gateway Proxy', gateway.address);
    printInfo('Current implementation', await gateway.implementation());
    printInfo('Upgrading to implementation', contractConfig.implementation);
    printInfo('Implementation codehash', implementationCodehash);
    printInfo('Setup params', setupParams);

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    printInfo('Gas options', JSON.stringify(gasOptions, null, 2));

    if (!yes) {
        console.log('Does this match any existing deployments?');
        const anwser = readlineSync.question(`Proceed with upgrade on ${chain.name}? ${chalk.green('(y/n)')} `);
        if (anwser !== 'y') return;
    }

    const tx = await gateway.upgrade(contractConfig.implementation, implementationCodehash, setupParams, gasOptions);
    printInfo('Upgrade transaction', tx.hash);

    await tx.wait(chain.confirmations);

    const newImplementation = await gateway.implementation();
    printInfo('New implementation', newImplementation);

    if (newImplementation !== contractConfig.implementation) {
        printWarn('Implementation not upgraded yet!');
        return;
    }

    printInfo('Upgraded to', newImplementation);
}

async function main(options) {
    const config = loadConfig(options.env);

    if (!options.upgrade) {
        await deploy(config, options);
    } else {
        await upgrade(config, options);
    }

    saveConfig(config, options.env);
}

async function programHandler() {
    const program = new Command();

    program.name('deploy-gateway-v5.x').description('Deploy gateway v5.x');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainName <chainName>', 'chain name').makeOptionMandatory(true).env('CHAIN'));
    program.addOption(new Option('-r, --rpc <rpc>', 'chain rpc url').env('URL'));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(new Option('-v, --verify', 'verify the deployed contract on the explorer').env('VERIFY'));
    program.addOption(new Option('-r, --reuseProxy', 'reuse proxy contract modules for new implementation deployment').env('REUSE_PROXY'));
    program.addOption(
        new Option('--reuseHelpers', 'reuse helper auth and token deployer contract modules for new implementation deployment').env(
            'REUSE_HELPERS',
        ),
    );
    program.addOption(new Option('-g, --governance <governance>', 'governance address').env('GOVERNANCE'));
    program.addOption(new Option('-m, --mintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'));
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    program.addOption(new Option('-k, --keyID <keyID>', 'key ID').env('KEY_ID'));
    program.addOption(new Option('-a, --amplifier', 'deploy amplifier gateway').env('AMPLIFIER'));
    program.addOption(new Option('--prevKeyIDs <prevKeyIDs>', 'previous key IDs to be used for auth contract'));
    program.addOption(new Option('-u, --upgrade', 'upgrade gateway').env('UPGRADE'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

if (require.main === module) {
    programHandler();
}

module.exports = {
    deployGatewayv5: deploy,
};
