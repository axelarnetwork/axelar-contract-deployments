import { StdFee } from '@cosmjs/stargate';
import { Command } from 'commander';
import * as fs from 'fs';

import { printInfo, printWarn, validateParameters } from '../common';
import { ConfigManager } from '../common/config';
import { addCoreOptions } from './cli-utils';
import { ClientManager, Options } from './processor';
import { mainProcessor } from './processor';
import { confirmProposalSubmission, submitProposalAndPrint } from './proposal-utils';
import {
    addressToBytes,
    encodeChainStatusRequest,
    encodeDeregisterControllerRequest,
    encodeRegisterAssetFeeRequest,
    encodeRegisterControllerRequest,
    encodeRotateKeyRequest,
    encodeSetGatewayRequest,
    encodeSetTransferRateLimitRequest,
    encodeStartKeygenRequest,
    encodeTransferOperatorshipRequest,
    getNexusProtoType,
    getProtoType,
    getUnitDenom,
    signAndBroadcastWithRetry,
} from './utils';

type RoleType = 'ROLE_ACCESS_CONTROL' | 'ROLE_CHAIN_MANAGEMENT';

interface CoreCommandOptions extends Options {
    yes?: boolean;
    title?: string;
    description?: string;
    direct?: boolean;
    output?: string;
    [key: string]: unknown;
}

const executeDirectEOA = async (
    client: ClientManager,
    options: CoreCommandOptions,
    messages: object[],
    fee?: string | StdFee,
): Promise<void> => {
    const signerAddress = client.accounts[0].address;
    printInfo('Executing directly as EOA', signerAddress);

    if (!options.yes) {
        printWarn('Direct execution mode. Use -y to skip confirmation in non-interactive mode.');
        const readline = await import('readline');
        const rl = readline.createInterface({ input: process.stdin, output: process.stdout });

        const answer = await new Promise<string>((resolve) => {
            rl.question('Proceed with direct execution? (y/N): ', resolve);
        });
        rl.close();

        if (answer.toLowerCase() !== 'y') {
            printInfo('Operation cancelled');
            return;
        }
    }

    const result = await signAndBroadcastWithRetry(client, signerAddress, messages, fee || 'auto', '');
    printInfo('Transaction hash', result.transactionHash);
    printInfo('Result', JSON.stringify(result, null, 2));
};

const getDefaultFee = (config: ConfigManager): StdFee => {
    const denom = getUnitDenom(config);
    return { amount: [{ denom, amount: '500000' }], gas: '500000' };
};

const isValidFeeObject = (fee: string | StdFee | undefined): fee is StdFee => {
    return typeof fee === 'object' && fee !== null && 'amount' in fee && 'gas' in fee;
};

const generateMultisigTx = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    messages: object[],
    fee?: string | StdFee,
    defaultTitle?: string,
): Promise<void> => {
    const signerAddress = client.accounts[0].address;
    const chainId = config.axelar.chainId;

    const { accountNumber, sequence } = await client.getSequence(signerAddress);

    // For multisig unsigned tx, we need a proper fee object (not 'auto' string)
    const txFee = isValidFeeObject(fee) ? fee : getDefaultFee(config);

    const unsignedTx = {
        chainId,
        accountNumber,
        sequence,
        fee: txFee,
        msgs: messages,
        memo: defaultTitle || 'Core operation',
    };

    const outputPath = options.output || `unsigned_tx_${Date.now()}.json`;
    fs.writeFileSync(outputPath, JSON.stringify(unsignedTx, null, 2));
    printInfo('Unsigned transaction saved to', outputPath);
    printInfo('', '');
    printInfo('Next steps for multisig signing:');
    printInfo('1. Share this file with all multisig signers');
    printInfo('2. Each signer signs with: axelard tx sign <file> --from <key> --multisig <multisig-addr> --chain-id ' + chainId);
    printInfo('3. Combine signatures: axelard tx multisign <file> <multisig-name> <sig1> <sig2> ...');
    printInfo('4. Broadcast: axelard tx broadcast <signed-file>');
};

