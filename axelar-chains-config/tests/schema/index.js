const axelarSchema = {
    id: '/info.axelar',
    type: 'object',
    properties: {
        axelarId: { type: 'string' },
        rpc: { type: 'string' },
        lcd: { type: 'string', pattern: '^$|^(https?:\\/\\/[^\\/\\:]+(:\\d+)?)$' },
        grpc: { type: 'string' },
        tokenSymbol: { type: 'string' },
        cosmosSDK: { type: 'string' },
    },
    required: ['axelarId', 'rpc', 'lcd', 'grpc', 'tokenSymbol', 'cosmosSDK'],
};

export const contractValueSchema = {
    id: '/info.chains.contracts.value',
    type: 'object',
    properties: {
        address: { type: 'string' },
    },
    required: ['address'],
};

export const axelarGatewaySchema = {
    id: '/info/chains.contracts.AxelarGateway',
    type: 'object',
    properties: {
        connectionType: {
            type: 'string',
            enum: ['consensus', 'amplifier'],
        },
    },
    required: ['connectionType'],
    additionalProperties: true,
};

export const contractSchema = {
    id: '/info.chains.contracts',
    type: 'object',
    patternProperties: {
        // PascalName e.g. 'AxelarGasService' etc.
        '^[a-zA-Z][a-zA-Z]*$': {
            $ref: contractValueSchema.id,
        },
    },
    properties: {
        AxelarGateway: {
            $ref: axelarGatewaySchema.id,
        },
        skipRevertTests: {
            type: 'boolean',
        },
    },
    required: [],
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
        axelarId: { type: 'string' },
        chainId: { type: 'number' },
        networkType: { type: 'string' },
        chainType: { type: 'string' },
        rpc: { type: 'string' },
        tokenSymbol: { type: 'string' },
        contracts: { $ref: contractSchema.id },
        explorer: { $ref: explorerSchema.id },
        gasOptions: { $ref: gasOptionSchema.id },
        confirmations: { type: 'number' },
        finality: { type: 'string' },
        approxFinalityWaitTime: { type: 'number' },
        timeout: { type: 'number' },
        decimals: { type: 'number' },
        deprecated: { type: 'boolean' },
    },
    required: [
        'name',
        'axelarId',
        'rpc',
        'tokenSymbol',
        'contracts',
        'explorer',
        'chainType',
        'finality',
        'approxFinalityWaitTime',
        'decimals',
    ],
};

export const chainsSchema = {
    id: '/info.chains',
    type: 'object',
    patternProperties: {
        '^[a-z][a-z0-9-]*$': {
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
    validator.addSchema(axelarGatewaySchema, axelarGatewaySchema.id);

    return validator;
}
