import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { printInfo, validateParameters } from '../common';
import { ConfigManager } from '../common/config';
import { addCoreOptions } from './cli-utils';
import { ClientManager, Options } from './processor';
import { mainProcessor } from './processor';
import { confirmProposalSubmission, submitProposalAndPrint } from './proposal-utils';
import {
    encodeChainStatusRequest,
    encodeSetTransferRateLimitRequest,
    encodeRegisterAssetFeeRequest,
    encodeRegisterControllerRequest,
    encodeDeregisterControllerRequest,
    encodeSetGatewayRequest,
    encodeTransferOperatorshipRequest,
    encodeStartKeygenRequest,
    encodeRotateKeyRequest,
} from './utils';

interface CoreCommandOptions extends Options {
    yes?: boolean;
    title?: string;
    description?: string;
    direct?: boolean;
    [key: string]: unknown;
}

const executeCoreOperation = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    messages: object[],
    fee?: string | StdFee,
    defaultTitle?: string,
    defaultDescription?: string,
): Promise<void> => {
    if (options.direct) {
        // TODO: Implement direct execution with custom registry
        // Direct execution requires registering custom Axelar protobuf types
        // (e.g., ActivateChainRequest, DeactivateChainRequest) in the client's registry.
        throw new Error('Direct execution is not yet supported for core operations. Please submit as a governance proposal.');
    }

    const title = options.title || defaultTitle;
    const description = options.description || defaultDescription || defaultTitle;
    validateParameters({ isNonEmptyString: { title, description } });

    if (!confirmProposalSubmission(options, messages)) {
        return;
    }

    await submitProposalAndPrint(client, config, { ...options, title, description }, messages, fee);
};

// ============================================
// Nexus Module Operations
// ============================================

const nexusChainState = async (
    action: 'activate' | 'deactivate',
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const requestType = action === 'activate' ? 'ActivateChainRequest' : 'DeactivateChainRequest';
    const message = encodeChainStatusRequest(args, requestType);

    const actionText = action.charAt(0).toUpperCase() + action.slice(1);
    const defaultTitle = `${actionText} ${args.join(', ')} on Nexus`;

    return executeCoreOperation(client, config, options, [message], fee, defaultTitle);
};

const activateChain = (client: ClientManager, config: ConfigManager, options: CoreCommandOptions, args: string[], fee?: string | StdFee) =>
    nexusChainState('activate', client, config, options, args, fee);

const deactivateChain = (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
) => nexusChainState('deactivate', client, config, options, args, fee);

const setTransferRateLimit = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [chain, limit, window] = args;

    if (!chain || !limit || !window) {
        throw new Error('Usage: set-transfer-rate-limit <chain> <limit> <window>');
    }

    const message = encodeSetTransferRateLimitRequest(chain, limit, window);
    const defaultTitle = `Set transfer rate limit for ${chain}`;

    return executeCoreOperation(client, config, options, [message], fee, defaultTitle);
};

const registerAssetFee = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [chain, asset, feeRate, minFee, maxFee] = args;

    if (!chain || !asset || !feeRate || !minFee || !maxFee) {
        throw new Error('Usage: register-asset-fee <chain> <asset> <fee-rate> <min-fee> <max-fee>');
    }

    const message = encodeRegisterAssetFeeRequest(chain, asset, feeRate, minFee, maxFee);
    const defaultTitle = `Register asset fee for ${asset} on ${chain}`;

    return executeCoreOperation(client, config, options, [message], fee, defaultTitle);
};

// ============================================
// Permission Module Operations
// ============================================

const registerController = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [controller] = args;

    if (!controller) {
        throw new Error('Usage: register-controller <controller-address>');
    }

    const message = encodeRegisterControllerRequest(controller);
    const defaultTitle = `Register controller ${controller}`;

    return executeCoreOperation(client, config, options, [message], fee, defaultTitle);
};

const deregisterController = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [controller] = args;

    if (!controller) {
        throw new Error('Usage: deregister-controller <controller-address>');
    }

    const message = encodeDeregisterControllerRequest(controller);
    const defaultTitle = `Deregister controller ${controller}`;

    return executeCoreOperation(client, config, options, [message], fee, defaultTitle);
};

// ============================================
// EVM Module Operations
// ============================================

const setGateway = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [chain, address] = args;

    if (!chain || !address) {
        throw new Error('Usage: set-gateway <chain> <address>');
    }

    const message = encodeSetGatewayRequest(chain, address);
    const defaultTitle = `Set gateway address for ${chain}`;

    return executeCoreOperation(client, config, options, [message], fee, defaultTitle);
};

const transferOperatorship = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [chain, keyId] = args;

    if (!chain || !keyId) {
        throw new Error('Usage: transfer-operatorship <chain> <keyID>');
    }

    const message = encodeTransferOperatorshipRequest(chain, keyId);
    const defaultTitle = `Transfer operatorship for ${chain} to key ${keyId}`;

    return executeCoreOperation(client, config, options, [message], fee, defaultTitle);
};

// ============================================
// Multisig Module Operations
// ============================================

