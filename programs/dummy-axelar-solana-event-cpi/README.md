# Memo program

## Contract id

Contract id is set to default value in `./src/lib.rs` as shown in here:

```bash
solana_program::declare_id!("cpi1111111111111111111111111111111111111111");
```

Currently, id values can be changed for `stagenet` or `devnet`. To apply it, pre-compilation script `./build.rs` is invoked before compilation and id update in `./src/lib.rs` is done when environment variable `CHAIN_ENV` is set in the following way:

```bash
CHAIN_ENV=stagenet cargo build-sbf
```

In case that id needs to be changed for `devnet`, id value needs to be reset to the default one. Here is an example of reset with versioning system:

```bash
git checkout -- .
CHAIN_ENV=devnet cargo build-sbf
```
