# Custom Token Upgrade Procedures


This document presents possible flows for upgrading custom ERC20 token contracts that have been connected to ITS and have ITS tokens deployed to other axelar connected chains.

## Assumptions

Developer team will have deployed a custom ERC20 contract that is either directly upgradable or will need to be replaced by an updated contract version.
The existing ERC20 will have already been connected to ITS by registering the token as a canonical token and remotely deploying ITS Tokens to other axelar connected chains. This assumption implies token manager on source chain is lock/unlock; but similar flow can be established to create mint/burn token manager but it requires operation overhead and assitance. 
Only a single chain (the source chain) has the custom contract deployed to it, and all other connected chains have standard ITS tokens deployed to them.
Token liquidity should be preserved post upgrade of the ERC20 on the source chain. 


## Scenario 1: ERC20 is directly upgradable

For ERC20 contracts that maintain same contract address after being upgraded the assumption is that on the source chain the liquidity can be managed directly by the developer team. However, for liquidity on destination chains connected by ITS the proposed solution is as follows:
1. Freeze all cross-chain transfers for old tokenID to free liquidity on each connected chain
2. Upgrade the source chain ERC20 contract
3. Deprecate the existing ITS ERC20 contracts deployed to destination chains and deploy new ITS contracts on each connected chain (new addresses generated). Depending on liquidity management the following choices are available
    a. If users are expected to redeem the old tokens in order to get new tokens we would need to re-anable cross-chain transactions where the old tokens can be sent back to the source chain. The tokens would flow through the old Token manager unlocking the tokens, which can be sent back to destination chain via a new token manager.
    b. If users are not expected to redeem the tokens, the new ITS contracts can be minted with substitute supply that can be distributed to each token holder on the destination chains. For this approach it is necessary to transfer an equivalent amount of tokens on the source chain to the new Token Manager to lock tokens that can be redeem whenever users sendtokens back to source chain using a newly created tokenId. 
4. Connect the new ITS contracts on the destination chains with the upgraded ocntracto n the source chain using custom token linking:
    a. On source chain and destination chain the token metadata must be registered 
    b. From the source chain the upgrade contract must be registered as custom token. Must define unique salt and deployer to manage connection moving forward.
        i. This step creates the new token manager on the source chain. Depending on liquidity selection it can be mint/burn or lock/unlock
    c. From the source chain the upgrade contract is linked to each new ITS contract on the destination chains. Must use same salt and deployer as previous steps.
        i. On this step it is necessary to define the token manager on the destination chains. Recommended to use mint/burn 
5. For the newly created token managers on the destination chains if they are using mint/burn transfer mintership



### Deploy ERC-20 Contract On Source Chain

Custom ERC-20 token deployment. In this step we use `hardhat` to deploy our custom token. First step is to create a hardhat repository using it's [command line tool](https://v2.hardhat.org/tutorial/creating-a-new-hardhat-project).

Source code of the token `token_repository/contracts/Token.sol`:

```
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract MyToken is ERC20 {
    constructor(uint256 initialSupply) ERC20("PruvTk", "PRUVTK") {
        _mint(msg.sender, initialSupply * (10 ** decimals()));
    }
}
```

Then we can run a command:

```bash
 % npm run deploy
```

Result:

```bash
> pruv-deployment@1.0.0 deploy
> hardhat run scripts/deploy.js --network fuji

Downloading compiler 0.8.20
Compiled 6 Solidity files successfully (evm target: paris).
Deploying Rwa contract to Avalanche Fuji...
Rwa contract deployed to: 0x73D01a07D0Ca8E0b01cc6465E8d122D229acbC8a
Contract name: PruvTk
Contract symbol: PRUVTK
Total supply: 1000000000000000000000000
Deployer address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233

To verify the contract, run:
npx hardhat verify --network fuji 0x73D01a07D0Ca8E0b01cc6465E8d122D229acbC8a 1000000
```


### Register Canonical Token On Source Chain

Command:

```bash
 % ts-node evm/interchainTokenFactory.js --action registerCanonicalInterchainToken --tokenAddress 0x73D01a07D0Ca8E0b01cc6465E8d122D229acbC8a -n avalanche -e testnet
```

Result:

