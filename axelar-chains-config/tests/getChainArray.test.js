import fs from 'fs';
import { describe, expect, it, beforeAll } from 'vitest';
import { Validator } from 'jsonschema';
import { chainValueSchema, addAllSchema } from './schema';
import { getChainArray } from '..';

describe('getChainArray', () => {
    let validator;

    beforeAll(() => {
        // init validator
        validator = addAllSchema(new Validator());
    });

    it('should return an array of chains', () => {
        const chains = getChainArray('mainnet');

        // validate each chain in the array
        for (const chain of chains) {
            expect(validator.validate(chain, chainValueSchema).valid).toBe(true);
        }

        // validate length of array
        const json = JSON.parse(fs.readFileSync('info/mainnet.json'));
        const totalChains = Object.keys(json.chains).length;
        expect(chains.length).toBe(totalChains);
    });

    it('should throw an error if env is not found', () => {
        expect(() => {
            getChainArray('notfound');
        }).toThrow();
    });
});
