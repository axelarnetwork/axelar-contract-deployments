<details>
<summary>Stellar Contracts Deployed</summary>

| Contract                  | TX                                                                                                          | CMD                                                                                                                                   |
| ------------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `AxelarGateway`           | https://stellar.expert/explorer/testnet/tx/6f9502c78a4ee9ae7299d3ab21da3c81ddfe1a30bd87bb0c2ffb057f532af31b | `ts-node stellar/deploy-contract.js deploy AxelarGateway --version v1.0.0 --minimum-rotation-delay 3600 --previous-signers-retention 15` |
| `AxelarOperators`         | https://stellar.expert/explorer/testnet/tx/1dcf9f86d789491bc309afbb1bf410e5ae31f57b07e1826449ef1fec76fdddde | `ts-node stellar/deploy-contract.js deploy AxelarOperators --version v1.0.0`                                                             |
| `AxelarGasService`        | https://stellar.expert/explorer/testnet/tx/86dca8168173ca1554ceb895bfcc9e0b5265a0ea99af9bdd672919e8dd64cfb5 | `ts-node stellar/deploy-contract.js deploy AxelarGasService --version v1.0.0`                                                            |
| `Upgrader`                | https://stellar.expert/explorer/testnet/tx/261741e64e56fe938365f651fb5c918d1d2ec172d1c148559cdfe3d31b163806 | `ts-node stellar/deploy-contract.js deploy Upgrader --version v1.0.0`                                                                    |
| `InterchainTokenService ` | https://stellar.expert/explorer/testnet/tx/4882c685c2da6ed45945a98c9eb3251a6e5149b82d48e17e1cd46b7df4510dac | `ts-node stellar/deploy-contract.js deploy InterchainTokenService --version v1.0.0`                                                      |
| `AxelarExample`           | https://stellar.expert/explorer/testnet/tx/faee5419b86ab4bab3bab67400b79246da6b205226a17380ddecfc5d44579c8e | `ts-node stellar/deploy-contract.js deploy AxelarExample --wasm-path artifacts/stellar_example-v1.0.0.optimized.wasm`                    |

</details>

<details>
<summary>Stellar WASM contracts</summary>

| Contract         | TX                                                                                                      | CMD                                                                                                                                                     |
| ---------------- | ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `VotingVerifier` | https://testnet.axelarscan.io/account/axelar1a4wt84rllhuwpdvymj4tql6cugfsmdmau9ufmzcm329yx9st96eqx05uam | `ts-node ./cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2 --admin "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"` |
| `Gateway`        | https://testnet.axelarscan.io/account/axelar1h5mvyzjjara9a5jk4psayas6pg9c55llay02mmaprzmfk5r6epfqqadvs4 | `ts-node ./cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2 --admin "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"`        |
| `MultisigProver` | https://testnet.axelarscan.io/account/axelar1cypnpcqk4zpk32stl7dutv3cdnag0q2v5a7dzfxh4jukrzxagxpqgjl5sr | `ts-node ./cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2 --admin "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"` |

</details>

<details>
<summary>Register stellar gateway at the Router</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js execute \
  -c Router \
  -t "Register Gateway for stellar" \
  -d "Register Gateway address for stellar at Router contract" \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"register_chain\": {
      \"chain\": \"stellar-2025-q1\",
      \"gateway_address\": \"$GATEWAY\",
      \"msg_id_format\": \"hex_tx_hash_and_event_index\"
      }
    }"
