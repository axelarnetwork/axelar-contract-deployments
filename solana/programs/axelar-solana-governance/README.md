
## High level overview

[![governance hl](https://github.com/user-attachments/assets/d771438c-b7ac-4d69-92d7-3c131ce91bd9)](https://excalidraw.com/#json=pVoAXLtjUps5y9nU8wYu2,cz_P-xoEobAN9qbfe0-MwQ)

The governance module allows decisions taken on the Axelar network to be propagated and executed on the different integrated chains, giving a chance (by timelock) to each chain maintainer to prepare for it's execution. So the governance module acts as a "approved proposal's forwarder" which is connected to the Axelar governance infrastructure via [GMP](https://www.axelar.network/blog/general-message-passing-and-how-can-it-change-web3) messages.

All the voting happens on the axelar side. Once an approved proposal
is forwarded to the governance module, it might be executed in the Solana blockchain either by a normal solana actor when the time lock ends, or by an operator actor role anytime, if and only if the operator was approved to do so by the Axelar network via another GMP message. Operators might be multisig schemes.

Apart from the scheduled time lock proposal command, there are other [GMP commands](./tests/module/gmp/) which helps managing the proposal lifecycle, like cancelling or putting the proposal under the control of the operator. The governance module will only accept GMP commands coming from the Axelar governance and verified by the gateway.

[Native instructions](./tests/module/) works as a second but separated part of the flow, used for local network (Solana in this case) operations.

## Governance module design

The governance module program is a port from the original Solidity implementation. We encourage reading 
it's [design  docs](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/DESIGN.md). before continuing.

It's good to clarify the above design docs contain 2 key contracts:

* [InterchainGovernance.sol](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/InterchainGovernance.sol) - This contract implements the first GMP commands `ScheduleTimeLockProposal` and `CancelTimeLockProposal` plus all their core logic.
* [AxelarServiceGovernance.sol](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol) - This extends the previous mentioned contract by adding all the operator
  role level GMP capabilities `ApproveOperatorProposal`, `CancelOperatorApproval` and all their supportive logic.

In Solana there's no existence of the inheritance concept, so you can find here a flat design, which preserves all original contract invariants.

All the data types coming from the Axelar network needs to be respected in all public interfaces/storage, as they are used for hashing operations that could be lead to further checkins.

## The GMP message payload

The GMP messages are coming from the Axelar network and we should respect their form and encoding (ABI encoding). In order to help on that, the crate [governance-gmp](./../../helpers/governance-gmp/) was created.

The payload ([call_data]((./../../helpers/governance-gmp/)) field) of the governance command structure is meant to be the borsh serialized version of the [ExecuteProposalData](./src/state/proposal.rs) type.

Building GMP messages is made easy for callers thanks to the ix builder. See [how to interact with this program](#how-to-interact-with-this-program) section for more information.


## PDA and data structures

There are some PDA's involved in the governance module:

* The `config PDA`, in which the program stores its configuration and which pubkey should be set as `upgrade_authority` when executing program updates through proposals. [See this test example](./tests/module/gateway_upgrade.rs) for a complete example.

* The `proposal PDA`. It is created with the hash of the elements of the proposal following original [EVM implementation](#governance-module-design) . Check [proposal.rs](./src/state/proposal.rs) to see hashing functions. This pda stores proposal related data, like the timelock eta and [canonical bump seeds](https://solana.com/developers/courses/program-security/bump-seed-canonicalization).

* The `managed proposal PDA`. This is just a "marker PDA" which tells the system whether a proposal can be directly executed by a Operator, without the need of accomplishing the proposal timelock. This PDA derivation
  [takes as a seed](./src/state/operator.rs) the proposal hash.

**A note on GMP payload PDA's**: In solana we have some limits (1232 bytes) regarding how much can be sent on a transaction. In order to workaround this, the GMP payload must be first stored on a dedicated account by the caller (on the governance module case, the Axelar governance) and such account to be sent along the GMP instruction.
We can check how we reference the message metadata this in the [GovernanceInstruction::ProcessGMP struct](./src/instructions.rs). For uploading the payload to the account, we can take as an example the test helpers [approve_ix_at_gateway()](./tests/module/helpers.rs).

## How to interact with this program

The best way to interact with this program is to use the [IxBuilder](./src/instructions.rs) provided by this program lib. It will help developers to quickly build the needed instructions without dealing with all the internal representations and program accounts order.

Check the `IxBuilder` tests on [it's module](./src/instructions.rs) for a better view of the example [use cases](./tests/module/).
