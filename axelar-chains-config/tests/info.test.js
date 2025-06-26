import fs from 'fs';
import { Validator } from 'jsonschema';
import { beforeAll, describe, expect, it } from 'vitest';

import { addAllSchema, schema } from './schema';

describe('Verify `info/*.json` files', () => {
    let jsons;
    let validator;

    beforeAll(() => {
        const files = fs.readdirSync('info');
        jsons = files
            .filter((file) => file.endsWith('.json'))
            .map((file) => {
                const data = fs.readFileSync(`info/${file}`);
                return JSON.parse(data);
            });

        validator = addAllSchema(new Validator());
    });

    it('should have consistent format', () => {
        jsons.forEach((json) => {
            const response = validator.validate(json, schema);

            if (!response.valid) {
                for (const error of response.errors) {
                    console.error(error);
                }
            }

            expect(response.valid).toBe(true);
        });
    });
});
