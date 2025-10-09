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
  --salt "salt34" \
  --env testnet \
  --args '{"name":"CrossChain Token34","symbol":"CCT34","admin": "0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f", "homeChain":"Avalanche"}'
```
- Snowtrace token deployment Ex -> https://testnet.snowtrace.io/tx/0x1143c0b49490cf358d7a1bb2643a03c607f82b460df1492030f93986fadb7b5a
- Sepolia token deployment Ex -> https://sepolia.etherscan.io/tx/0x308df36137ff6e84d0a13619b8d391d17232b9fd95b0b502494ca8a17e78cd36

- Snowtrace multisig deployment Ex -> https://testnet.snowtrace.io/tx/0xd7a57dfd00ad438253330f7ab51d958485fa1ff78131baf65b73ecca4389e2ae?chainid=43113


### Mint Tokens
Mint tokens on a specific chain (owner only):
```bash
node evm/demo/index.js mint 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 1000 --chainNames avalanche --tokenAddress 0xb0cf7E20CA9aD11a2a7E5c3c7A27654470524fab --env testnet --yes
```
- Mint tokens snowtrace -> https://testnet.snowtrace.io/tx/0x40f7ba75c2ed62519f45940e5ed3de874fd1a2db7920a8c4d97bbea98c1e3e20


### Check Balance
```bash
node evm/demo/index.js balance --chainNames avalanche --tokenAddress 0xb0cf7E20CA9aD11a2a7E5c3c7A27654470524fab --env testnet
```

- Expected Response: `Balance of 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f: 1000.0 CCT34`

## ITS Commands


### Register Token Metadata
```bash
node evm/its.js register-token-metadata 0x3f4d2D6727ab6D5cd1dEcEa39930390FfCA3959d --chainNames ethereum-sepolia --env testnet --gasValue 100000000000000000 --yes
```

- Sepolia Register Token Metadata -> https://testnet.axelarscan.io/gmp/0x6f145714f560fc9c77273a791d176fbcdba96bec17dfbb33c77d6d76ef5a92d5

```bash
node evm/its.js register-token-metadata 0xb0cf7E20CA9aD11a2a7E5c3c7A27654470524fab --chainNames avalanche --env testnet --gasValue 100000000000000000 --yes
```

- Snowtrace Register Token Metadata Ex -> https://testnet.axelarscan.io/gmp/0xa9505a1fc623c59487c22d44b7fc89ea1576aa3f68346c469b7990b44d4c4970

### Register Custom Token
```bash
node evm/interchainTokenFactory.js --action registerCustomToken --tokenAddress 0xb0cf7E20CA9aD11a2a7E5c3c7A27654470524fab --chainNames avalanche --tokenManagerType 4 --operator 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f --rawSalt 0x043793c09788960d705c545f34bd17c22efc01bde67b409f1c520229bf6d8a29 --env testnet --yes
```
- Register Custom Token Ex -> https://testnet.snowtrace.io/tx/0x19e4e35c43d8002f17d85ea0a2d32be8b4e270025069f23f03b3a0af806fd718

### Link Token
```bash
node evm/interchainTokenFactory.js --action linkToken --chainNames avalanche --destinationChain ethereum-sepolia --destinationTokenAddress 0x3f4d2D6727ab6D5cd1dEcEa39930390FfCA3959d --tokenManagerType 4 --linkParams 0x --rawSalt 0x043793c09788960d705c545f34bd17c22efc01bde67b409f1c520229bf6d8a29 --gasValue 500000000000000000 --env testnet --yes
```
- Link Token Ex -> https://testnet.axelarscan.io/gmp/0xe3bf0db42ccb7143582c3d80f0ecd34791164f594465be2ff182b9dd750891a8

### Interchain Transfer
```bash
node evm/its.js interchain-transfer ethereum-sepolia 0xdf6334c5d94db1d5b4d8e15a671b6fdb3c194b580abb5f246739b4b40fc739e3 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 123 --chainNames avalanche --gasValue 500000000000000000 --env testnet --yes
```
- Interchain Transfer Ex -> https://testnet.axelarscan.io/gmp/0xc727c9a9a51140b8447bfe0d3ffbcd707cc3c2879bcc5353575c26c7a3274f76

### Check Balance To Confirm Successful Transfer
```bash
node evm/demo/index.js balance --chainNames ethereum-sepolia --tokenAddress 0x3f4d2D6727ab6D5cd1dEcEa39930390FfCA3959d --env testnet
```

### Setup CrossChain Burn
```bash
node evm/demo/index.js setup-burn 0x7b921F39dcdBA1B84ed305cFdAFa0857ffc645fc --chainNames avalanche --tokenAddress 0xb0cf7E20CA9aD11a2a7E5c3c7A27654470524fab --env testnet --yes
```

- Setup Burn Tx -> https://testnet.snowtrace.io/tx/0xf240809afae2c61ff24da156ec0052cd9a5323964754ad1e15e29fbeebb63e80

### Cross-Chain Burn

**IMPORTANT**
Before this function can be called, make sure you have funded the gnosis safe with enough tokens to execute the cross-chain burn.

```bash
node evm/demo/index.js cross-chain-burn 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 1 0x7b921F39dcdBA1B84ed305cFdAFa0857ffc645fc --chainNames avalanche --tokenAddress 0xb0cf7E20CA9aD11a2a7E5c3c7A27654470524fab --env testnet --yes --destinationChain ethereum-sepolia --destinationChainTokenAddress 0x3f4d2D6727ab6D5cd1dEcEa39930390FfCA3959d
```
- Ex -> https://testnet.axelarscan.io/gmp/0x4eaa1ff0a59b979275c793b2f32a77ee0379f8c37f192ca90a9c41aa55036742

## Cross-chain Freeze

**IMPORTANT**
Before this function can be called, make sure you have funded the gnosis safe with enough tokens to execute the cross-chain freeze.

```bash
node evm/demo/index.js cross-chain-freeze 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 0x7b921F39dcdBA1B84ed305cFdAFa0857ffc645fc --chainNames avalanche --tokenAddress 0xb0cf7E20CA9aD11a2a7E5c3c7A27654470524fab --env testnet --yes --destinationChain ethereum-sepolia --destinationChainTokenAddress 0x3f4d2D6727ab6D5cd1dEcEa39930390FfCA3959d
```
