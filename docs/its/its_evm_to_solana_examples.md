# Token Deployment Commands

For all the commands you can use the -y flag to automatically answer yes to any y/n prompts

## Deploy Remote Interchain Token

First deploy an interchain token locally on EVM:

```bash
ts-node evm/interchainTokenFactory.js deploy-interchain-token -s 5009 --name "MyToken" --symbol "MTK" --decimals 18 --initialSupply 1000000 --minter 0x0000000000000000000000000000000000000000 -n avalanche-fuji --env devnet-amplifier
```

And then bridge it over to another chain (e.g. Solana) using the same salt:

```bash
ts-node evm/interchainTokenFactory.js deploy-remote-interchain-token solana-18 -s 5009 -n avalanche-fuji --env devnet-amplifier
```

## Interchain Transfer

Transfer some tokens over to another chain using the `tokenId` of the previously deployed interchain token (you can get the tokenId from the output of the `deploy-remote-interchain-token` or the `deploy-interchain-token script`):

```bash
ts-node evm/its.js interchain-transfer --destinationChain solana-18 --tokenId 0xd288b98a469604b26d2a0a6676e60e92c492f19416457f51f5e47d3c932d1c3a --destinationAddress 3xWeeoom7LUpfML89bSVet5TZ8QmTqM6B6uwS2MjYPS9 --amount 1000 -n avalanche-fuji --env devnet-amplifier
```

Note that the token needs to first be deployed to the remote chain as well using `deploy-remote-interchain-token`

## Link Custom Token

This is used to link two custom tokens, one manually deployed in the source chain (e.g. EVM, could be deployed using remix etc), and one manually deployed in the destination chain (e.g. Solana)

Custom Token example contract in solidity that can be easily deployed (e.g. through Remix):

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract ExampleCustomToken is ERC20 {
    constructor() ERC20("ExampleCustomToken", "ECT") {
        _mint(msg.sender, 1000000 * 10 ** decimals());
    }
}
```

To register that custom token in Avalanche use the following script:

```bash
ts-node evm/interchainTokenFactory.js register-custom-token --tokenAddress 0x163af8D66B3cE1CFa602bD13006887E172F6287f -
-tokenManagerType 2 --operator 0xba76c6980428A0b10CFC5d8ccb61949677A61233 -s 420  -n avalanche-fuji
```

Where operator can be your address, salt is whatever you choose and tokenManagerType is 2 meaning LOCK_UNLOCK (since this is the source chain we need to lock/unlock the real tokens that are being used in their wrapped version), and then we will specify 4 being MINT_BURN in the destination chain (so that we can mint and burn wrapped tokens)

To create a custom token on Solana follow the following steps:

```bash
spl-token create-token --decimals 9
```

Note the mint address from the previous step and use it to create an account:

```bash
spl-token create-account <MINT_ADDRESS>
```

Mint some tokens:

```bash
spl-token mint <MINT_ADDRESS> 1000000
```

Register the metadata for the token on Solana ITS Hub:

```bash
solana/cli send its register-token-metadata --mint 6tcpPCLdHyu5BXhtfCHHBzpixv5mpQP1miCw969kaSMH --gas-value 10000000
```

Register the metadata for the token on EVM ITS Hub:

```bash
ts-node evm/its.js register-token-metadata 0x163af8D66B3cE1CFa602bD13006887E172F6287f -n avalanche-fuji --env devnet-amplifier
```

Then perform the linking, using no params ("0x") and the correct tokenManagerType:

```bash
ts-node evm/interchainTokenFactory.js link-token --destinationChain solana-18 --destinationTokenAddress 6tcpPCLdHyu5BXhtfCHHBzpixv5mpQP1miCw969kaSMH --tokenManagerType 4 --linkParams "0x" -s 420 -n avalanche-fuji --env devnet-amplifier
```

Finally using that `token_id`, you can also perform an Interchain Transfer using the same command as before between the two linked tokens. You will need to perform the following commands before the interchain transfer:

Get the TokenManager PDA address:

```bash
solana/cli query its token-manager <TOKEN_ID>
```

Transfer mint authority to the TokenManager:

```bash
spl-token authorize <SOLANA_MINT_ADDRESS> mint <TOKEN_MANAGER_PDA>
```

## Register Canonical Token

First deploy a canonical token on EVM (e.g. ERC20 with Remix), and then we register it on EVM:

```bash
ts-node evm/interchainTokenFactory.js register-canonical-interchain-token 0x1D786A2de31B7ED1D3861Cba22D9f2992A0AA5d6 -n avalanche-fuji --env devnet-amplifier
```

Then deploy the remote canonical interchain token to Solana:

```bash
ts-node evm/interchainTokenFactory.js deploy-remote-canonical-interchain-token 0x1D786A2de31B7ED1D3861Cba22D9f2992A0AA5d6  solana-18 -n avalanche-fuji --env devnet-amplifier
```
