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
- **Unique Identification**: Each `Channel` has a unique `UID` (Universal Identifier) field, as well as an object ID on Sui. Transferring the object to another owner will transfer the `Channel`'s capabilities (operator, distributor, etc.) but will not modify the `UID` or object ID of the `Channel`.
- **Message Routing**: `Channel`s are used by the Axelar protocol to route approved messages to the correct destination.
- **Cross-chain Messaging**: `Channel`s enable bidirectional communication with other blockchain networks.

In the context of ITS, the `Channel` serves multiple critical roles:
1. **ITS Instance Identity**: The ITS contract maintains its own `Channel` with the Axelar Gateway which represents the ITS instance's address for receiving messages from the Axelar Hub.
2. **Message Verification**: When receiving cross-chain messages from GMP, the `Channel` validates incoming `ApprovedMessage` objects are destined for the correct recipient.
3. **Source Attribution**: When preparing outbound messages, the `Channel` identifies the sender, and it replaces the need to track coin deployers by their Sui address.

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

### `Channel` Ownership in ITS

**ITS Global Channel (Stored in Contract State)**

The ITS contract stores a `Channel` within its versioned storage:
```move
public struct InterchainTokenService_v0 has store {
    channel: Channel,
    // ... other fields
}
```

This `Channel` is embedded in the shared ITS object and represents the ITS instance itself. It cannot be transferred or destroyed by external actors because it's wrapped within the ITS storage.

When users register and transfer tokens, or when the ITS contract receives linked tokens, permissions are controlled by a `Channel` object that can be owned by users or packages. 

This Channel serves as:
- Proof of deployment authority (deployer identity)
- An authentication token for administrative operations
- A reference for deriving token IDs

## Coin Admin `Channel` Types

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

**Purpose**: Has minting and burning privileges for tokens with MINT_BURN token manager (`CoinManagement`) types.

**Channel Relationship**:
- A distributor is identified by their `Channel` object
- The `Channel` is used to authenticate minting and burning operations:
  - `mint_as_distributor`: Mint new tokens
  - `mint_to_as_distributor`: Mint tokens directly to a recipient
  - `burn_as_distributor`: Burn tokens

**Distribution Assignment**: When unlinked coins are registered with the MINT_BURN token manager (`CoinManagement`) type, a `Channel` is created by ITS and assigned as the distributor:
```move
let distributor = axelar_gateway::channel::new(ctx);
let mut coin_management = coin_management::new_with_cap(treasury_cap);
coin_management.add_distributor(distributor.to_address());
```

This `Channel` is then returned to the caller, giving them the minting/burning capabilities for the token.

### 3. Deployer

**Purpose**: The original entity that registered or deployed a token, used for deriving custom token IDs, token linking security, and initiating remote operators.

**Channel Relationship**:
- The deployer's `Channel` is used to deterministically derive token IDs for custom tokens
- `Channel` ownership proves deployment authority for linking operations

**Token ID Derivation**:
Custom token IDs (required for coin linking) are derived using the deployer's `Channel` and provided `salt`:
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
| **Deployer** | Identity proof for token registration | Register custom coins, link coins, token ID derivation |

### Linking Process Using `Channel`

When linking a custom token from Sui to another chain:

**Step 1 - Register Custom Coin** (Source Chain):
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

To receive a `LinkToken` message from another chain, the Sui ITS contract (the destination) must prepare the "unlinked coin" before the link message arrives. This is done by giving the unlinked coin to ITS. 

If the coin's `TreasuryCap` is given to ITS in the `give_unlinked_coin` transaction, ITS creates a new `Channel` that will validate the deployer for coin linking; and, in the case of MINT_BURN token managers (`CoinManagement`), ITS makes this `Channel` a distributor of the coin.

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

The `sui/its interchain-transfer` command creates and destroys and temporary `Channel`. All other Sui interchain commands support using the channel flag (`--channel <channel>`). If no channel flag is passed to the command, a new channel will be created and transferred to the user. 

When a new channel is created by a command it can be found in the transaction block data (see the tx hash logged in the console).

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
