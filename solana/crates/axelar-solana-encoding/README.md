# Axelar Solana Encoding crate

This crate defines utilities that are used on the following components:
- Encodes data on the [**Multisig Prover**](https://github.com/eigerco/axelar-amplifier/blob/acd6d68da408ff9ea8859debd3b04427b08f5be3/contracts/multisig-prover/src/encoding/mod.rs#L21) in a Merkelised way
- [**Relayer**](https://github.com/eigerco/axelar-solana-relayer) uses the encoded data to send many small transactions to the Axelar Solana Gateway to approve messages or rotate signer sets
- [**Axelar Solana Gateway**](../../programs/axelar-solana-gateway/README.md) uses the encoding crate to create hashes that are consistent between all implementations
- All components listed above use the data types defined in this crate. For a high-level overview, see [the program README.md](../../programs/README.md)

## Using `borsh`

While `abi` encoding can be used inside Solana programs, the ecosystem primarily has settled on using `borsh`. Borsh encoding is simple to use, relatively cheap (in compute unit consumption), and has JS libraries. It's the default used by the Anchor framework [[source]](https://solana.com/developers/courses/native-onchain-development/serialize-instruction-data-frontend#serialization). Also, Solana is not the only blockchain that uses borsh [[link]](https://docs.near.org/build/smart-contracts/anatomy/serialization), meaning it was a natural choice, and as the limitations above highlight - encoding and decoding the raw data is not the limitation.

## Merkelising the data

> [!NOTE]
> For a better understanding of the following chapter - [Wikipedia Merkle Tree](https://en.wikipedia.org/wiki/Merkle_tree).
> 
> Our `axelar-solana-encoding` library protects against [second preimage attacks](https://en.wikipedia.org/wiki/Merkle_tree#Second_preimage_attack).

The `axelar-solana-encoding` crate uses Merkle Trees to Merkelise the data and builds commitment schemes. This is necessary because Solana TX and compute limitations prevent doing _everything_ that the EVM gateway can do in a single TX. The defining property of `axelar-solana-encoding` allows the Relayer to send many small transactions without complex on-chain state tracking. Merkle Roots are the commitment values that tie all the small transactions together.

![image](https://github.com/user-attachments/assets/84047adf-15de-4473-aad1-7851e65718eb)

The fundamental idea of the Merkle Tree: 
- You can prove that an item is part of the set without requiring the whole set present (e.g. prove **that a message is part of the message batch** or a **verifier is part of a verifier set**)
- Each item of the set is represented as a Leaf Node. Each leaf node contains all the information about the set, such as size, domain separator, leaf node position, etc.
- Given a leaf node, proof (an array of hashes), and the Merkle root, you can prove that an item is part of a set.

The unique property of this approach is that:
- we reduce the amount of data we need to expose for an action. For example, for 1000 items in a batch, the Merkle proof would be 10 hashes.
- we can verify each signature as a separate transaction by verifying that a verifier is part of the verifier set without passing the whole verifier set.
- We can verify that a message is similar to a message batch.

Let's take a look at how we construct leaf nodes from verifier sets and message batches:

![image](https://github.com/user-attachments/assets/825de271-9655-4611-8a0b-7a27ff2e6d73)

> [!NOTE]
> **Payload digest**: this is the data that the verifiers sign. It is a hash that consists of all the messages, verifiers, and other metadata. 

| Action | Verifier Set | Message batch |
|-|-|-|
|Base data structure layout|This is the base data representation without any extra metadata, representing a single verifier set|A vector of messages, aka a batch of messages. All other integrations (like EVM) operate directly on this data type|
|Constructing leaf node|A leaf node is constructed by flattening the data, extracting metadata like set size and verifier position, and injecting Axelar-specific information like the domain separator. We **don't** inject the "sigingin verifier set". |A leaf node is constructed by flattening the data, extracting metadata like set size and message position, and injecting Axelar-specific information like the domain separator. We also inject the "signing verifier set" so that every leaf node is tied directly to the verifier set that is signing it.|
|Constructing leaves|A simple iterator over the leaves|A simple iterator over the leaves|
|Merkle tree root|This is the logical equivalent of "signer set hash" from the EVM abi encoding|This is the logical equivalent of payload hash from the EVM abi encoding|
|Payload digest|We inject a "signing verifier set" (also a Merkle root) so that the payload digest knows the verifier set that will sign it. This allows us to have two logically tied data values: the unique hash for the verifier set and the hash that the verifiers are going to sign.|We use the Merkle root from 👆|

### Execute Data

> [!NOTE]
> **Execute Data**: This is the data that the Multisig Prover returns after getting all the signatures. It aggregates the signatures and all the data used to create a **payload digest**. The goal of the data is to allow the gateway to check that the verifiers have signed a payload digest and that the provided messages can be re-hashed to create the payload digest. 

![image](https://github.com/user-attachments/assets/066bf866-4130-4808-8901-5bf493d895fb)

After the data has been Merkelised, the Multisig Prover neatly packs it together for the Relayer to consume.
It encodes:
- the verifier set that signed the data
- all the signatures and proofs of every signer in the set
- the payload digest (either of the verifier set or the message batch)

As a result, this approach allows us to do the following:

| Action | Description | Semantical difference from EVM |
|-|-|-|
| Verify that a verifier set is valid | The Merkle root (a hash), just as in the EVM version, acts as a unique identifier of the verifier set. It can be done via simple on-chain hash comparison. | None |
| Verify signature | Every signature in a signing verifier set can be validated as an individual transaction, tracking the progress on-chain | None |
| Approving messages | After all signatures have been approved for a given payload digest, we can check if a given message is part of an approved message batch, and if it is, then mark its status as "approved" in an on-chain state. | None |
| Rotating singers | After all signatures have been approved for a given payload digest, we can provide the new verifier set hash, together with the verifier set hash that signed the message, and reconstruct the "verifier set payload digest" on-chain, to check if it matches the one that has been signed over. The end goal of this indirection is to prevent malicious actors from providing "Message batch payload digest" as the hash of the new verifier set. | We don't reconstruct the new verifier set hash on-chain; we only operate on hashes |

### Hashing data

The hashing of data needs to be consistent across all users of this crate: 
- Multisig Prover running in wasm on Axelar chain because it constructs the digests that the verifiers sign over;
- Relayer running on a server because it needs to use the hashes to compute PDAs when interacting with the Axelar Solana Gateway;
- Axelar Solana Gateway constructing data on-chain and validating that the hashes match

Because of Solana's computing limitations, [a syscall is the best way to hash data](https://docs.rs/solana-program/2.1.5/src/solana_program/keccak.rs.html#118-141). Solana provides syscalls for multiple hash functions, but we settled on using the keccak256 hash function.
This means that our `axelar-solana-encoding` code has a branching mechanism that allows it to be generic over the hasher:
- on Solana, we leverage the syscall for a minimal compute unit footprint
- on Axelars CosmWASM runtime (Multisig Prover), we leverage the Rust-native keccak256 implementation
- on the Relayer, we leverage the Rust-native keccak256 implementation

![image](https://github.com/user-attachments/assets/d8257c24-abb2-4064-9414-b50618bb07e4)

For encoding the data, we use [udigest crate](https://docs.rs/udigest/0.2.2/udigest/encoding/index.html), which allows us to transform a set of data into a vector of bytes. [Read this article about hashing the data and creating digests](https://www.dfns.co/article/unambiguous-hashing).

### Current limits of the Merkelised implementation
Now let's take a look at [the full requirements that the Axelar Solana Gateway must follow and see how it affects the `axelar-solana-encoding` scheme we use](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/432449d7b330ec6edf5a8e0746644a253486ca87/contracts/gateway/INTEGRATION.md?plain=1#L261):

| Limit | Minimum | Recommended | Axelar Solana Gateway |
|-|-|-|-|
| Cross-chain Message Size | 16 KB | 64 KB | < 10 KB |
| Signer Set Size | 40 signers | 100 signers | < 256 |
| Signature Verification | 27 signatures | 67 signatures | < 256 |
| Message Approval Batching | 1 | Configurable | Practically unlimited |
| Storage Limit for Messages | Practically unlimited (2^64) | Practically unlimited (2^64) | Practically unlimited |

The message size is tackled purely on the Gateway and is not part of the `axelar-solana-encoding` scheme. The Merkeliesd data allows us to:
- have a signer set up for 256 participants
- verify a signature for every single signer
- allows us to have a practically unlimited amount of messages in a batch (limited by how many hashes we can do in a single tx)

The _Storage Limit for Messages_ requirement is a given using Solana PDAs, no extra effort required from the Gateway or the encoding crate.

---

## Understanding the EVM encoding for comparison sake

To better understand how this approach differs from the EVM Gateways ABI encoding, let's analyze it. The EVM encoding works the following way:
[![evm encoding](https://github.com/user-attachments/assets/9ffb61a4-74ec-4734-862a-1027fa0e797b)](https://link.excalidraw.com/readonly/91ctxas9n1417Y1XXKwQ)

Summary from the **payload digest**:
- all messages in a batch get encoded and hashed in one go
  - This means that to reconstruct the payload hash, the smart contract requires all of the messages to be directly available as function arguments
- all signers in a verifier set get hashed in one go
  - Same as for messages: to reconstruct the signer hash, the smart contract requires all of the signers to be directly available as function arguments
- the verifiers that will sign the payload digest also get hashed, and their hash is part of it. This is a security measure.
- The verifiers sign over the payload digest

How Gateway operates on the **execute data**:
- this is the piece of data that the Multisig Prover returns to the relayer
- relayer passes ExecuteData structure directly to the EVM Gateway smart contract
  - The Gateway will reconstruct the payload digest and use the created hash to verify signatures. To rebuild the payload digest:
    - on-chain logic will reconstruct the hash for all messages / new verifiers
    - on-chain logic will reconstruct the hash of all verifiers to ensure that the verifier set has been approved
- Verify every signature in the batch against the created payload digest. If the quorum is met:
  - for every message in the batch, mark it as "approved" by updating the Gateway contract state

### Payload size implications

Some napkin math to understand the implications of such a payload [let's take a look at the **minimal** requirements](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/432449d7b330ec6edf5a8e0746644a253486ca87/contracts/gateway/INTEGRATION.md?plain=1#L261):

| Data piece | Minimum | Size | Total |
|-|-|-|-|
| Signer Set Size | 40 signers |  `address` size is 20 bytes | 20 * 40 = 800 bytes |
| Signature Verification | 27 signatures | `secp256k1 signature` is 65 bytes | 65 * 27 = 1755 bytes |
| Message Approval Batching | 1 | 20 bytes source chain name; ~50 bytes message id; ~30 bytes source address; 20 bytes contract address; 32 bytes payload hash | 20 + ~50 + ~30 + 20 + 32= ~152 bytes |
|  |  |  | ~2707 bytes |

This total size (2707 bytes) is just the _raw data_ for minimal requirements, excluding extra information added by the abi encoding itself, like padding the bytes between data types, len prefixes for arrays, etc.

### Why such an approach will never work for Solana

Solana has quite a few different limitations, both in how many actions can be done in a single transaction (the ceiling for max compute units) and how large the transaction can be.
2 most notable things to keep in mind about Solana's limitations:
- [Transaction size is capped at 1232 bytes](https://solana.com/docs/core/transactions#key-points). This tx info contains the raw data to send, as well as a list of all of the accounts to be used by the transaction (if you are reading this as a Solidity dev, imagine that you need to provide a list of `[]address` of every storage slot that your on-chain contract will try to read or mutate, including all the storage slots that an internal contract call may touch). This information itself also eats up precious bytes. There's also extra metadata, like the signatures, block hash, and header data, that eat away at the tx size—the more sophisticated the contract, the more accounts it needs to access.
- Computationally, every operation on Solana has a cost, measured in compute units. The heavier the math operation ([e.g. division](https://solana.com/docs/programs/limitations#signed-division)), the more compute units it will take. Many operations like hashing and signature verification can leverage "syscalls" [(which also have a fixed cost)](https://github.com/anza-xyz/agave/blob/b7bbe36918f23d98e2e73502e3c4cba78d395ba9/program-runtime/src/compute_budget.rs#L133-L178) where the runtime calls a static function on the host machine, leveraging pre-compiled code instead of emulating heavy computations inside the virtual machine.

The most significant limitation of 1232 bytes per tx means that it is impossible to pack the minimum required data (2707 bytes) into a single transaction. In the initial stages of the Axelar-Solana Gateway, we implemented a logic that would store the ExecuteData on-chain in a PDA (aka storage slot for drawing parallels with Solidity). This allowed us to get rid of the size limitations. However, we quickly discovered that computing the desired data in a single TX is impossible, and we require a multi-step computation model. One approach besides Merkelisng the flow would be introducing on-chain state tracking of ExecuteData processing. However, an internal discussion led to the conclusion that it brings no extra security measures, makes the process more expensive in terms of gas fees and introduces a lot of additional complexity. [(For details, see this public report on our attempt)](https://docs.google.com/document/d/1I3PQQ7H6oZNiayteJcrb6T1o2UHrRBcsAkSa7mOBCCY/edit?usp=sharing), we quickly found out that we cannot hash & verify the amount of data we require when "approving messages" on the gateway (reconstructing the payload digest and verifying signatures). Although this example used `bcs` encoding instead of `abi`, which was developed & maintained by Axelar, the internal encoding structure is the same as in the `abi` example described above. Our conclusion was:
  - The maximum number of signers we can have is 5. It will be less, but never more than 5, depending on the other variables.
  - The maximum number of messages in one batch is 3.  Depending on the other variables, it will be less, but never more than 3.
  - The maximum number of accounts is 20. Depending on the message size, it will be less, but never more than 20.
  - The maximum message size is 635 bytes. Depending on the number of accounts, it will be smaller but never larger than 635 bytes.
  - The bottleneck for the number of signers and number of messages per batch is the gateway. In contrast, the bottleneck for the message size and number of accounts is the destination contract.

This meant that we just could not put enough data inside our transactions **and** we could not do enough computations in a **single** transaction. Hence why we had to redesign the whole approach of how we encode the data on the Multisig Prover side. We required splitting all of the work between many small transactions while still ensuring that the state of the computation is being tracked on-chain.
