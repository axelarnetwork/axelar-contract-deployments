const { Command, Option } = require('commander');
const { loadConfig, saveConfig, getChainConfig, printInfo } = require('../common/utils');
const {
    addBaseOptions,
    addOptionsToCommands,
    getWallet,
} = require('./utils');
const {
    makeContractCall,
    PostConditionMode,
    AnchorMode,
    broadcastTransaction,
    Cl,
    fetchCallReadOnlyFunction,
    ResponseOkCV,
    BufferCV,
} = require('@stacks/transactions');
const { getVerificationParams, getTokenTxId } = require('./utils/its-utils');
const { validateParameters } = require('../common');

async function registerTokenManager(privateKey, networkType, chain, args, options) {
    const [contractName] = args;

    const contracts = chain.contracts;
    if (!contracts[contractName]?.address) {
        throw new Error(`Contract ${contractName} not yet deployed`);
    }
    if (!contracts[contractName]?.token) {
        throw new Error(`Contract ${contractName} does not have a token registered yet`);
    }
    if (!contracts.InterchainTokenFactory?.address) {
        throw new Error(`Contract InterchainTokenFactory not yet deployed`);
    }
    if (!contracts.InterchainTokenFactoryImpl?.address) {
        throw new Error(`Contract InterchainTokenFactoryImpl not yet deployed`);
    }
    if (!contracts.GatewayImpl?.address) {
        throw new Error(`Contract GatewayImpl not yet deployed`);
    }
    if (!contracts.GasImpl?.address) {
        throw new Error(`Contract GasImpl not yet deployed`);
    }
    if (!contracts.InterchainTokenServiceImpl?.address) {
        throw new Error(`Contract InterchainTokenServiceImpl not yet deployed`);
    }

    const itsFactoryImplAddress = contracts.InterchainTokenFactoryImpl.address.split('.');
    const res = await fetchCallReadOnlyFunction({
        contractAddress: itsFactoryImplAddress[0],
        contractName: itsFactoryImplAddress[1],
        functionName: 'get-canonical-interchain-token-id',
        functionArgs: [
            Cl.address(contracts.InterchainTokenServiceImpl.address),
            Cl.address(contracts[contractName].token),
        ],
        senderAddress: itsFactoryImplAddress[0],
        network: networkType,
    });
    const interchainTokenId = `0x${res.value.value}`;

    printInfo(`Registering token manager ${contractName} for token ${contracts[contractName].token} on Stacks ITS`);

    const tmTxHash = await getTokenTxId(contracts[contractName].address, chain.rpc);
    const verificationParams = await getVerificationParams(tmTxHash, chain.rpc);

    const itsFactoryAddress = contracts.InterchainTokenFactory.address.split('.');
    const registerTransaction = await makeContractCall({
        contractAddress: itsFactoryAddress[0],
        contractName: itsFactoryAddress[1],
        functionName: 'register-canonical-interchain-token',
        functionArgs: [
            Cl.address(contracts.InterchainTokenFactoryImpl.address),
            Cl.address(contracts.GatewayImpl.address),
            Cl.address(contracts.GasImpl.address),
            Cl.address(contracts.InterchainTokenServiceImpl.address),
            Cl.address(contracts[contractName].token),
            Cl.address(contracts[contractName].address),
            verificationParams,
        ],
        senderKey: privateKey,
        network: networkType,
        postConditionMode: PostConditionMode.Allow,
        anchorMode: AnchorMode.Any,
        fee: 10_000,
    });
    const result = await broadcastTransaction({
        transaction: registerTransaction,
        network: networkType,
    });

    // Update chain configuration
    contracts[contractName] = {
        ...contracts[contractName],
        interchainTokenId,
    };

    printInfo(`Finished registering canonical token with tokenId ${interchainTokenId} for token ${contracts[contractName].token}`, result.txid);
}

