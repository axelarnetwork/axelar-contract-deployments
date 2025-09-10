# Contract deployments for Axelar

Install dependencies via
`npm ci`

Build the project via
`npm run build`

## Dependency Management Policy

To reduce supply chain attack risk, we avoid updating non-internal dependencies to the latest version. We prefer to trail behind the latest by a few months. All package dependency updates require security team review and approval.

## Deployment Instructions

- [EVM](./evm/README.md)
- [Cosmwasm](./cosmwasm/README.md)
- [Sui](./sui/README.md)
- [Stellar](./stellar/README.md)
- [XRPL](./xrpl/README.md)

## Javascript -> Typescript Migration

Please note that this project currently supports both Javascript and Typescript.

### To migrate to Typescript, you may use the following steps

1. When touching a file or creating a new file, ensure the file's extension is `.ts`
2. Complete your implementation, and ensure any relevant testing is also in TS
3. Use types appropriately throughout your implementation. You may type code unrelated to your changes only as necessary
4. Run `npm run build` to compile the project and ensure your new TS is valid via `ts-node <your_new_script>`

### Once migration is complete

1. Remove this information from this README
2. Remove JS related artifacts from the package.json, .eslintrc, .prettierrc.ts, .mocharc.yaml
3. In tsconfig.json, set `allowJs` to `false` and `strict` to `true`
4. Remove the `global.d.ts` file
5. Search globally for '.js' to ensure all references have been removed
