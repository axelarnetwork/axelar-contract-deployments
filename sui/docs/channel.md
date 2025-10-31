# Channels in ITS

Understanding `Channel` objects is essential for implementing secure token linking and managing cross-chain token operations in ITS.

The `Channel` object in Sui ITS is a foundational abstraction that:

- Provides unique, verifiable identity for ITS instances, deployers, operators, and distributors
- Enables deterministic token ID derivation for custom token linking
- Serves as an authentication mechanism for privileged operations
- Leverages Sui's object model for secure ownership and transfer semantics

## Role of `Channel`

A `Channel` serves as a destination address for cross-chain messages on Sui, via the [Axelar Gateway contract](https://github.com/axelarnetwork/axelar-cgp-sui/tree/main/move/axelar_gateway). It is an identifier that allows applications to send and receive messages between Sui and other chains through the Axelar network. <!-- skip-check -->

Key characteristics:
- **Unique Identification**: Each Channel has a unique `UID` (Universal Identifier) field, as well as an object ID, that's used in address on Sui. Transferring the object to another owner will not modify the `UID`, nor the object ID.
- **Message Routing**: The `Channel`'s ID is used by the Axelar protocol to route approved messages to the correct destination.
- **Cross-chain Messaging**: `Channel`s enable bidirectional communication with other blockchain networks

In the context of ITS, the `Channel` serves multiple critical roles:
1. **ITS Instance Identity**: The main ITS contract maintains its own `Channel` with the Axelar Gateway which represents the ITS instance's address for receiving messages from the Axelar Hub.
2. **Message Verification**: When receiving cross-chain messages from GMP, the `Channel` validates incoming `ApprovedMessage` objects are destined for the correct recipient.
3. **Source Attribution**: When preparing outbound messages, the `Channel`'s address is included to identify the sender, and replaces the need to track the coin deployer by their Sui address.

## Sui Object Model and `Channel` Ownership

The `Channel` object leverages Sui's unique object model with specific ownership semantics:

### Object Structure
```move
public struct Channel has key, store {
    id: UID,
}
```

The `Channel` has two important abilities:
- **`key`**: Makes it a Sui object that can be owned, shared, or wrapped
- **`store`**: Allows it to be stored inside other objects or data structures

### Ownership Patterns in ITS

**ITS Global Channel (Stored in Contract State)**

The main ITS contract stores a `Channel` within its versioned storage:
```move
public struct InterchainTokenService_v0 has store {
    channel: Channel,
    // ... other fields
}
```

This `Channel` is embedded in the shared ITS object and represents the ITS instance itself. It cannot be transferred or destroyed by external actors because it's wrapped within the ITS storage.

**User-Owned Channels**

When users register custom tokens, or when the ITS contract receives linked tokens, user permissions are controlled by a `Channel` object that they own. 

This Channel serves as:
- Proof of deployment authority (deployer identity)
- An authentication token for administrative operations
- A reference for deriving token IDs

## Related Entities Using `Channel`

In the ITS architecture, the `Channel` object is closely tied to three key roles:

### 1. Operator

**Purpose**: Controls operational settings for a token manager, including flow limits and pausing functionality.

**Channel Relationship**
- An operator is identified by their `Channel` object
- `Channel` ownership proves the operator's identity when calling privileged functions like:
  - `set_flow_limit_as_token_operator`: Set flow limits for token transfers
  - `transfer_operatorship`: Transfer operator role to a new address

The operator cannot steal tokens but can modify settings that affect the token manager's operation. The `Channel` serves as cryptographic proof that the caller is the authorized operator.

### 2. Distributor

**Purpose**: Has minting and burning privileges for tokens with MINT_BURN token manager types.

**Channel Relationship**:
- A distributor is identified by their `Channel` object
- The `Channel` is used to authenticate minting and burning operations:
  - `mint_as_distributor`: Mint new tokens
  - `mint_to_as_distributor`: Mint tokens directly to a recipient
  - `burn_as_distributor`: Burn tokens

**Distribution Assignment**: When unlinked coins are registered with the MINT_BURN token manager type, a `Channel` is created by ITS and assigned as the distributor:
```move
let distributor = axelar_gateway::channel::new(ctx);
let mut coin_management = coin_management::new_with_cap(treasury_cap);
coin_management.add_distributor(distributor.to_address());
```

This `Channel` is then returned to the caller, giving them the minting/burning capabilities for the token.

### 3. Deployer

**Purpose**: The original entity that registered or deployed a token, used for deriving custom token IDs, token linking security, and initiating remote operators.

**Channel Relationship**:
- The deployer's `Channel` is used in the deterministic derivation of token IDs for custom tokens
- `Channel` ownership proves deployment authority for linking operations

**Token ID Derivation**:
Custom token IDs are derived using the deployer's Channel address:
```move
pub fun custom_token_id(
    chain_name_hash: &Bytes32,
    deployer: &Channel,
    salt: &Bytes32,
): TokenId {
    // ... 
    let token_id = hash(PREFIX_CUSTOM_TOKEN_ID, chain_name_hash, deployer.to_address(), salt)
    // ...
}
```

### Role Summary

| Role | Channel Purpose | Key Operations |
|------|----------------|----------------|
| **Operator** | Authentication for administrative operations | Set flow limits, pause/unpause, transfer operatorship |
| **Distributor** | Authentication for mint/burn privileges | Mint tokens, burn tokens, transfer distributorship |
| **Deployer** | Identity proof for token registration | Register custom tokens, link tokens, token ID derivation |

## Role of `Channel` in `salt` and `TokenId` Derivation for Coin Linking

The `Channel` plays a critical role in ensuring deterministic, collision-resistant token ID generation for custom token linking.

### Custom Token ID Derivation

When registering a custom coin for linking, the token ID is derived from three components:

1. **Chain Name Hash** (`Bytes32`): Unique identifier for the Sui chain instance
2. **Deployer `Channel`**: The deployer's `Channel` object
3. **Salt** (`Bytes32`): A user-provided unique value. The `salt` must be 32 bytes (e.g. 0x + 64 characters) matching the Sui address format

**Token ID Derivation Formula**:
```move
let token_id = hash(PREFIX_CUSTOM_TOKEN_ID, chain_name_hash, deployer.to_address(), salt)
```

This derivation scheme ensures:
- **Uniqueness**: Different deployers or different salts produce different token IDs
- **Determinism**: The same inputs always produce the same token ID
- **Collision Resistance**: Cryptographic hashing prevents accidental collisions
- **Chain Specificity**: Tokens deployed on different Sui chains have different IDs even with the same deployer/salt

### Linking Process Using Channel

When linking a custom token from Sui to another chain:

**Step 1 - Register Custom Token** (Source Chain):
```move
public fun register_custom_coin<T>(
    self: &mut InterchainTokenService,
    deployer: &Channel,  // Proves deployer identity
    salt: Bytes32,
    coin_metadata: &CoinMetadata<T>,
    coin_management: CoinManagement<T>,
    ctx: &mut TxContext,
): (TokenId, Option<TreasuryCapReclaimer<T>>)
```

The `Channel` is used to:
- Derive the custom token ID
- Emit an event claiming the token ID for this deployer/salt combination
- Establish the deployer's ownership rights over this token

**Step 2 - Link Coin** (Source Chain):
```move
public fun link_coin(
    self: &InterchainTokenService,
    deployer: &Channel,  // Same Channel from registration
    salt: Bytes32,        // Same salt from registration
    destination_chain: String,
    destination_token_address: vector<u8>,
    token_manager_type: TokenManagerType,
    link_params: vector<u8>,
): MessageTicket
```

Functionality:
1. Re-derives the token ID using the same `deployer` Channel and `salt` (validates the deployer)
2. Verifies the token exists and is registered as a custom token
3. Constructs a LINK_TOKEN message to be sent via ITS Hub
4. Uses the ITS contract's internal `Channel` to send the constructed message

**Security Considerations**:

- **Salt Uniqueness**: The salt must be unique per token linking operation to prevent collisions
- **Channel Ownership**: Only the holder of the deployer `Channel` can link tokens registered with that `Channel`
- **Immutability**: Once a token ID is claimed by a deployer/salt pair, it cannot be re-registered
- **Salt Format**: On Sui, the `salt` must be exactly 32 bytes (matching Sui address format)

### Example Token Linking Flow

```move
// User creates their deployer channel
let deployer_channel = channel::new(ctx);

// Define a unique salt (0x + 64 chars.)
let salt = bytes32::new(0x0000...0001);

// Register the custom token
let (token_id, treasury_cap_reclaimer) = its.register_custom_coin(
    &deployer_channel,  // Channel used for token ID derivation
    salt,
    &coin_metadata,
    coin_management,
    ctx
);

// Link to destination chain
let message_ticket = its.link_coin(
    &deployer_channel,  // Same channel ensures same token_id
    salt,               // Same salt ensures same token_id  
    destination_chain,
    destination_token_address,
    token_manager_type,
    link_params
);
```

### Receiving Unlinked Tokens

To receive a link token message from another chain, the destination must prepare the "unlinked coin" before the link message arrives. This is done by giving the unlinked coin to ITS. If the coin's `TreasuryCap` is given to ITS in the `give_unlinked_coin` transaction, ITS creates a new `Channel` that's used both to validate the deployer so that the coin may be linked later (e.g. using the same `Channel`), as well as make them a distributor of the coin in the case of MINT_BURN token managers.

```move
public fun give_unlinked_coin<T>(
    self: &mut InterchainTokenService,
    token_id: TokenId,  // The expected token ID from the link operation
    coin_metadata: &CoinMetadata<T>,
    treasury_cap: Option<TreasuryCap<T>>,
    ctx: &mut TxContext,
): (Option<TreasuryCapReclaimer<T>>, Option<Channel>)
```

This is particularly important for MINT_BURN type token managers, where the workflow for `give_unlinked_coin` is:

1. ITS creates a new `Channel`
2. ITS adds the newly created `Channel` as distributor for the coin
3. `give_unlinked_coin` returns the `Channel` to the caller, giving them distributor privileges. This ensures that the deployer on the destination chain receives appropriate control over the linked token once the linking is complete.

## Creating & Using Channels

Disregarding the `sui/its interchain-transfer` command, which creates and destroys and temporary `Channel`, all interchain Sui commands support using the channel flag (`--channel <channel>`). 

If no channel flag is passed to the command, a new channel will be created and transferred to the user. Whenever a new channel is created for the caller, it can be found in the transaction block data (e.g. using the transaction hash logged in the console), or by searching the user's owned objects for the type (`Channel`).

Example creation:

```bash
# Register a custom coin and create a new `Channel`
# (channel object id can be found in the transaction data)
ts-node sui/its register-custom-coin SYMBOL "Coin Name" 9 --salt "0x..."
```

Example usage: 

```bash
# Register a custom coin using an existing `Channel`
ts-node sui/its register-custom-coin SYMBOL "Coin Name" 9 --salt "0x..." --channel "0x..."
```

**Note:** The value of the channel flag is it's object ID on the Sui blockchain which is different from its `UID` in the Axelar Gateway contract's storage.