async function deployRemoteCanonicalInterchainToken(privateKey, networkType, chain, args, options) {
    validateParameters({
        isNonEmptyString: { destinationChain: options.destinationChain },
        isValidNumber: { gasValue: options.gasValue },
    });

    const [contractName] = args;

    const contracts = chain.contracts;
    if (!contracts[contractName]?.address) {
        throw new Error(`Contract ${contractName} not yet deployed`);
    }
    if (!contracts[contractName]?.token) {
        throw new Error(`Contract ${contractName} does not have a token registered yet`);
    }
    if (!contracts[contractName]?.interchainTokenId) {
        throw new Error(`Contract ${contractName} not yet registered with ITS`);
    }
    if (!contracts.InterchainTokenFactory?.address) {
        throw new Error(`Contract InterchainTokenFactory not yet deployed`);
    }
    if (!contracts.InterchainTokenFactoryImpl?.address) {
        throw new Error(`Contract InterchainTokenFactoryImpl not yet deployed`);
    }
    if (!contracts.GatewayImpl?.address) {
        throw new Error(`Contract GatewayImpl not yet deployed`);
    }
    if (!contracts.GasImpl?.address) {
        throw new Error(`Contract GasImpl not yet deployed`);
    }
    if (!contracts.InterchainTokenServiceImpl?.address) {
        throw new Error(`Contract InterchainTokenServiceImpl not yet deployed`);
    }

    printInfo(`Deploying remote canonical interchain token ${contracts[contractName].token} on destination chain ${options.destinationChain}`);

    const itsFactoryAddress = contracts.InterchainTokenFactory.address.split('.');
    const registerTransaction = await makeContractCall({
        contractAddress: itsFactoryAddress[0],
        contractName: itsFactoryAddress[1],
        functionName: 'deploy-remote-canonical-interchain-token',
        functionArgs: [
            Cl.address(contracts.InterchainTokenFactoryImpl.address),
            Cl.address(contracts.GatewayImpl.address),
            Cl.address(contracts.GasImpl.address),
            Cl.address(contracts.InterchainTokenServiceImpl.address),
            Cl.address(contracts[contractName].token),
            Cl.stringAscii(options.destinationChain),
            Cl.uint(options.gasValue),
        ],
        senderKey: privateKey,
        network: networkType,
        postConditionMode: PostConditionMode.Allow,
        anchorMode: AnchorMode.Any,
        fee: 10_000,
    });
    const result = await broadcastTransaction({
        transaction: registerTransaction,
        network: networkType,
    });

    // Update chain configuration
    contracts[contractName] = {
        ...contracts[contractName],
        remoteChains: [
            ...(contracts[contractName].remoteChains || []),
            options.destinationChain,
        ],
    };

    printInfo(`Finished deploying remote canonical interchain token ${contracts[contractName].token} on destination chain ${options.destinationChain}`, result.txid);
}

