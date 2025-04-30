## Interchain Token Service

### Encode ITS Destination Address

When sending interchain tokens across different chains using the **Interchain Token Service (ITS)**, the recipient address must be encoded into a chain-specific format. This encoding ensures that the ITS protocol can correctly route the token to the intended recipient on the destination chain. Without this encoding, the destination address would not be understood by the ITS contract, leading to failed or misrouted transactions.

```bash
# Usage
node common/its.js encode-recipient [destination-chain] [destination-address]

# Example
node common/its.js encode-recipient stellar CC6FYRUBDJVTATQ55KGPMD2JQFY775BTSJQMRNJEWPEJFUXPOBFSMEOX

# Example Output
Human-readable destination address: CC6FYRUBDJVTATQ55KGPMD2JQFY775BTSJQMRNJEWPEJFUXPOBFSMEOX

Encoded ITS destination address: 0x4343364659525542444a565441545135354b47504d44324a5146593737354254534a514d524e4a455750454a465558504f4246534d454f58
```
