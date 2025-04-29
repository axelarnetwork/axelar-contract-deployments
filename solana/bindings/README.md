# Instructions for working with bindings

Here are already prepared bindings which work out of the box with the `memo-program`, `gateway` and `its` programs. 

Tests are run on the local node.

External binary `native-to-anchor` created the `idl.json` file and from it, lib file that represents each program and a `src` folder with typescript code.

On top of that, plain `Anchor.toml` has been provided here that has been copied from `hello_world` example and adjusted accordingly.

## NO NEED TO RUN `generate_bindings.sh`

CAUTION: If it is necessary to create new bindings, proper version of `native-to-anchor` has to be used.

From the `root` of the folder, clone the following repo in a folder above:

```bash
cd ..
git clone git@github.com:eigerco/native-to-anchor.git
cd native-to-anchor/generator/
cargo build
```

Because of the complexity of our programs, custom made `anchor` files are already prepared in `anchor_lib/`.

Binary `native-to-anchor` that uses these files, was not developed for 3 years, and last version of `anchor` that is used in it is `0.25`.

Because advanced features are necessary for generating bindings, original `native-to-anchor` repository has been forked and version of `anchor` was bumped to `0.29`.

That is why in script, `native-to-anchor` is being called with an absolute path. Probably the version is going to be bumped also in the original repository so that the updated version of `native-to-anchor` could be installed via `cargo install`.

Initially, folder `generated` has been built just by calling `native-to-anchor` on the target `program`, but the binary also generates additional files which are unnecessary and are stored in the `temp` folder. There is no need to run `native-to-anchor`, because the bindings are already prepared.

In case that bindings need to be generated, it can be used to generate `memo-program`, `gateway` or `its` program. It is used in the following way:

```bash
cd <repo_root>/solana
```

To generate `memo-program`:

```bash
cargo xtask create-bindings memo-program
```

To generate `gateway`:

```bash
cargo xtask create-bindings gateway
```

To generate `its`:

```bash
cargo xtask create-bindings its
```

In that way, new bindings are created in the `temp/` folder. In case it is necessary to update the bindings in the correspondings folder, it needs to be called like this:

```bash
cargo xtask create-bindings gateway -u
```

## Run Memo Program on local node

In separate terminal, start local node.

```bash
solana-test-validator --reset
```

Additionally, logs could be added in separate terminal.

```bash
solana logs
```

From the `root` of the repository build `memo-program`:

```bash
cd solana/programs/axelar-solana-memo-program && cargo build-sbf
```

And deploy it:

```bash
cd ../../../ && solana program deploy solana/target/deploy/axelar_solana_memo_program.so --program-id solana/target/deploy/axelar_solana_memo_program-keypair.json
```

CAUTION: Different Program Id could be provided when deployed. In that case, it is necessary to update this newly created Id in `solana/programs/axelar-solana-memo-program/src/lib.rs`, rebuild it and redeploy it. Additionally in `bindings/generated/axelar-solana-memo-program/program.ts`, value `AXELAR_SOLANA_MEMO_PROGRAM_PROGRAM_ID` has to be updated with the same value.

## Run Gateway on local node

From the `root` of the repository build `gateway`:

```bash
cd solana/programs/axelar-solana-gateway && cargo build-sbf 
```

And deploy it:


```bash
cd ../../../ && solana program deploy solana/target/deploy/axelar_solana_gateway.so --program-id solana/target/deploy/axelar_solana_gateway-keypair.json
```

CAUTION: Different Program Id could be provided when deployed. In that case, it is necessary to update this newly created Id in `solana/programs/axelar-solana-gateway/src/lib.rs`, rebuild it and redeploy it. Additionally in `bindings/generated/axelar-solana-gateway/program.ts`, value `AXELAR_SOLANA_GATEWAY_PROGRAM_ID` has to be updated with the same value.

## Run ITS on local node

From the `root` of the repository build `its`:

```bash
cd solana/programs/axelar-solana-its && cargo build-sbf 
```

And deploy it:


```bash
cd ../../../ && solana program deploy solana/target/deploy/axelar_solana_its.so --program-id solana/target/deploy/axelar_solana_its-keypair.json
```

CAUTION: Different Program Id could be provided when deployed. In that case, it is necessary to update this newly created Id in `solana/programs/axelar-solana-its/src/lib.rs`, rebuild it and redeploy it. Additionally in `bindings/generated/axelar-solana-its/program.ts`, value `AXELAR_SOLANA_ITS_PROGRAM_ID` has to be updated with the same value.

## Invoke the test

Before starting the test, change folder to `bindings`:

```bash
cd bindings
```

Install the dependencies:
 
```bash
pnpm install
```

And run the test:

```bash
anchor test --skip-local-validator
```

Following similar messages should appear:

```bash
Ping Gateway
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ ApproveMessage (76ms)
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ RotateSigners
```

In case that some of the tests have failed, issue report has to be done.

## Additional things to be checked

`native-to-anchor` bump to version 0.29 in the original repo so that everyone can call it.

Bunch of typescript packages are defined in `package.json`. Some of them are probably not necessary.
