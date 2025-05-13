'use strict';

const { saveConfig, loadConfig, printInfo, getChainConfig, getCurrentVerifierSet } = require('../common/utils');
const { getWallet, addBaseOptions } = require('./utils');
const { Command, Option } = require('commander');
const {
    PostConditionMode,
    AnchorMode,
    broadcastTransaction,
    standardPrincipalCV,
    Cl,
    makeContractCall,
} = require('@stacks/transactions');
const { addOptionsToCommands } = require('../sui/utils');
const { getDomainSeparator, validateParameters } = require('../common');
const { encodeAmplifierVerifiersForStacks } = require('./utils/utils');

const GAS_SERVICE_CMD_OPTIONS = [
    new Option('--gasCollector <gasCollector>', 'the gas collector address'),
];

const GATEWAY_CMD_OPTIONS = [
    new Option('--operator <operator>', 'operator for the gateway'),
    new Option('--minimumRotationDelay <minimumRotationDelay>', 'minium delay for signer rotations (in second)')
        .argParser((val) => parseInt(val))
        .default(24 * 60 * 60),
    new Option('--previousSignerRetention <previousSignerRetention>', 'the number of previous signers that are considered valid')
        .argParser((val) => parseInt(val))
        .default(15),
    new Option(
        '--domainSeparator <domainSeparator>',
        'domain separator (pass in the keccak256 hash value OR "" meaning that its computed locally and checked against deployed StacksMultisigProver)',
    ).default(''),
];

const ITS_CMD_OPTIONS = [
    new Option('--operator <operator>', 'operator for the InterchainTokenService'),
    new Option('--trustedChains <trustedChains>', 'a list of trusted chains separated by , (comma)'),
];

const GOVERNANCE_CMD_OPTIONS = [
    new Option('--governanceChain <governanceChain>', 'the address of the governance chain'),
    new Option('--governanceAddress <governanceAddress>', 'the address of the governance contract on the respective chain'),
];

function getGasServiceFunctionArgs(config, chain, options) {
    validateParameters({
        isNonEmptyString: { gasCollector: options.gasCollector },
    });

    return {
        functionArgs: [
            standardPrincipalCV(options.gasCollector),
        ],
        updateConfigArgs: {
            gasCollector: options.gasCollector,
        },
    };
}

async function getGatewayFunctionArgs(config, chain, options) {
    validateParameters({
        isNonEmptyString: { operator: options.operator },
        isValidNumber: {
            minimumRotationDelay: options.minimumRotationDelay,
            previousSignerRetention: options.previousSignerRetention,
        },
    });

    const { verifierSet, signers } = await getCurrentVerifierSet(config, chain.axelarId, 'StacksMultisigProver');
    const domainSeparator = await getDomainSeparator(config, chain, options, 'StacksMultisigProver');

    const {
        claritySigners,
        weightedSigners,
        threshold,
        nonce,
    } = encodeAmplifierVerifiersForStacks(verifierSet, signers);

    return {
        functionArgs: [
            Cl.bufferFromHex(claritySigners),
            standardPrincipalCV(options.operator),
            Cl.bufferFromHex(domainSeparator),
            Cl.uint(options.minimumRotationDelay),
            Cl.uint(options.previousSignerRetention),
        ],
        updateConfigArgs: {
            signers: {
                weightedSigners,
                threshold,
                nonce,
            },
            claritySigners,
            operator: options.operator,
            domainSeparator,
            minimumRotationDelay: options.minimumRotationDelay,
            previousSignerRetention: options.previousSignerRetention,
        },
    };
}

