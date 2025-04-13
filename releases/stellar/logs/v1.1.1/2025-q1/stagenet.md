# GMP Deployments

### Upgrade the AxelarGateway contract and migrate storage schema

```
node stellar/deploy-contract.js upgrade AxelarGateway --version 1.1.1 --migration-data '[["stellar-2025-q1","0x92de6c8db2aabb7f7d42c7257b0b417a82118d6cdaac15ab7baede65a4879875-2"],["stellar-2025-q1","0x21f4d0219798fc132aad1329fe31f7c8ef838cbdb0e1c1a116c0e11f240d7dd0-2"],["stellar-2025-q1","0x183ae1e6933a4f800d2dbccde49e366bb72e2e01befd5bc0c7c2f9ca08a217ad-2"],["stellar-2025-q1","0x0ecaf9dfb0cdd75edad060f91cf9959cee1e13859b292e6a8c01a265be04ffb6-2"],["stellar-2025-q1","0xfa224e4184d1b98ce3d4852fad2eb7ebee8da64d3913b683cd9c16a89af9fe59-2"],["stellar-2025-q1","0x5cea873108514d2cf78365ae6d0e24e2a3afd639caf25b44483efe183cc22610-2"],["stellar-2025-q1","0x6bfc1d27e994da15ecf816b5b774033bd964ef0cb77ac8b965bd51bdc9167e4e-2"],["stellar-2025-q1","0x79a40ba475526130615a27f715f767890660a061f293bf5668758103c2af35f0-2"],["stellar-2025-q1","0x55e56efbf26f4663b3545db71b379c3e0e2f6976805fdce7714753947cc52832-2"],["stellar-2025-q1","0x63e1478a85ae18909f6a76162c625baf2c66623513404a0acaaa00f05a5b7b2e-2"],["stellar-2025-q1","0x88f46dbe5aff9ebb0861862fcaecc584f8025b9eab74a379e02f9c4cbbeb2d16-2"]]'

Upgraded contract tx: a2731a2d09ae14aa0c99c6f140322871ce0cf79b1a4bb8a0ec27f6cace35f713
```

### Upgrade AxelarOperators & migrate storage schema

```
node stellar/deploy-contract.js upgrade AxelarOperators --version 1.1.1 --migration-data '["GAY6RYZLHDSYQ7Y3X2CRSOTB6PVXAQ3IRTQFIAATYSP2TX7N25HVSJEV","GBNSB3AHRLVVXBWZFCFSWBUG6RZHNSOIDIH7VGKL2GHVFXFVV3I6I5AM"]'

Upgraded contract tx: 7937e2673b8bc447cb19478fbfc4d16d421120413601bc455bb0446aa63ff236
```

### Upgrade the AxelarGasService contract

```
node stellar/deploy-contract.js upgrade AxelarGasService --version 1.1.1

Upgraded contract tx: bf40a43f69cc88bcb6ac1be1a42be4072fb77bf398fa0c70a8149ae0080d15d1
```

### Deploy the AxelarExample contract

```
node stellar/deploy-contract.js deploy AxelarExample --version 1.0.3

Initialized contract tx: ae1df3153084cc432050d22185db7e1f5bc9bc25468d69d41d98ddf4e75ea6b4
```

### Deploy the Multicall contract

```
node stellar/deploy-contract.js deploy Multicall --version 1.0.1

Contract initialized at address: CDY327IFXISX2WW6OFCCHM5VVQ2W3DFT2LPEWOFERSRFLWGPH3RLZVMK
```

## Verify GMP call

### Verify Stellar → EVM GMP Call

```
node stellar/gmp.js send avalanche 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234 --gas-amount 100000000

https://stagenet.axelarscan.io/gmp/064a45a48c072f578b69b9b0db21079065f31d00fc752775f8f4efdf4a42714c
```

### Verify EVM → Stellar GMP Call

```
node evm/gateway.js -n flow --action callContract --destinationChain stellar-2025-q1 --destination CDMT7AQM5WE7KGVS2257SGDZH6TA7KBFHQM2N4VWOWSWUV3GACR4YU3H --payload 0x1234

https://stagenet.axelarscan.io/gmp/0x005d0a518b477215cad1ffdfc1b253144451359fbc0fd620ffe2a17c21d9bd92
```