```bash
Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233
Wallet balance: 47.44761194825246 
Wallet nonce: 6124
Contract name: InterchainTokenFactory
Contract address: 0x83a93500d23Fbc3e82B410aD07A6a9F7A0670D66
Action: registerCanonicalInterchainToken
tokenId: 0x645c5eab8f63da25fbcf84bf478c66d081425c9dacb94261cc2767af5f69c0b0
Token name: PruvTk
Token symbol: PRUVTK
Token decimals: 18
registerCanonicalInterchainToken tx: 0x1d108eff5432c9993da0779744996c24f8255f418bf78478e4939b67d9a5da02
```


### Register Remote Canonical Token On Remote Chain

Command:

```bash
 % ts-node evm/interchainTokenFactory.js --action deployRemoteCanonicalInterchainToken --tokenAddress 0x73D01a07D0Ca8E0b01cc6465E8d122D229acbC8a --destinationChain flow -n avalanche -e testnet --gasValue 1000000000000000000
```

Result:

```bash
Environment: testnet
Chain: Avalanche
Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233
Wallet balance: 47.446834733751935 
Wallet nonce: 6125
Contract name: InterchainTokenFactory
Contract address: 0x83a93500d23Fbc3e82B410aD07A6a9F7A0670D66
Gas options: {}
Action: deployRemoteCanonicalInterchainToken
Proceed with action deployRemoteCanonicalInterchainToken (y/n) y
tokenId: 0x645c5eab8f63da25fbcf84bf478c66d081425c9dacb94261cc2767af5f69c0b0
Token name: PruvTk
Token symbol: PRUVTK
Token decimals: 18
deployRemoteCanonicalInterchainToken tx: 0xe56acc39af4028d2e624578e73d41cc16db12785f3273ad40a5e97f014616449
```


### Test Interchain Transfer

Command:

```bash
 % ts-node evm/its.js interchain-transfer flow 0x645c5eab8f63da25fbcf84bf478c66d081425c9dacb94261cc2767af5f69c0b0 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0.0123 --chainNames avalanche --gasValue 500000000000000000 --yes --env testnet
```

Result:

```bash
Environment: testnet
Chain: Avalanche
Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233
Wallet balance: 47.44440856057631 
Wallet nonce: 6128
Contract name: InterchainTokenService
Contract address: 0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C
Action: interchain-transfer
Token name: PruvTk
Token symbol: PRUVTK
Token decimals: 18
Approving ITS for a transfer for token with token manager type: 2
Human-readable destination address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233
interchain-transfer tx: 0x9a73a54445c38b1cd5df534b6be6075f7c0f1b44c6de5fccff282892858bd74b
```


## Upgrade procedure

Whenever there's a need to upgrade the token contract, the following procedure can be executed.

### Obtain Current Remote Token Contract Address

Command:

```bash
ts-node evm/its.js interchain-token-address 0x645c5eab8f63da25fbcf84bf478c66d081425c9dacb94261cc2767af5f69c0b0 -n flow -e testnet
```

Result:

```bash
InterchainToken address for tokenId: 0x645c5eab8f63da25fbcf84bf478c66d081425c9dacb94261cc2767af5f69c0b0: 0xe6F6094fDf8Fd1B55e2c4A3fa904c8fF9789A1c9
Token for tokenId: 0x645c5eab8f63da25fbcf84bf478c66d081425c9dacb94261cc2767af5f69c0b0 exists at address:: 0xe6F6094fDf8Fd1B55e2c4A3fa904c8fF9789A1c9
```

### Register Token Metadata

We need to register new tokens metadata on both chains to create a new link between the already deployed token contracts.

```bash
node evm/its.js register-token-metadata 0x73D01a07D0Ca8E0b01cc6465E8d122D229acbC8a --chainNames avalanche --env testnet --gasValue 100000000000000000 --yes
```

Result:

```bash
Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233
Wallet balance: 47.20016974997588 
Wallet nonce: 6153
Contract address: 0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C
Action: register-token-metadata
register-token-metadata tx: 0x150dd9b0b31a2e3741e1a0832197404fef3fe5fe5221e5d276452b22eb66e3b6
```

```bash
node evm/its.js register-token-metadata 0xe6F6094fDf8Fd1B55e2c4A3fa904c8fF9789A1c9 --chainNames flow --env testnet --gasValue 100000000000000000 --yes
```

Result:

```bash
Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233
Wallet balance: 165311.77017535226 
Wallet nonce: 2693
Contract name: InterchainTokenService
Contract address: 0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C
Action: register-token-metadata
register-token-metadata tx: 0x83e87bf46459b0d8d83dab79326624c3fdf9e403b0cbc935a046f82c5c6ad267
```

