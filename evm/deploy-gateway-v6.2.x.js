'use strict';

require('dotenv').config();

const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    ContractFactory,
    Contract,
    Wallet,
    utils: { defaultAbiCoder, getContractAddress, AddressZero },
    getDefaultProvider,
} = ethers;

const {
    saveConfig,
    getBytecodeHash,
    verifyContract,
    printInfo,
    getProxy,
    getEVMAddresses,
    httpGet,
    printError,
    printWalletInfo,
    printWarn,
    prompt,
    mainProcessor,
    isContract,
    deployContract,
} = require('./utils');
const { storeSignedTx, signTransaction, getWallet } = require('./sign-utils.js');

const AxelarGatewayProxy = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/AxelarGatewayProxy.sol/AxelarGatewayProxy.json');
const AxelarGateway = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/AxelarGateway.sol/AxelarGateway.json');
const AxelarAuthWeighted = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/auth/AxelarAuthWeighted.sol/AxelarAuthWeighted.json');
const TokenDeployer = require('@axelar-network/axelar-cgp-solidity/artifacts/contracts/TokenDeployer.sol/TokenDeployer.json');

async function checkKeyRotation(config, chain) {
    let resp;

    // check if key rotation is in progress
    try {
        resp = await httpGet(`${config.axelar.lcd}/axelar/multisig/v1beta1/next_key_id/${chain}`);
    } catch (err) {
        return;
    }

    throw new Error(`Key rotation is in progress for ${chain.name}: ${resp}`);
}

async function getAuthParams(config, chain, options) {
    printInfo('Retrieving auth key');

    if (!options.amplifier) {
        await checkKeyRotation(config, chain);
    }

    const params = [];
    const keyIDs = [];

    if (options.prevKeyIDs) {
        for (const keyID of options.prevKeyIDs.split(',')) {
            const { addresses, weights, threshold } = await getEVMAddresses(config, chain, { ...options, keyID });
            printInfo(JSON.stringify({ status: 'old', keyID, addresses, weights, threshold }));
            params.push(defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [addresses, weights, threshold]));
            keyIDs.push(keyID);
        }
    }

    const { addresses, weights, threshold, keyID } = await getEVMAddresses(config, chain, options);
    printInfo(JSON.stringify({ status: 'latest', keyID, addresses, weights, threshold }));
    params.push(defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [addresses, weights, threshold]));
    keyIDs.push(keyID);

    return { params, keyIDs };
}

function getProxyParams(governance, mintLimiter) {
    return defaultAbiCoder.encode(['address', 'address', 'bytes'], [governance, mintLimiter, '0x']);
}