# ITS Deployments

### Upload TokenManager & retrieve WASM hash

```
node stellar/deploy-contract.js upload TokenManager --version 1.1.1

Uploaded TokenManager wasm tx: 820edbe4f401491a38c1ff001ed1443fbea61d96a5494b5be5e2b303568dc1ef
```

### Upload InterchainToken & retrieve WASM hash

```
node stellar/deploy-contract.js upload InterchainToken --version 1.1.1

Uploaded InterchainToken wasm tx: b5e7eed7c5d2d8002d248275362b92fa3472fa9456a165e2602cb9ef3ccb4c64
```

### Upgrade InterchainTokenService & migrate storage schema

```
node stellar/deploy-contract.js upgrade InterchainTokenService --version 1.1.2 --migration-data '{"newTokenManagerWasmHash":"9883cdb6740d685007a88dbd5a1269ffc1550a81a182fed5de3748356d0e342c","newInterchainTokenWasmHash":"71648e1b8ee1231ea03c096257ed3c6d55478effbfab77b8e587a5139ab84cbe"}'

Uploaded InterchainTokenService wasm tx: 36e9adada74f4f927c37a3269951b664f99f6af38aa9f711f6ed81edbfdf3cc1
```

### Call ITS::migrate_token for ALL deployed tokenIds (base64)

```
node stellar/its.js migrate-tokens "bk5TOcAqehReN14EFgsCy2nm+cXT4e+zjs+IsJ5zq6g=" --version 1.1.1

Migrating token: 0x6e4e5339c02a7a145e375e04160b02cb69e6f9c5d3e1efb38ecf88b09e73aba8

Upgrader address: CA5JVDIOAEEQ5F2S2D5OGPNM7Z27Y7ZT4ZVHMFALRE6PEAFFMFXDTSCZ

Retrieved TokenManager address tx: c470e0180d2857c59c824bf20d3318c5b17f604b1e6f360fbef342e09332fc54

TokenManager address: CBFVXVDMUDRL74GDCV3ZYILNXW345O22IU6I35H2M4POVYSDTWZ33ZMZ

Retrieved InterchainToken address tx: 2471dbd08cd4e59f871232ce83da96c8e3a813e692a7d6a411b7f42ab7e4241a

InterchainToken address: CBUZMNF6UIN4QHQLBPQX2XJTEAOUYGVVIRXGNTAPWXYVOVVSYOTGKY7U

Migrated token tx: b8b147071de0081ca91553bd3aa17a71d67be9400678cd682b9406a9fadef87b
```

### Deploy Upgrader

```
node stellar/deploy-contract.js deploy Upgrader --version 1.1.1

Initialized contract tx: 174c4794d93e49ff246d99fb8563b1d6bfa7d4ef7ad9ed59c1609fcab7587518
```

# ITS Checklist

## Stellar → EVM

```
node stellar/its.js deploy-interchain-token TEST3 test3 18 0x8910 10000000

Interchain Token Deployed tx: b249f1096c8a775b3aa4b0940ec54c7043f92cdca04b1d9a2146c1b015e409e3
```

```
node stellar/its.js deploy-remote-interchain-token 0x8910 flow --gas-amount 50000000

https://stagenet.axelarscan.io/gmp/21f04be1f6cc5ee8773ea65e988b322df14c95a5e2d038f0af24879dd09668d5
```

## EVM to Stellar

```
node evm/interchainTokenFactory.js --action deployInterchainToken -n flow --destinationChain stellar-2025-q1 --salt "test3" --name "test3" --symbol "TEST3" --decimals 18

deployInterchainToken tx: 0x9356315fa63f62f069956b1dba9dd2e5019b0029224d510278697f6b82d8c0fc
```

```
node evm/interchainTokenFactory.js --action deployRemoteInterchainToken -n flow --destinationChain stellar-2025-q1 --salt "test3" --gasValue 1000000000000000000

https://stagenet.axelarscan.io/gmp/0x4c5ba293957c9c08b3872c735067ea0d6bb7c60e2f83eb0560d441a971fee9c2
```