async function getItsFunctionArgs(config, chain, options) {
    validateParameters({
        isNonEmptyString: { operator: options.operator, trustedChains: options.trustedChains },
    });

    const trustedChains = options.trustedChains.split(',');

    const trustedChainsClarity = trustedChains.map((trustedChain) => Cl.tuple({
        'chain-name': Cl.stringAscii(trustedChain),
        'address': Cl.stringAscii('hub'),
    }));

    const {
        axelar: {
            contracts: {
                InterchainTokenService: { address: itsHubAddress },
            },
            axelarId: itsHubChainName,
        },
    } = config;

    // Add its hub address
    trustedChainsClarity.push(Cl.tuple({
        'chain-name': Cl.stringAscii(itsHubChainName),
        'address': Cl.stringAscii(itsHubAddress),
    }));

    const itsContractAddressName = chain.contracts.InterchainTokenService.address;
    const gasServiceAddress = chain.contracts.GasService.address;

    return {
        functionArgs: [
            Cl.stringAscii(itsContractAddressName),
            Cl.principal(gasServiceAddress),
            standardPrincipalCV(options.operator),
            Cl.list(trustedChainsClarity),
            Cl.stringAscii(itsHubChainName),
            Cl.none(),
        ],
        updateConfigArgs: {
            itsContractAddressName,
            gasServiceAddress,
            operator: options.operator,
            trustedChains,
            itsHubChainName,
            itsHubAddress,
        },
    };
}

function getGovernanceFunctionArgs(config, chain, options) {
    validateParameters({
        isNonEmptyString: { governanceChain: options.governanceChain, governanceAddress: options.governanceAddress },
    });

    return {
        functionArgs: [
            Cl.stringAscii(options.governanceChain),
            Cl.stringAscii(options.governanceAddress),
        ],
        updateConfigArgs: {
            governanceChain: options.governanceChain,
            governanceAddress: options.governanceAddress,
        },
    };
}

const CONTRACT_CONFIGS = {
    cmdOptions: {
        GasService: GAS_SERVICE_CMD_OPTIONS,
        Gateway: GATEWAY_CMD_OPTIONS,
        InterchainTokenService: ITS_CMD_OPTIONS,
        Governance: GOVERNANCE_CMD_OPTIONS,
    },
    preDeployFunctionArgs: {
        GasService: getGasServiceFunctionArgs,
        Gateway: getGatewayFunctionArgs,
        InterchainTokenService: getItsFunctionArgs,
        Governance: getGovernanceFunctionArgs,
    },
};

const addDeployOptions = (program) => {
    // Get the contract name from the program name
    const contractName = program.name();
    // Find the corresponding options for the package
    const cmdOptions = CONTRACT_CONFIGS.cmdOptions[contractName];

    if (cmdOptions) {
        // Add the options to the program
        cmdOptions.forEach((option) => program.addOption(option));
    }

    return program;
};

async function processCommand(contractName, config, chain, options) {
    const { privateKey, stacksAddress, networkType } = await getWallet(chain, options);

    if (!chain.contracts[contractName]?.address) {
        throw new Error(`Contract ${contractName} not yet deployed`);
    }

    printInfo(`Setting up contract ${contractName}`);

    const {
        functionArgs,
        updateConfigArgs,
    } = await CONTRACT_CONFIGS.preDeployFunctionArgs[contractName](config, chain, options);

    const address = chain.contracts[contractName].address.split('.');
    const setupTx = await makeContractCall({
        contractAddress: address[0],
        contractName: address[1],
        functionName: 'setup',
        functionArgs,
        senderKey: privateKey,
        network: networkType,
        postConditionMode: PostConditionMode.Allow,
        anchorMode: AnchorMode.Any,
        fee: 10_000,
    });
    const result = await broadcastTransaction({
        transaction: setupTx,
        network: networkType,
    });

    // Update chain configuration
    chain.contracts[contractName] = {
        ...chain.contracts[contractName],
        ...updateConfigArgs,
    };

    printInfo(`Finished calling setup for contract`, result.txid);
}

async function mainProcessor(contractName, options, processor) {
    const config = loadConfig(options.env);
    const chain = getChainConfig(config, options.chainName);
    await processor(contractName, config, chain, options);
    saveConfig(config, options.env);
}

if (require.main === module) {
    const program = new Command();

    program
        .name('setup-contract')
        .description('Setup a contract');

    const deployContractCmds = Object.keys(CONTRACT_CONFIGS.preDeployFunctionArgs).map((supportedContract) => {
        const command = new Command(supportedContract).description(`Deploy ${supportedContract} contract`);

        return addDeployOptions(command).action((options) => {
            mainProcessor(supportedContract, options, processCommand);
        });
    });

    deployContractCmds.forEach((cmd) => program.addCommand(cmd));

    addOptionsToCommands(program, addBaseOptions);

    program.parse();
}
