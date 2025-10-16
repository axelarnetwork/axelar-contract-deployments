import { Options } from '../processor';

export interface MigrationOptions extends Options {
    fees: string;
    address: string;
    deposit: string;
    dry?: boolean;
    direct?: boolean;
    ignoreChains?: string;
    title?: string;
    description?: string;
    codeId?: number;
}

export interface MigrationCheckOptions extends Options {
    address?: string;
    coordinator?: string;
    multisig?: string;
}

export interface ProtocolContracts {
    service_registry: string;
    router: string;
    multisig: string;
}

export interface InstantiatePermission {
    permission?: string;
    address?: string;
    addresses?: string[];
}
