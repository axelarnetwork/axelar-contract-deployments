export const schema = {
    id: '/info',
    type: 'object',
    properties: {
        axelar: { $ref: '/info.axelar' },
        chains: { $ref: '/info.chains' },
    },
    required: ['axelar', 'chains'],
};

const axelarSchema = {
    id: '/info.axelar',
    type: 'object',
    properties: {
        id: { type: 'string' },
        rpc: { type: 'string' },
        lcd: { type: 'string' },
        grpc: { type: 'string' },
        tokenSymbol: { type: 'string' },
    },
    required: ['id', 'rpc', 'lcd', 'grpc', 'tokenSymbol'],
};

export const chainsSchema = {
    id: '/info.chains',
    type: 'object',
    patternProperties: {
        '^[a-z]+$': { $ref: '/info.chains.value' },
    },
};

export const chainSchema = {
    id: '/info.chains.value',
    type: 'object',
    properties: {
        name: { type: 'string' },
        id: { type: 'string' },
        chainId: { type: 'number' },
        rpc: { type: 'string' },
        tokenSymbol: { type: 'string' },
        contracts: { $ref: '/info.chains.contracts' },
        explorer: { $ref: '/info.chains.explorer' },
        gasOptions: { $ref: '/info.chains.gasOption' },
    },
    required: ['name', 'id', 'chainId', 'rpc', 'tokenSymbol', 'contracts', 'explorer'],
};

export const contractsSchema = {
    id: '/info.chains.contracts',
    type: 'object',
    patternProperties: {
        // capitalized words e.g. 'AxelarGasService', 'AxelarGateway', 'InterchainGovernanceExecutor', etc.
        '\b[A-Z][a-z]*(?:[A-Z][a-z]+)*\b': {
            $ref: '/info.chains.contracts.value',
        },
    },
    properties: {
        skipRevertTests: {
            type: 'boolean',
        },
    },
};

export const contractSchema = {
    id: '/info.chains.contracts.value',
    type: 'object',
    properties: {
        address: { type: 'string' },
    },
    required: ['address'],
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
    },
    required: ['gasLimit'],
};

export function addAllSchema(validator) {
    validator.addSchema(axelarSchema, axelarSchema.id);
    validator.addSchema(chainsSchema, chainsSchema.id);
    validator.addSchema(chainSchema, chainSchema.id);
    validator.addSchema(contractsSchema, contractsSchema.id);
    validator.addSchema(contractSchema, contractSchema.id);
    validator.addSchema(explorerSchema, explorerSchema.id);
    validator.addSchema(gasOptionSchema, gasOptionSchema.id);

    return validator
}
