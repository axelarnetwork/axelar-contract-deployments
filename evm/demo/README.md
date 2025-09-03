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



### Mint Tokens
Mint tokens on a specific chain (owner only):
```bash
node evm/demo/index.js mint 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 1000 --chainNames avalanche --tokenAddress 0x842DE028BE165E6482EC4D694025e617DA6D86e0 --env testnet --yes
```


### Check Balance
```bash
node evm/demo/index.js balance --chainNames avalanche --tokenAddress 0x842DE028BE165E6482EC4D694025e617DA6D86e0 --env testnet
```

## ITS Commands


### Register Token Metadata
```bash
node evm/its.js register-token-metadata 0x8D160E694909AF519FcdeaA87382450C805a455A --chainNames ethereum-sepolia --env testnet --gasValue 100000000000000000 --yes
```

```bash
node evm/its.js register-token-metadata 0x066fA18A236D8c34929Ef3EA185ac5c402a863E5 --chainNames avalanche --env testnet --gasValue 100000000000000000 --yes
```

### Register Custom Token
```bash
node evm/interchainTokenFactory.js --action registerCustomToken --tokenAddress 0x066fA18A236D8c34929Ef3EA185ac5c402a863E5 --chainNames avalanche --tokenManagerType 4 --operator 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f --rawSalt 0x0000000000000000000000000000000000000000000000000000000000000001 --env testnet --yes
```

### Link Token
```bash
node evm/interchainTokenFactory.js --action linkToken --chainNames avalanche --destinationChain ethereum-sepolia --destinationTokenAddress 0x8D160E694909AF519FcdeaA87382450C805a455A --tokenManagerType 4 --linkParams 0x --rawSalt 0x0000000000000000000000000000000000000000000000000000000000000001 --gasValue 500000000000000000 --env testnet --yes
```

### Interchain Transfer
```bash
node evm/its.js interchain-transfer ethereum-sepolia 0x3c2bfd2d6ada17dcb2b7d3ff1885ee374bc72e367f7151648d9b55ebb33f9e79 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 123 --chainNames avalanche --gasValue 500000000000000000 --env testnet --yes
```

### Cross-Chain Burn
```bash
node evm/demo/index.js cross-chain-burn 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 1 --chainNames avalanche --tokenAddress 0xF7190de34bcE761918cb4a04C3e89625846A9233 --env testnet --yes --destinationChain ethereum-sepolia --destinationChainTokenAddress 0xFBF598747DA9D10d966A16D314bFFc3839556bE1
```



