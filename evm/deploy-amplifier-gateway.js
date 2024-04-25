'use strict';

const { Command, Option } = require('commander');
const chalk = require('chalk');
const { ethers } = require('hardhat');
const {
    ContractFactory,
    Contract,
    Wallet,
    utils: { defaultAbiCoder, getContractAddress },
    constants: { HashZero },
    getDefaultProvider,
} = ethers;

const {
    saveConfig,
    getBytecodeHash,
    printInfo,
    getAmplifierKeyAddresses,
    printError,
    printWalletInfo,
    printWarn,
    prompt,
    mainProcessor,
    deployContract,
    getGasOptions,
    isValidAddress,
} = require('./utils');
const { addExtendedOptions } = require('./cli-utils');
const { storeSignedTx, signTransaction, getWallet } = require('./sign-utils.js');

const { WEIGHTED_SIGNERS_TYPE } = require('@axelar-network/axelar-gmp-sdk-solidity/scripts/utils');
const AxelarAmplifierGatewayProxy = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/gateway/AxelarAmplifierGatewayProxy.sol/AxelarAmplifierGatewayProxy.json');
const AxelarAmplifierGateway = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/gateway/AxelarAmplifierGateway.sol/AxelarAmplifierGateway.json');

async function getWeightedSigners(config, chain, options) {
    printInfo(`Retrieving verifier addresses for ${chain.name} from Axelar network`);

    let signers;

    if (isValidAddress(options.keyID)) {
        // set the keyID as the signer for debug deployments
        signers = {
            signers: [
                {
                    signer: options.keyID,
                    weight: 1,
                },
            ],
            threshold: 1,
            nonce: HashZero,
        };
    } else {
        const addresses = getAmplifierKeyAddresses(config, chain.axelarId);
        signers = {
            signers: addresses.addresses.map(({ address, weight }) => ({ signer: address, weight })),
            threshold: addresses.threshold,
            nonce: HashZero, // TODO: set nonce
        };
    }

    return defaultAbiCoder.encode([`${WEIGHTED_SIGNERS_TYPE}[]`], [[signers]]);
}

