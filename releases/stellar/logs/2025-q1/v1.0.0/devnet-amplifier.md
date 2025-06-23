<details>
<summary>Stellar Contracts Deployed</summary>

| Contract                  | TX                                                                                                          | CMD                                                                                                                                |
| ------------------------- | ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `AxelarGateway`           | https://stellar.expert/explorer/testnet/tx/71a172220b70f3d18b26d9066524994f8f40db76d06812e69181148365dd9901 | `ts-node stellar/deploy-contract.js deploy AxelarGateway --version v1.0.0 --minimum-rotation-delay 0 --previous-signers-retention 15` |
| `AxelarOperators`         | https://stellar.expert/explorer/testnet/tx/5f41466342a8a51908c32bada4123f0f16388bd1addd246dd09c59f6da25a162 | `ts-node stellar/deploy-contract.js deploy AxelarOperators --version v1.0.0`                                                          |
| `AxelarGasService`        | https://stellar.expert/explorer/testnet/tx/d33dd8764200f750a13716b68addd1fc84f6973edfc765d854106a72119bbee8 | `ts-node stellar/deploy-contract.js deploy AxelarGasService --version v1.0.0`                                                         |
| `Upgrader`                | https://stellar.expert/explorer/testnet/tx/3c44441cc1a39c6b0e58252ad25d65d442962e94398d175346f6ff98390ae4ff | `ts-node stellar/deploy-contract.js deploy Upgrader --version v1.0.0`                                                                 |
| `InterchainTokenService ` | https://stellar.expert/explorer/testnet/tx/a1296b8851d3b5c0e204300a6400fad98effc547935eead9e0a7270607fd78ef | `ts-node stellar/deploy-contract.js deploy InterchainTokenService --version v1.0.0`                                                   |
| `AxelarExample`           | https://stellar.expert/explorer/testnet/tx/7f1a70b65431d6cf51e2e854ee452285ef32592ab1d30f87a65bd4c39eed9aa5 | `ts-node stellar/deploy-contract.js deploy AxelarExample --wasm-path artifacts/stellar_example-v1.0.0.optimized.wasm`                 |

</details>

<details>
<summary>Stellar WASM contracts</summary>

| Contract         | TX                                                                                                               | CMD                                                                                                                                                     |
| ---------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `VotingVerifier` | https://devnet-amplifier.axelarscan.io/account/axelar1w78434ta3l83fstf743xh9v5vkh4k203x2rh7l6w093n4s4vzv0suz09g3 | `ts-node ./cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2 --admin "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"` |
| `Gateway`        | https://devnet-amplifier.axelarscan.io/account/axelar1tatg2n9gsq6vkkafm6pv8hsunr236wgdk4gdc7lw0hs2e3cnspmsw75rld | `ts-node ./cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2 --admin "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"`        |
| `MultisigProver` | https://devnet-amplifier.axelarscan.io/account/axelar1lgtcv0lfxwaz2prmea2y73lespr3mnwkfjv9kr0q6j7qhvhvwx6s54cesj | `ts-node ./cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2 --admin "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"` |

</details>

