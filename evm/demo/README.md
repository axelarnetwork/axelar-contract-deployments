# CrossChain Burn Demo

## Token Commands

### Deploy Tokens
```bash
node evm/deploy-contract.js \
  --contractName CrossChainBurn \
  --artifactPath "$PWD/artifacts/evm/solidity/" \
  --chainNames "ethereum-sepolia,avalanche" \
  --salt "salt5" \
  --env testnet \
  --args '{"name":"CrossChain Token","symbol":"CCT","admin": "0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f", "homeChain":"Avalanche"}'
  ```
Ex -> https://testnet.snowtrace.io/tx/0x7a84e8943dbd100de19cfb53a276b864a9cec09575a7512542908c5e475b994d?chainid=43113
Ex -> https://sepolia.etherscan.io/tx/0x70f189d5e620b412a7615be4a3e57f4dad671fad3b76900ba6241ce355c7304a


### Mint Tokens
Mint tokens on a specific chain (owner only):
```bash
node evm/demo/index.js mint 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 1000 --chainNames avalanche --tokenAddress 0x842DE028BE165E6482EC4D694025e617DA6D86e0 --env testnet --yes
```
Ex -> https://testnet.snowtrace.io/tx/0x5fde9624c567d79a9447a8457cdc99c520d9786ba34f653f878287bd09261c33?chainid=43113


### Check Balance
```bash
node evm/demo/index.js balance --chainNames avalanche --tokenAddress 0x842DE028BE165E6482EC4D694025e617DA6D86e0 --env testnet
```

## ITS Commands


### Register Token Metadata
```bash
node evm/its.js register-token-metadata 0x8D160E694909AF519FcdeaA87382450C805a455A --chainNames ethereum-sepolia --env testnet --gasValue 100000000000000000 --yes
```
Ex -> https://testnet.axelarscan.io/gmp/0x9bd787f2ed75d2c48f471a1f051b8174238543ab6e75597440c55313fe0647d2-2

```bash
node evm/its.js register-token-metadata 0x066fA18A236D8c34929Ef3EA185ac5c402a863E5 --chainNames avalanche --env testnet --gasValue 100000000000000000 --yes
```
Ex -> https://testnet.axelarscan.io/gmp/0x485b3391e68b586757eab02cea3141c32ba35da86c39b89501699f22e367c002-2

### Register Custom Token
```bash
node evm/interchainTokenFactory.js --action registerCustomToken --tokenAddress 0x066fA18A236D8c34929Ef3EA185ac5c402a863E5 --chainNames avalanche --tokenManagerType 4 --operator 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f --rawSalt 0x0000000000000000000000000000000000000000000000000000000000000001 --env testnet --yes
```
Ex -> https://testnet.snowtrace.io/tx/0xb02fc3aee6f4ea515acc007d713ba10e7f490e9c3e5c3b2adc51405319e42bc2?chainid=43113

### Link Token
```bash
node evm/interchainTokenFactory.js --action linkToken --chainNames avalanche --destinationChain ethereum-sepolia --destinationTokenAddress 0x8D160E694909AF519FcdeaA87382450C805a455A --tokenManagerType 4 --linkParams 0x --rawSalt 0x0000000000000000000000000000000000000000000000000000000000000001 --gasValue 500000000000000000 --env testnet --yes
```
Ex -> https://testnet.axelarscan.io/gmp/0x05f35d12d4dcc68abc5ac3594ceba66b674452956cd2e07e90625d60564ccfdb-3

### Interchain Transfer
```bash
node evm/its.js interchain-transfer ethereum-sepolia 0x3c2bfd2d6ada17dcb2b7d3ff1885ee374bc72e367f7151648d9b55ebb33f9e79 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 123 --chainNames avalanche --gasValue 500000000000000000 --env testnet --yes
```
Ex -> https://testnet.axelarscan.io/gmp/0xea8caadb7e8b66de281235a798d357347efe78ed168a658bcc30185577a1f5c4-3


### Cross-Chain Burn
```bash
node evm/demo/index.js cross-chain-burn 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 1 --chainNames avalanche --tokenAddress 0xF7190de34bcE761918cb4a04C3e89625846A9233 --env testnet --yes --destinationChain ethereum-sepolia --destinationChainTokenAddress 0xFBF598747DA9D10d966A16D314bFFc3839556bE1
```
Ex -> https://testnet.axelarscan.io/gmp/0xea8d287f23952c8f772767da4bc456ae73cfc12eac0ea384739aabf5439083f8-1