Encoded /cosmwasm.wasm.v1.ExecuteContractProposal: {
  "title": "Register Gateway for stellar",
  "description": "Register Gateway address for stellar at Router contract",
  "runAs": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
  "contract": "axelar1d9atnamjjhtc46zmzyc202llqs0rhtxnphs6mkjurekath3mkgtq7hsk93",
  "msg": {
    "register_chain": {
      "chain": "stellar-2025-q1",
      "gateway_address": "axelar1h5mvyzjjara9a5jk4psayas6pg9c55llay02mmaprzmfk5r6epfqqadvs4",
      "msg_id_format": "hex_tx_hash_and_event_index"
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 238
```

</details>

<details>
<summary>Register Multisig Prover for stellar</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js execute \
  -c Coordinator \
  -t "Register Multisig Prover for stellar" \
  -d "Register Multisig Prover address for stellar at Coordinator contract" \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"register_prover_contract\": {
      \"chain_name\": \"stellar-2025-q1\",
      \"new_prover_addr\": \"$MULTISIG_PROVER\"
    }
  }"
Encoded /cosmwasm.wasm.v1.ExecuteContractProposal: {
  "title": "Register Multisig Prover for stellar",
  "description": "Register Multisig Prover address for stellar at Coordinator contract",
  "runAs": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
  "contract": "axelar1rwy79m8u76q2pm3lrxednlgtqjd8439l7hmctdxvjsv2shsu9meq8ntlvx",
  "msg": {
    "register_prover_contract": {
      "chain_name": "stellar-2025-q1",
      "new_prover_addr": "axelar1cypnpcqk4zpk32stl7dutv3cdnag0q2v5a7dzfxh4jukrzxagxpqgjl5sr"
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 239
```

</details>

<details>
<summary>Authorize Multisig Prover for stellar</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js execute \
  -c Multisig \
  -t "Authorize Multisig Prover for stellar" \
  -d "Authorize Multisig Prover address for stellar at Multisig contract" \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"authorize_callers\": {
      \"contracts\": {
        \"$MULTISIG_PROVER\": \"stellar-2025-q1\"
      }
    }
  }"
Encoded /cosmwasm.wasm.v1.ExecuteContractProposal: {
  "title": "Authorize Multisig Prover for stellar",
  "description": "Authorize Multisig Prover address for stellar at Multisig contract",
  "runAs": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
  "contract": "axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5",
  "msg": {
    "authorize_callers": {
      "contracts": {
        "axelar1cypnpcqk4zpk32stl7dutv3cdnag0q2v5a7dzfxh4jukrzxagxpqgjl5sr": "stellar-2025-q1"
      }
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 240
```

</details>

<details>
<summary>Create pool for stellar in stellar voting verifier</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for stellar in stellar voting verifier" \
  -d "Create pool for stellar in stellar voting verifier" \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"create_pool\": {
      \"params\": {
        \"epoch_duration\": \"100\",
        \"participation_threshold\": [\"7\", \"10\"],
        \"rewards_per_epoch\": \"100\"
      },
      \"pool_id\": {
        \"chain_name\": \"stellar-2025-q1\",
        \"contract\": \"$VOTING_VERIFIER\"
      }
    }
  }"
Encoded /cosmwasm.wasm.v1.ExecuteContractProposal: {
  "title": "Create pool for stellar in stellar voting verifier",
  "description": "Create pool for stellar in stellar voting verifier",
  "runAs": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
  "contract": "axelar1harq5xe68lzl2kx4e5ch4k8840cgqnry567g0fgw7vt2atcuugrqfa7j5z",
  "msg": {
    "create_pool": {
      "params": {
        "epoch_duration": "14845",
        "participation_threshold": [
          "7",
          "10"
        ],
        "rewards_per_epoch": "100"
      },
      "pool_id": {
        "chain_name": "stellar-2025-q1",
        "contract": "axelar1a4wt84rllhuwpdvymj4tql6cugfsmdmau9ufmzcm329yx9st96eqx05uam"
      }
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 241
```

</details>

<details>
<summary>Create pool for stellar in axelar multisig</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for stellar in axelar multisig" \
  -d "Create pool for stellar in axelar multisig" \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"create_pool\": {
      \"params\": {
        \"epoch_duration\": \"100\",
        \"participation_threshold\": [\"7\", \"10\"],
        \"rewards_per_epoch\": \"100\"
      },
      \"pool_id\": {
        \"chain_name\": \"stellar-2025-q1\",
        \"contract\": \"$MULTISIG\"
      }
    }
  }"
