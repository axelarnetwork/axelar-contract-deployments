// - TODO: successfully process execute when there are no commands
// - TODO: successfully process execute when there are 3 validate contract call
// commands - emits message approved events
// - TODO: successfully process execute when there is 1 transfer
// operatorship commands
// - TODO: successfully process execute when there is 1
// transfer operatorship and 3 validate contract call commands
// - TODO: successfully process execute when there are 3 transfer operatorship
// commands - only the first one should be executed
// - TODO: if a given command is
// a part of another batch and it's been executed, it should be ignored in
// subsequent batches if its present in those (to replicate this: transfer ops 1
// batch, transfer another one, then another batch with the command from the
// first one)
//
// - TODO: fail if gateway config not initialized
// - TODO: fail if execute data not initialized
// - TODO: fail if invalid account for gateway passed (e.g. initialized command)
// - TODO: fail if invalid account for execute data passed (e.g. initialized
//   command)
// - TODO: fail if epoch for operators was not found (inside `validate_proof`)
// - TODO: fail if operator epoch is older than 16 epochs away (inside
//   `validate_proof`)
// - TODO: fail if signatures were invlaid (inside `validate_proof`)
// - TODO: disallow operatorship transfer if any other operator besides the most
//   recent epoch signed the proof (inside `validate_proof`)
// - TODO: fail if command len does not match provided account iter len
// - TODO: fail if command was not intialized
// - TODO: fail if order of commands is not the same as the order of accounts
// - TODO: fail if approved command is not pending, thus cannot be set as
//   approved
// - TODO: fail if transfer ops command is not pending, thus cannot be set as
//   executed
// - TODO: `transfer_operatorship` is ignored if new operator len is 0 (tx
//   succeeds)
// - TODO: `transfer_operatorship` is ignored if new operators are not sorted
//   (tx succeeds)
// - TODO: `transfer_operatorship` is ignored if operator len does not match
//   weigths len (tx succeeds)
// - TODO: `transfer_operatorship` is ignored if total weights sum exceed u256
//   max (tx succeeds)
// - TODO: `transfer_operatorship` is ignored if total weights == 0 (tx
//   succeeds)
// - TODO: `transfer_operatorship` is ignored if total weight is smaller than
//   new command weight quorum (tx succeeds)
// - TODO: `transfer_operatorship` is ignored if operator hash collides (tx
//   succeeds)
