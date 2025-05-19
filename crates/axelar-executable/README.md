# Axelar Executable

If we look at the event data that the EVM Gateway produces when making a [`gateway.callContract()`](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/432449d7b330ec6edf5a8e0746644a253486ca87/contracts/interfaces/IAxelarGateway.sol#L19-L25) call when communicating with an external chain:
- `destinationContractAddress` is the identifier of _where_ the Message should land on the destination chain.
- `payload` is the payload data used for the contract call. The raw data that the Relayer must send to the destination contract
- `payloadHash` is the `keccak256` hash of the sent payload data, used by the recipient to ensure that the data is not tampered with.

Relayer for the edge chain **must** know how to call a contract on the given edge chain so that the contract can properly process the Axelar GMP message. The same concept applies to Solana, that every contract that wants to be compatible with the Axelar protocol **must** implement the `axelar-executable` interface. Implementing the `axelar-executable` interface allows the Relayer to send transactions to it.

![high level interactoin](https://github.com/user-attachments/assets/c676c8c6-8867-4560-a0cc-fa476b2b21a7)

1. Relayer will compose a transaction that can interact with the destination program on Solana
2. `axelar-executable` exposes a function that allows the program to parse the Message into a format that it can work with
3. `axelar-executable` exposes a utility function that validates that all the relayer-provided accounts are valid and not malicious. It then performs a CPI call to the Solana Gateway to set the message status to `Executed`.


## Solana specific rundown

> [!NOTE]
> For better clarity, read the following Solana docs:
> - [`Solana Account Model`](https://solana.com/docs/core/accounts)
> - [`Solana Transactions and Instructions`](https://solana.com/docs/core/transactions)
> - [`Solana CPI`](https://solana.com/docs/core/cpi)
> - [`Solana PDAs`](https://solana.com/docs/core/pda)

**Accounts**

A notable difference between Solana and EVM chains is that every interaction with some Solana programs must define an array of `accounts[]`. Accounts can be looked at as on-chain storage memory slots from Solidity. A contract can only read & write to the accounts provided when the instruction is created on the relayer/user level. The `accounts[]` must also include all the accounts that any internal CPI calls may require. Every storage slot the instruction may touch must be visible to the entity that crafts the transaction before submitting the TX for on-chain execution.

**PDAs**

Programs can have their accounts, called Program Derived Addresses (PDA for short), which sets the program ID as the owner. A contract can use PDAs to store data up to 10kb, or be a signer to make actoins on the behalf of a program. This can be imagined as having contract storage that only the on-chain program can modify. The PDAs must be deterministically derived.

**CPI**

Solana programs can call other programs (CPI - cross-program invocation), and a PDA can sign the calls on behalf of a program (this is because the program ID cannot be a signer itself; only its PDAs can be signers). Also, the CPI calls require a list of accounts to be proxied from the top-level call for the CPI call to operate correctly.

Key points:
- If the proper accounts are not provided for a contract interaction, then **the call will fail**.
- The array of accounts **must** be known by the Relayer before calling the destination contract.
- The on-chain logic must validate that the provided accounts have been properly derived and are not malicious (e.g. checking if they were derived correctly, checking the expected owners, etc.)
- A Solana contract (represented by a `program_id`) cannot be a signer for a CPI call, but a PDA owned by the given `program_id` can be a signer. This is important for when the program makes a `gateway.validate_call()` CPI call

## Providing the `accounts[]` in the payload 

Solana identifies an instance of the on-chain program by the contract address AND the provided accounts. This means that by having a different array of accounts passed to a program, we may communicate to a completely different instance of the same program.

For example, the [SPL token program](https://spl.solana.com/token#creating-a-new-token-type) is a singular program everyone uses to create their on-chain tokens. By having a different token mint account, you are effectively talking to a different token — all while the `program_id` has not even changed.

This means that the base interface for GMP messages defined by the Axelar protocol is not expressive enough for communicating with Solana programs - because there’s no place to put the account arrays. Therefore a workaround is needed, where we need to provide the information about the accounts within the scope of the existing `ContractCall` data structure.

Solanas `axelar-executable` now expects that the `payload` emitted on the source chain also includes all the `account[]` data for the Relayer to create a proper transaction. If the `account[]` is absent on the GMP call, the Relayer cannot know what accounts to provide when calling the destination contract.

![Axelar Message Payload](https://github.com/user-attachments/assets/29a96677-83f5-4727-befa-3f815e31ad39)


The source chain that wants to interact with Solana must encode the messages in a specific format that the Solana destination contract understands, and the Relayer can understand. `Accounts[]` requirement also lets the Relayer deterministically derive the accounts when crafting the transaction.

Currently, the `[0]th` byte of the payload indicates the encoding that specifies how the rest of the data is encoded:

| value | meaning |
|--|--|
| 0b00000000 | The rest of the data is Borsh encoded |
| 0b00000001 | The rest of the data is ABI encoded |

The way how accounts and the payload are encoded is encoding-specific. The `axelar-executable` and the Relayer maintainers can add new encoding support over time. As new chains get added to Axelar, they may not play nicely with ABI or Borsh (or just be expensive to compute). Making the encoding flexible gives us room to support new encodings in the future. 

## Examples

| Item | Explanation |
|--|--|
| [`SolanaGatewayPayload.sol`](https://github.com/eigerco/solana-axelar/blob/main/evm-contracts/src/SolanaGatewayPayload.sol) | Can be used to encode the accounts together with the actual payload in Solidity so that the Solana relayer can properly interpret them |
| [`AxelarMemo.sol`](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/evm-contracts/src/AxelarMemo.sol#L46) | An example contract showcasing how the Solidity contract would send a `string` message to Solana, while prefixing some arbitrary accounts  |
| [`AxelarSolanaMemo::processor`](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/programs/axelar-solana-memo-program/src/processor.rs#L33-L36) | Solana program example that implements `axelar-executable`; receives a string message and prints all the accounts it has received |
| [end to end unittest](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/programs/axelar-solana-memo-program/tests/evm-e2e/from_evm_to_solana.rs#L202) | A unit test you can run locally and see for yourself how it works together | 

## Differences from EVM

| Action | EVM Axelar Executable | Solana Axelar Executable |
|--|--|--|
| Interface that the contract must implement | EVM contracts must inherit the [AxelarExecutable.sol](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/432449d7b330ec6edf5a8e0746644a253486ca87/contracts/executable/AxelarExecutable.sol) base contract for the Relayer to be able to call it.  | On Solana, there is no concept for inheriting contracts. There is only a single entry point for a Solana program, and the branching logic of how the raw bytes are interpreted is up to the contract logic. On Solana, a contract developer **must** try to parse the incoming bytes using [`parse_axelar_message()`](https://github.com/eigerco/solana-axelar-internal/blob/aee979d2c83d875a9255d73b4eb31eb0f38e544d/solana/crates/axelar-executable/src/lib.rs#L282) before attempting any other action.  | 
| Relayer calling `destination_contract.execute()` | The Relayer will encode the call with the full payload and call the destination contract; the `AxelarExecutable.sol` interface allows to process the incoming payload | The Relayer will split the raw payload from the `accounts[]` because the tx layout requires them to be split. It will encode the payload in a way that allows the destination contract to parse it using this library |
| Calling `validate_message()` | The `AxelarExecutable.sol` contract takes care of the internal cross-contract call to the Gateway to validate that the message has been approved | The Solana contract developer *must* immediately call [`validate_message()`](https://github.com/eigerco/solana-axelar-internal/blob/aee979d2c83d875a9255d73b4eb31eb0f38e544d/solana/crates/axelar-executable/src/lib.rs#L47) form the `axelar-executable` library. This internally will make a CPI call to the Gateway using the messages command id to create a short-lived PDA that will be the signer of the call.  |
| Providing the raw payload | As tx arguments | Raw payload is stored on a PDA owned by the Gateway; it must be read from there |
| Providing the raw `accounts[]` from the original payload | _No such concept_ | Relayer will parse the payload, extract the provided accounts and append them in the order that they were provided in the original message |
| Providing accounts for the raw payload, gateway, etc | _No such concept_ | Relayer will prefix hardcoded accounts for the raw payload PDA |
| Validating payload hash | `AxelarExecutable.sol` will take care of hashing the raw payload from tx args and comparing it with the provided payload hash | `axelar_executable::validate_message` will validate all the arguments passed to the instruction (including accounts)  match the ones defined on `MessagePayload PDA`, and ensure that the data hashes match. |

You can see the anatomy of an instruction that the Solana Relayer will send to the destination program when sending the raw payload to it:

![Anatomy of an ix](https://github.com/user-attachments/assets/0312abb4-fe7f-45c7-a8ae-1318489da9d2)


## Exceptions of the `accounts[]` rule: ITS & Governance

[Interchain Token Service](https://github.com/axelarnetwork/interchain-token-service/blob/main/DESIGN.md#interchain-tokens) and [Governance contract](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/432449d7b330ec6edf5a8e0746644a253486ca87/test/utils.js#L24-L44) have a legacy ABI interface that must be respected. This means that we cannot enforce arbitrary new encoding for the existing protocols; we only need them to be able to interact with Solana. As a result, the Relayer has special handling when interacting with ITS & Governance contracts; it will decode the `abi` encoded messages, introspect into the message contents and attempt to deterministically derive all the desired accounts for the action the message wants to make. This approach only works when the message layout is known beforehand (meaning that the Relayer can decode it) AND the Relayer has hardcoded custom logic to derive the accounts. This means that this special handling is not possible for the generic case.

Also, ITS & Governance use a different entry point on [`axelar_executable::validate_with_gmp_metadata`](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/crates/axelar-executable/src/lib.rs#L128C8-L128C34). The only difference is that the `accounts[]` validation is no longer done by `axelar_executable`. Instead, it becomes the responsibility of the destination contract itself to validate the provided `accounts[]`.