async function deploy(config, chain, options) {
    const { privateKey, reuseProxy, yes, predictOnly } = options;

    const contractName = 'AxelarGateway';

    const rpc = options.rpc || chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey).connect(provider);
    await printWalletInfo(wallet);

    if (chain.contracts === undefined) {
        chain.contracts = {};
    }

    if (chain.contracts[contractName] === undefined) {
        chain.contracts[contractName] = {};
    }

    const contractConfig = chain.contracts[contractName];
    const owner = options.owner || chain.contracts.InterchainGovernance?.address || wallet.address;

    if (!reuseProxy) {
        if (owner === undefined) {
            throw new Error('owner address is required');
        }

        if (owner !== wallet.address) {
            printWarn(
                'Governance address is not set to the wallet address. This is needed for official deployment and is transferred after deployment',
            );
        }

        printInfo('Owner address', owner);
    }

    const gasOptions = await getGasOptions(chain, options, contractName);

    const gatewayFactory = new ContractFactory(AxelarAmplifierGateway.abi, AxelarAmplifierGateway.bytecode, wallet);

    const deployerContract =
        options.deployMethod === 'create3' ? chain.contracts.Create3Deployer?.address : chain.contracts.ConstAddressDeployer?.address;

    let gateway;
    let proxyAddress;

    if (reuseProxy) {
        proxyAddress = chain.contracts.AxelarGateway?.address;

        if (proxyAddress === undefined) {
            throw new Error('Proxy address is missing in the config file');
        }

        printInfo('Reusing Gateway Proxy address', proxyAddress);
        gateway = gatewayFactory.attach(proxyAddress);
    } else {
        const transactionCount = await wallet.getTransactionCount();
        proxyAddress = getContractAddress({
            from: wallet.address,
            nonce: transactionCount + 1,
        });
        printInfo('Predicted gateway proxy address', proxyAddress, chalk.cyan);
    }

    let existingAddress;

    for (const chainConfig of Object.values(config.chains)) {
        existingAddress = chainConfig.contracts?.[contractName]?.address;

        if (existingAddress !== undefined) {
            break;
        }
    }

    if (existingAddress !== undefined && proxyAddress !== existingAddress) {
        printWarn(`Predicted address ${proxyAddress} does not match existing deployment ${existingAddress} in chain configs.`);
        printWarn('For official deployment, recheck the deployer, salt, args, or contract bytecode.');
    }

    if (predictOnly || prompt(`Does derived address match existing gateway deployments? Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    contractConfig.deployer = wallet.address;
    const domainSeparator = HashZero; // TODO: retrieve domain separator from amplifier / calculate the same way
    const salt = options.salt || '';

    printInfo(`Deploying gateway implementation contract`);
    printInfo('Gateway Implementation args', `${options.previousSignerRetention}, ${domainSeparator}`);
    printInfo('Deploy method', options.deployMethod);
    printInfo('Deploy salt (if not create based deployment)', salt);

    let implementation;

    if (options.skipExisting && contractConfig.implementation) {
        implementation = gatewayFactory.attach(contractConfig.implementation);
    } else {
        const implementationSalt = `${salt} Implementation`;

        implementation = await deployContract(
            options.deployMethod,
            wallet,
            AxelarAmplifierGateway,
            [options.previousSignerRetention, domainSeparator],
            { salt: implementationSalt, deployerContract },
            gasOptions,
            {},
            chain,
        );
    }

    printInfo('Gateway Implementation', implementation.address);

    const implementationCodehash = await getBytecodeHash(implementation, chain.axelarId);
    printInfo('Gateway Implementation codehash', implementationCodehash);

    if (options.skipExisting && contractConfig.address) {
        proxyAddress = chain.contracts.AxelarGateway?.address;
        gateway = gatewayFactory.attach(proxyAddress);
    } else if (!reuseProxy) {
        const params = await getWeightedSigners(config, chain, options);

        printInfo('Deploying gateway proxy contract');
        printInfo('Proxy deployment args', `${implementation.address}, ${params}`);

        const gatewayProxy = await deployContract(
            options.deployMethod,
            wallet,
            AxelarAmplifierGatewayProxy,
            [implementation.address, owner, params],
            { salt, deployerContract },
            gasOptions,
            {},
            chain,
        );

        printInfo('Gateway Proxy', gatewayProxy.address);

        gateway = gatewayFactory.attach(gatewayProxy.address);
    }

    // Verify deployment
    let error = false;

    const ownerAddress = await gateway.owner();

    printInfo(`Existing owner`, ownerAddress);

    if (!reuseProxy && owner !== ownerAddress) {
        printError(`ERROR: Retrieved governance address is different:`);
        printError(`   Actual:   ${ownerAddress}`);
        printError(`   Expected: ${owner}`);
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
    contractConfig.deploymentMethod = options.deployMethod;
    contractConfig.previousSignerRetention = options.previousSignerRetention;
    contractConfig.domainSeparator = domainSeparator;

    if (options.deployMethod !== 'create') {
        contractConfig.salt = salt;
    }

    if (!chain.contracts.InterchainGovernance) {
        chain.contracts.InterchainGovernance = {};
    }

    chain.contracts.InterchainGovernance.address = owner;

    printInfo('Deployment status', 'SUCCESS');

    saveConfig(config, options.env);
}

async function upgrade(_, chain, options) {
    const { privateKey, yes, offline, env, predictOnly } = options;
    const contractName = 'AxelarGateway';
    const chainName = chain.name.toLowerCase();

    const rpc = options.rpc || chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = await getWallet(privateKey, provider, options);

    const { address } = await printWalletInfo(wallet, options);

    const contractConfig = chain.contracts[contractName];

    const gateway = new Contract(contractConfig.address, AxelarAmplifierGateway.abi, wallet);
    let implementationCodehash = contractConfig.implementationCodehash;
    const owner = options.owner || chain.contracts.InterchainGovernance?.address;
    const setupParams = '0x';

    if (!chain.contracts.InterchainGovernance) {
        chain.contracts.InterchainGovernance = {};
    }

    chain.contracts.InterchainGovernance.address = owner;

    if (!offline) {
        const codehash = await getBytecodeHash(contractConfig.implementation, chain.axelarId, provider);

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

    printInfo('Gateway Proxy', gateway.address);

    if (!offline) {
        printInfo('Current implementation', await gateway.implementation());
    }

    printInfo('Upgrading to implementation', contractConfig.implementation);
    printInfo('New Implementation codehash', implementationCodehash);
    printInfo('Owner', owner);
    printInfo('Setup params', setupParams);

    const gasOptions = await getGasOptions(chain, options, contractName);

    if (predictOnly || prompt(`Proceed with an upgrade on ${chain.name}?`, yes)) {
        return;
    }

    const tx = await gateway.populateTransaction.upgrade(contractConfig.implementation, implementationCodehash, setupParams, gasOptions);

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

    program.name('deploy-amplifier-gateway').description('Deploy Amplifier Gateway');

    // use create3 as default deploy method
    addExtendedOptions(program, { salt: true, deployMethod: 'create3', skipExisting: true, upgrade: true, predictOnly: true });

    program.addOption(new Option('-r, --rpc <rpc>', 'chain rpc url').env('URL'));
    program.addOption(new Option('--previousSignerRetention <previousSignerRetention>', 'previous signer retention').default(15));

    program.addOption(new Option('--reuseProxy', 'reuse proxy contract modules for new implementation deployment'));
    program.addOption(new Option('--ignoreError', 'Ignore deployment errors and proceed to next chain'));
    program.addOption(new Option('--owner <owner>', 'owner/governance address').env('OWNER'));
    program.addOption(new Option('--keyID <keyID>', 'use the specified key ID address instead of the querying the chain').env('KEY_ID'));
    program.addOption(new Option('--offline', 'Run in offline mode'));
    program.addOption(new Option('--nonceOffset <nonceOffset>', 'The value to add in local nonce if it deviates from actual wallet nonce'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

if (require.main === module) {
    programHandler();
}

module.exports = {
    deployAmplifierGateway: deploy,
};
