## EVM Emergency Response Playbook
## AxelarGateway

### Governance Actions

Actions that require governance proposal submission, voting period, and timelock delay.

**Upgrade**

- **Before upgrading**: Deploy new implementation with `--reuseProxy` flag:
  - **Legacy connection**: Use `deploy-gateway-v6.2.x.js` with `--reuseProxy`. See [Gateway Upgrade](../README.md#gateway-upgrade).
  - **Amplifier connection**: Use `deploy-amplifier-gateway.js` with `--reuseProxy`. See [Axelar Amplifier Gateway](../README.md#axelar-amplifier-gateway).
- Replace `schedule` with `schedule-operator` to skip timelock.
- Documentation: [Upgrade Workflow](./governance-workflows.md#upgrade-workflow)

**Transfer Ownership**

- Add `--governance --activationTime <time>` flags. Add `--operatorProposal` flag to skip timelock.

- **Transfer Ownership via Governance**: `evm/ownership.js --governance -n <chain> -c <contractName> --action transferOwnership --newOwner <addr>` - [Contract Ownership Management](./contract-ownership.md)
- **Propose Ownership via Governance**: `evm/ownership.js --governance -n <chain> -c <contractName> --action proposeOwnership --newOwner <addr>` - [Contract Ownership Management](./contract-ownership.md)
- **Accept Ownership via Governance**: `evm/ownership.js --governance -n <chain> -c <contractName> --action acceptOwnership` - [Contract Ownership Management](./contract-ownership.md)
- Documentation: [Contract Ownership Management](./contract-ownership.md)

**Transfer Governance**

- Replace `schedule` with `schedule-operator` to skip timelock.
- Documentation: [Transfer Gateway Governance via Governance](./governance-workflows.md#transfer-gateway-governance-via-governance)

**Transfer Operatorship**

- Add `--operatorProposal` flag to skip timelock.
- Documentation: [Transfer Gateway Operatorship via Governance](./governance-workflows.md#transfer-gateway-operatorship-via-governance-amplifier)

**Note:** To skip timelock, replace `schedule` with `schedule-operator` for `governance.js` actions or add `--operatorProposal` flag for contract-specific scripts. Operator proposals use `AxelarServiceGovernance` and bypass timelock via operator approval. See [AxelarServiceGovernance Commands](./governance.md#axelarservicegovernance-commands) for details.

### Operator Actions

Actions that can be executed directly from operator wallet. Immediate execution, no timelock.

- **Rotate Signers**: `evm/gateway.js --action rotateSigners` - [Rotate signers (emergency)](../README.md#rotate-signers-emergency)
- **Transfer Operatorship**: `evm/gateway.js --action transferOperatorship --newOperator <addr>` - [Transfer operatorship](../README.md#transfer-operatorship) in Gateway operator commands

### Mint Limiter Actions

Actions that can be executed directly from mint limiter wallet. Immediate execution, no timelock.

- **Set Token Mint Limits**: `evm/gateway.js --action setTokenMintLimits` - [Mint limit management](./mint-limit.md#1-updating-mint-limits-via-gatewayjs)
- **Set Token Mint Limits (Multisig)**: `evm/multisig.js --action setTokenMintLimits` - [Mint limit management](./mint-limit.md#2-updating-mint-limits-via-multisigjs)

For complete documentation on mint limit management, see [Mint Limit Management](./mint-limit.md).

### Owner Actions

For ownership management actions, see [Contract Ownership Management](./contract-ownership.md).

---

## AxelarGasService

### Governance Actions

Actions that require governance proposal submission, voting period, and timelock delay.

**Upgrade**

- **Before upgrading**: Deploy new implementation with `--reuseProxy` flag. See [AxelarGasService and AxelarDepositService](../README.md#axelargasservice-and-axelardepositservice).
- Replace `schedule` with `schedule-operator` to skip timelock.
- Documentation: [Upgrade Workflow](./governance-workflows.md#upgrade-workflow)

**Note:** To skip timelock, replace `schedule` with `schedule-operator` for `governance.js` actions or add `--operatorProposal` flag for contract-specific scripts. Operator proposals use `AxelarServiceGovernance` and bypass timelock via operator approval. See [AxelarServiceGovernance Commands](./governance.md#axelarservicegovernance-commands) for details.

### Owner Actions

For ownership management actions, see [Contract Ownership Management](./contract-ownership.md).

---

## InterchainTokenService

### Governance Actions

Actions that require governance proposal submission, voting period, and timelock delay.

**Upgrade**

- **Before upgrading**: Deploy new implementation with `--reuseProxy` flag. See [InterchainTokenService](../README.md#interchaintokenservice).
- Replace `schedule` with `schedule-operator` to skip timelock.
- Documentation: [Upgrade Workflow](./governance-workflows.md#upgrade-workflow)

**Set Trusted Chains**

- Add `--governance --activationTime <time>` flags. Add `--operatorProposal` flag to skip timelock.
- Documentation: [Set Trusted Chains via ITS](./governance-workflows.md#set-trusted-chains-via-its)

**Remove Trusted Chains**

- Add `--governance --activationTime <time>` flags. Add `--operatorProposal` flag to skip timelock.
- Documentation: [Remove Trusted Chains via ITS](./governance-workflows.md#remove-trusted-chains-via-its)

**Set Pause Status**

- Add `--governance --activationTime <time>` flags. Add `--operatorProposal` flag to skip timelock.
- Documentation: [Pause / Unpause ITS](./governance-workflows.md#pause--unpause-its)

**Migrate Interchain Token**

- Add `--governance --activationTime <time>` flags. Add `--operatorProposal` flag to skip timelock.
- Documentation: [Migrate Interchain Token via ITS](./governance-workflows.md#migrate-interchain-token-via-its)

**Note:** To skip timelock, replace `schedule` with `schedule-operator` for `governance.js` actions or add `--operatorProposal` flag for contract-specific scripts. Operator proposals use `AxelarServiceGovernance` and bypass timelock via operator approval. See [AxelarServiceGovernance Commands](./governance.md#axelarservicegovernance-commands) for details.

### Operator Actions

Actions that can be executed directly from operator wallet. Immediate execution, no timelock.

- **Set Flow Limits**: `evm/its.js set-flow-limit <tokenId> <chain> <limit>` - [Set flow limit (single)](./rate-limits.md#3-set-flow-limit-single-token)
- **Freeze Tokens**: `evm/its.js freeze-tokens <tokenId> <chain>` - [Freeze tokens](./rate-limits.md#4-freeze-tokens-on-a-chain)
- **Unfreeze Tokens**: `evm/its.js unfreeze-tokens <tokenId> <chain>` - [Unfreeze tokens](./rate-limits.md#5-unfreeze-tokens-on-a-chain)
- **Transfer Operatorship**: `evm/its.js transfer-operatorship <newOperator>` - [Transfer operatorship](../README.md#transfer-operatorship-1) in ITS operator commands
- **Propose Operatorship**: `evm/its.js propose-operatorship <newOperator>` - [Propose operatorship](../README.md#propose-operatorship)
- **Set Pause Status**: `evm/its.js set-pause-status <true/false>` - [Set pause status (pause/unpause)](./pause-unpause.md) (owner or operator)

### Owner Actions

Actions that can be executed directly from owner wallet. Immediate execution, no timelock.

- **Set Trusted Chains**: `evm/its.js set-trusted-chains <chains...>` - [Set trusted chains](../README.md#set-trusted-chains)
- **Remove Trusted Chains**: `evm/its.js remove-trusted-chains <chains...>` - [Remove trusted chains](../README.md#remove-trusted-chains)
- **Set Pause Status**: `evm/its.js set-pause-status <true/false>` - [Set pause status (pause/unpause)](./pause-unpause.md) (owner or operator)
- **Migrate Interchain Token**: `evm/its.js migrate-interchain-token <tokenId>` - [Migrate interchain token](../README.md#migrate-interchain-token)
- For ownership management actions, see [Contract Ownership Management](./contract-ownership.md).

---

## Operators Contract

### Owner Actions

Actions that can be executed directly from owner wallet. Immediate execution, no timelock.

- **Add Operator**: `evm/operators.js addOperator <operator>` - [Add operator](../README.md#add-operator)
- **Remove Operator**: `evm/operators.js removeOperator <operator>` - [Remove operator](../README.md#remove-operator)
- **Transfer Ownership**: `evm/operators.js transferOwnership <newOwner>` - [Transfer ownership](../README.md#transfer-ownership)
- **Propose Ownership**: `evm/operators.js proposeOwnership <newOwner>` - [Propose ownership](../README.md#propose-ownership)
- For ownership management actions via `ownership.js`, see [Contract Ownership Management](./contract-ownership.md).

### Operator Actions

Actions that can be executed directly from operator wallet. Immediate execution, no timelock.

- **Execute Contract**: `evm/operators.js executeContract <target> <calldata> <value>` - [Execute contract (operators role)](../README.md#execute-contract-operators-role)

---

## Governance Execution Flow

For complete governance workflow details, see [Governance Workflows](./governance-workflows.md) and [Governance Documentation](./governance.md).

**Quick Reference:**
1. **Schedule**: Add `--governance --activationTime <time>` flags. Replace `schedule` with `schedule-operator` (for `governance.js`) or add `--operatorProposal` flag (for contract scripts) to skip timelock.
2. **Submit**: auto if `MNEMONIC` set; or submit `proposal.json` via Cosmos CLI (see [Submission Methods](./governance.md#submission-methods))
3. **Wait** for vote + GMP; if relayers fail, use `submit` or `submit-operator` commands (see [Steps After Scheduling](./governance-workflows.md#steps-after-scheduling-a-proposal))
4. **ETA**: Check proposal ETA (see [Check Proposal ETA](./governance.md#check-proposal-eta))
5. **Execute**: Execute proposal after ETA passes (see [Execute Proposal](./governance.md#execute-proposal))

**Notes**
- `--nativeValue` must match the value used when scheduling (hash includes it).
- Use `--generate-only proposal.json` to inspect calldata/payload before submitting.
- Operator proposals bypass timelock via operator approval on `AxelarServiceGovernance`.