Encoded /cosmwasm.wasm.v1.ExecuteContractProposal: {
  "title": "Create pool for stellar in axelar multisig",
  "description": "Create pool for stellar in axelar multisig",
  "runAs": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
  "contract": "axelar1harq5xe68lzl2kx4e5ch4k8840cgqnry567g0fgw7vt2atcuugrqfa7j5z",
  "msg": {
    "create_pool": {
      "params": {
        "epoch_duration": "14845",
        "participation_threshold": [
          "7",
          "10"
        ],
        "rewards_per_epoch": "100"
      },
      "pool_id": {
        "chain_name": "stellar-2025-q1",
        "contract": "axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
      }
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 242
```

</details>

<details>
<summary>Register Stellar to ITSHub contract</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js \
    its-hub-register-chains $CHAIN \
    -t "Register $CHAIN on ITS Hub" \
    -d "Register $CHAIN on ITS Hub" \
    --deposit $DEPOSIT_VALUE
Encoded /cosmwasm.wasm.v1.ExecuteContractProposal: {
  "title": "Register stellar-2025-q1 on ITS Hub",
  "description": "Register stellar-2025-q1 on ITS Hub",
  "runAs": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
  "contract": "axelar1aqcj54lzz0rk22gvqgcn8fr5tx4rzwdv5wv5j9dmnacgefvd7wzsy2j2mr",
  "msg": {
    "register_chains": {
      "chains": [
        {
          "chain": "stellar-2025-q1",
          "its_edge_contract": "CCXT3EAQ7GPQTJWENU62SIFBQ3D4JMNQSB77KRPTGBJ7ZWBYESZQBZRK",
          "truncation": {
            "max_uint": "170141183460469231731687303715884105727",
            "max_decimals_when_truncating": 255
          }
        }
      ]
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 243

https://testnet.axelarscan.io/proposal/243
```

</details>

<details>
<summary>Stellar WASM contract Registration</summary>

| Operation                     | TX                                                                                                 | CMD                                                                                                             |
| ----------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `Create genesis verifier set` | https://stagenet.axelarscan.io/tx/C60A4A5A1A31D047B75D24C7A3017C682F4D3D349DB54FD7015A72FD30F7A40B | `axd tx wasm execute $MULTISIG_PROVER '"update_verifier_set"' --from amplifier --gas auto --gas-adjustment 1.2` |

</details>

<details>
<summary>Stellar Operations</summary>

| Ops             | TX                                                                                                          | CMD                              |
| --------------- | ----------------------------------------------------------------------------------------------------------- | -------------------------------- |
| `Rotate Signer` | https://stellar.expert/explorer/testnet/tx/1826e00ce3856e9b25711d9168a4d43f4149b4f551d7b0a96ac961de2194f842 | `ts-node stellar/gateway.js rotate` |

</details>

<details>
<summary>Trust All EVM Chains on Stellar</summary>

```
ts-node stellar/its.js add-trusted-chains all
Wallet address: GBP4FSAOFV5O72AB3YQRDCYVD47W4N7KQK3OJODXSU3OBPNGKX4SQTJ3

Wallet balances: 9962.4203987 XLM

Wallet sequence: 2418066587705

Proceed with action addTrustedChains (y/n) y

Missing InterchainTokenService edge contract for chain: Centrifuge

Missing InterchainTokenService edge contract for chain: Hedera

set_trusted_chain: ethereum-sepolia

Is trusted chain tx: a0bc647315a843c4225baa400edcfef47aee12f2cb348ffe33ec0d9ed9d515f3

set_trusted_chain tx: 4a7ef3d5db1fdd2143e76d00057d422064ff160d45de59eeedc228d90e175064

Successfully added trusted chain: ethereum-sepolia

set_trusted_chain: Avalanche

Is trusted chain tx: f02077dadc52e8edcd57d0edebc1217d46fe8806f64bde90b8b9bac98350cc9d

set_trusted_chain tx: 893691c04ae6d51496fb7fc75f9c73c429e30ea24b7a896124eb38f4c3f7bda4

Successfully added trusted chain: Avalanche

set_trusted_chain: Fantom

Is trusted chain tx: 796567cad33d7c0b4b6909b6c6837501e7cba833aad2b8022a3b3e15b6a59a10

set_trusted_chain tx: 6989c39a84974016f9c72ee705d584db8663549e7e55f583a8d54c5c16f0dd5d

Successfully added trusted chain: Fantom

set_trusted_chain: Moonbeam

Is trusted chain tx: 28fd48f7bcf2ac7ff21a3acb539f80a1bbfd37949627272e812c4ee1fd3bade3

set_trusted_chain tx: f239c40b4fefa9235b9b15c90500bd94c5ee28a4614631e6db4eb0d257be95b4

Successfully added trusted chain: Moonbeam

set_trusted_chain: binance

Is trusted chain tx: 941b88f181c806a3af426e9dd691bee6f9e2f82980d9a9a8b1d78d037056c2ff

set_trusted_chain tx: 38fe2ebef696e0686d0204d8b70f7d810686d2cd8900f72f8692d6332fbf7040

Successfully added trusted chain: binance

set_trusted_chain: celo

Is trusted chain tx: efe441c5c66c4effe00f9599452769dce99b94b6d057142236038d5eca99023c

set_trusted_chain tx: 986f49358a6628646be49803d728723ab1b761fbe26d17bd89791173ebe7cca0

Successfully added trusted chain: celo

set_trusted_chain: kava

Is trusted chain tx: 13c317c22acabcf4f2787cbe3c386d8dfe674ef705a8fdefef2be4a90f553599

set_trusted_chain tx: cf8bc5afaf4e3b274713e8c756c5b3d19ece561abfaeedf3bb884eaff73a1bcf

Successfully added trusted chain: kava

set_trusted_chain: filecoin-2

Is trusted chain tx: 9a941b6927f674fa2d5d2884cefd88559b29776199a5d1b4e9e7010e75559739

set_trusted_chain tx: 667fa72e65d2f285d55e09462cccd2a7f949e2a8007b10c6d0f225332ce0f57c

Successfully added trusted chain: filecoin-2

set_trusted_chain: scroll

Is trusted chain tx: b35549b6968c68c5ae99b1cfa1898ad8e4f4236a46d25e2217f6adb32a2b570c

set_trusted_chain tx: 93d0d141ef98a28ba651fb9b2bfdd0293a486caa7a215d86395621b74ce772d3

Successfully added trusted chain: scroll

set_trusted_chain: immutable

Is trusted chain tx: e5951b7ff7f92f683c6e117bc04bdd0c199fde8f38bea6fd8bfaae36a69b7c72

set_trusted_chain tx: f78177aa475cfa0443ee09372852d2dd658b62ca98b0db9affc266729daddecc

Successfully added trusted chain: immutable

set_trusted_chain: arbitrum-sepolia

Is trusted chain tx: ca22894b0106e7199e4a26c8a5ac22eca6372b8eae71ff5396c3aaa873dcca2d

set_trusted_chain tx: 04558755b936ecef865385d2834fd24d04ef95ef0884c74531f5505ff5b0f2d3

Successfully added trusted chain: arbitrum-sepolia

set_trusted_chain: fraxtal

Is trusted chain tx: be8d2af620a65ba4eac9e190b2da26275468648b4c572990364f7f3a1f621fc5

set_trusted_chain tx: d680e426e11a8a465ac42dead2b26547737742f9e576af4e9ae293a1b6c62779

Successfully added trusted chain: fraxtal

set_trusted_chain: optimism-sepolia

Is trusted chain tx: 55dd87eb8553624be61c44be7ef98dc3c525e99933c0b1251420eec546ef45a7

set_trusted_chain tx: 56ddd3a48d7046a3ce2b85a096ba59054d66966fca069765d1cf7d963217cd92

Successfully added trusted chain: optimism-sepolia

set_trusted_chain: base-sepolia

Is trusted chain tx: c451e9387113e8b6c8942ac0e6e72df72befde4da22d99a800695edd553c5fdf

set_trusted_chain tx: 8578b4d13b9a5616db6ecc58a41520d1df5165bdf9a98cbd055f171248396ad1

Successfully added trusted chain: base-sepolia

set_trusted_chain: blast-sepolia

Is trusted chain tx: d2e75b148c9509291ede65aaec4a2009a92c3211fdc9b33d4abb7a7e474b1aea

set_trusted_chain tx: 69fd80412375233b8ebdfa151d18282cc173404060b15643a4e8cba67021ce6f

Successfully added trusted chain: blast-sepolia

set_trusted_chain: mantle-sepolia

Is trusted chain tx: 353d18bb0b7251c07a4371b47f138ac0f83cf4304244fd5a8a0297638848f89d

set_trusted_chain tx: 724b7f78564f5ae6315b80d710617fb6602da9d6d130c18e7093cd361500cf97

Successfully added trusted chain: mantle-sepolia

set_trusted_chain: polygon-sepolia

Is trusted chain tx: 5a0fd4f4e993677af40da2812690811e491614b854ffe161dbb88ea3aaad6f90

set_trusted_chain tx: a214c7326fa7a18256f27d86d0aca5913a3d46d1ef1cbebb42a368ad17b8ce29

Successfully added trusted chain: polygon-sepolia

set_trusted_chain: linea-sepolia

Is trusted chain tx: 9a3f6d5ef3265618be52402ab67ce9a43dbd54a6f051fcb962e520c859764f4b

set_trusted_chain tx: 421e12889fdc907725378b31fa61f3a9845af72e565354a8dea6801f72ff4156

Successfully added trusted chain: linea-sepolia

set_trusted_chain: flow

Is trusted chain tx: 382a072f3a47536c43e6393fb3f79d9630dcaf4ba1af0a0b3b492aa9b698657a

Successfully added trusted chain: flow

set_trusted_chain: sui

Is trusted chain tx: 9bd4ee16cffa41f14f2ae0f97db6dfcf529d39b2a3390ea601f515a3237fd990

set_trusted_chain tx: 084576b1420dd1e56589002af9926800cca76840535811576a9529fb53e22eb0

Successfully added trusted chain: sui

set_trusted_chain: xrpl-evm-test-1

Is trusted chain tx: 4534c686cad9dcafebbce45c67d3e3bbcace4aac291ae9bb10a288a8e90d8f4e

set_trusted_chain tx: 64891a8ffcaeefb9550079afa8e901b5d4c65302cffef113a2b9fbf7b41b7ac0

Successfully added trusted chain: xrpl-evm-test-1

set_trusted_chain: xrpl-evm

Is trusted chain tx: 15d29606b0cf20e1f775bd14b42b77e0c2756483d904a3b28ad6ba46036ca3a4

set_trusted_chain tx: 4def86798a6cd54b84462ae83c513c1bd75949fe0a4e4db3878925db4896eea1

Successfully added trusted chain: xrpl-evm

set_trusted_chain: stellar-2024-q4

Is trusted chain tx: d3539289d6c9294669a69f8475850b5e43bb486a9fc8271be8addfe02a0ffbf3

set_trusted_chain tx: 41907dfef3e14dec66e9913d1f4bab11a53f89457f5def7202658fd96ae8e6af

Successfully added trusted chain: stellar-2024-q4

set_trusted_chain: stellar-2025-q1

Is trusted chain tx: f5d708fba20a35896a308a2b041f27acd0135206d3917d610fee3b6fa7f29f59

set_trusted_chain tx: 10c362311b49c7b213e926b115e0f48416177f916619347e9e70dfb23d3760e9

Successfully added trusted chain: stellar-2025-q1
```

</details>

<details>
<summary>Untrust previous Stellar chain</summary>

```
ts-node stellar/its.js remove-trusted-chains stellar-2024-q4
Wallet address: GBP4FSAOFV5O72AB3YQRDCYVD47W4N7KQK3OJODXSU3OBPNGKX4SQTJ3

Wallet balances: 9955.3214594 XLM

Wallet sequence: 2418066587795

Proceed with action removeTrustedChains (y/n) y

remove_trusted_chain: stellar-2024-q4

Is trusted chain tx: 7a88abbe3647e1f35df9b0bc7877c27d2ab5dfafe9efac1edf11950eadf37651

remove_trusted_chain tx: 9e34e88cc6c8ea6a83865e16e37502191aa74f9a9feadf74784d8235305aa6af

Successfully removed trusted chain: stellar-2024-q4
```

</details>

<details>
<summary>Trust Stellar on EVM Chains </summary>

Completed

</details>
