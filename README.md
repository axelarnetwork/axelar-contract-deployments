# Contract deployments for Axelar

Install dependencies via
`npm ci`

[EVM deployment instructions](./evm/README.md)

## Scripts Usage and Example

1. Test mock deployment of contracts using the `contracts-deployment-test.js` script:

```bash
node evm/contracts-deployment-test.js -e <environment> -n <chainNames>
```

For example, to deploy contracts on the Polygon chain in the testnet environment:
```bash
node evm/contracts-deployment-test.js -e testnet -n fantom
```
The script also supports optional flag parameters -y and --deployDepositService, which can also be specified in a .env file under the variables YES and DEPLOY_DEPOSIT_SERVICE.

```bash
# Example with optional flags
node evm/contracts-deployment-test.js -e testnet -n fantom -y --deployDepositService
```