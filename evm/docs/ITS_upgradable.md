# Custom Token Upgrade Procedures


This document presents possible flows for upgrading custom ERC20 token contracts that have been connected to ITS and have ITS tokens deployed to other axelar connected chains.


## Scenario 1: ERC20 is directly upgradable on source chain and was connected to other chains via ITS by deploying ITS tokens

### Assumptions

 * Developer team will have deployed a custom ERC20 contract that is directly upgradable.
 * The existing ERC20 will have already been connected to ITS by registering the token as a canonical token and remotely deployed to other chains as a standard ITS Tokens using ITS portal.
 * Only a single chain (the source chain) has the custom contract deployed to it, and all other connected chains have standard ITS tokens deployed to them.
 * Token liquidity should be preserved post upgrade of the ERC20 on the source chain. 

### Procedure

1. Change flow limit to 0 to freeze all cross-chain transfers for old tokenID
2. Upgrade the source chain ERC20 contract
3. Deprecate the existing ITS ERC20 contracts deployed to destination chains and deploy new ITS contracts on each connected chain (new addresses generated). Depending on liquidity management the following choices are available
    * If users are expected to redeem the old tokens in order to get new tokens, we would need to re-enable cross-chain transactions where the old tokens can be sent back to the source chain. The tokens would flow through the old Token manager unlocking the tokens, which can be sent back to destination chain via a new token manager.
    * If users are not expected to redeem the tokens, the new ITS contracts can be created with a substitute supply that can be distributed to each token holder on the destination chains. For this approach it is necessary to transfer an equivalent amount of tokens on the source chain to the new Token Manager to lock tokens that can be redeemed whenever users send tokens back to source chain using a newly created tokenId.
4. Connect the new ITS contracts on the destination chains with the upgraded contract on the source chain using custom token linking:
5. For the newly created token managers on the destination chains, if they are using mint/burn, transfer mintership to the token manager.


## Scenario 2: ERC20 is directly upgradable on both source and destination chains and was connected via ITS Custom Token Linking

### Assumptions

 * Developer team will have deployed a custom ERC20 contract that is directly upgradable on all chains the token is integrated with.
 * Contract must be able to grant mintership role to another address
 * The individual contracts are connected using custom token linking using mint/burn token managers. Developer team is in charge of performing the necessary steps and axelar will provide instructions and scripts.
 * Developer team has identified a single chain to act as the source chain for creating all connections.

### Procedure

1. Change flow limit to freeze all cross-chain transfers for current tokenId
2. Upgrade the source chain ERC20 contracts
3. Upgrade all the destination ERC20 contracts
4. Burning and reminting of tokens on any affected chain should be managed directly by developer team via contract mintership role.  
5. Re-enable flow limits (tokenId has not changed).

## Scenario 3: Contracts are not directly upgradable

If the custom ERC20 contract can't be directly upgraded and new contracts must be deployed then it is mandatory that the contracts be deployed on source and destination chain and that the tokens be connected using custom token linking. The contracts must also allow the owners/operators to burn the supply since the old contracts will be deprecated and substituted by new contracts on source and destination chain connected via custom token linking mechanism. 


### Custom Token Linking Process
Once contracts are deployed on source and destination chain:
 * On source chain and destination chain the token metadata must be registered 
```bash
ts-node evm/its.js register-token-metadata <tokenAddress>
```

 * From the source chain the upgrade contract must be registered as custom token. Must define unique salt and deployer to manage connection moving forward.
   * This step creates the new token manager on the source chain. Depending on liquidity management strategy, different token manager types may be selected. 

```bash
ts-node evm/interchainTokenFactory.js --action registerCustomToken --tokenAddress [tokenAddress] --tokenManagerType 4 --operator [wallet] --salt "salt6789"
```

 * From the source chain the upgrade contract is linked to each new ITS contract on the destination chains. During linking process must use same salt and deployer as previous steps.
    * On this step it is necessary to define the token manager of the destination chains (mint/burn or lock/unlock)

```bash
ts-node evm/interchainTokenFactory.js --action linkToken --destinationChain chain2 --destinationTokenAddress [remote token address] --tokenManagerType 4 --linkParams "0x" --salt "salt6789"
```

 * If any of the token managers was deployed as Mint/Burn the token mintership must be transfered to the token manager. If the contracts are ITS standard tokens the following procedure can be applied
    * Fetch tokenManager address for deployed token on target chain

```bash
ts-node evm/its.js token-manager-address <tokenId>
```
    * Transfer mintership for the token to the token manager retrieved in previous step

```bash
ts-node evm/its.js transfer-mintership <tokenAddress> <tokenManagerAddress>
```