### Register Custom Token Using Already Deployed Token

Command:

```bash
 % ts-node evm/interchainTokenFactory.js --action registerCustomToken --tokenAddress 0x73D01a07D0Ca8E0b01cc6465E8d122D229acbC8a --chainNames avalanche --tokenManagerType 4 --operator 0xba76c6980428A0b10CFC5d8ccb61949677A61233 --salt "pruvtk_upgrade_10" --env testnet --yes --gasValue 1000000000000000000
```

Result:
```bash
Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233
Wallet balance: 47.19781847552809
Wallet nonce: 6154
Contract name: InterchainTokenFactory
Contract address: 0x83a93500d23Fbc3e82B410aD07A6a9F7A0670D66
Action: registerCustomToken
tokenId: 0x4e8a489283ac9adcb7eff67643a0ac341adfa961a86caded0b58e5ec0976717f
Token name: PruvTk
Token symbol: PRUVTK
Token decimals: 18
registerCustomToken tx: 0x6e1fbf51d881fe72ff9126ef227834f448ac4a80a2e2797b5e500f63f40bdf5b
```


### Link Custom Token With New Salt To Connect To The Token Address On Stellar

Command:

```bash
 % ts-node evm/interchainTokenFactory.js --action linkToken --chainNames avalanche --destinationChain flow --destinationTokenAddress 0xe6F6094fDf8Fd1B55e2c4A3fa904c8fF9789A1c9 --tokenManagerType 4 --linkParams 0x --salt "pruvtk_upgrade_10" --gasValue 500000000000000000 --env testnet --yes
```

Result:

```bash
Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233
Wallet balance: 47.19220633966702 
Wallet nonce: 6158
Contract name: InterchainTokenFactory
Contract address: 0x83a93500d23Fbc3e82B410aD07A6a9F7A0670D66
Action: linkToken
Human-readable destination token address: 0xe6F6094fDf8Fd1B55e2c4A3fa904c8fF9789A1c9
tokenId: 0x4e8a489283ac9adcb7eff67643a0ac341adfa961a86caded0b58e5ec0976717f
linkToken tx: 0xa330ba408165a1855a2319c2f0d7842d68fd447f224580893240b34d36621f88
```

### Obtain Token Manager Address

Command

```bash
 % ts-node evm/its.js token-manager-address 0x4e8a489283ac9adcb7eff67643a0ac341adfa961a86caded0b58e5ec0976717f -n avalanche,flow --env testnet
```

Result:

```bash
Environment: testnet
Chain: Avalanche
Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233
Contract address: 0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C
TokenManager address for tokenId: 0x4e8a489283ac9adcb7eff67643a0ac341adfa961a86caded0b58e5ec0976717f: 0xF916a6D450b0f4BD53558C4bCD0113972BE766C7
TokenManager for tokenId: 0x4e8a489283ac9adcb7eff67643a0ac341adfa961a86caded0b58e5ec0976717f exists at address:: 0xF916a6D450b0f4BD53558C4bCD0113972BE766C7

Chain: Flow
Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233
Contract address: 0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C
TokenManager address for tokenId: 0x4e8a489283ac9adcb7eff67643a0ac341adfa961a86caded0b58e5ec0976717f: 0xF916a6D450b0f4BD53558C4bCD0113972BE766C7
TokenManager for tokenId: 0x4e8a489283ac9adcb7eff67643a0ac341adfa961a86caded0b58e5ec0976717f exists at address:: 0xF916a6D450b0f4BD53558C4bCD0113972BE766C7
```

### Transfer Mintership

Command:

```bash
 % ts-node evm/its.js transfer-mintership 0x73D01a07D0Ca8E0b01cc6465E8d122D229acbC8a 0xF916a6D450b0f4BD53558C4bCD0113972BE766C7 --env testnet -n avalanche
```

Result:
```bash

```

Command:

```bash
 % ts-node evm/its.js transfer-mintership 0xe6F6094fDf8Fd1B55e2c4A3fa904c8fF9789A1c9 0xF916a6D450b0f4BD53558C4bCD0113972BE766C7 --env testnet -n flow
```

Result:

```bash

```

### Test New Interchain Token

Command:

```bash
 % ts-node evm/its.js interchain-transfer flow 0x4e8a489283ac9adcb7eff67643a0ac341adfa961a86caded0b58e5ec0976717f 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0.001 --chainNames avalanche --gasValue 500000000000000000 --env testnet --yes
```

Result:

```bash

```
