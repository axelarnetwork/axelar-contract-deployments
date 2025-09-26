import { Options } from '../processor';

export interface MigrationOptions extends Options {
    fees: string;
    address: string;
    deposit: string;
    dry?: boolean;
    proposal?: boolean;
}
