# CrossChain Burn Demo


## Compile

`npx hardhat compile`

## Token Commands

Note: `safeDeploymentConfig` needs to have unique salt when deploying

### Deploy Tokens
```bash
ts-node evm/deploy-contract.js \
  --contractName CrossChainBurn \
  --artifactPath "$PWD/artifacts/evm/solidity/" \
  --chainNames "ethereum-sepolia,avalanche" \
  --salt "salt37" \
  --env testnet \
  --args '{"name":"CrossChain Token37","symbol":"CCT37","admin": "0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f", "homeChain":"Avalanche"}'
```
- Snowtrace token deployment Ex -> https://testnet.snowtrace.io/tx/0x1bab37283b0b2bbf8f421f6bab0c8e67f06eab710a759ee353779436d8044bc2?chainid=43113
- Sepolia token deployment Ex -> https://sepolia.etherscan.io/tx/0x63a604d21d4b51e00b527277e2cf6fd00ca844cb0b749470e88a6218eaafa304

- Snowtrace multisig deployment Ex -> https://testnet.snowtrace.io/tx/0x1ea8aba2020fb1ee2c1d27862ef5468b0f2daa3358828f81154c5591d345dcd5?chainid=43113


### Mint Tokens
Mint tokens on a specific chain (owner only):
```bash
ts-node evm/demo/index.js mint 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 1000 --chainNames avalanche --tokenAddress 0xE195fCdcFF72F054E64d95e84fD039957405ED06 --env testnet --yes
```
- Mint tokens snowtrace -> https://testnet.snowtrace.io/tx/0x8fb821d7721f6baead0ef7a41ef89e00adfc96cd58f0ed830a0d8060531acf1d


### Check Balance
```bash
ts-node evm/demo/index.js balance --chainNames avalanche --tokenAddress 0xE195fCdcFF72F054E64d95e84fD039957405ED06 --env testnet
```

- Expected Response: `Balance of 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f: 1000.0 CCT37`

## ITS Commands


### Register Token Metadata
```bash
ts-node evm/its.js register-token-metadata 0xFdb59e85ed3d79F846eB6D4e5653D25a48033ee5 --chainNames ethereum-sepolia --env testnet --gasValue 10000000000000000 --yes
```

- Sepolia Register Token Metadata -> https://testnet.axelarscan.io/gmp/0xbbb4acc265819c6d0e1e8ea1d95096e2657d47c481dbbcab69e3b2ad2a9ec381

```bash
ts-node evm/its.js register-token-metadata 0xE195fCdcFF72F054E64d95e84fD039957405ED06 --chainNames avalanche --env testnet --gasValue 10000000000000000 --yes
```

- Snowtrace Register Token Metadata Ex -> https://testnet.axelarscan.io/gmp/0xda9dad1b650f193e83060fec7e7c2d25b1aef3f492e19c4d6f93cb6ce47a78a3

### Register Custom Token
```bash
ts-node evm/interchainTokenFactory.js --action registerCustomToken --tokenAddress 0xE195fCdcFF72F054E64d95e84fD039957405ED06 --chainNames avalanche --tokenManagerType 4 --operator 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f --rawSalt 0xec4b025593475aed99f28b43d4986883a207f947ae245c87ebf007647fcffc06 --env testnet --yes
```
- Register Custom Token Ex -> https://testnet.snowtrace.io/tx/0xc7550ddee578732f4031e45bb9510debce1be470b35e01aa954ae7892afa6520

### Link Token
```bash
ts-node evm/interchainTokenFactory.js --action linkToken --chainNames avalanche --destinationChain ethereum-sepolia --destinationTokenAddress 0xFdb59e85ed3d79F846eB6D4e5653D25a48033ee5 --tokenManagerType 4 --linkParams 0x --rawSalt 0xec4b025593475aed99f28b43d4986883a207f947ae245c87ebf007647fcffc06 --gasValue 100000000000000000 --env testnet --yes
```
- Link Token Ex -> https://testnet.axelarscan.io/gmp/0x16f4c38bf9e5053da7bd63c2eedfbe18604016f21b8fe30613d5434e7965c5c0

### Interchain Transfer
```bash
ts-node evm/its.js interchain-transfer ethereum-sepolia 0x5f4f70d32dfcabc292bae7584cd160c97c795b3dc7dcba7afeeb8bc78c374d43 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 123 --chainNames avalanche --gasValue 100000000000000000 --env testnet --yes
```
- Interchain Transfer Ex -> https://testnet.axelarscan.io/gmp/0x1fc63a8a53f0b0197eab7cdc04f0b8518fad13d63ddee85a22f5c455bd389968

### Check Balance To Confirm Successful Transfer
```bash
ts-node evm/demo/index.js balance --chainNames ethereum-sepolia --tokenAddress 0xFdb59e85ed3d79F846eB6D4e5653D25a48033ee5 --env testnet
```

### Setup CrossChain Burn
```bash
ts-node evm/demo/index.js setup-burn 0x80d56BcA38C32799316c87B1662A1088F75C30dA --chainNames avalanche --tokenAddress 0xE195fCdcFF72F054E64d95e84fD039957405ED06 --env testnet --yes
```

- Setup Burn Tx -> https://testnet.snowtrace.io/tx/0x019b2e7bef38decdad078061c83789547d7d78d16114c09a097b029d0cf43844

### Cross-Chain Burn

**IMPORTANT**
Before this function can be called, make sure you have funded the gnosis safe with enough tokens to execute the cross-chain burn.

```bash
ts-node evm/demo/index.js cross-chain-burn 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 1 0x80d56BcA38C32799316c87B1662A1088F75C30dA --chainNames avalanche --tokenAddress 0xE195fCdcFF72F054E64d95e84fD039957405ED06 --env testnet --yes --destinationChain ethereum-sepolia --destinationChainTokenAddress 0xFdb59e85ed3d79F846eB6D4e5653D25a48033ee5
```
- Cross-chain burn -> https://testnet.axelarscan.io/gmp/0x9053352cc78d3b7b16f93328bce682f603890af2596d00aae1b4b7a1bdb9ee38

## Cross-chain Freeze

**IMPORTANT**
Before this function can be called, make sure you have funded the gnosis safe with enough tokens to execute the cross-chain freeze.

```bash
ts-node evm/demo/index.js cross-chain-freeze 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 0x80d56BcA38C32799316c87B1662A1088F75C30dA --chainNames avalanche --tokenAddress 0xE195fCdcFF72F054E64d95e84fD039957405ED06 --env testnet --yes --destinationChain ethereum-sepolia --destinationChainTokenAddress 0xFdb59e85ed3d79F846eB6D4e5653D25a48033ee5

```
- Cross-chain Freeze Tx -> https://testnet.axelarscan.io/gmp/0xebc3335e18dc73c201cc5caf010db87fe6c0fcadb68021948d0836c784d8db46


