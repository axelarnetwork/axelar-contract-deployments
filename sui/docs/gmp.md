## Usage

Before executing the gmp command, ensure the following three contracts are deployed: `gateway`, `gas_service`, and `test` by following the instruction [here](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/sui#scripts).

### Send Command

The order of arguments is `destChain`, `destContractAddress`, `feeAmount`, and `payload`.

```bash
node sui/gmp.js sendCall ethereum 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05 0.1 0x1234
```