<details>
<summary>Register stellar gateway at the Router</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js execute \
  -c Router \
  -t "Register Gateway for stellar" \
  -d "Register Gateway address for stellar at Router contract" \
  --runAs $RUN_AS_ACCOUNT \
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
  "runAs": "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9",
  "contract": "axelar14jjdxqhuxk803e9pq64w4fgf385y86xxhkpzswe9crmu6vxycezst0zq8y",
  "msg": {
    "register_chain": {
      "chain": "stellar-2025-q1",
      "gateway_address": "axelar1tatg2n9gsq6vkkafm6pv8hsunr236wgdk4gdc7lw0hs2e3cnspmsw75rld",
      "msg_id_format": "hex_tx_hash_and_event_index"
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 131
```

</details>

<details>
<summary>Register Multisig Prover for stellar</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js execute \
  -c Coordinator \
  -t "Register Multisig Prover for stellar" \
  -d "Register Multisig Prover address for stellar at Coordinator contract" \
  --runAs $RUN_AS_ACCOUNT \
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
  "runAs": "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9",
  "contract": "axelar1m2498n4h2tskcsmssjnzswl5e6eflmqnh487ds47yxyu6y5h4zuqr9zk4g",
  "msg": {
    "register_prover_contract": {
      "chain_name": "stellar-2025-q1",
      "new_prover_addr": "axelar1lgtcv0lfxwaz2prmea2y73lespr3mnwkfjv9kr0q6j7qhvhvwx6s54cesj"
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 132
```

</details>

<details>
<summary>Authorize Multisig Prover for stellar</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js execute \
  -c Multisig \
  -t "Authorize Multisig Prover for stellar" \
  -d "Authorize Multisig Prover address for stellar at Multisig contract" \
  --runAs $RUN_AS_ACCOUNT \
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
  "runAs": "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9",
  "contract": "axelar19jxy26z0qnnspa45y5nru0l5rmy9d637z5km2ndjxthfxf5qaswst9290r",
  "msg": {
    "authorize_callers": {
      "contracts": {
        "axelar1lgtcv0lfxwaz2prmea2y73lespr3mnwkfjv9kr0q6j7qhvhvwx6s54cesj": "stellar-2025-q1"
      }
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 133
```

</details>

<details>
<summary>Create pool for stellar in stellar voting verifier</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for stellar in stellar voting verifier" \
  -d "Create pool for stellar in stellar voting verifier" \
  --runAs $RUN_AS_ACCOUNT \
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
  "runAs": "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9",
  "contract": "axelar1vaj9sfzc3z0gpel90wu4ljutncutv0wuhvvwfsh30rqxq422z89qnd989l",
  "msg": {
    "create_pool": {
      "params": {
        "epoch_duration": "100",
        "participation_threshold": [
          "7",
          "10"
        ],
        "rewards_per_epoch": "100"
      },
      "pool_id": {
        "chain_name": "stellar-2025-q1",
        "contract": "axelar1w78434ta3l83fstf743xh9v5vkh4k203x2rh7l6w093n4s4vzv0suz09g3"
      }
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 135
```

</details>

<details>
<summary>Create pool for stellar in axelar multisig</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for stellar in axelar multisig" \
  -d "Create pool for stellar in axelar multisig" \
  --runAs $RUN_AS_ACCOUNT \
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
  "runAs": "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9",
  "contract": "axelar1vaj9sfzc3z0gpel90wu4ljutncutv0wuhvvwfsh30rqxq422z89qnd989l",
  "msg": {
    "create_pool": {
      "params": {
        "epoch_duration": "100",
        "participation_threshold": [
          "7",
          "10"
        ],
        "rewards_per_epoch": "100"
      },
      "pool_id": {
        "chain_name": "stellar-2025-q1",
        "contract": "axelar19jxy26z0qnnspa45y5nru0l5rmy9d637z5km2ndjxthfxf5qaswst9290r"
      }
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 136
```

</details>

<details>
<summary>Register Stellar to ITSHub contract</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node cosmwasm/submit-proposal.js \
    its-hub-register-chains stellar-2025-q1 \
    -t "Register stellar-2025-q1 on ITS Hub" \
    -d "Register stellar-2025-q1 on ITS Hub" \
    --deposit 100000000 \
    --runAs axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9
Encoded /cosmwasm.wasm.v1.ExecuteContractProposal: {
  "title": "Register stellar-2025-q1 on ITS Hub",
  "description": "Register stellar-2025-q1 on ITS Hub",
  "runAs": "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9",
  "contract": "axelar157hl7gpuknjmhtac2qnphuazv2yerfagva7lsu9vuj2pgn32z22qa26dk4",
  "msg": {
    "register_chains": {
      "chains": [
        {
          "chain": "stellar-2025-q1",
          "its_edge_contract": "CATNQHWMG4VOWPSWF4HXVW7ASDJNX7M7F6JLFC544T7ZMMXXAE2HUDTY",
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

Proposal submitted: 137
```

</details>

<details>
<summary>Stellar WASM contract Registration</summary>

| Operation                     | TX                                                                                                         | CMD                                                                                                             |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `Create genesis verifier set` | https://devnet-amplifier.axelarscan.io/tx/30B0C468A9ACD13E9CFDAC5656D27CA51ADE4758ABD033CF9B152DDA8F51AAD9 | `axd tx wasm execute $MULTISIG_PROVER '"update_verifier_set"' --from amplifier --gas auto --gas-adjustment 1.2` |

</details>

<details>
<summary>Stellar Operations</summary>

| Ops             | TX                                                                                                          | CMD                              |
| --------------- | ----------------------------------------------------------------------------------------------------------- | -------------------------------- |
| `Rotate Signer` | https://stellar.expert/explorer/testnet/tx/8742a7cf3a49ecd7de380903cf0a4e83571876b3f862f9cb64a89ef4c8fc82ae | `ts-node stellar/gateway.js rotate` |

</details>

<details>
<summary>Trust All EVM Chains on Stellar</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node stellar/its.js add-trusted-chains all
Wallet address: GCRN3JXRVXHQTFQFM7NR4TTTORGZDCJWPIOLPQQHL6WMAQGVMWSXJL3Q

Wallet balances: 9927.9954339 XLM

Wallet sequence: 2667174690838

Proceed with action addTrustedChains (y/n) y

set_trusted_chain: core-avalanche

Is trusted chain tx: 4e020aefd74e77fca083b97c96bb85a2461d596222c311f1f68fcb79deaa5c7a

set_trusted_chain tx: 185cd694f0ea02c0f661c8bc624a93cb7acc55d66f1bc9866c6dc472a0386955

Successfully added trusted chain: core-avalanche

set_trusted_chain: core-ethereum

Is trusted chain tx: 0415f36a015362829fc8686371d59deece0254af6e6a40b355e2ebed7c0f8722

set_trusted_chain tx: 337ba02a9f6157b1901f235ae7d5a5e16379f01ed6f4e218b741ebda597048a1

Successfully added trusted chain: core-ethereum

set_trusted_chain: core-optimism

Is trusted chain tx: fd2a4f5632ce019daf9d2e209ba57f9f6724e9f5b8826ef0e49e6e8c76df32f6

set_trusted_chain tx: 596c49660e673e4969b689c4ac8720e017b907de4894e8136f9e999bbf76dd2f

Successfully added trusted chain: core-optimism

set_trusted_chain: avalanche-fuji

Is trusted chain tx: f1489357bef1f7728702ee91a68538711edb949d4115e243c58f3e210092cd11

set_trusted_chain tx: 0fc0461825047c997626c2cbb341e709f1429d2f9930c7df4ec61ef8c6a66aea

Successfully added trusted chain: avalanche-fuji

set_trusted_chain: eth-sepolia

Is trusted chain tx: 747b6a1f87f5d72a70e09e6317f9250e578c5287bae49b407ff486b6113fc2ce

set_trusted_chain tx: 43bf0ae5e931a7fb4d380c80d65f5d605ea7aa8d746e4fb846069e8e5edd0f9e

Successfully added trusted chain: eth-sepolia

set_trusted_chain: optimism-sepolia

Is trusted chain tx: a9ab5fd55e2e847d03268111425cdffaa2227df5307b0a65a12ba4b3d8b1b386

set_trusted_chain tx: a56a485576adccb0e0ea3e9a4cc7a76cbecbf0ce5eb1bc98e825d41159898495

Successfully added trusted chain: optimism-sepolia

set_trusted_chain: flow

Is trusted chain tx: 7b2a5b858f1d42cc86e3cde9bfb065eb84f95431e3d0d1e0b55dacbab6406629

set_trusted_chain tx: 8fec9d8c52964bf25d909ffbfd76d1dac76b85dcaae76b49eab2e4b3414cc5d0

Successfully added trusted chain: flow

set_trusted_chain: stellar-2024-q4

Is trusted chain tx: ce5ca44d03b38cb1e3735dc6e21b49d86a5b1212ff1053e90a2e44635995ca71

set_trusted_chain tx: cc3ac08cb0b9c0dc67b75af7c6a55f2dda69d4da8b965320d85fa66ece491d81

Successfully added trusted chain: stellar-2024-q4

set_trusted_chain: stellar-2025-q1

Is trusted chain tx: 0d710bc14e38907bdd4392142ffca41645860a0ae3f6e196e66c1c3f197414be

set_trusted_chain tx: 9d23dd53f670fd045b9e58947479de39425874ad6b85d6e2dfea99e32faf837a

Successfully added trusted chain: stellar-2025-q1

set_trusted_chain: sui-2

Is trusted chain tx: 9319c6cef6f9ca30ec7079be4155cb846167ff2799a2ccca7bb6f40f7ed92f22

set_trusted_chain tx: 585170f085816e11cfdb3b4d81423e3b0cb935a6e323d20fae234230d5612c52

Successfully added trusted chain: sui-2
```

</details>

<details>
<summary>Untrust previous Stellar chain</summary>

```
➜  axelar-contract-deployments git:(release/2025-q1-devnet) ✗ ts-node stellar/its.js remove-trusted-chains stellar-2024-q4
Wallet address: GCRN3JXRVXHQTFQFM7NR4TTTORGZDCJWPIOLPQQHL6WMAQGVMWSXJL3Q

Wallet balances: 9926.5595192 XLM

Wallet sequence: 2667174690858

Proceed with action removeTrustedChains (y/n) y

remove_trusted_chain: stellar-2024-q4

Is trusted chain tx: 07eb74e67aa7842a181609a278591ff1307de7e63d3335fae455250f5b34ff30

remove_trusted_chain tx: 2a802e6d759b0a117da1abab566f27c6983dc4283021a4431b5c19c1cda51102

Successfully removed trusted chain: stellar-2024-q4
```

</details>

<details>
<summary>Trust Stellar on EVM Chains </summary>

```
ts-node evm/its.js set-trusted-chains stellar-2025-q1 hub -n all -y
Environment: devnet-amplifier

Chain: Avalanche Fuji

Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233

Wallet balance: 90.71079925323565

Wallet nonce: 3540

Contract name: InterchainTokenService

Contract address: 0x77883201091c08570D55000AB32645b88cB96324

Gas options: {}

Action: set-trusted-chains

set-trusted-chains tx: 0x93d622bf98f8d5a50c31ebbdb0e5920247e6a8c545ce138ef665d86a49695721

Chain: Ethereum Sepolia

Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233

Wallet balance: 182.63010404650808

Wallet nonce: 1198

Contract name: InterchainTokenService

Contract address: 0x77883201091c08570D55000AB32645b88cB96324

Gas options: {}

Action: set-trusted-chains

set-trusted-chains tx: 0xb391a56d598c6f1935d9a104ecff3bb299953dc74824693c6c67da18be4cec3d

Chain: Optimism Sepolia

Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233

Wallet balance: 3.5643049057279796

Wallet nonce: 668

Contract name: InterchainTokenService

Contract address: 0x77883201091c08570D55000AB32645b88cB96324

Gas options: {}

Action: set-trusted-chains

set-trusted-chains tx: 0x0504d73835371b934b28be910cfb934fc7db04e2a849586cd0c82f648d4d471e

Chain: Avalanche Fuji

Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233

Wallet balance: 90.71074006623562

Wallet nonce: 3541

Contract name: InterchainTokenService

Contract address: 0x2269B93c8D8D4AfcE9786d2940F5Fcd4386Db7ff

Gas options: {}

Action: set-trusted-chains

set-trusted-chains tx: 0xd25f83c0de11fe86dec1ac885b674ad664e6f7a2a6155869d215e4d9187ff840

Chain: Eth Sepolia

Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233

Wallet balance: 182.62998487857308

Wallet nonce: 1199

Contract name: InterchainTokenService

Contract address: 0x2269B93c8D8D4AfcE9786d2940F5Fcd4386Db7ff

Gas options: {}

Action: set-trusted-chains

set-trusted-chains tx: 0x176631c03de02673deda0bc311f08d6ce918fdf948c40cd9c940be9e76b9f616

Chain: Optimism Sepolia

Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233

Wallet balance: 3.564186018654893

Wallet nonce: 669

Contract name: InterchainTokenService

Contract address: 0x2269B93c8D8D4AfcE9786d2940F5Fcd4386Db7ff

Gas options: {}

Action: set-trusted-chains

set-trusted-chains tx: 0x973a208fce7ad1befe90abf2a6380271d04eee647a0e9c5918f35d7c745934e5

Chain: Flow

Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233

Wallet balance: 101209.19285255017

Wallet nonce: 1416

Contract name: InterchainTokenService

Contract address: 0x2269B93c8D8D4AfcE9786d2940F5Fcd4386Db7ff

Gas options: {
"gasLimit": 8000000
}

Action: set-trusted-chains

set-trusted-chains tx: 0x9f64f1945798ccbc2762fc666b8b287adc95882deb35707acdfe2fb9c950a470
```

</details>
