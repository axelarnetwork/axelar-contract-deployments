# Axelar Deployment Setup

## Overview

This script facilitates the deployment of Axelar network components by automating configurations, wallet setup, contract deployments, and governance integrations. It supports multiple environments, including `mainnet`, `testnet`, `stagenet`, `devnet-amplifier`, and custom `devnet` deployments.

## Features

- Selectable network environments
- Automatic wallet creation
- Configuration file generation
- Contract deployment
- Governance and admin configurations
- Verifier and multisig setup

## Usage

### Running the Script

To start the deployment process, run:

```bash
./deployment.sh
```

Follow the interactive prompts to configure the deployment.

### Network Selection

Upon execution, you will be prompted to select a network:

- `mainnet`
- `testnet`
- `stagenet`
- `devnet-amplifier`
- Custom `devnet`

### Required Inputs

The script will request the following details:

- **Chain Name**: The name of the blockchain.
- **Chain ID**: The numerical ID for the chain.
- **Token Symbol**: Native token symbol.
- **Gas Limit**: Gas limit per transaction.
- **Private Key**: Private key for transaction signing. This key needs to be properly funded to deploy and interact with smart contracts on the target chain.
- **RPC URL**: Endpoint for blockchain interactions.
- **Axelar RPC URL**: Endpoint for Axelar node interactions.
- **Mnemonic**: Wallet mnemonic for key management. The deployer will need to have a properly funded Axelar wallet to perform deployments and proposals on the target Axelar network. The deploying wallet will be responsible for funding the validator reward pools that will support the chain.

### Custom Network Requirements

For custom devnets, the corresponding config JSON is required and should be copied to the `axelar-chains-config/info` directory.

## Deployment Steps

Deployments are broken into three steps:

1. **Initial Deployment**: This step sets up the necessary configurations, deploys contracts, and generates the required environment files.
2. **Verifier Registration & Multisig Proposals**: After the initial deployment, external actions must take place:
   - Verifiers must register the chain with Axelar.
   - Once verifiers have registered the chain, multisig proposals must be created and submitted.
   - The proposals then require validation and approval before moving forward.
3. **Completing Deployment**: Once the verifiers have registered the chain and multisig proposals have been verified, the script can proceed with finalizing the deployment steps. This includes confirming multisig contract interactions and ensuring the system is fully integrated.

## Configuration and Deployment Steps

### Environment Variable Setup

The script sets up and exports essential environment variables to be used in subsequent steps.

### JSON Configuration Generation

Creates `config.json` with the network and contract details. This is used to update the Axelar-chains-config file for the target network.

### Wallet Management

- Validates or creates a wallet using the provided mnemonic.
- Extracts wallet addresses and governance addresses.

### Contract Deployment

- Deploys `VotingVerifier`, `Gateway`, and `MultisigProver` contracts.
- Updates configuration files to include deployed contract addresses.

### Chain Registration

- Registers the deployed chain on Axelar’s governance structure.
- Configures voting verifier and multisig setups.

### Reward Pool Creation

For networks that support it, the script:

- Creates reward pools.
- Adds funds to reward pools.

### Verification Steps

- Verifies execution of the deployed contracts.
- Checks the registration of multisig contracts.

## Resuming Deployment

If deployment is interrupted, it can be resumed by selecting `no` at the prompt:

```bash
Is this a new deployment? (yes/no): no
```

This loads `deployment_config.json` to restore environment variables and continue from the last successful step.

After resuming, the script will ask additional questions to determine where the deployment left off:

1. **Have verifiers registered support for the chain?** If `no`, the script will exit, and verifiers must complete their process before resuming.
2. **Have multisig proposals been approved?** If `no`, the script will guide users through rechecking proposal status.
3. If both verifiers and multisig proposals are confirmed, the script will automatically proceed to the **Completing Deployment** stage.

If any step encounters a failure, the script will prompt for retry options, allowing users to:

- Retry failed steps immediately.
- Exit and manually resolve any issues before resuming.
- Re-run the verification process to confirm state consistency before proceeding.

## Custom Devnet Considerations

For custom devnets, the script handles:

- Dynamic governance setups
- Contract uploads using pre-defined WASM files
- Automatic retrieval of necessary parameters

## Error Handling

Common failure points include:

- **Invalid Private Key**: Ensure the private key starts with `0x`.
- **Invalid RPC URLs**: Must start with `http://` or `https://`.
- **Chain Already Exists in Config**: Prevents accidental overwrites.

## Conclusion

This script automates the complex deployment process for Axelar network integrations, handling configurations, deployments, and verifications efficiently. Users should ensure valid inputs and a stable internet connection during execution.

## Stopping Points

The entire deployment process is broken into multiple steps that account for asynchronous actions that must be coordinated to ensure the necessary deployments and configuration updates have occurred.

### Verifier Updates

The first stopping point is after the Gateway registration proposal has been submitted and presumably approved. The necessary actions that would need to be coordinated are the `config.toml` updates that verifiers need to perform by updating `ampd` with the `$CHAIN` chain name and the `http_url` (RPC node URL) they will utilize.

```bash
[[handlers]]
chain_finalization="RPCFinalizedBlock"
chain_name="$CHAIN"
chain_rpc_url=[http url]
cosmwasm_contract="$VOTING_VERIFIER"
type="EvmMsgVerifier"

[[handlers]]
chain_finalization="RPCFinalizedBlock"
chain_name="$CHAIN"
chain_rpc_url=[http url]
cosmwasm_contract="$VOTING_VERIFIER"
type="EvmVerifierSetVerifier"
```

```bash
ampd register-chain-support validators $CHAIN
```

### Multisig Proposals

After the gateway is registered and verifiers have been approved, the deployment script will perform a verification that the gateway has been properly registered and proceed to submit two proposals for registering in the multisig address: the `register_prover_contract` and the `authorize_callers`. Since these proposals need to be approved, the script will again stop to let voting proceed.

### Reward Pools and Final Contract Execution

Once the multisig proposals are approved, the script can continue, and the reward pools can be created directly along with the genesis verifier set. The last step is to deploy the gateway on the target chain.

## Major Pain Points

For each asynchronous step, there is coordination overhead. For the first stop, the verifier set needs to be arranged. Although this process does not need to be changed in `mainnet`, we can certainly automate further the `stagenet`, `testnet`, and `devnet-amplifier` deployments if we are able to register chains more directly. A more ambitious goal would be to allow the chain integrators to define the subset of validators that would serve the chain. Further discussion on security implications and technical challenges of such a refactor is needed.

The second aspect relevant for all stops is that the proposal flow has a time window where it can succeed or fail. We should be able to monitor if and when a proposal passes to proceed with the deployment steps automatically. Effectively, having some mechanism to poll a given proposal that can then continue deployment would further streamline the deployment.

Private key management is not going to pass audits for this script as we handle private key data too publicly. In general, the underlying usage of `MNEMONIC` and `PRIVATE_KEY` would need to be extended to support remote signing operations. Key sharing is not recommended in most cases, so having the ability to remotely sign transactions can be desirable for more security-oriented teams.