async function interchainTransfer(privateKey, networkType, chain, args, options) {
    validateParameters({
        isNonEmptyString: { destinationChain: options.destinationChain, destinationAddress: options.destinationAddress },
        isValidNumber: { value: options.value, gasValue: options.gasValue },
    });

    const [contractName] = args;

    const contracts = chain.contracts;
    if (!contracts[contractName]?.address) {
        throw new Error(`Contract ${contractName} not yet deployed`);
    }
    if (!contracts[contractName]?.token) {
        throw new Error(`Contract ${contractName} does not have a token registered yet`);
    }
    if (!contracts[contractName]?.interchainTokenId) {
        throw new Error(`Contract ${contractName} not yet registered with ITS`);
    }
    if (!contracts.InterchainTokenService?.address) {
        throw new Error(`Contract InterchainTokenService not yet deployed`);
    }
    if (!contracts.GatewayImpl?.address) {
        throw new Error(`Contract GatewayImpl not yet deployed`);
    }
    if (!contracts.GasImpl?.address) {
        throw new Error(`Contract GasImpl not yet deployed`);
    }
    if (!contracts.InterchainTokenServiceImpl?.address) {
        throw new Error(`Contract InterchainTokenServiceImpl not yet deployed`);
    }

    printInfo(`Transferring ${options.value} of token ${contracts[contractName].token} to destination chain ${options.destinationChain} and destination address ${options.destinationAddress}`);

    const itsAddress = contracts.InterchainTokenService.address.split('.');
    const registerTransaction = await makeContractCall({
        contractAddress: itsAddress[0],
        contractName: itsAddress[1],
        functionName: 'interchain-transfer',
        functionArgs: [
            Cl.address(contracts.GatewayImpl.address),
            Cl.address(contracts.GasImpl.address),
            Cl.address(contracts.InterchainTokenServiceImpl.address),
            Cl.address(contracts[contractName].address),
            Cl.address(contracts[contractName].token),
            Cl.bufferFromHex(contracts[contractName].interchainTokenId),
            Cl.stringAscii(options.destinationChain),
            Cl.bufferFromHex(options.destinationAddress),
            Cl.uint(options.value),
            Cl.tuple({ data: Cl.bufferFromHex(''), version: Cl.uint(0) }),
            Cl.uint(options.gasValue),
        ],
        senderKey: privateKey,
        network: networkType,
        postConditionMode: PostConditionMode.Allow,
        anchorMode: AnchorMode.Any,
        fee: 10_000,
    });
    const result = await broadcastTransaction({
        transaction: registerTransaction,
        network: networkType,
    });

    // Update chain configuration
    contracts[contractName] = {
        ...contracts[contractName],
        remoteChains: [
            ...(contracts[contractName].remoteChains || []),
            options.destinationChain,
        ],
    };

    printInfo(`Finished transferring interchain token ${contracts[contractName].token} to destination chain ${options.destinationChain}`, result.txid);
}

async function processCommand(command, chain, args, options) {
    const { privateKey, networkType } = await getWallet(chain, options);

    await command(privateKey, networkType, chain, args, options);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(command, chain, args, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('InterchainTokenService Example').description('Stacks InterchainTokenService Example scripts');

    const registerTokenManagerCmd = new Command()
        .name('register-token-manager')
        .description('Register a token manager for a token on the Stacks ITS')
        .command('register-token-manager <contractName>')
        .action((contractName, options) => {
            mainProcessor(registerTokenManager, options, [contractName], processCommand);
        });

    const deployRemoteCanonicalInterchainTokenCmd = new Command()
        .name('deploy-remote-canonical-interchain-token')
        .description('Deploy a token on another chain')
        .command('deploy-remote-canonical-interchain-token <contractName>')
        .addOption(new Option('--destinationChain <destinationChain>', 'the chain to which to deploy this contract').makeOptionMandatory(true))
        .addOption(new Option('--gasValue <gasValue>', 'the gas value to use when paying cross chain gas').makeOptionMandatory(true))
        .action((contractName, options) => {
            mainProcessor(deployRemoteCanonicalInterchainToken, options, [contractName], processCommand);
        });

    const interchainTransferCmd = new Command()
        .name('interchain-transfer')
        .description('Transfer a token to another chain')
        .command('interchain-transfer <contractName>')
        .addOption(new Option('--destinationChain <destinationChain>', 'the chain to which to deploy this contract').makeOptionMandatory(true))
        .addOption(new Option('--destinationAddress <destinationAddress>', 'the address to transfer to in the destination chain format as hex').makeOptionMandatory(true))
        .addOption(new Option('--value <value>', 'the amount of token to transfer').makeOptionMandatory(true))
        .addOption(new Option('--gasValue <gasValue>', 'the gas value to use when paying cross chain gas').makeOptionMandatory(true))
        .action((contractName, options) => {
            mainProcessor(interchainTransfer, options, [contractName], processCommand);
        });

    program.addCommand(registerTokenManagerCmd);
    program.addCommand(deployRemoteCanonicalInterchainTokenCmd);
    program.addCommand(interchainTransferCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
