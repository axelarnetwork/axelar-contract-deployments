# Axelar Solana Gateway

> [!NOTE]
> Mandatory reading prerequisites:
> - [`Solidity Gateway reference implementation`](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/432449d7b330ec6edf5a8e0746644a253486ca87/contracts/gateway/INTEGRATION.md) developed by Axelar.
>
> Important Solana details are described in the docs:
> - [`Solana Account Model`](https://solana.com/docs/core/accounts)
> - [`Solana Transactions and Instructions`](https://solana.com/docs/core/transactions)
> - [`Solana CPI`](https://solana.com/docs/core/cpi)
> - [`Solana PDAs`](https://solana.com/docs/core/pda)
> 
> 👆 a shorter-summary version is available [on Axelar Executable docs](../../crates/axelar-executable/README.md#solana-specific-rundown).

When integrating with it, you are not expected to be exposed to the Axelar Solana Gateway's inner workings and security mechanisms. 
- To receive GMP messages from other chains, read [Axelar Executable docs](../../crates/axelar-executable/README.md).
- To send messages to other chains, read [Sending messages from Solana](#sending-messages-from-solana).

## Sending messages from Solana

Here, you can see the entire flow of how a message gets proxied through the network when sending a message from Solana to any other chain:

![Solana to other chains](https://github.com/user-attachments/assets/61d9934e-221a-4858-be62-a70c5a12d21d)

A CPI must be made to the Axelar Solana Gateway for a destination contract to communicate with it.
- On Solana, there is no `msg.sender` concept as in Solidity.
- On Solana `program_id`'s **cannot** be signers.
- On Solana, only PDAs can sign on behalf of a program. The only way for programs to send messages is to create PDAs that use [`invoke_signed()`](https://docs.rs/solana-cpi/latest/solana_cpi/fn.invoke_signed.html) and sign over the CPI call.
- The interface of `axelar_solana_gateway::GatewayInstruction::CallContract` instruction defines that the first account in the `accounts[]` must be the `program_id` that is sending the GMP payload.
The second account is a `signing PDA`, meaning the source program must generate a PDA with specific parameters and sign the CPI call for `gateway.call_contract`. This Signature acts as an authorization token that allows the Gateway to interpret that the provided `program_id` is indeed the one that made the call and thus will use the `program_id` as the sender.


| PDA name | description | users | notes | owner |
| - | - | - | - | - |
| [CallContract](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/lib.rs#L312-L317) | This acts only as a signing PDA, never initialized; Permits the destination program to call `CallContract` on the Gateway | Destination program will craft this when making the CPI call to the Gateway | Emulates `msg.sender` from Solidity | Destination program |

[Full-fledged example](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-memo-program/src/processor.rs#L123-L157): Memo program that leverages a PDA for signing the `Call Contract` CPI call.

[Full-fledged example](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-memo-program/src/processor.rs#L164-L198): Memo program that leverages a PDA for signing the `Call Contract Offchain Data` CPI call.

| Gateway Instruction |  Use Case | Caveats |
| - | - | - |
| [Call Contract](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/instructions.rs#L52-L67) | When you can create the data fully on-chain. Or When the data is small enough to fit into tx arguments  | Even if you can generate all the data on-chain, the Solana tx log is limited to 10kb. And if your program logs more than that, there won't be any error on the transaction level. The log will be truncated, and the message will be malformed. **Please be careful when making this API call.**  |
| [Call Contract Offchain Data](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/instructions.rs#L69-L85) | When the payload data cannot be generated on-chain or it does not fit into tx size limitations. This instruction only requires the payload hash. The full payload is expected to be provided to the Relayer directly | Whether the payload gets provided before or after sending this instruction is fully up to the Relayer and not part of the Gateway spec. |

### Axelar network steps

After the Relayer sends the message to Amplifier API, Axelar network and `ampd` perform all the validations.

![image](https://github.com/user-attachments/assets/e7a137e7-6545-4161-be7e-91ec9d6223a5)

- Relevant `ampd` code is located [here, axelar-amplifier/solana/ampd](https://github.com/eigerco/axelar-amplifier/tree/solana/ampd)
- `ampd` will query the Solana RPC network for a given tx hash (in Solanas case, it's the tx signature, which is 64 bytes)
  - retrieve the logs, parse the logs using [`gateway-event-stack` crate](https://github.com/eigerco/solana-axelar/tree/next/solana/crates/gateway-event-stack), and then try to find an event at the given index. If the event exists and the contents match, then `ampd` will produce signatures for the rest of the Axelar network to consume.

## Receiving messages on Solana

Receiving messages on Solana is more complex than sending messages. There are a couple of PDAs involved in the process.

![image](https://github.com/user-attachments/assets/43e0ac3b-04e9-4d76-9075-8b325aec278b)

| PDA name | description | users | notes | owner |
| - | - | - | - | - |
| [Gateway Config](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/state/config.rs) | Tracks all the information about the Gateway, the verifier set epoch, verifier set hashes, verifier rotation delays, etc.  | This PDA is present in all the public interfaces on the Gateway. Relayer and every contract is expected to interact with it | | Gateway |
| [Verifier Set Tracker](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/state/verifier_set_tracker.rs) | Tracks information about an individual verifier set | Relayer, when rotating verifier sets; Relayer, when approving messages; | Solana does not have built-in infinite size hash maps as storage variables, using PDA for each verifier set entry allows us to ensure that duplicate verifier sets never get created | Gateway |
| [Signtautre Verification Session](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/state/signature_verification_pda.rs) | Tracks that all the signatures for a given payload batch get verified | Relayer uses this in the multi-tx message approval process, where each Signature from a verifier is sent individually to the Gateway for verification | | Gateway |
| [Incoming Message](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/state/incoming_message.rs) | Tracks the state of an individual GMP message (executed/approved + metadata). | Relayer - After all the signatures have been approved, each GMP message must be initialized individually as well, and the Relayer takes care of that. The destination program will receive this PDA in its `execute` flow when receiving the payload | | Gateway |
| [Message Payload](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/state/message_payload.rs) | Contains the raw payload of a message. Limited of up to 10kb. Directly linked to an `IncomingMessage` PDA. | Relayer will upload the raw payload to a PDA and, after message execution (or failure of execution), will close the PDA, regaining all the funds. The destination program will receive this PDA in its `execute` flow. | Solana tx size limitation prevents sending large payloads directly on the chain. Thus, the payload is stored directly on-chain | Gateway; the Relayer that created this PDA can also close it |
| [Validate Call](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/lib.rs#L286-L291) | This acts only as a signing PDA, never initialized; Permits the destination program to set `IncomingMessage` status to `executed`; | Destination program will craft this when making the CPI call to the Gateway | Emulates `msg.sender` from Solidity | Destination program |

### Signature verification

**Prerequisite:** initialized `Gateway Root Config PDA` with a valid verifier set; active `Multisig Prover`; active `Relayer`;

![Execute Data](https://github.com/user-attachments/assets/d039ad91-b7aa-40d2-9c33-b53d3926ad22)


Due to Solana limitations, we cannot verify the desired amount of signatures in a single on-chain transaction to fulfil the minimal requirements imposed by the Axelar protocol. For detailed reading, please look at the [axelar-solana-encoding/README.md](../crates/axelar-solana-encoding/README.md#execute-data).

The approach taken here is that:
1. Relayer receives fully Merkelised data [`ExecuteData`](../crates/axelar-solana-encoding/README.md#current-limits-of-the-merkelised-implementation) from the Multisig Prover, which fulfils the following properties:
    1. we can prove that each `message` is part of the `payload digest` with the corresponding Merkle Proof
    2. we can prove that each `verifier` is part of the `verifier set` that signed the `payload digest` with the corresponding Merkle Proof
    3. each `verifier` has a corresponding Signature attached to it
  
| action | tx count | description |
| - | - | - |
| Relayer calls `Initialize Payload Verification Session` on the Gateway [[link to the processor]](https://github.com/eigerco/solana-axelar/blob/c73300dec01547634a80d85b9984348015eb9fb2/solana/programs/axelar-solana-gateway/src/processor/initialize_payload_verification_session.rs) | 1 | This creates a new PDA that will keep track of the verified signatures. The `payload digest` is used as the core seed parameter for the PDA. This is safe because a `payload digest` will only be duplicated if the `verifier set` remains the same (this is often the case) AND all of the messages are the same. Even if all the messages remain the same, `Axelar Solana Gateway` has idempotency on a per-message level, meaning duplicate execution is impossible. |
| The Relayer sends a tx [`VerifySignature` (link to the processor)](https://github.com/eigerco/solana-axelar/blob/c73300dec01547634a80d85b9984348015eb9fb2/solana/programs/axelar-solana-gateway/src/processor/verify_signature.rs). | For each `verifier` + Signature in the `ExecuteData` that signed the payload digest | The core logic is that we:  <ol><li>ensure that the `verifier` is part of the `verifier set` that signed the data using Merkle Proof. </li><li>check if the `signature` is valid for a given `payload digest` and if it matches the given `verifier` (by performing ECDSA recovery).</li><li>update the `signature verification PDA` to track the current weight of the verifier that was verified and the index of its Signature</li><li>repeat this tx for every `signature` until the `quorum` has been reached</li></ol> |

**Artefact:** We have reached the quorum, tracked on `Signature Verification Session PDA`.

### Message approval

**Prerequisite:** `Signature Verification Session PDA` that has reached its quorum.

As in the signature verification step, we cannot approve dozens of Messages in a single transaction due to Solana limitations. 

| action | tx count | description |
| - | - | - |
| Relayer calls [`Approve Message` (link to the processor)](https://github.com/eigerco/solana-axelar/blob/c73300dec01547634a80d85b9984348015eb9fb2/solana/programs/axelar-solana-gateway/src/processor/approve_message.rs). | For each GMP message in the `ExecuteData` | <ol><li>Validating that a `message` is part of a `payload digest` using Merkle Proof.</li><li>Validating that the `payload digest` corresponds to `Signature Verification PDA`, and it has reached its quorum.</li><li>Validating that the `message` has not already been initialized</li><li>Initializes a new PDA (called `Incoming Message PDA`) responsible for tracking a message's `approved`/`executed` state. The core seed of this PDA is `command_id`. You can read more about `command_id` in the [EVM docs #replay prevention section](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/gateway/INTEGRATION.md#replay-prevention); our implementation is the same.</li><li>This action emits a log for the Relayer to capture.</li><li>Repeat this tx for every `message` in a batch.</li></ol> |
  
**Artefact:** We have initialized a new `Incoming Message PDA` for each message with its state set as `approved`. There have been no changes to PDA contents for messages approved in previous batches.

### Message Execution

**Prerequisite:** `Incoming Message PDA` for a message.

![Caliing the destination program](https://github.com/user-attachments/assets/f7c1eaf9-cae7-4a74-8cea-19b17caaad0a)

[Full-fledged example](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-memo-program/src/processor.rs#L87-L103): Memo program that leverages receives a GMP message and implements `axelar-executable`

After the Relayer reports the event to Amplifier API about a message being approved, the Relayer will receive the raw payload to call the destination program. Because of Solana limitations, the Relayer cannot send large enough payloads in the transaction arguments to satisfy the minimum requirements of Axelar protocol. Therefore, the Relayer does chunk uploading of the raw data to a PDA for the end program to consume. 


| action | tx count | description |
| - | - | - |
| Relayer calls [`Initialize Message Payload` (link to processor)](https://github.com/eigerco/solana-axelar/blob/c73300dec01547634a80d85b9984348015eb9fb2/solana/programs/axelar-solana-gateway/src/processor/initialize_message_payload.rs). | 1 | The seed of the PDA is directly tied to the Relayer and the `Incoming Message PDA` (`command_id`). This means that if multiple concurrent relayers exist, they will not override each others' payload data. |
| Relayer chunks the raw payload and uploads it in batches using [`Write Message Payload`](https://github.com/eigerco/solana-axelar/blob/main/solana/programs/axelar-solana-gateway/src/processor/write_message_payload.rs). | new tx for each chunk of the payload; max size of a chunk ~800 bytes | Such an approach allows us to **upload up to 10kb of raw message data. That is the upper bound of the Solana integration**. |
| Relayer calls [`Commit Message Payload`](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/programs/axelar-solana-gateway/src/processor/commit_message_payload.rs) | 1 | Computes the hash of the raw payload. This also ensures that after the hash has been calculated & committed, the payload can no longer be mutated in place by the Relayer. |

    As a result, we now have the following PDAs:
    - `Incoming Message PDA`: contains the execution status of a message (will be `approved` state after message approval). Relationship - 1 PDA for each unique message on the Axelar network.
    - `Message Payload PDA`: contains the raw payload of a message. There can be many `Message Payload PDA`s, one for each operation relayer. Each `Message Payload PDA` points to a specific `Incoming Message PDA`.
  
Next, the Relayer must communicate with the destination program. For a third-party developer to build an integration with the `Axelar Solana Gateway` and receive GMP messages, the only expectation is for the contract to implement [`axelar-executable`](../../crates/axelar-executable/README.md) interface. This allows the Relayer PDA to have a known interface to compose and send transactions after they've been approved on the Gateway. Exception of the rule is [`Interchain Token Service`](../axelar-solana-its/README.md) & [`Governance`](../axelar-solana-governance/README.md) programs, which do not implement `axelar-executable`.

| action | tx count | description |
| - | - | - |
| Relayer calls the `destination program`| 1 | Composes a tx using `axelar-executable` |
| `Destination program` (via `axelar-executable`) Calls [`Validate Message`](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/programs/axelar-solana-gateway/src/processor/validate_message.rs). | Internal CPI of 👆 | <ol><li>The `destination program` needs to craft a `signing pda` to ensure that the given `program id` is the message's desired recipient (akin to `msg.sender` on Solidity). </li><li>`Incoming Message PDA` status gets set to `executed`</li><li>event gets emitted</li></ol>
| The Relayer can close `Message Payload PDA` using [`Close Message Payload`](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/programs/axelar-solana-gateway/src/processor/close_message_payload.rs) call. | 1 | This will return ~99% of the funds spent uploading the raw data on-chain. |

**Artifact:** Message has been successfully executed; `Incoming Message PDA` marked as `executed`; `Message Payload PDA` has been closed, and funds refunded to the Relayer.

### Verifier rotation

**Prerequisite:** `Signature Verification Session PDA` that has reached its quorum.

| action | tx count | description |
| - | - | - |
| The Relayer calls [`Rotate Signers`](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/programs/axelar-solana-gateway/src/processor/rotate_signers.rs). | 1 | <ol><li>The processor will validate the following logic:<ul><li>If the tx **was not** submitted by `operator`, then check if signer rotation is not happening too frequently (the `rotation delay` parameter is configured on the `Gateway Config PDA`)</li><li>If the tx **was** submitted by the `operator`, then skip the rotation delay check </li></ul></li><li>Check: Only rotate the verifiers if the `verifier set` that signed the action is the **latest** `verifier set`</li><li>Check: ensure that the new verifier set is not a duplicate of an old one</li><li>Initialize a new `Verifier Tracker PDA` that will track the epoch and the hash of the newly created `verifier set`</li><li>Update the `Gateway Config PDA` to update the latest verifier set epoch</li><li>This will emit an event for the relayer to capture and report back to `ampd`</li></ol> |

## Operator role

This role can rotate the `verifier set` without enforcing the `minimum rotation delay`.

The role can be updated using [`Transfer Operatorship`](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/programs/axelar-solana-gateway/src/processor/transfer_operatorship.rs#L33). The ix is accessible to:
- **The old operator** can transfer operatorship to a new user
- The **`bpf_loader_upgadeable::upgrade_authority`** can also transfer operatorship. This is equivalent to the upgrade authority on the Solidity implementation.

## Differences from the EVM implementation

| Action | EVM reference impl | Solana implementation | Reasoning |
| - | - | - | - |
| [Authentication](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/gateway/INTEGRATION.md#authentication) | Every verifier and all the messages get hashed together in a single hash, then signatures get verified against that hash. All done in a single tx. | Every action is done in a separate tx. Signatures get verified against a hash first. Then, we use Merkle Proofs to prove that a message is part of the hash. | Solana cannot do that many actions in a single transaction (e.g. hashing multiple messages and creating a big hash out of that); we need to split up the approval process into many small transactions. This is described in detail on [axelar-solana-encoding](../crates/axelar-solana-encoding/README.md#current-limits-of-the-merkelised-implementation) crate |
| Receiving the message on the destination contract | Payload is passed as tx args. | Payload is chunked and uploaded to on-chain storage in many small transactions | Otherwise, the average payload size we could provide would be ~600-800 bytes; Solana tx size is limited to 1232 bytes, and a lot of that is consumed by metadata | 
| [Message size](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/gateway/INTEGRATION.md#limits) | 16kb is min; more than 1mb on EVM | 10kb is max with options to increase this in the future | The maximum amount of PDA storage (on-chain contract owned account) is 10kb when initialized up-front |
| Updating verifier set | Requires the whole verifier set to be present, then it is re-hashed and then re-validated on chain | Only the verifier set hash is provided in tx parameters; we don't re-hash individual entries from the verifier set upon verifier set rotation. We take the verifier set hash from the Multisig Prover as granted and only validate that the latest verifier set signed it. We expect the hash always to be valid. | We cannot hash that many entries (67 verifiers being the minimum requirement) in a single transaction. The only thing we can do is _"prove that a verifier belongs to the verifier set"_ (like we do during signature verification). Still, even that would not change the underlying verifier set hash we set; thus, the operation would be pointless. |
| [Upgradability](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/gateway/INTEGRATION.md#upgradability) | Gateway is deployed via a proxy contract | Gateway is deployed using `bpf_loader_upgradeable` program | This is the standard on Solana |
