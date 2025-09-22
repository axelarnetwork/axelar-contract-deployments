export enum Contract {
    ServiceRegistry,
    Router,
    Multisig,
    Coordinator,
    Rewards,
    AxelarnetGateway,
    InterchainTokenService,
}

export const ContractMap = new Map<Contract, string>([
    [Contract.ServiceRegistry, 'ServiceRegistry'],
    [Contract.Router, 'Router'],
    [Contract.Multisig, 'Multisig'],
    [Contract.Coordinator, 'Coordinator'],
    [Contract.Rewards, 'Rewards'],
    [Contract.AxelarnetGateway, 'AxelarnetGateway'],
    [Contract.InterchainTokenService, 'InterchainTokenService'],
]);
