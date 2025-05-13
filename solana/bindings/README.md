# SOLANA PROGRAMS BINDINGS

Inside the folder `generated`, there are already prepared bindings which work out of the box with the `memo-program`, `gateway` and `its` programs. 

Tests are run on the local node.

External binary `native-to-anchor` created the `idl.json` file and from it, lib file that represents each program and a `src` folder with typescript code.

On top of that, plain `Anchor.toml` has been provided that has been copied from `hello_world` example and adjusted accordingly.

# Testing bindings

To test if the generated bindings work, following procedure has to be done:

1. Start test validator
2. Run `gateway`, `its` and `memo-program` on local node
3. Invoke tests

## 1. Start test validator

In separate terminal, start local node.

```bash
solana-test-validator --reset
```

Additionally, logs could be added in separate terminal.

```bash
solana logs
```

## 2. Deploy Programs on local node

Use the `./prepare_test.sh` script from bindings folder.

```bash
./prepare_test.sh
```

## 3. Invoke tests

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

In case that some of the tests have failed, new issue reporting failures should be created.

# Regenerating bindings

To create new bindings, following procedure has to be done:

1. Preparing `native-to-anchor`
2. Generating programs bindings
3. Checking changes

## 1. Preparing `native-to-anchor`

First of all, proper version of `native-to-anchor` has to be used.

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

## 2. Generating programs bindings

To generate `memo-program`, `gateway` or `its` program bindings, it should be run in the following way:

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

In that way, new bindings are created in the `temp/` folder.

CAUTION: Updating bindings in their corresponding folders might cause significant changes. In case that it is necessary, it needs to be called like this:

```bash
cargo xtask create-bindings gateway -u
```

## 3. Checking changes

It is vital to check changes that have been provided from regenerating bindings. Post generating modifications have been done due to the limitations of the `native-to-anchor` binary. Therefore, comparing with the help of versioning system, re-running tests and detailed code review is necessary so that the bindings functionality remains on point.

### Current hacks that need to be respected

* The current import paths, which `native-to-anchor` tries to fix. Current ones need to be respected.
* The Roles enum. This struct has encoding issues if we allow `native-to-anchor` to generate it, as we need to respect the value bits in the original enum.

# Additional things to be checked

`native-to-anchor` bump to version 0.29 in the original repo so that everyone can call it.

Bunch of typescript packages are defined in `package.json`. Some of them are probably not necessary.
