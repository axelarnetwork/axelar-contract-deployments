'use strict';

const {
    printObj,
    getBytecodeHash,
    verifyContract,
    printInfo,
    printWarn,
    printError,
    getEVMAddresses,
    saveConfig,
    loadConfig,
    printWalletInfo,
    isAddressArray,
    isNumber,
    prompt,
    getGasOptions,
} = require('./utils');
const { addExtendedOptions } = require('./cli-utils');
const { ethers } = require('hardhat');
const {
    getContractFactory,
    Wallet,
    utils: { defaultAbiCoder, getContractAddress },
    getDefaultProvider,
} = ethers;
const { Command, Option } = require('commander');

async function getAuthParams(config, chain) {
    const { addresses, weights, threshold, keyID } = await getEVMAddresses(config, chain);
    printObj(JSON.stringify({ status: 'latest', keyID, addresses, weights, threshold }));
    const paramsAuth = [defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [addresses, weights, threshold])];
    return paramsAuth;
}

function getProxyParams(adminAddresses, adminThreshold) {
    const admins = JSON.parse(adminAddresses);
    return defaultAbiCoder.encode(['address[]', 'uint8', 'bytes'], [admins, adminThreshold, '0x']);
}

async function deploy(config, options) {
    const { privateKey, skipExisting, adminAddresses, adminThreshold, verify, yes, predictOnly } = options;
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
    const transactionCount = await wallet.getTransactionCount();
    const proxyAddress = getContractAddress({
        from: wallet.address,
        nonce: transactionCount + 3,
    });
    printInfo('Predicted proxy address', proxyAddress);

    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo('Is verification enabled?', verify ? 'y' : 'n');
    printInfo('Skip existing contracts?', skipExisting ? 'y' : 'n');

    const gatewayFactory = await getContractFactory('AxelarGateway', wallet);
    const authFactory = await getContractFactory('AxelarAuthWeighted', wallet);
    const tokenDeployerFactory = await getContractFactory('TokenDeployer', wallet);
    const gatewayProxyFactory = await getContractFactory('AxelarGatewayProxy', wallet);

    if (!adminAddresses || !isAddressArray(JSON.parse(adminAddresses))) {
        printError('Invalid admin addresses', `${adminAddresses}`);
        return;
    }

    if (!adminThreshold || !isNumber(parseInt(adminThreshold))) {
        printError('Invalid admin threshold', `${adminThreshold}`);
        return;
    }

    let gateway;
    let auth;
    let tokenDeployer;
    let implementation;
    const contractsToVerify = [];

    if (predictOnly || prompt(`Does derived address match existing gateway deployments? Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    contractConfig.deployer = wallet.address;

    if (skipExisting && contractConfig.authModule) {
        auth = authFactory.attach(contractConfig.authModule);
    } else {
        printInfo(`Deploying auth contract`);

        const params = await getAuthParams(config, chain.axelarId);
        printInfo('Auth deployment args', params);

        auth = await authFactory.deploy(params, gasOptions);
        await auth.deployTransaction.wait(chain.confirmations);

        contractsToVerify.push({
            address: auth.address,
            params: [params],
        });
    }

    if (skipExisting && contractConfig.tokenDeployer) {
        tokenDeployer = tokenDeployerFactory.attach(contractConfig.tokenDeployer);
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

    if (skipExisting && contractConfig.implementation) {
        implementation = gatewayFactory.attach(contractConfig.implementation);
    } else {
        implementation = await gatewayFactory.deploy(auth.address, tokenDeployer.address, gasOptions);
        await implementation.deployTransaction.wait(chain.confirmations);
    }

    printInfo('Gateway Implementation', implementation.address);

    const implementationCodehash = await getBytecodeHash(implementation, chainName);
    printInfo('Gateway Implementation codehash', implementationCodehash);

    contractsToVerify.push({
        address: implementation.address,
        params: [auth.address, tokenDeployer.address],
    });

    if (skipExisting && contractConfig.address) {
        gateway = gatewayFactory.attach(contractConfig.address);
    } else {
        const params = getProxyParams(adminAddresses, adminThreshold);
        printInfo(`Deploying gateway proxy contract`);
        printInfo(`Proxy deployment args`, `${implementation.address},${params}`);

        const gatewayProxy = await gatewayProxyFactory.deploy(implementation.address, params, gasOptions);
        await gatewayProxy.deployTransaction.wait(chain.confirmations);

        printInfo('Gateway Proxy', gatewayProxy.address);

        gateway = gatewayFactory.attach(gatewayProxy.address);

        contractsToVerify.push({
            address: gatewayProxy.address,
            params: [implementation.address, params],
        });
    }

    if (!(skipExisting && contractConfig.address)) {
        printInfo('Transferring auth ownership');
        await auth.transferOwnership(gateway.address, gasOptions).then((tx) => tx.wait(chain.confirmations));
        printInfo('Transferred auth ownership. All done!');
    }

    var error = false;
    const epoch = await gateway.adminEpoch();
    const admins = `${await gateway.admins(epoch)}`.split(',');
    printInfo(`Existing admins ${admins}`);
    const encodedAdmins = JSON.parse(adminAddresses);

    if (`${admins}` !== `${encodedAdmins}`) {
        printError(`ERROR: Retrieved admins are different:`);
        printError(`   Actual:   ${admins}`);
        printError(`   Expected: ${encodedAdmins}`);
        error = true;
    }

    const authModule = await gateway.authModule();

    if (authModule !== auth.address) {
        printError(`ERROR: Auth module retrieved from gateway ${authModule} doesn't match deployed contract ${auth.address}`);
        error = true;
    }

    const tokenDeployerAddress = await gateway.tokenDeployer();

    if (tokenDeployerAddress !== tokenDeployer.address) {
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

    if (gatewayImplementation !== implementation.address) {
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
    contractConfig.implementationCodehash = implementationCodehash;
    contractConfig.authModule = auth.address;
    contractConfig.tokenDeployer = tokenDeployer.address;

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

    const gatewayFactory = await getContractFactory('AxelarGateway', wallet);
    const gateway = gatewayFactory.attach(contractConfig.address);
    const implementationCodehash = await getBytecodeHash(contractConfig.implementation, chainName, provider);
    const setupParams = '0x';

    printInfo('Gateway Proxy', gateway.address);
    printInfo('Current implementation', await gateway.implementation());
    printInfo('Upgrading to implementation', contractConfig.implementation);
    printInfo('Implementation codehash', implementationCodehash);

    const gasOptions = await getGasOptions(chain, options, contractName);

    if (prompt(`Proceed with upgrade on ${chain.name}?`, yes)) {
        return;
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

    program.name('deploy-gateway-v4.3.x').description('Deploy gateway v4.3.x');

    addExtendedOptions(program, { skipExisting: true, upgrade: true, predictOnly: true });

    program.addOption(new Option('-r, --rpc <rpc>', 'chain rpc url').env('URL'));
    program.addOption(new Option('-a, --adminAddresses <adminAddresses>', 'admin addresses').env('ADMIN_ADDRESSES'));
    program.addOption(new Option('-t, --adminThreshold <adminThreshold>', 'admin threshold').env('ADMIN_THRESHOLD'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

if (require.main === module) {
    programHandler();
}

module.exports = {
    deployGatewayv4: deploy,
};