async function deploy(config, chain, options) {
    const { env, privateKey, reuseProxy, reuseHelpers, verify, yes } = options;

    const contractName = 'AxelarGateway';

    const rpc = options.rpc || chain.rpc;
    const provider = getDefaultProvider(rpc);

    printInfo('Chain', chain.name);

    const wallet = new Wallet(privateKey).connect(provider);
    await printWalletInfo(wallet);

    if (chain.contracts[contractName] === undefined) {
        chain.contracts[contractName] = {};
    }

    const contractConfig = chain.contracts[contractName];
    const governance = options.governance || contractConfig.governance || chain.contracts.InterchainGovernance?.address;
    const mintLimiter = options.mintLimiter || contractConfig.mintLimiter || chain.contracts.Multisig?.address;

    if (!reuseProxy) {
        if (governance === undefined) {
            throw new Error('governance address is required');
        }

        if (mintLimiter === undefined) {
            throw new Error('mintLimiter address is required');
        }

        if (!(await isContract(governance, provider))) {
            printWarn('governance address is not a contract');
        }

        if (!(await isContract(mintLimiter, provider))) {
            printWarn('mintLimiter address is not a contract');
        }
    }

    const gatewayFactory = new ContractFactory(AxelarGateway.abi, AxelarGateway.bytecode, wallet);
    const authFactory = new ContractFactory(AxelarAuthWeighted.abi, AxelarAuthWeighted.bytecode, wallet);
    const tokenDeployerFactory = new ContractFactory(TokenDeployer.abi, TokenDeployer.bytecode, wallet);
    const gatewayProxyFactory = new ContractFactory(AxelarGatewayProxy.abi, AxelarGatewayProxy.bytecode, wallet);

    const deployerContract =
        options.deployMethod === 'create3' ? chain.contracts.Create3Deployer?.address : chain.contracts.ConstAddressDeployer?.address;

    let gateway;
    let auth;
    let tokenDeployer;
    const contractsToVerify = [];

    if (reuseProxy) {
        const gatewayProxy = chain.contracts.AxelarGateway?.address || (await getProxy(config, chain.id));
        printInfo('Reusing Gateway Proxy address', gatewayProxy);
        gateway = gatewayFactory.attach(gatewayProxy);
    } else {
        const transactionCount = await wallet.getTransactionCount();
        const proxyAddress = getContractAddress({
            from: wallet.address,
            nonce: transactionCount + 3,
        });
        printInfo('Predicted proxy address', proxyAddress);
    }

    const gasOptions = JSON.parse(JSON.stringify(contractConfig.gasOptions || chain.gasOptions || {}));

    // Some chains require a gas adjustment
    if (env === 'mainnet' && !gasOptions.gasPrice && (chain.name === 'Fantom' || chain.name === 'Binance' || chain.name === 'Polygon')) {
        gasOptions.gasPrice = Math.floor((await provider.getGasPrice()) * 1.6);
    }

    printInfo('Gas override', JSON.stringify(gasOptions, null, 2));
    printInfo('Is verification enabled?', verify ? 'y' : 'n');

    if (prompt(`Does derived address match existing gateway deployments? Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    contractConfig.deployer = wallet.address;

    if (reuseProxy && reuseHelpers) {
        auth = authFactory.attach(await gateway.authModule());
    } else {
        printInfo(`Deploying auth contract`);

        const { params, keyIDs } = await getAuthParams(config, chain.id, options);
        printInfo('Auth deployment args', params);

        contractConfig.startingKeyIDs = keyIDs;

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

        const salt = 'TokenDeployer' + (options.salt || '');

        tokenDeployer = await deployContract(
            options.deployMethod !== 'create' ? 'create2' : 'create',
            wallet,
            TokenDeployer,
            [],
            { salt, deployerContract },
            gasOptions,
            {},
            chain,
        );

        contractsToVerify.push({
            address: tokenDeployer.address,
            params: [],
        });
    }

    printInfo('Auth address', auth.address);
    printInfo('Token Deployer address', tokenDeployer.address);

    printInfo(`Deploying gateway implementation contract`);
    printInfo('Gateway Implementation args', `${auth.address},${tokenDeployer.address}`);

    const salt = 'AxelarGateway v6.2' + (options.salt || '');

    const implementation = await deployContract(
        options.deployMethod,
        wallet,
        AxelarGateway,
        [auth.address, tokenDeployer.address],
        { salt, deployerContract },
        gasOptions,
        {},
        chain,
    );

    printInfo('Gateway Implementation', implementation.address);

    const implementationCodehash = await getBytecodeHash(implementation, chain.id);
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
        printWarn(`WARN: Failed to retrieve governance address. Expected when reusing a gateway <v6 proxy`);
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
        printWarn(`WARN: Failed to retrieve mint limiter address. Expected when reusing a gateway <v6 proxy`);
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
    contractConfig.implementationCodehash = implementationCodehash;
    contractConfig.authModule = auth.address;
    contractConfig.tokenDeployer = tokenDeployer.address;
    contractConfig.governance = governance;
    contractConfig.mintLimiter = mintLimiter;
    contractConfig.deployer = wallet.address;
    contractConfig.deploymentMethod = options.deployMethod;

    if (options.deployMethod !== 'create') {
        contractConfig.salt = salt;
    }

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

async function upgrade(_, chain, options) {
    const { privateKey, yes, offline, env } = options;
    const contractName = 'AxelarGateway';
    const chainName = chain.name.toLowerCase();

    const rpc = options.rpc || chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = await getWallet(privateKey, provider, options);

    const { address } = await printWalletInfo(wallet, options);

    const contractConfig = chain.contracts[contractName];

    const gateway = new Contract(contractConfig.address, AxelarGateway.abi, wallet);
    let implementationCodehash = contractConfig.implementationCodehash;
    let governance = options.governance || contractConfig.governance || chain.contracts.InterchainGovernance?.address;
    let mintLimiter = options.mintLimiter || contractConfig.mintLimiter || chain.contracts.Multisig?.address;
    let setupParams = '0x';
    contractConfig.governance = governance;
    contractConfig.mintLimiter = mintLimiter;

    if (!offline) {
        if (governance && !(await isContract(governance, provider))) {
            throw new Error('governance address is not a contract');
        }

        if (mintLimiter && !(await isContract(mintLimiter, provider))) {
            throw new Error('mintLimiter address is not a contract');
        }

        const codehash = await getBytecodeHash(contractConfig.implementation, chain.id, provider);

        if (!implementationCodehash) {
            // retrieve codehash dynamically if not specified in the config file
            implementationCodehash = codehash;
        } else if (codehash !== implementationCodehash) {
            throw new Error(
                `Implementation codehash mismatch. Expected ${implementationCodehash} but got ${codehash}. Please check if the implementation contract is deployed correctly.`,
            );
        }
    } else {
        if (!implementationCodehash) {
            throw new Error('Implementation codehash is missing in the config file');
        }
    }

    if (governance || mintLimiter) {
        governance = governance || AddressZero;
        mintLimiter = mintLimiter || AddressZero;
        setupParams = getProxyParams(governance, mintLimiter);
    }

    printInfo('Chain', chain.name);
    printInfo('Gateway Proxy', gateway.address);

    if (!offline) {
        printInfo('Current implementation', await gateway.implementation());
    }

    printInfo('Upgrading to implementation', contractConfig.implementation);
    printInfo('New Implementation codehash', implementationCodehash);
    printInfo('Governance', governance);
    printInfo('Mint limiter', mintLimiter);
    printInfo('Setup params', setupParams);

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    printInfo('Gas options', JSON.stringify(gasOptions, null, 2));

    if (prompt(`Proceed with an upgrade on ${chain.name}?`, yes)) {
        return;
    }

    const tx = await gateway.populateTransaction.upgrade(contractConfig.implementation, implementationCodehash, setupParams);

    const { baseTx, signedTx } = await signTransaction(wallet, chain, tx, options);

    if (offline) {
        const filePath = `./tx/signed-tx-${env}-gateway-upgrade-${chainName}-address-${address}-nonce-${baseTx.nonce}.json`;
        printInfo(`Storing signed Tx offline in file ${filePath}`);

        // Storing the fields in the data that will be stored in file
        const data = {
            msg: `This transaction will upgrade gateway ${gateway.address} to implementation ${contractConfig.implementation} on chain ${chain.name}`,
            unsignedTx: baseTx,
            signedTx,
            status: 'PENDING',
        };

        storeSignedTx(filePath, data);
    } else {
        const newImplementation = await gateway.implementation();
        printInfo('New implementation', newImplementation);

        if (newImplementation !== contractConfig.implementation) {
            printWarn('Implementation not upgraded yet!');
            return;
        }

        printInfo('Upgraded!');
    }
}

async function processCommand(config, chain, options) {
    if (!options.upgrade) {
        await deploy(config, chain, options);
    } else {
        await upgrade(config, chain, options);
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

async function programHandler() {
    const program = new Command();

    program.name('deploy-gateway-v6.2.x').description('Deploy gateway v6.2.x');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chains to run the script over').makeOptionMandatory(true).env('CHAINS'));
    program.addOption(new Option('--skipChains <skipChains>', 'chains to skip over'));
    program.addOption(new Option('-r, --rpc <rpc>', 'chain rpc url').env('URL'));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(
        new Option('--deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create'),
    );
    program.addOption(new Option('-s, --salt <salt>', 'salt to use for deployment method'));
    program.addOption(new Option('-v, --verify', 'verify the deployed contract on the explorer').env('VERIFY'));
    program.addOption(new Option('-r, --reuseProxy', 'reuse proxy contract modules for new implementation deployment').env('REUSE_PROXY'));
    program.addOption(
        new Option('--reuseHelpers', 'reuse helper auth and token deployer contract modules for new implementation deployment').env(
            'REUSE_HELPERS',
        ),
    );
    program.addOption(new Option('--ignoreError', 'Ignore deployment errors and proceed to next chain'));
    program.addOption(new Option('-g, --governance <governance>', 'governance address').env('GOVERNANCE'));
    program.addOption(new Option('-m, --mintLimiter <mintLimiter>', 'mint limiter address').env('MINT_LIMITER'));
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    program.addOption(new Option('-k, --keyID <keyID>', 'key ID').env('KEY_ID'));
    program.addOption(new Option('-a, --amplifier', 'deploy amplifier gateway').env('AMPLIFIER'));
    program.addOption(new Option('--prevKeyIDs <prevKeyIDs>', 'previous key IDs to be used for auth contract'));
    program.addOption(new Option('-u, --upgrade', 'upgrade gateway').env('UPGRADE'));
    program.addOption(new Option('--offline', 'Run in offline mode'));
    program.addOption(new Option('--nonceOffset <nonceOffset>', 'The value to add in local nonce if it deviates from actual wallet nonce'));
    program.addOption(new Option('-x, --skipExisting', 'skip existing if contract was already deployed on chain'));

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
