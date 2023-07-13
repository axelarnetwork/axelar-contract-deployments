'use strict';

require('dotenv').config();

const { printObj, writeJSON, getBytecodeHash, verifyContract, printInfo, printLog, getProxy, getEVMAddresses } = require('./utils');
const { ethers } = require('hardhat');
const {
    getContractFactory,
    Wallet,
    utils: { defaultAbiCoder, getContractAddress },
    getDefaultProvider,
} = ethers;
const { Command, Option } = require('commander');
const chalk = require('chalk');

async function getAuthParams(config, chain) {
    printLog('retrieving addresses');
    const { addresses, weights, threshold } = await getEVMAddresses(config, chain);
    printObj(JSON.stringify({ addresses, weights, threshold }));
    const paramsAuth = [defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [addresses, weights, threshold])];
    return paramsAuth;
}

function getProxyParams(adminAddresses, adminThreshold) {
    const admins = JSON.parse(adminAddresses);
    return defaultAbiCoder.encode(['address[]', 'uint8', 'bytes'], [admins, adminThreshold, '0x']);
}

async function deploy(config, options) {
    const { env, chainName, privateKey, reuseProxy, adminAddresses, adminThreshold, verify } = options;

    const contractName = 'AxelarGateway';

    const chain = config.chains[chainName] || { contracts: {}, id: chainName, tokenSymbol: 'ETH' };
    const rpc = options.rpc || chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey).connect(provider);
    printInfo('Deployer address', wallet.address);

    console.log(
        `Deployer has ${(await provider.getBalance(wallet.address)) / 1e18} ${chalk.green(
            chain.tokenSymbol,
        )} and nonce ${await provider.getTransactionCount(wallet.address)} on ${chainName}.`,
    );

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

    const gatewayFactory = await getContractFactory('AxelarGateway', wallet);
    const authFactory = await getContractFactory('AxelarAuthWeighted', wallet);
    const tokenDeployerFactory = await getContractFactory('TokenDeployer', wallet);
    const gatewayProxyFactory = await getContractFactory('AxelarGatewayProxy', wallet);

    var gateway;
    var auth;
    var tokenDeployer;
    var contractsToVerify = [];

    if (reuseProxy) {
        printLog(`reusing gateway proxy contract`);
        const gatewayProxy = chain.contracts.AxelarGateway?.address || (await getProxy(config, chain.id));
        printLog(`proxy address ${gatewayProxy}`);
        gateway = gatewayFactory.attach(gatewayProxy);
    }

    if (reuseProxy) {
        auth = authFactory.attach(await gateway.authModule());
    } else {
        printLog(`deploying auth contract`);
        const params = await getAuthParams(config, chain.id);
        printLog(`auth deployment args: ${params}`);

        auth = await authFactory.deploy(params).then((d) => d.deployed());
        printLog(`deployed auth at address ${auth.address}`);

        contractsToVerify.push({
            address: auth.address,
            params: [params],
        });
    }

    if (reuseProxy) {
        tokenDeployer = tokenDeployerFactory.attach(await gateway.tokenDeployer());
    } else {
        printLog(`deploying token deployer contract`);
        tokenDeployer = await tokenDeployerFactory.deploy().then((d) => d.deployed());
        printLog(`deployed token deployer at address ${tokenDeployer.address}`);

        contractsToVerify.push({
            address: tokenDeployer.address,
            params: [],
        });
    }

    printLog(`deploying gateway implementation contract`);
    printLog(`authModule: ${auth.address}`);
    printLog(`tokenDeployer: ${tokenDeployer.address}`);
    printLog(`implementation deployment args: ${auth.address},${tokenDeployer.address}`);

    const gatewayImplementation = await gatewayFactory.deploy(auth.address, tokenDeployer.address).then((d) => d.deployed());
    printLog(`implementation: ${gatewayImplementation.address}`);
    const implementationCodehash = await getBytecodeHash(gatewayImplementation, chainName);

    printLog(`implementation codehash: ${implementationCodehash}`);

    contractsToVerify.push({
        address: gatewayImplementation.address,
        params: [auth.address, tokenDeployer.address],
    });

    if (!reuseProxy) {
        const params = getProxyParams(adminAddresses, adminThreshold);
        printLog(`deploying gateway proxy contract`);
        printLog(`proxy deployment args: ${gatewayImplementation.address},${params}`);
        const gatewayProxy = await gatewayProxyFactory.deploy(gatewayImplementation.address, params).then((d) => d.deployed());
        printLog(`deployed gateway proxy at address ${gatewayProxy.address}`);
        gateway = gatewayFactory.attach(gatewayProxy.address);

        contractsToVerify.push({
            address: gatewayProxy.address,
            params: [gatewayImplementation.address, params],
        });
    }

    if (!reuseProxy) {
        printLog('transferring auth ownership');
        await auth.transferOwnership(gateway.address, chain.contracts.AxelarGateway?.gasOptions || {}).then((tx) => tx.wait());
        printLog('transferred auth ownership. All done!');
    }

    var error = false;
    const epoch = await gateway.adminEpoch();
    const admins = `${await gateway.admins(epoch)}`.split(',');
    printLog(`Existing admins ${admins}`);
    const encodedAdmins = JSON.parse(adminAddresses);

    if (!reuseProxy && `${admins}` !== `${encodedAdmins}`) {
        printLog(`ERROR: Retrieved admins are different:`);
        printLog(`   Actual:   ${admins}`);
        printLog(`   Expected: ${encodedAdmins}`);
        error = true;
    }

    const authModule = await gateway.authModule();

    if (authModule !== auth.address) {
        printLog(`ERROR: Auth module retrieved from gateway ${authModule} doesn't match deployed contract ${auth.address}`);
        error = true;
    }

    const tokenDeployerAddress = await gateway.tokenDeployer();

    if (tokenDeployerAddress !== tokenDeployer.address) {
        printLog(
            `ERROR: Token deployer retrieved from gateway ${tokenDeployerAddress} doesn't match deployed contract ${tokenDeployer.address}`,
        );
        error = true;
    }

    const authOwner = await auth.owner();

    if (authOwner !== gateway.address) {
        printLog(`ERROR: Auth module owner is set to ${authOwner} instead of proxy address ${gateway.address}`);
        error = true;
    }

    const implementation = await gateway.implementation();

    if (implementation !== gatewayImplementation.address) {
        printLog(
            `ERROR: Implementation contract retrieved from gateway ${implementation} doesn't match deployed contract ${gatewayImplementation.address}`,
        );
        error = true;
    }

    if (error) {
        printLog('Deployment failed!');
        return;
    }

    contractConfig.address = gateway.address;
    contractConfig.implementation = gatewayImplementation.address;
    contractConfig.authModule = auth.address;
    contractConfig.tokenDeployer = tokenDeployer.address;
    contractConfig.deployer = wallet.address;

    printLog(`Deployment completed`);

    if (verify) {
        // Verify contracts at the end to avoid deployment failures in the middle
        for (const contract of contractsToVerify) {
            await verifyContract(options.env, chain, contract.address, contract.params);
        }

        printLog('Verified all contracts!');
    }
}

async function main(options) {
    const config = require(`${__dirname}/../info/${options.env}.json`);

    await deploy(config, options);

    writeJSON(config, `${__dirname}/../info/${options.env}.json`);
}

async function programHandler() {
    const program = new Command();

    program.name('deploy-gateway-v4.3.x').description('Deploy gateway v4.3.x');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainName <chainName>', 'chain name').makeOptionMandatory(true).env('CHAIN'));
    program.addOption(new Option('-r, --rpc <rpc>', 'chain rpc url').env('URL'));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(new Option('-v, --verify', 'verify the deployed contract on the explorer').env('VERIFY'));
    program.addOption(new Option('-r, --reuseProxy', 'reuse proxy contract modules for new implementation deployment').env('REUSE_PROXY'));
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
    deployGatewayv4_3: deploy,
};
