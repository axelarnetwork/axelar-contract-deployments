import { Command, Option } from 'commander';

import { getChainConfig, itsEdgeContract, printInfo, prompt } from '../common';
import { addAmplifierOptions, addOptionalProposalOptions } from './cli-utils';
import { CoordinatorManager } from './coordinator';
import { mainProcessor } from './processor';
import { execute } from './submit-proposal';
import { executeTransaction, getChainTruncationParams, usesGovernanceBypass, validateItsChainChange } from './utils';

const confirmDirectExecution = (options, messages, contractAddress) => {
    printInfo('Contract address', contractAddress);

    const msgs = Array.isArray(messages) ? messages : [messages];
    msgs.forEach((msg, index) => {
        const message = typeof msg === 'string' ? JSON.parse(msg) : msg;
        printInfo(`Message ${index + 1}/${msgs.length}`, JSON.stringify(message, null, 2));
    });

    if (prompt('Proceed with direct execution?', options.yes)) {
        return false;
    }
    return true;
};

const executeDirectly = async (client, config, options, contractAddress, msg, fee) => {
    const msgs = Array.isArray(msg) ? msg : [msg];

    for (let i = 0; i < msgs.length; i++) {
        const msgJson = msgs[i];
        const message = typeof msgJson === 'string' ? JSON.parse(msgJson) : msgJson;

        const { transactionHash } = await executeTransaction(client, contractAddress, message, fee);
        printInfo(`Transaction ${i + 1}/${msgs.length} executed`, transactionHash);
    }
};

const registerItsChain = async (client, config, options, _args, fee) => {
    if (options.itsEdgeContract && options.chains.length > 1) {
        throw new Error('Cannot use --its-edge-contract option with multiple chains.');
    }

    const itsMsgTranslator = options.itsMsgTranslator || config.axelar?.contracts?.ItsAbiTranslator?.address;

    if (!itsMsgTranslator) {
        throw new Error('ItsMsgTranslator address is required for registerItsChain');
    }

    const chains = options.chains.map((chain) => {
        const chainConfig = getChainConfig(config.chains, chain);
        const { maxUintBits, maxDecimalsWhenTruncating } = getChainTruncationParams(config, chainConfig);
        const itsEdgeContractAddress = options.itsEdgeContract || itsEdgeContract(chainConfig);

        return {
            chain: chainConfig.axelarId,
            its_edge_contract: itsEdgeContractAddress,
            msg_translator: itsMsgTranslator,
            truncation: {
                max_uint_bits: maxUintBits,
                max_decimals_when_truncating: maxDecimalsWhenTruncating,
            },
        };
    });

    if (options.update) {
        for (let i = 0; i < options.chains.length; i++) {
            const chain = options.chains[i];
            await validateItsChainChange(client, config, chain, chains[i]);
        }
    }

    const operation = options.update ? 'update' : 'register';
    const msg = `{ "${operation}_chains": { "chains": ${JSON.stringify(chains)} } }`;
    const contractAddress = config.getContractConfig('InterchainTokenService').address;

    if (!contractAddress) {
        throw new Error('InterchainTokenService contract address not found in config');
    }

    if (usesGovernanceBypass(config, 'InterchainTokenService')) {
        if (!confirmDirectExecution(options, [msg], contractAddress)) {
            return;
        }
        return executeDirectly(client, config, options, contractAddress, msg, fee);
    } else {
        if (!options.title || !options.description) {
            throw new Error('Title and description are required for proposal submission');
        }
        return execute(
            client,
            config,
            {
                ...options,
                contractName: 'InterchainTokenService',
                msg,
            },
            undefined,
            fee,
        );
    }
};

const registerProtocol = async (client, config, options, _args, fee) => {
    const serviceRegistry = config.axelar?.contracts?.ServiceRegistry?.address;
    const router = config.axelar?.contracts?.Router?.address;
    const multisig = config.axelar?.contracts?.Multisig?.address;

    const msg = JSON.stringify({
        register_protocol: {
            service_registry_address: serviceRegistry,
            router_address: router,
            multisig_address: multisig,
        },
    });
    const contractAddress = config.getContractConfig('Coordinator').address;

    if (!contractAddress) {
        throw new Error('Coordinator contract address not found in config');
    }

    if (usesGovernanceBypass(config, 'Coordinator')) {
        if (!confirmDirectExecution(options, [msg], contractAddress)) {
            return;
        }
        return executeDirectly(client, config, options, contractAddress, msg, fee);
    } else {
        if (!options.title || !options.description) {
            throw new Error('Title and description are required for proposal submission');
        }
        return execute(client, config, { ...options, contractName: 'Coordinator', msg }, undefined, fee);
    }
};

const registerDeployment = async (client, config, options, _args, fee) => {
    const { chainName } = options;
    const coordinator = new CoordinatorManager(config);
    const message = coordinator.constructRegisterDeploymentMessage(chainName);
    const msg = JSON.stringify(message);
    const contractAddress = config.getContractConfig('Coordinator').address;

    if (!contractAddress) {
        throw new Error('Coordinator contract address not found in config');
    }

    if (usesGovernanceBypass(config, 'Coordinator')) {
        if (!confirmDirectExecution(options, [msg], contractAddress)) {
            return;
        }
        return executeDirectly(client, config, options, contractAddress, msg, fee);
    } else {
        if (!options.title || !options.description) {
            throw new Error('Title and description are required for proposal submission');
        }
        return execute(client, config, { ...options, contractName: 'Coordinator', msg }, undefined, fee);
    }
};

const programHandler = () => {
    const program = new Command();

    program.name('contract').description('Execute contract operations');

    const registerItsChainCmd = program
        .command('its-hub-register-chains')
        .description('Register or update an InterchainTokenService chain')
        .argument('<chains...>', 'list of chains to register or update on InterchainTokenService hub')
        .addOption(
            new Option(
                '--its-msg-translator <itsMsgTranslator>',
                'address for the message translation contract associated with the chain being registered or updated on ITS Hub',
            ),
        )
        .addOption(
            new Option(
                '--its-edge-contract <itsEdgeContract>',
                'address for the ITS edge contract associated with the chain being registered or updated on ITS Hub',
            ),
        )
        .addOption(new Option('--update', 'update existing chain registration instead of registering new chain'))
        .action((chains, options) => {
            options.chains = chains;
            return mainProcessor(registerItsChain, options);
        });
    addAmplifierOptions(registerItsChainCmd, {});
    addOptionalProposalOptions(registerItsChainCmd);

    const registerProtocolCmd = program
        .command('register-protocol-contracts')
        .description('Register the main protocol contracts (e.g. Router)')
        .action((options) => mainProcessor(registerProtocol, options));
    addAmplifierOptions(registerProtocolCmd, {});
    addOptionalProposalOptions(registerProtocolCmd);

    const registerDeploymentCmd = program
        .command('register-deployment')
        .description('Register a deployment')
        .requiredOption('-n, --chainName <chainName>', 'chain name')
        .action((options) => mainProcessor(registerDeployment, options));
    addAmplifierOptions(registerDeploymentCmd, {});
    addOptionalProposalOptions(registerDeploymentCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
