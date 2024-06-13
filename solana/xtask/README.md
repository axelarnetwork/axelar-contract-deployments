## Xtask CLI :joystick:

Xtask CLi helps humans and non humans to execute common operations on this workspace,
like building, deploying and intialising programas (A.k.a Solana smart contracts)

### How to run

This CLI is designed to only work within the workspace. It should be called with
`cargo` from anywhere in the workspace:

```bash
$ cd solana
$ cargo xtask --help
```

### How to test

```bash
$ cargo test
```

### Solana programs

- Solana programs use `cargo build-sbf` subcommand to build the programs, this produces a `[contract-name].so` artifact that needs to be deployed on the Solana chan
- The deployment of Solana programs is not exactly straight forward. Every program has a hardcoded program id `solana_program::declare_id!()` which is an ED25519 Public key. For you to be able to deploy the program and actually have it working, you need to have the coresponding private key for the hardcded `program id`.
- After the solana program has been deployed, it is stateless, and the initializatoin process is defined per-program level. Generally, initialization is a separate step that needs to be done post-deployment. Unless the PDAs are designed to have a configuration singleton, the program can be intialised multiple tiemes.

### EVM contracts

- The evm contracts don't need to built explicitly using xtask, that is handled by a `build.rs` file, that invokes `forge build` under the hood. The build script also generates Rust bindings for the EVM code.
- Contract deployment and intialization is done as a singular step.

### Cosmwasm contracts

- Cosmwasm contracts are built from a git submodule that points to the `axelar-amplifier` repo. Building the contracts requires setting up the appropriate Rust toolchain, installing wasm target, building the contracts and applying `wasm-opt` optimiser. This is handled by the `xtask` - it will also downolad the optimiser and run it over the compiled wasm contract code.
