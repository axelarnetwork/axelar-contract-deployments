import fs from 'fs';
import { Validator } from 'jsonschema';
import { beforeAll, describe, expect, it } from 'vitest';

import { addAllSchema, schema } from './schema';

describe('Verify `info/*.json` files', () => {
    let jsons;
    let validator;
    let jsonFiles;

    beforeAll(() => {
        const files = fs.readdirSync('info');
        jsonFiles = files.filter((file) => file.endsWith('.json'));
        jsons = jsonFiles.map((file) => {
            const data = fs.readFileSync(`info/${file}`);
            return {
                fileName: file,
                data: JSON.parse(data),
            };
        });

        validator = addAllSchema(new Validator());
    });

    it('should have consistent format', () => {
        jsons.forEach((json) => {
            const response = validator.validate(json.data, schema);

            if (!response.valid) {
                console.error(`Validation failed for file: ${json.fileName}`);
                for (const error of response.errors) {
                    console.error(error);
                }
            }

            expect(response.valid).toBe(true);
        });
    });

    it('should have chain names match lowercase axelarId', () => {
        jsons.forEach((json, _) => {
            if (json.data.chains) {
                Object.entries(json.data.chains).forEach(([chain, chainConfig]) => {
                    expect(chain).toBe(chainConfig.axelarId.toLowerCase());
                });
            }
        });
    });
});
