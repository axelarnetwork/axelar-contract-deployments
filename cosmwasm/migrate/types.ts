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
    [key: string]: unknown;
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
