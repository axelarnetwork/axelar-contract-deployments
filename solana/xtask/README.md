## Xtask CLI :joystick:

Xtask CLi helps humans and non humans to execute common operations on this workspace,
like building, deploying and intialising programas (A.k.a Solana smart contracts)

### How to run

This CLI is designed to only work within the workspace. It should be called with
`cargo` from anywhere in the workspace:

```bash
$ cd solana
$ cargo run -p xtask -- --help
```

### How to test

```bash
$ cargo test
```

**A note on manual testing**

TL;DR - Create or replace the `solana/target/deploy/gmp_gateway-keypair.json` file with the content of our [our custom keypair](https://www.notion.so/Environments-108e35601c544847811a98bc716740b0?pvs=4#e133b9794b3143f0abf49362d3317e09)
in order for the manual deploy workflow to work.

When we build contracts a `contract-keypair.json` is placed next to it's `contract.so` file. Such json file contains
the Keypair data, which it's public half identifies the program in the network. This fixed identity is enforced inside the program with the `solana_program::declare_id!()` helper.