const startKeygen = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [keyId] = args;

    if (!keyId) {
        throw new Error('Usage: start-keygen <keyID>');
    }

    const message = encodeStartKeygenRequest(keyId);
    const defaultTitle = `Start keygen for key ${keyId}`;

    return executeCoreOperation(client, config, options, [message], fee, defaultTitle);
};

const rotateKey = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    args: string[],
    fee?: string | StdFee,
): Promise<void> => {
    const [chain, keyId] = args;

    if (!chain || !keyId) {
        throw new Error('Usage: rotate-key <chain> <keyID>');
    }

    const message = encodeRotateKeyRequest(chain, keyId);
    const defaultTitle = `Rotate key for ${chain} to ${keyId}`;

    return executeCoreOperation(client, config, options, [message], fee, defaultTitle);
};

// ============================================
// CLI Program Handler
// ============================================

const programHandler = () => {
    const program = new Command();

    program.name('core').description('Execute core Axelar protocol operations via governance proposals');

    const activateChainCmd = program
        .command('activate-chain')
        .description('Activate chain(s) on Nexus module (ROLE_ACCESS_CONTROL)')
        .argument('<chains...>', 'chain name(s) to activate')
        .action((chains, options) => mainProcessor(activateChain, options, chains));
    addCoreOptions(activateChainCmd);

    const deactivateChainCmd = program
        .command('deactivate-chain')
        .description('Deactivate chain(s) on Nexus module (ROLE_ACCESS_CONTROL)')
        .argument('<chains...>', 'chain name(s) to deactivate')
        .action((chains, options) => mainProcessor(deactivateChain, options, chains));
    addCoreOptions(deactivateChainCmd);

    const setTransferRateLimitCmd = program
        .command('set-transfer-rate-limit')
        .description('Set transfer rate limit for an asset on a chain (ROLE_ACCESS_CONTROL)')
        .argument('<chain>', 'chain name')
        .argument('<limit>', 'rate limit amount')
        .argument('<window>', 'time window (e.g., "24h")')
        .action((chain, limit, window, options) => mainProcessor(setTransferRateLimit, options, [chain, limit, window]));
    addCoreOptions(setTransferRateLimitCmd);

    const registerAssetFeeCmd = program
        .command('register-asset-fee')
        .description('Register fees for an asset on a chain (ROLE_CHAIN_MANAGEMENT)')
        .argument('<chain>', 'chain name')
        .argument('<asset>', 'asset name')
        .argument('<fee-rate>', 'fee rate (decimal)')
        .argument('<min-fee>', 'minimum fee')
        .argument('<max-fee>', 'maximum fee')
        .action((chain, asset, feeRate, minFee, maxFee, options) =>
            mainProcessor(registerAssetFee, options, [chain, asset, feeRate, minFee, maxFee]),
        );
    addCoreOptions(registerAssetFeeCmd);

    const registerControllerCmd = program
        .command('register-controller')
        .description('Register a controller account (ROLE_ACCESS_CONTROL)')
        .argument('<controller>', 'controller address to register')
        .action((controller, options) => mainProcessor(registerController, options, [controller]));
    addCoreOptions(registerControllerCmd);

    const deregisterControllerCmd = program
        .command('deregister-controller')
        .description('Deregister a controller account (ROLE_ACCESS_CONTROL)')
        .argument('<controller>', 'controller address to deregister')
        .action((controller, options) => mainProcessor(deregisterController, options, [controller]));
    addCoreOptions(deregisterControllerCmd);

    const setGatewayCmd = program
        .command('set-gateway')
        .description('Set the gateway address for an EVM chain (ROLE_ACCESS_CONTROL)')
        .argument('<chain>', 'EVM chain name')
        .argument('<address>', 'gateway contract address (0x...)')
        .action((chain, address, options) => mainProcessor(setGateway, options, [chain, address]));
    addCoreOptions(setGatewayCmd);

    const transferOperatorshipCmd = program
        .command('transfer-operatorship')
        .description('Transfer operatorship for an EVM chain (ROLE_CHAIN_MANAGEMENT)')
        .argument('<chain>', 'EVM chain name')
        .argument('<keyID>', 'key ID to transfer operatorship to')
        .action((chain, keyId, options) => mainProcessor(transferOperatorship, options, [chain, keyId]));
    addCoreOptions(transferOperatorshipCmd);

    const startKeygenCmd = program
        .command('start-keygen')
        .description('Start key generation protocol (ROLE_CHAIN_MANAGEMENT)')
        .argument('<keyID>', 'unique ID for the new key')
        .action((keyId, options) => mainProcessor(startKeygen, options, [keyId]));
    addCoreOptions(startKeygenCmd);

    const rotateKeyCmd = program
        .command('rotate-key')
        .description('Rotate a chain to a new key (ROLE_CHAIN_MANAGEMENT)')
        .argument('<chain>', 'chain name')
        .argument('<keyID>', 'key ID to rotate to')
        .action((chain, keyId, options) => mainProcessor(rotateKey, options, [chain, keyId]));
    addCoreOptions(rotateKeyCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}

export {
    activateChain,
    deactivateChain,
    setTransferRateLimit,
    registerAssetFee,
    registerController,
    deregisterController,
    setGateway,
    transferOperatorship,
    startKeygen,
    rotateKey,
};
