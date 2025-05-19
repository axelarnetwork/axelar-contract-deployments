# Interchain Token Service (ITS)

This is the Solana implementation of the Interchain Token Service. From the [EVM](https://github.com/axelarnetwork/interchain-token-service) reference implementation:

> The interchain token service is meant to allow users/developers to easily create their own token bridge. All the underlying interchain communication is done by the service, and the deployer can either use an `InterchainToken` that the service provides, or their own implementations.
> There are quite a few different possible configurations for bridges, and any user of any deployed bridge needs to trust the deployer of said bridge, much like any user of a token needs to trust the operator of said token. We plan to eventually remove upgradability from the service to make this protocol more trustworthy. Please reference the [design description](https://github.com/axelarnetwork/interchain-token-service/blob/main/DESIGN.md) for more details on the design. Please look at the [docs](https://github.com/axelarnetwork/interchain-token-service/blob/main/docs/index.md) for more details on the contracts.

---

## Comparison with the EVM Reference Implementation

On EVM, the ERC-20 standard is used for tokens, while Solana uses the SPL standard. Deploying an ERC-20 token on EVM involves creating a new contract instance, whereas on Solana, a new mint account is created due to its account model.

In EVM, an `InterchainToken` is an implementation of the ERC-20 standard, allowing multiple minters. This enables the ITS contract to be granted the _minter_ role. On Solana, a mint account representing an SPL token can have only a **single** mint authority. For `TokenManager` types such as `NativeInterchainToken`, `Mint/BurnFrom`, and `Mint/Burn`, which require minting and burning tokens during interchain transfers, the mint authority must be assigned to ITS (technically, to the `TokenManager` created for the token).  

Consequently, `InterchainToken`s deployed with these `TokenManager` types on Solana have ITS set as their mint authority, with an additional `minter` optionally defined during deployment. This minter can mint tokens but must use the proxy mint instruction in the ITS program instead of the standard `spl-token` instruction. ITS also provides instructions to transfer this internal minter role to another account. Note that the `mint_authority` on the SPL token cannot be changed once transferred to the `TokenManager`.

---

## PDA Structure

This is a list of [Program Derived Addresses (PDAs)](https://solana.com/docs/core/pda) used within ITS. The list follows a hierarchy in terms of dependency for derivation (i.e.: the second PDA in the list uses the first in its derivation, the third uses the second, so on and so forth), with exception of the User Roles PDA, which depends on the resource the role is being tracked, either ITS Root Config PDA or Token Manager PDA.

| PDA | Description | Owner | Deriving function | State |
|-----|-------------|-------|-------------------|-------|
| Gateway Root Config | This is a singleton PDA that addresses an account that keeps the state of the Gateway | Gateway program | [get_gateway_root_config_pda](../axelar-solana-gateway/src/lib.rs#L77) | [GatewayConfig](../axelar-solana-gateway/src/state/config.rs#L21) |
| ITS Root Config | This is a singleton PDA that addresses an account that keeps the state of ITS. | ITS program | [find_its_root_pda](./src/lib.rs#L132) | [InterchainTokenService](./src/state/mod.rs) |
| Interchain Token | This is the address used for the mint accounts created by ITS (Native Interchain Tokens). | ITS program | [find_interchain_token_pda](./src/lib.rs#L274) | [Mint](https://docs.rs/spl-token-2022/latest/spl_token_2022/state/struct.Mint.html) |
| Token Manager | Addresses for Token Manager accounts. | ITS program | [find_token_manager_pda](./src/lib.rs#L197) | [TokenManager](./src/state/token_manager.rs) |
| Flow Slot | These are addresses for accounts that track the flow of an interchain token. | ITS program | [find_flow_slot_pda](./src/lib.rs#L311) | [FlowSlot](./src/state/flow_limit.rs) |
| User Roles | These are addresses for accounts that track user roles (Minter, FlowLimiter, Operator) on resources (ITS Root Config, TokenManager). | ITS program | [find_user_roles_pda](../../helpers/role-management/src/lib.rs#L68) | [UserRoles](../../helpers/role-management/src/state.rs#L43) |

---

## Interaction with Different Token Standards

When bridging a token, the main consideration is how it is represented on the source and destination chains. ERC-20 tokens typically use 18 decimals, whereas SPL tokens usually have 9 decimals. While higher decimals are possible, they can hinder usability on Solana, where token amounts are represented using unsigned 64-bit integers.

Additionally, some token standards impose limitations on token metadata, such as the name, symbol, and other properties. See [Metadata Limitations](#metadata-limitations) for details on Solana's restrictions.

---

## Custom Token Linking

Custom token linking on Axelar involves deploying `TokenManager`s for existing tokens. The [axelar-examples](https://github.com/axelarnetwork/axelar-examples) repository includes an [example](https://github.com/axelarnetwork/axelar-examples/blob/main/examples/evm/its-custom-token/index.js) for linking custom tokens between EVM chains. The same process applies when linking tokens between EVM and Solana.

As noted earlier, mint accounts on Solana can have only one mint authority. Therefore, when deploying a `Mint/Burn` or `Mint/BurnFrom` `TokenManager` on Solana, the mint authority role must be transferred to the `TokenManager` PDA.  

- **Local Deployment**: This transfer is handled automatically, requiring the _payer_ to be the current mint authority.  
- **Remote Deployment**: If the `TokenManager` is deployed via a message from another chain, the mint authority must be manually transferred using the [`SetAuthority`](https://docs.rs/spl-token-2022/latest/spl_token_2022/instruction/enum.TokenInstruction.html#variant.SetAuthority) instruction from the `spl-token(-2022)` program. Failure to do so will prevent the token bridge from functioning, as the `TokenManager` cannot mint tokens for interchain transfers.

The [from_solana_to_evm.rs](https://github.com/eigerco/solana-axelar/blob/main/solana/programs/axelar-solana-its/tests/module/from_solana_to_evm.rs) test module includes several examples of token linking with different `TokenManager` types, which can serve as a guide for using `axelar-solana-its` instructions.

---

## Token Metadata

Unlike ERC-20 tokens, SPL tokens do not natively include metadata such as name, symbol, or URI. The `spl-token-2022` program introduces extensions, including `TokenMetadata` and `MetadataPointer`, to add this information to mint accounts.  

However, the Solana ecosystem has historically addressed this gap using the Metaplex `mpl-token-metadata` program, which remains the most common method for managing token metadata. Consequently, `InterchainToken`s deployed on Solana follow the Metaplex metadata specification. During deployment, the metadata is created via the `mpl-token-metadata` program.

### Metadata Limitations

Some basic limitation exists when creating the token metadata on Solana:

- Maximum length for the `name`: 32
- Maximum length for the `symbol`: 10
- There is also a maximum length of 200 for the `uri`, but `uri` is currently not used by the ITS protocol.

When deploying an `InterchainToken`, if any of these limits are not respected, the transaction will fail.

---

## Calling an External Contract with a Token Transfer

The ITS implements functionality to allow token transfers to carry instruction data, enabling tokens to be transferred to a contract which is then executed in the same transaction. The destination program has to implement the required interfaces. The [axelar-solana-memo-program](https://github.com/eigerco/solana-axelar-internal/blob/main/solana/programs/axelar-solana-memo-program/src/processor.rs#L38) implements it, you can see it in action in the [ITS tests](https://github.com/eigerco/solana-axelar-internal/blob/main/solana/programs/axelar-solana-its/tests/module/from_evm_to_solana.rs#L144).

The diagram below shows how the message flows from the EVM ITS contract to a Solana program while also showing how the message is structured (click to open on Excalidraw an be able to zoom-in).

[![EVM->Solana](https://github.com/user-attachments/assets/bf0bf75e-3acc-404e-8425-0d7779a2893b)](https://excalidraw.com/#json=0lVeKwoyvgoGjZqq37iVP,dHVboAUXzZ-tsfo2KUizpQ)

When calling a contract on the Solana chain using this flow, the Solana instruction should be serialized using Borsh (as expected by the solana program). Due to the Solana account model, the accounts required by the instruction should also be provided. The [AbiSolanaGateway](../../../evm-contracts/src/SolanaGatewayPayload.sol#L20) solidity library can be used directly or as a guide for creating the executable payload to send from EVM to Solana. This payload should then be used to populate the `data` field of the `InterchainTransfer` message.

When calling a contract on EVM from Solana, encoding the call data with ABI encoding and populating the `data` field of the `InterchainTransfer` message is enough.

## Relationship with other Axelar Solana Programs

### Gateway

ITS leverages the Axelar GMP protocol and therefore follows the same approval and validation process through the Gateway (see [image above](#calling-an-external-contract-with-a-token-transfer)). However, unlike regular cross-chain contract calls, ITS messages must adhere to the ITS protocol. Specifically, the payload of each GMP message must match one of the message types defined in the [ITS design document](https://github.com/axelarnetwork/interchain-token-service/blob/main/DESIGN.md) and must be ABI-encoded.

Because of these requirements, we cannot use [SolanaGatewayPayload](../../../evm-contracts/src/SolanaGatewayPayload.sol) for ITS messages. This raises an important question: **How do we provide the necessary accounts to Solana ITS?** To solve this, we created a helper crate, [its-instruction-builder](../../helpers/its-instruction-builder/src/lib.rs). It exposes a function that parses the original ITS message and queries the Solana blockchain via RPC, gathering all required accounts and creating the corresponding instructions for the Solana ITS.

### Gas Service

ITS uses the Axelar GMP protocol, and thus gas is paid as any other message on the network. For more info on the Gas Service, please check its [README](../axelar-solana-gas-service/README.md).
