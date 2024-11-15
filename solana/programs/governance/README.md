## Governance module design

The governance module program is a port from the original Solidity implementation. We encourage reading 
it's [design  docs](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/DESIGN.md). before continuing.

It's good to clarify the above design docs contain 2 key contracts:

* [InterchainGovernance.sol](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/InterchainGovernance.sol) - This contract implements the first GMP commands `ScheduleTimeLockProposal` and `CancelTimeLockProposal` plus all their core logic.
* [AxelarServiceGovernance.sol](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol) - This extends the previous mentioned contract by adding all the operator
  role level GMP capabilities `ApproveOperatorProposal`, `CancelOperatorApproval` and all their supportive logic.

In Solana there's no existence of the inheritance concept, so you can find here a flat design, which preserves all original contract invariants.

All the data types coming from the Axelar network needs to be respected in all public interfaces/storage, as they are used for hashing operations that could be lead to further checkins.

## How to interact with this program

The best way to interact with this program is to use the [IxBuilder](./src/instructions.rs) provided by this program lib. It will help developers to quickly build the needed instructions without dealing with all the internal representations and program accounts order.

Check the `IxBuilder` tests on [it's module](./src/instructions.rs) for a better view of the example use cases.
