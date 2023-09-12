import { describe, expect, it, beforeAll } from 'vitest';
import { Validator } from 'jsonschema';
import { contractSchema, contractValueSchema } from './schema';

describe('Verify schema', () => {
    let validator;

    beforeAll(() => {
        validator = new Validator();
    });

    it('should validate contract correctly', () => {
        const validObject = {
            AxelarGateway: {
                address: '0x000000000',
            },
            AxelarGasService: {
                address: '0x000000000',
            },
        };

        validator.addSchema(contractValueSchema, contractValueSchema.id);
        expect(validator.validate(validObject, contractSchema).valid).toBe(true);

        const missingAddressObject = {
            AxelarGateway: {},
            AxelarGasService: {},
        };
        expect(validator.validate(missingAddressObject.AxelarGateway, contractValueSchema).valid).toBe(false);
    });
});