const executeCoreOperation = async (
    client: ClientManager,
    config: ConfigManager,
    options: CoreCommandOptions,
    messages: object[],
    roleType: RoleType,
    fee?: string | StdFee,
    defaultTitle?: string,
    defaultDescription?: string,
): Promise<void> => {
    if (options.direct) {
        const signerAddress = client.accounts[0].address;

        const messagesWithSender = messages.map((msg: { typeUrl: string; value: Uint8Array }) => {
            const RequestType = getRequestTypeFromMessage(msg);
            if (RequestType) {
                const decoded = RequestType.decode(msg.value);
                decoded.sender = addressToBytes(signerAddress);
                return {
                    typeUrl: msg.typeUrl,
                    value: Uint8Array.from(RequestType.encode(decoded).finish()),
                };
            }
            return msg;
        });

        if (roleType === 'ROLE_CHAIN_MANAGEMENT') {
            return executeDirectEOA(client, options, messagesWithSender, fee);
        } else {
            printInfo('ROLE_ACCESS_CONTROL operation requires multisig signing');
            return generateMultisigTx(client, config, options, messagesWithSender, fee, defaultTitle);
        }
    }

    const title = options.title || defaultTitle;
    const description = options.description || defaultDescription || defaultTitle;
    validateParameters({ isNonEmptyString: { title, description } });

    if (!confirmProposalSubmission(options, messages)) {
        return;
    }

    await submitProposalAndPrint(client, config, { ...options, title, description }, messages, fee);
};

interface ProtoMessage {
    typeUrl: string;
    value: Uint8Array;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ProtoType = any;

const getRequestTypeFromMessage = (msg: ProtoMessage): ProtoType | null => {
    const typeUrlToProto: Record<string, () => ProtoType> = {
        '/axelar.nexus.v1beta1.ActivateChainRequest': () => getNexusProtoType('ActivateChainRequest'),
        '/axelar.nexus.v1beta1.DeactivateChainRequest': () => getNexusProtoType('DeactivateChainRequest'),
        '/axelar.nexus.v1beta1.SetTransferRateLimitRequest': () => getNexusProtoType('SetTransferRateLimitRequest'),
        '/axelar.nexus.v1beta1.RegisterAssetFeeRequest': () => getNexusProtoType('RegisterAssetFeeRequest'),
        '/axelar.permission.v1beta1.RegisterControllerRequest': () =>
            getProtoType('permission.proto', 'axelar.permission.v1beta1', 'RegisterControllerRequest'),
        '/axelar.permission.v1beta1.DeregisterControllerRequest': () =>
            getProtoType('permission.proto', 'axelar.permission.v1beta1', 'DeregisterControllerRequest'),
        '/axelar.evm.v1beta1.SetGatewayRequest': () => getProtoType('evm.proto', 'axelar.evm.v1beta1', 'SetGatewayRequest'),
        '/axelar.evm.v1beta1.CreateTransferOperatorshipRequest': () =>
            getProtoType('evm.proto', 'axelar.evm.v1beta1', 'CreateTransferOperatorshipRequest'),
        '/axelar.multisig.v1beta1.StartKeygenRequest': () =>
            getProtoType('multisig.proto', 'axelar.multisig.v1beta1', 'StartKeygenRequest'),
        '/axelar.multisig.v1beta1.RotateKeyRequest': () => getProtoType('multisig.proto', 'axelar.multisig.v1beta1', 'RotateKeyRequest'),
    };

    const getType = typeUrlToProto[msg.typeUrl];
    return getType ? getType() : null;
};

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

    return executeCoreOperation(client, config, options, [message], 'ROLE_ACCESS_CONTROL', fee, defaultTitle);
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

    return executeCoreOperation(client, config, options, [message], 'ROLE_ACCESS_CONTROL', fee, defaultTitle);
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

    return executeCoreOperation(client, config, options, [message], 'ROLE_CHAIN_MANAGEMENT', fee, defaultTitle);
};

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

    return executeCoreOperation(client, config, options, [message], 'ROLE_ACCESS_CONTROL', fee, defaultTitle);
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

    return executeCoreOperation(client, config, options, [message], 'ROLE_ACCESS_CONTROL', fee, defaultTitle);
};

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

    return executeCoreOperation(client, config, options, [message], 'ROLE_ACCESS_CONTROL', fee, defaultTitle);
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

    return executeCoreOperation(client, config, options, [message], 'ROLE_CHAIN_MANAGEMENT', fee, defaultTitle);
};

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

    return executeCoreOperation(client, config, options, [message], 'ROLE_CHAIN_MANAGEMENT', fee, defaultTitle);
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

    return executeCoreOperation(client, config, options, [message], 'ROLE_CHAIN_MANAGEMENT', fee, defaultTitle);
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
        .argument('<limit>', 'rate limit amount with optional denom (e.g., "1000000uaxl" or "1000000")')
        .argument('<window>', 'time window (e.g., "3600", "60m", "24h", "7d")')
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
