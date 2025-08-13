const { Command, Option } = require('commander');
const { loadConfig, saveConfig, getChainConfig, printInfo, encodeITSDestination } = require('../common/utils');
const { addBaseOptions, addOptionsToCommands, getWallet } = require('./utils');
const { Cl } = require('@stacks/transactions');
const { getVerificationParams, getTokenTxId, getCanonicalInterchainTokenId } = require('./utils/its-utils');
const { validateParameters } = require('../common');
const { sendContractCallTransaction } = require('./utils/sign-utils');

async function registerTokenManager(wallet, chain, args, options) {
    const [contractName, token] = args;

    const contracts = chain.contracts;
    if (!contracts[contractName]?.address) {
        throw new Error(`Contract ${contractName} not yet deployed`);
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

    const interchainTokenId = getCanonicalInterchainTokenId(token);

    printInfo(`Registering token manager ${contractName} for token ${token} on Stacks ITS`);

    const tmTxHash = await getTokenTxId(contracts[contractName].address, chain.rpc);
    const verificationParams = await getVerificationParams(tmTxHash, chain.rpc);

    const result = await sendContractCallTransaction(
        contracts.InterchainTokenFactory.address,
        'register-canonical-interchain-token',
        [
            Cl.address(contracts.InterchainTokenFactoryImpl.address),
            Cl.address(contracts.GatewayImpl.address),
            Cl.address(contracts.GasImpl.address),
            Cl.address(contracts.InterchainTokenServiceImpl.address),
            Cl.address(token),
            Cl.address(contracts[contractName].address),
            verificationParams,
        ],
        wallet,
    );

    // Save token to token manager configuration
    chain.contracts[contractName] = {
        ...chain.contracts[contractName],
        token,
    };

    printInfo(`Finished registering canonical token with tokenId ${interchainTokenId} for token ${token}`, result.txid);
}

async function deployRemoteCanonicalInterchainToken(wallet, chain, args, options) {
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

    printInfo(
        `Deploying remote canonical interchain token ${contracts[contractName].token} on destination chain ${options.destinationChain}`,
    );

    const result = await sendContractCallTransaction(
        contracts.InterchainTokenFactory.address,
        'deploy-remote-canonical-interchain-token',
        [
            Cl.address(contracts.InterchainTokenFactoryImpl.address),
            Cl.address(contracts.GatewayImpl.address),
            Cl.address(contracts.GasImpl.address),
            Cl.address(contracts.InterchainTokenServiceImpl.address),
            Cl.address(contracts[contractName].token),
            Cl.stringAscii(options.destinationChain),
            Cl.uint(options.gasValue),
        ],
        wallet,
    );

    printInfo(
        `Finished deploying remote canonical interchain token ${contracts[contractName].token} on destination chain ${options.destinationChain}`,
        result.txid,
    );
}

async function interchainTransfer(wallet, chain, args, options, config) {
    validateParameters({
        isNonEmptyString: {
            destinationChain: options.destinationChain,
            destinationAddress: options.destinationAddress,
        },
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

    const interchainTokenId = options?.interchainTokenId || getCanonicalInterchainTokenId(contracts[contractName].token);

    printInfo(
        `Transferring ${options.value} of token ${contracts[contractName].token} with interchain token id ${interchainTokenId} to destination chain ${options.destinationChain} and destination address ${options.destinationAddress}`,
    );

    const itsDestinationAddress = encodeITSDestination(config.chains, options.destinationChain, options.destinationAddress);
    printInfo('Human-readable destination address', options.destinationAddress);
    printInfo('Encoded ITS destination address', itsDestinationAddress);

    const result = await sendContractCallTransaction(
        contracts.InterchainTokenService.address,
        'interchain-transfer',
        [
            Cl.address(contracts.GatewayImpl.address),
            Cl.address(contracts.GasImpl.address),
            Cl.address(contracts.InterchainTokenServiceImpl.address),
            Cl.address(contracts[contractName].address),
            Cl.address(contracts[contractName].token),
            Cl.bufferFromHex(interchainTokenId),
            Cl.stringAscii(options.destinationChain),
            Cl.bufferFromHex(itsDestinationAddress),
            Cl.uint(options.value),
            Cl.tuple({ data: Cl.bufferFromHex(''), version: Cl.uint(0) }),
            Cl.uint(options.gasValue),
        ],
        wallet,
    );

    printInfo(
        `Finished transferring interchain token ${contracts[contractName].token} to destination chain ${options.destinationChain}`,
        result.txid,
    );
}

async function helloWorld(wallet, chain, args) {
    const [destinationChain, destinationContract, payload, gasValue] = args;

    validateParameters({
        isNonEmptyString: { destinationChain, destinationContract, payload },
        isValidNumber: { gasValue },
    });

    const helloWorldContract = 'HelloWorld';

    const contracts = chain.contracts;
    if (!contracts[helloWorldContract]?.address) {
        throw new Error(`Contract ${helloWorldContract} not yet deployed`);
    }
    if (!contracts.GatewayImpl?.address) {
        throw new Error(`Contract GatewayImpl not yet deployed`);
    }
    if (!contracts.GasImpl?.address) {
        throw new Error(`Contract GasImpl not yet deployed`);
    }

    printInfo(
        `Calling ${helloWorldContract} set-remote-value, destination chain ${destinationChain}, destination contract ${destinationContract}, payload hex ${payload}, gas amount ${gasValue}`,
    );

    const result = await sendContractCallTransaction(
        contracts[helloWorldContract].address,
        'set-remote-value',
        [
            Cl.stringAscii(destinationChain),
            Cl.stringAscii(destinationContract),
            Cl.bufferFromHex(payload),
            Cl.uint(gasValue),
            Cl.address(contracts.GatewayImpl.address),
            Cl.address(contracts.GasImpl.address),
        ],
        wallet,
    );

    printInfo(`Successfully called set-remote-value`, result.txid);
}

async function processCommand(command, chain, args, options, config) {
    const wallet = await getWallet(chain, options);

    await command(wallet, chain, args, options, config);
}

async function mainProcessor(command, options, args, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config.chains, options.chainName);
    await processor(command, chain, args, options, config);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();
    program.name('InterchainTokenService Example').description('Stacks InterchainTokenService Example scripts');

    const registerTokenManagerCmd = new Command()
        .name('register-token-manager')
        .description('Register a token manager for a token on the Stacks ITS')
        .command('register-token-manager <contractName> <token>')
        .action((contractName, token, options) => {
            mainProcessor(registerTokenManager, options, [contractName, token], processCommand);
        });

    const deployRemoteCanonicalInterchainTokenCmd = new Command()
        .name('deploy-remote-canonical-interchain-token')
        .description('Deploy a token on another chain')
        .command('deploy-remote-canonical-interchain-token <contractName>')
        .addOption(
            new Option('--destinationChain <destinationChain>', 'the chain to which to deploy this contract').makeOptionMandatory(true),
        )
        .addOption(new Option('--gasValue <gasValue>', 'the gas value to use when paying cross chain gas').makeOptionMandatory(true))
        .action((contractName, options) => {
            mainProcessor(deployRemoteCanonicalInterchainToken, options, [contractName], processCommand);
        });

    const interchainTransferCmd = new Command()
        .name('interchain-transfer')
        .description('Transfer a token to another chain')
        .command('interchain-transfer <contractName>')
        .addOption(
            new Option('--destinationChain <destinationChain>', 'the chain to which to deploy this contract').makeOptionMandatory(true),
        )
        .addOption(
            new Option(
                '--destinationAddress <destinationAddress>',
                'the address to transfer to in the destination chain format as hex',
            ).makeOptionMandatory(true),
        )
        .addOption(new Option('--value <value>', 'the amount of token to transfer').makeOptionMandatory(true))
        .addOption(new Option('--gasValue <gasValue>', 'the gas value to use when paying cross chain gas').makeOptionMandatory(true))
        .addOption(
            new Option(
                '--interchainTokenId <interchainTokenId>',
                'the interchain token id of the token, defaults to computed canonical interchain token id',
            ),
        )
        .action((contractName, options) => {
            mainProcessor(interchainTransfer, options, [contractName], processCommand);
        });

    const helloWorldCmd = new Command()
        .name('hello-world')
        .description('Call the hello world contract')
        .command('hello-world <destinationChain> <destinationContract> <payload> <gasValue>')
        .action((destinationChain, destinationContract, payload, gasValue, options) => {
            mainProcessor(helloWorld, options, [destinationChain, destinationContract, payload, gasValue], processCommand);
        });

    program.addCommand(registerTokenManagerCmd);
    program.addCommand(deployRemoteCanonicalInterchainTokenCmd);
    program.addCommand(interchainTransferCmd);
    program.addCommand(helloWorldCmd);

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
