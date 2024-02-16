const axelarSchema = {
    id: '/info.axelar',
    type: 'object',
    properties: {
        id: { type: 'string' },
        axelarId: { type: 'string' },
        rpc: { type: 'string' },
        lcd: { type: 'string' },
        grpc: { type: 'string' },
        tokenSymbol: { type: 'string' },
    },
    required: ['id', 'axelarId', 'rpc', 'lcd', 'grpc', 'tokenSymbol'],
};

export const contractValueSchema = {
    id: '/info.chains.contracts.value',
    type: 'object',
    properties: {
        address: { type: 'string' },
    },
    required: ['address'],
};

export const contractSchema = {
    id: '/info.chains.contracts',
    type: 'object',
    patternProperties: {
        // PascalName e.g. 'AxelarGasService', 'AxelarGateway', 'InterchainGovernanceExecutor', etc.
        '\b[A-Z][a-z]*([A-Z][a-z]*)*\b': {
            $ref: contractValueSchema.id,
        },
    },
    properties: {
        skipRevertTests: {
            type: 'boolean',
        },
    },
    required: ['AxelarGateway', 'AxelarGasService'],
};

export const explorerSchema = {
    id: '/info.chains.explorer',
    type: 'object',
    properties: {
        url: { type: 'string' },
        api: { type: 'string' },
    },
    required: ['url'],
};

export const gasOptionSchema = {
    id: '/info.chains.gasOption',
    type: 'object',
    properties: {
        gasLimit: { type: 'number' },
        gasPrice: { type: 'number' },
        maxPriorityFeePerGas: { type: 'number' },
        maxFeePerGas: { type: 'number' },
        gasPriceAdjustment: { type: 'number' },
    },
};

export const chainValueSchema = {
    id: '/info.chains.value',
    type: 'object',
    properties: {
        name: { type: 'string' },
        id: { type: 'string' },
        axelarId: { type: 'string' },
        chainId: { type: 'number' },
        rpc: { type: 'string' },
        tokenSymbol: { type: 'string' },
        contracts: { $ref: contractSchema.id },
        explorer: { $ref: explorerSchema.id },
        gasOptions: { $ref: gasOptionSchema.id },
        confirmations: { type: 'number' },
    },
    required: ['name', 'id', 'axelarId', 'chainId', 'rpc', 'tokenSymbol', 'contracts', 'explorer'],
};

export const chainsSchema = {
    id: '/info.chains',
    type: 'object',
    patternProperties: {
        '^[a-z]+$': {
            $ref: chainValueSchema.id,
        },
    },
};

export const schema = {
    id: '/info',
    type: 'object',
    properties: {
        axelar: { $ref: axelarSchema.id },
        chains: { $ref: chainsSchema.id },
    },
    required: ['axelar', 'chains'],
};

export function addAllSchema(validator) {
    validator.addSchema(axelarSchema, axelarSchema.id);
    validator.addSchema(chainsSchema, chainsSchema.id);
    validator.addSchema(chainValueSchema, chainValueSchema.id);
    validator.addSchema(contractValueSchema, contractValueSchema.id);
    validator.addSchema(contractSchema, contractSchema.id);
    validator.addSchema(explorerSchema, explorerSchema.id);
    validator.addSchema(gasOptionSchema, gasOptionSchema.id);

    return validator;
}
