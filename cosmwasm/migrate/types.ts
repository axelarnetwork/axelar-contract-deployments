import { Options } from '../processor';

export interface MigrationOptions extends Options {
    fees: string;
    address: string;
    deposit: string;
    dry?: boolean;
    proposal?: boolean;
    ignoreChains?: string;
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
