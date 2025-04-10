'use strict';

const { Command, Option } = require('commander');
const chalk = require('chalk');
const { ethers } = require('hardhat');
const {
    ContractFactory,
    Contract,
    Wallet,
    utils: { defaultAbiCoder, keccak256 },
    getDefaultProvider,
} = ethers;

const {
    saveConfig,
    getBytecodeHash,
    printInfo,
    printError,
    printWalletInfo,
    printWarn,
    prompt,
    mainProcessor,
    deployContract,
    getGasOptions,
    getWeightedSigners,
    getContractJSON,
    getDeployedAddress,
    getDeployOptions,
    getDomainSeparator,
    isContract,
} = require('./utils');
const { addEvmOptions } = require('./cli-utils');
const { storeSignedTx, signTransaction, getWallet } = require('./sign-utils.js');

const { WEIGHTED_SIGNERS_TYPE, encodeWeightedSigners } = require('@axelar-network/axelar-gmp-sdk-solidity/scripts/utils');
const AxelarAmplifierGatewayProxy = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/gateway/AxelarAmplifierGatewayProxy.sol/AxelarAmplifierGatewayProxy.json');
const AxelarAmplifierGateway = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/gateway/AxelarAmplifierGateway.sol/AxelarAmplifierGateway.json');

async function getSetupParams(config, chain, operator, options) {
    const { signers: signerSets, verifierSetId } = await getWeightedSigners(config, chain, options);
    printInfo('Setup params', JSON.stringify([operator, signerSets], null, 2));
    return { params: defaultAbiCoder.encode([`address`, `${WEIGHTED_SIGNERS_TYPE}[]`], [operator, signerSets]), verifierSetId };
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
                'Owner address is not set to the wallet address. This is needed for official deployment and is transferred after deployment',
            );
        }

        printInfo('Owner address', owner);
    }

    const gasOptions = await getGasOptions(chain, options, contractName);

    const gatewayFactory = new ContractFactory(AxelarAmplifierGateway.abi, AxelarAmplifierGateway.bytecode, wallet);
    const { deployerContract, salt } = getDeployOptions(options.deployMethod, options.salt || 'AxelarAmplifierGateway', chain);

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
        if (options.deployMethod === 'create2') {
            // TODO: support create2 prediction
            printError('create2 prediction is not supported yet');
        }

        proxyAddress = await getDeployedAddress(wallet.address, options.deployMethod, {
            salt,
            deployerContract,
            contractJson: getContractJSON('AxelarAmplifierGatewayProxy'),
            constructorArgs: [], // TODO: populate constructor args for create2 prediction to work
            provider: wallet.provider,
            nonce: (await wallet.getTransactionCount()) + 1,
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
        printWarn('This is NOT required if the deployments are done by different integrators');
    }

    if (await isContract(proxyAddress, wallet.provider)) {
        printError(`Contract already deployed at predicted address "${proxyAddress}"!`);
    }

    if (predictOnly || prompt(`Does derived address match existing gateway deployments? Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    contractConfig.deployer = wallet.address;
    const domainSeparator = await getDomainSeparator(config, chain, options);
    const minimumRotationDelay = Number(options.minimumRotationDelay);

    printInfo(`Deploying gateway implementation contract`);
    printInfo('Gateway Implementation args', `${options.previousSignersRetention}, ${domainSeparator}, ${minimumRotationDelay}`);
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
            [options.previousSignersRetention, domainSeparator, minimumRotationDelay],
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
        proxyAddress = contractConfig?.address;
        gateway = gatewayFactory.attach(proxyAddress);
    } else if (!reuseProxy) {
        const operator = options.operator || contractConfig.operator || wallet.address;
        const { params, verifierSetId } = await getSetupParams(config, chain, operator, options);

        printInfo('Deploying gateway proxy contract');
        printInfo('Proxy deployment args', `${implementation.address}, ${params}`);

        contractConfig.operator = operator;

        const proxyDeploymentArgs = [implementation.address, owner, params];
        contractConfig.proxyDeploymentArgs = proxyDeploymentArgs;
        contractConfig.initialVerifierSetId = verifierSetId;

        const gatewayProxy = await deployContract(
            options.deployMethod,
            wallet,
            AxelarAmplifierGatewayProxy,
            proxyDeploymentArgs,
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
        printError(`ERROR: Retrieved owner address is different:`);
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

    if (Number(options.previousSignersRetention) !== (await gateway.previousSignersRetention()).toNumber()) {
        printError('ERROR: Previous signer retention mismatch');
        error = true;
    }

    if (domainSeparator !== (await gateway.domainSeparator())) {
        printError('ERROR: Domain separator mismatch');
        error = true;
    }

    if (minimumRotationDelay !== (await gateway.minimumRotationDelay()).toNumber()) {
        printError('ERROR: Minimum rotation delay mismatch');
        error = true;
    }

    if (contractConfig.operator !== (await gateway.operator())) {
        printError('ERROR: Operator mismatch');
        error = true;
    }

    if (!reuseProxy) {
        const { signers: signerSets } = await getWeightedSigners(config, chain, options);

        for (let i = 0; i < signerSets.length; i++) {
            const signersHash = keccak256(encodeWeightedSigners(signerSets[i]));
            const epoch = (await gateway.epochBySignersHash(signersHash)).toNumber();
            const signersHashByEpoch = await gateway.signersHashByEpoch(i + 1);

            if (epoch !== i + 1) {
                printError(`ERROR: Epoch mismatch for signer set ${i + 1}`);
                printError(`   Actual:   ${epoch}`);
                printError(`   Expected: ${i + 1}`);
                error = true;
            }

            if (signersHashByEpoch !== signersHash) {
                printError(`ERROR: Signer hash mismatch for signer set ${i + 1}`);
                printError(`   Actual:   ${signersHashByEpoch}`);
                printError(`   Expected: ${signersHash}`);
                error = true;
            }
        }
    }

    if (error) {
        printError('Deployment status', 'FAILED');
        return;
    }

    contractConfig.address = gateway.address;
    contractConfig.implementation = implementation.address;
    contractConfig.implementationCodehash = implementationCodehash;
    contractConfig.deploymentMethod = options.deployMethod;
    contractConfig.previousSignersRetention = options.previousSignersRetention;
    contractConfig.domainSeparator = domainSeparator;
    contractConfig.minimumRotationDelay = minimumRotationDelay;
    contractConfig.connectionType = 'amplifier';
    contractConfig.owner = owner;
    chain.chainType = 'evm';

    if (options.deployMethod !== 'create') {
        contractConfig.salt = salt;
    }

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
    const setupParams = '0x';

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
    addEvmOptions(program, { salt: true, deployMethod: 'create3', skipExisting: true, upgrade: true, predictOnly: true });

    program.addOption(new Option('-r, --rpc <rpc>', 'chain rpc url').env('URL'));
    program.addOption(new Option('--previousSignersRetention <previousSignersRetention>', 'previous signer retention').default(15));
    program.addOption(new Option('--domainSeparator <domainSeparator>', 'domain separator'));
    program.addOption(
        new Option('--minimumRotationDelay <minimumRotationDelay>', 'minium delay for signer rotations').default(24 * 60 * 60),
    ); // 1 day

    program.addOption(new Option('--reuseProxy', 'reuse proxy contract modules for new implementation deployment'));
    program.addOption(new Option('--ignoreError', 'Ignore deployment errors and proceed to next chain'));
    program.addOption(new Option('--owner <owner>', 'owner/governance address').env('OWNER'));
    program.addOption(new Option('--operator <operator>', 'gateway operator address'));
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
