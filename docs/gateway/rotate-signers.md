# Rotate Signers

## Overview

Rotate signers updates the signer set used by the Gateway to validate cross-chain messages. This is a critical security operation used when signer keys are compromised or during scheduled key rotations.

The signer set controls which validators can approve cross-chain commands, making this operation essential for maintaining the security of the cross-chain protocol.

## Routine Operations

- **Scheduled Key Rotation**: Regular signer key rotation following security best practices and schedules
- **Planned Signer Updates**: Updating signer set as part of planned operational transitions
- **Key Management Procedures**: Rotating signer keys as part of routine key management procedures

## Emergency Scenarios

- **Signer Key Compromise**: Signer keys are compromised or suspected of being compromised
- **Immediate Threat Response**: Need to quickly rotate to a new secure signer set to prevent unauthorized message approvals
- **Active Security Incident**: Security event requires immediate revocation of current signer privileges

## Execution

### EVM

**Required Role:** Gateway Operator (Amplifier Gateway) or Governance (Consensus Gateway)

**Amplifier Gateway - Direct Execution:**
```bash
ts-node evm/gateway.js --action rotateSigners
```

**Note:** The script generates the signer set and proof internally.

**Amplifier Gateway - Via Amplifier Proof:**
```bash
ts-node evm/gateway.js --action submitProof --multisigSessionId <sessionId>
```

**Consensus Gateway - Via Governance (Raw Action):**

For consensus gateways, signer rotation is performed by calling the AuthModule's `rotateSigners` function via governance. Use the `raw` action with the AuthModule address as the target:

```bash
ts-node evm/governance.js schedule raw <activationTime> \
  --target <authModuleAddress> \
  --calldata <encodedRotateSignersCalldata> \
  --generate-only proposal.json
```

**Examples:**

Amplifier Gateway - Rotate signers via Amplifier proof:
```bash
ts-node evm/gateway.js --action submitProof --multisigSessionId 12345
```

### Stellar

**Required Role:** Gateway Operator

**Rotate Signers:**
```bash
ts-node stellar/gateway.js rotate --signers <signers-json|wallet>
```

**Via Amplifier Proof:**
```bash
ts-node stellar/gateway.js submit-proof <multisigSessionId>
```

**Examples:**

Rotate signers using wallet (uses wallet address as new signer):
```bash
ts-node stellar/gateway.js rotate --signers wallet
```

Rotate signers with custom signer set:
```bash
ts-node stellar/gateway.js rotate --signers '{"signers":[{"signer":"GB2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3","weight":1}],"threshold":1,"nonce":"0x0000000000000000000000000000000000000000000000000000000000000000"}'
```

Submit Amplifier proof:
```bash
ts-node stellar/gateway.js submit-proof 12345
```

### Sui

**Required Role:** Gateway Operator (OperatorCap holder)

**Rotate Signers:**
```bash
ts-node sui/gateway.js rotate [--signers <signers-json|wallet>] --proof <proof-json|wallet>
```

**Note:** If `--signers` is omitted, signers are fetched from the current verifier set. Use `wallet` to use the wallet's pubkey as the new signer.

**Via Amplifier Proof:**
```bash
ts-node sui/gateway.js submit-proof <multisigSessionId>
```

**Examples:**

Rotate signers using wallet for both signers and proof:
```bash
ts-node sui/gateway.js rotate --signers wallet --proof wallet
```

Rotate signers using current verifier set with wallet proof:
```bash
ts-node sui/gateway.js rotate --proof wallet
```

Submit Amplifier proof:
```bash
ts-node sui/gateway.js submit-proof 12345
```

## Verification

After execution, verify the new signer set:

Check signer set via block explorer or contract query.

