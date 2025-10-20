# Common scripts

## Interchain Token Service

### ITS Destination Address Encoding

When sending interchain tokens across different chains using the **Interchain Token Service (ITS)**, the recipient address must be encoded into a chain-specific format. This encoding ensures that the ITS protocol can correctly route the token to the intended recipient on the destination chain. Without it, the destination address may not be recognized by the ITS contract, potentially leading to failed or misrouted transactions.

#### Notes
- For **EVM** and **Sui**, addresses do not require special encoding and are used as-is.
- For **Stellar**, addresses must be converted to ASCII byte arrays to be properly recognized by the ITS contract.

### Usage

```bash
ts-node common/its.js encode-recipient [destination-chain] [destination-address]
```

#### Example (EVM)
```bash
ts-node common/its.js encode-recipient flow 0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C

# Output
Human-readable destination address: 0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C

Encoded ITS destination address: 0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C
```

#### Example (Stellar)
```bash
ts-node common/its.js encode-recipient stellar CC6FYRUBDJVTATQ55KGPMD2JQFY775BTSJQMRNJEWPEJFUXPOBFSMEOX

# Output
Human-readable destination address: CC6FYRUBDJVTATQ55KGPMD2JQFY775BTSJQMRNJEWPEJFUXPOBFSMEOX

Encoded ITS destination address: 0x4343364659525542444a565441545135354b47504d44324a5146593737354254534a514d524e4a455750454a465558504f4246534d454f58
```
