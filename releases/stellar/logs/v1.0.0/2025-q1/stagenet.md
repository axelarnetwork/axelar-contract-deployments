<details>
<summary>Stellar Contracts Deployed</summary>

| Contract                  | TX                                                                                                          | CMD                                                                                                                                  |
| ------------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `AxelarGateway`           | https://stellar.expert/explorer/testnet/tx/fe42374aa0b111b61adb6685f7c46fe5f8605fdf2a887c07f5401e4c6596a51d | `ts-node stellar/deploy-contract.js deploy AxelarGateway --version v1.0.0 --minimum-rotation-delay 300 --previous-signers-retention 15` |
| `AxelarOperators`         | https://stellar.expert/explorer/testnet/tx/51e60e0765b23a82be78d827bfd9df288285ff29de72a7f2d96e58d9c4898929 | `ts-node stellar/deploy-contract.js deploy AxelarOperators --version v1.0.0`                                                            |
| `AxelarGasService`        | https://stellar.expert/explorer/testnet/tx/d6334af721597bad2758463dee94b73617dbcbf8adcfa1cf40e29803a7541e2f | `ts-node stellar/deploy-contract.js deploy AxelarGasService --version v1.0.0`                                                           |
| `Upgrader`                | https://stellar.expert/explorer/testnet/tx/693606d8e8bc2731a81a20b4e09bbe916608541383efb6198c75fa83a0a4ca6c | `ts-node stellar/deploy-contract.js deploy Upgrader --version v1.0.0`                                                                   |
| `InterchainTokenService ` | https://stellar.expert/explorer/testnet/tx/c856259a34fdb9afaee835a2f3e4deb8f9ab9a091c48c793b9d394e6a97464cc | `ts-node stellar/deploy-contract.js deploy InterchainTokenService --version v1.0.0`                                                     |
| `AxelarExample`           | https://stellar.expert/explorer/testnet/tx/e4d20181d41b09639d2e6b2183f808fc9aac46b3d5fd93d080fcbf990e63d094 | `ts-node stellar/deploy-contract.js deploy AxelarExample --wasm-path artifacts/stellar_example-v1.0.0.optimized.wasm`                   |

</details>

<details>
<summary>Stellar WASM contracts</summary>

| Contract         | TX                                                                                                       | CMD                                                                                                                                                     |
| ---------------- | -------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `VotingVerifier` | https://stagenet.axelarscan.io/account/axelar1f7unnl3uu8mgjecmaasv5p3e6khx2u99th9lhkhl2l9e05ldrzwq3vu33h | `ts-node ./cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2 --admin "axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky"` |
| `Gateway`        | https://stagenet.axelarscan.io/account/axelar1lkrxspjqge3dkuwgfw4mjtmxjnyx8pr7tm9ksszvlh4c3u3z2nlswxexcz | `ts-node ./cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2 --admin "axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky"`        |
| `MultisigProver` | https://stagenet.axelarscan.io/account/axelar1emmjd57gha5vchnysth3g6fsyzyrjm4xhj4dzlg2ulj4dz8p9kxs2l6zvy | `ts-node ./cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2 --admin "axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky"` |

</details>

<details>
<summary>Register stellar gateway at the Router</summary>

```
ts-node cosmwasm/submit-proposal.js execute \
  -c Router \
  -t "Register Gateway for stellar" \
  -d "Register Gateway address for stellar at Router contract" \
  --deposit 100000000 \
  --msg "{
    \"register_chain\": {
      \"chain\": \"stellar-2025-q1\",
      \"gateway_address\": \"axelar1lkrxspjqge3dkuwgfw4mjtmxjnyx8pr7tm9ksszvlh4c3u3z2nlswxexcz\",
      \"msg_id_format\": \"hex_tx_hash_and_event_index\"
      }
    }"
```

</details>

<details>
<summary>Register Multisig Prover for stellar</summary>

```
ts-node cosmwasm/submit-proposal.js execute \
  -c Coordinator \
  -t "Register Multisig Prover for stellar" \
  -d "Register Multisig Prover address for stellar at Coordinator contract" \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"register_prover_contract\": {
      \"chain_name\": \"$CHAIN\",
      \"new_prover_addr\": \"$MULTISIG_PROVER\"
    }
  }"
Encoded /cosmwasm.wasm.v1.ExecuteContractProposal: {
  "title": "Register Multisig Prover for stellar",
  "description": "Register Multisig Prover address for stellar at Coordinator contract",
  "runAs": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
  "contract": "axelar1nc3mfplae0atcchs9gqx9m6ezj5lfqqh2jmqx639kf8hd7m96lgq8a5e5y",
  "msg": {
    "register_prover_contract": {
      "chain_name": "stellar-2025-q1",
      "new_prover_addr": "axelar1emmjd57gha5vchnysth3g6fsyzyrjm4xhj4dzlg2ulj4dz8p9kxs2l6zvy"
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 118
```

</details>

<details>
<summary>Authorize Multisig Prover for stellar</summary>

```
ts-node cosmwasm/submit-proposal.js execute \
  -c Multisig \
  -t "Authorize Multisig Prover for stellar" \
  -d "Authorize Multisig Prover address for stellar at Multisig contract" \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"authorize_callers\": {
      \"contracts\": {
        \"$MULTISIG_PROVER\": \"$CHAIN\"
      }
    }
  }"
Encoded /cosmwasm.wasm.v1.ExecuteContractProposal: {
  "title": "Authorize Multisig Prover for stellar",
  "description": "Authorize Multisig Prover address for stellar at Multisig contract",
  "runAs": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
  "contract": "axelar143vjln56ke4pjmj5ut7u3358ywyfl7h5rg58js8gprr39664wcqs72vs3u",
  "msg": {
    "authorize_callers": {
      "contracts": {
        "axelar1emmjd57gha5vchnysth3g6fsyzyrjm4xhj4dzlg2ulj4dz8p9kxs2l6zvy": "stellar-2025-q1"
      }
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 119
```

</details>

<details>
<summary>Create pool for stellar in stellar voting verifier</summary>

```
ts-node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for stellar in stellar voting verifier" \
  -d "Create pool for stellar in stellar voting verifier" \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"create_pool\": {
      \"params\": {
        \"epoch_duration\": \"600\",
        \"participation_threshold\": [\"7\", \"10\"],
        \"rewards_per_epoch\": \"100\"
      },
      \"pool_id\": {
        \"chain_name\": \"$CHAIN\",
        \"contract\": \"$VOTING_VERIFIER\"
      }
    }
  }"
Encoded /cosmwasm.wasm.v1.ExecuteContractProposal: {
  "title": "Create pool for stellar in stellar voting verifier",
  "description": "Create pool for stellar in stellar voting verifier",
  "runAs": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
  "contract": "axelar1nyhmtdrzx77ynqgu8cug0u7eqz2kzfk9mctvaa4stqpekg4s9vnsgu54at",
  "msg": {
    "create_pool": {
      "params": {
        "epoch_duration": "600",
        "participation_threshold": [
          "7",
          "10"
        ],
        "rewards_per_epoch": "100"
      },
      "pool_id": {
        "chain_name": "stellar-2025-q1",
        "contract": "axelar1f7unnl3uu8mgjecmaasv5p3e6khx2u99th9lhkhl2l9e05ldrzwq3vu33h"
      }
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 120
```

</details>

<details>
<summary>Create pool for stellar in axelar multisig</summary>

```
ts-node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for stellar in axelar multisig" \
  -d "Create pool for stellar in axelar multisig" \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"create_pool\": {
      \"params\": {
        \"epoch_duration\": \"600\",
        \"participation_threshold\": [\"7\", \"10\"],
        \"rewards_per_epoch\": \"100\"
      },
      \"pool_id\": {
        \"chain_name\": \"$CHAIN\",
        \"contract\": \"$MULTISIG\"
      }
    }
  }"
Encoded /cosmwasm.wasm.v1.ExecuteContractProposal: {
  "title": "Create pool for stellar in axelar multisig",
  "description": "Create pool for stellar in axelar multisig",
  "runAs": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
  "contract": "axelar1nyhmtdrzx77ynqgu8cug0u7eqz2kzfk9mctvaa4stqpekg4s9vnsgu54at",
  "msg": {
    "create_pool": {
      "params": {
        "epoch_duration": "600",
        "participation_threshold": [
          "7",
          "10"
        ],
        "rewards_per_epoch": "100"
      },
      "pool_id": {
        "chain_name": "stellar-2025-q1",
        "contract": "axelar143vjln56ke4pjmj5ut7u3358ywyfl7h5rg58js8gprr39664wcqs72vs3u"
      }
    }
  },
  "funds": []
}

Proceed with proposal submission? (y/n) y

Proposal submitted: 121
```

</details>

<details>
<summary>Register Stellar to ITSHub contract</summary>

```
ts-node cosmwasm/submit-proposal.js \
    its-hub-register-chains stellar-2025-q1 \
    -t "Register stellar-2025-q1 on ITS Hub" \
    -d "Register stellar-2025-q1 on ITS Hub" \
    --deposit 100000000

Proceed with proposal submission? (y/n) y

Proposal submitted: 122
```

</details>

<details>
<summary>Stellar WASM contract Registration</summary>

| Operation                     | TX                                                                                                 | CMD                                                                                                             |
| ----------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `Create genesis verifier set` | https://stagenet.axelarscan.io/tx/AAC64DA407A120F6B7243127801D184EE6E628C0CAAD63EC855B9E1C6FAEDC05 | `axd tx wasm execute $MULTISIG_PROVER '"update_verifier_set"' --from amplifier --gas auto --gas-adjustment 1.2` |

</details>

<details>
<summary>Stellar Operations</summary>

| Ops             | TX                                                                                                          | CMD                              |
| --------------- | ----------------------------------------------------------------------------------------------------------- | -------------------------------- |
| `Rotate Signer` | https://stellar.expert/explorer/testnet/tx/4202d96b06d426c26ba0a51720149fee2f9dfcbe66dc87fa26b7a9afd49fb0c9 | `ts-node stellar/gateway.js rotate` |

</details>

<details>
<summary>Trust All EVM Chains on Stellar</summary>

```
ts-node stellar/its.js add-trusted-chains flow
Wallet address: GBP4FSAOFV5O72AB3YQRDCYVD47W4N7KQK3OJODXSU3OBPNGKX4SQTJ3

Wallet balances: 9963.7279667 XLM

Wallet sequence: 2418066587700

Proceed with action addTrustedChains (y/n) y

set_trusted_chain: flow

Is trusted chain tx: 45c87ec80477b7261ebfa5fc01f58feb2ad4099d729ba902351bebdaf0999ff6

set_trusted_chain tx: 49079de570bd15a751fc4afb16878b4c65f7091f94a4e73a89078bd436ed52c4

Successfully added trusted chain: flow
```

```
ts-node stellar/its.js add-trusted-chains all
Wallet address: GBP4FSAOFV5O72AB3YQRDCYVD47W4N7KQK3OJODXSU3OBPNGKX4SQTJ3

Wallet balances: 9957.7893173 XLM

Wallet sequence: 2418066587758

Proceed with action addTrustedChains (y/n) y

Missing InterchainTokenService edge contract for chain: Binance

Missing InterchainTokenService edge contract for chain: Centrifuge

Missing InterchainTokenService edge contract for chain: Hedera

set_trusted_chain: avalanche

Is trusted chain tx: 06fbb7eb53a40b9e262e3c201784e058712a80efeda947852265f733a06c795d

set_trusted_chain tx: d4a5c8183ec75838f92378fa83c3baf7204285d1a3d3fb157fa15e0615393d77

Successfully added trusted chain: avalanche

set_trusted_chain: fantom

Is trusted chain tx: 718fffcaaf9e3500d2f4c3d8bb09c150e497bcf652d26f114778df7d8265997f

set_trusted_chain tx: 4881f3527fa11cda02982683fd193e65c29829e0cef674f0c1bfb11bd38bbb7f

Successfully added trusted chain: fantom

set_trusted_chain: moonbeam

Is trusted chain tx: 439df7bd3fca47b3c2ba11c86bb2bd85bad06cd84713f2ce07cb5c0586c7629c

set_trusted_chain tx: e95e1c9e9de43551a1fcef64c4c1a308b10ffc7b43efb7437d531f883be45c5e

Successfully added trusted chain: moonbeam

set_trusted_chain: kava

Is trusted chain tx: 78dbe535021546bb067c19bb4f370681c24328a97315f863c66da0fb05fc6aea

set_trusted_chain tx: 9861a3ba06c814e528b4c5a5ac98dc465ce1e3318c826587231d2a9401aebb94

Successfully added trusted chain: kava

set_trusted_chain: ethereum-sepolia

Is trusted chain tx: cbb492c6630a8d5f62d47a3f9fd8926d49cd9332682b26b9d89466f2fb44fe70

set_trusted_chain tx: 1f2a4e8dc3adb242bc3634afbd64e52274aef3b227ea64ae85b35b73609d740c

Successfully added trusted chain: ethereum-sepolia

set_trusted_chain: arbitrum-sepolia

Is trusted chain tx: e364e13414eb6815f7151f3fceabbce9dd008936981a19e0d9a42ac828b512f4

set_trusted_chain tx: c769cf4a8d5a8285b18d32704c389f2035d015db3107244ac7b1eb9eb30aa002

Successfully added trusted chain: arbitrum-sepolia

set_trusted_chain: linea-sepolia

Is trusted chain tx: 346902bbcf289c176326dc08f2032fb0847755a7bc686f191fe9467645d3709d

set_trusted_chain tx: eca581e3589db440fb7ecfe5897ff003f4b8d666c0fa80287fe79113fdbeed76

Successfully added trusted chain: linea-sepolia

set_trusted_chain: polygon-sepolia

Is trusted chain tx: 09dab43dbbbd0bb926630218a256b4173ce1e80d242936c61fb43e2e2c4a69b0

set_trusted_chain tx: 897b8ce3dd35216581906f63713bb19c25b06891addd70b7b2380576765d5994

Successfully added trusted chain: polygon-sepolia

set_trusted_chain: base-sepolia

Is trusted chain tx: 3d26d17e9932a2001cd1b8aba451f7f4affc412ebc8b2882f6f1824fd52e9989

set_trusted_chain tx: 1462caf6895013530151b687728abf53ac415c9806aa0e78d544337cd5f8e6d3

Successfully added trusted chain: base-sepolia

set_trusted_chain: blast-sepolia

Is trusted chain tx: fbc7352b26d4a63b2dbd5c2ff980633719d100548547b12a2eb0572e04f617a6

set_trusted_chain tx: 01a5dfc9b817f28d48c5768be02f0a7041c9452da3d647c885099ebd4efce7c2

Successfully added trusted chain: blast-sepolia

set_trusted_chain: fraxtal

Is trusted chain tx: 3af8b51d7c9f0a60404bcb414f02503b401d00595085f2a6d620ee0eb024405f

set_trusted_chain tx: 1045cf811e26dc45fe0583963fd6e7afc7939f3a173789c8aba1ae8883a6bb48

Successfully added trusted chain: fraxtal

set_trusted_chain: mantle-sepolia

Is trusted chain tx: 041cc145837925d477ef8ea5bb0f494c0814888cc30e6842cbfbc878b443d37d

set_trusted_chain tx: 37316b49353f02327db67fa990b2cfa820b890fd3ee53fd124a0319b9ffcbded

Successfully added trusted chain: mantle-sepolia

set_trusted_chain: optimism-sepolia

Is trusted chain tx: fb84808661a1372314f758647767a7a9e24ad7bb51f003e5a31042d08c068686

set_trusted_chain tx: 5106059e6849d3f9de09ca5ed5953bb73bf5a5839b4f2b37a7b899396d245573

Successfully added trusted chain: optimism-sepolia

set_trusted_chain: flow

Is trusted chain tx: fb26f02b12214860ca398bc45ba46f7c7e5c2519b247d6d93a64fcde8aa0e599

The chain is already trusted: flow

set_trusted_chain: sui

Is trusted chain tx: 1319601a75ec659f302d16feff0154f126f5d807d2b37f15b387e9dab6d9887e

set_trusted_chain tx: 7bc05ee2f54d2ddc2479f3512c8d022410509537b836eba30952275afdc9bf5a

Successfully added trusted chain: sui

set_trusted_chain: stellar

Is trusted chain tx: 18b5738e19c7fb3b44617fa867c8acdb3f6f3c3f9c7ac8c5fd30a3b63ad91cb8

set_trusted_chain tx: e7ca6cfee0de4a0439392f714cc9ef37f7546a763a1c1e35ccc1c4b3e4dda2fd

Successfully added trusted chain: stellar

set_trusted_chain: xrpl-evm

Is trusted chain tx: 52eb0ea96038acca11118d6f912f487f4208abb69a345965d6a6520b3838cd09

set_trusted_chain tx: e819162e60c5af14b65bb4d3780bbe633c45fbc184c3eb80335fed31510b0234

Successfully added trusted chain: xrpl-evm

set_trusted_chain: stellar-2025-q1

Is trusted chain tx: cd7376feeb6f331bb8caee0dd63e5a37607105342afcd7e0f4a320b21bcf9cf2

set_trusted_chain tx: 1ea4fcbc84cbb4754e9bd0e5c332a686139080e4925b960eb5a22997935ce395

Successfully added trusted chain: stellar-2025-q1
```

</details>

<details>
<summary>Untrust previous Stellar chain</summary>

```
ts-node stellar/its.js remove-trusted-chains stellar
Wallet address: GBP4FSAOFV5O72AB3YQRDCYVD47W4N7KQK3OJODXSU3OBPNGKX4SQTJ3

Wallet balances: 9955.3544173 XLM

Wallet sequence: 2418066587793

Proceed with action removeTrustedChains (y/n) y

remove_trusted_chain: stellar

Is trusted chain tx: bc1e3b305a5b990df8121e316aaf873c04c864912af654aeebd27c33b0cbac49

remove_trusted_chain tx: f6a5aba6fb30dd71328606d5a42527bad72ddf11ca4511180b84b546230d27bc

Successfully removed trusted chain: stellar
```

</details>

<details>
<summary>Trust Stellar on EVM Chains </summary>

```
ts-node evm/its.js set-trusted-chains stellar-2025-q1 hub -n all -y
Environment: stagenet

Chain: Avalanche

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 0.9430444442639586

Wallet nonce: 156

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {}

Action: set-trusted-chains

set-trusted-chains tx: 0x1eb8104fd0edca787cee5e117173de4c287a4b02743353038cdfe461a5fc33f2

Chain: Fantom

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 4.7214387573889

Wallet nonce: 111

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {}

Action: set-trusted-chains

set-trusted-chains tx: 0x7129570db30c9c746143ae3a8e87f961e1ad1efe8c4b6cb49009b48590a42a3e

Chain: Moonbeam

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 0.9095408893750001

Wallet nonce: 67

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {
"gasLimit": 12000000
}

Action: set-trusted-chains

set-trusted-chains tx: 0x4848cb77131c8dd5c138adf02059861b398a067f29a8455acec5296bf7c9218a

Chain: Binance

No InterchainTokenService address found for chain Binance

Chain: Kava

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 99.867039808

Wallet nonce: 82

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {}

Action: set-trusted-chains

set-trusted-chains tx: 0x425722bf6fe207880147d83e9014874f6ad5903f51ede9d4369e27f8976fc8aa

Chain: Ethereum-Sepolia

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 1.8011767740660383

Wallet nonce: 47

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {
"gasLimit": 15000000
}

Action: set-trusted-chains

set-trusted-chains tx: 0x3ebdc3205559000c3e15d81834111245b846b358338ca748db2c5a60f92e55d4

Chain: Arbitrum-Sepolia

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 1.0939013659

Wallet nonce: 60

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {}

Action: set-trusted-chains

set-trusted-chains tx: 0xca40db3f4536f38cbee25f80bd3c7c1539dfa978c354c5be13fc8f1dd67841f9

Chain: Centrifuge

No InterchainTokenService address found for chain Centrifuge

Chain: Linea-Sepolia

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 1.9693591157601187

Wallet nonce: 57

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {
"gasLimit": 7000000,
"gasPrice": 631217073
}

Action: set-trusted-chains

set-trusted-chains tx: 0x207b9f2d9888cb3ef5809121858af102cfc75f0be9f2a08b1613eb5f3e351a8f

Chain: Polygon-Sepolia

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 2.123475437438371

Wallet nonce: 58

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {
"gasLimit": 6000000,
"gasPrice": 62250000022
}

Action: set-trusted-chains

set-trusted-chains tx: 0xc3a3c52ddf1c9cb048c35f4407d6be1999d0c8c8234138f99b1ab3d255904af1

Chain: Base-Sepolia

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 1.0144066366175863

Wallet nonce: 56

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {
"gasLimit": 8000000
}

Action: set-trusted-chains

set-trusted-chains tx: 0x027e91a2589577be8831b5ed8f94dfa32c068d14b63ecb259b1c265057e1807a

Chain: Blast-Sepolia

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 0.1992174794625414

Wallet nonce: 51

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {
"gasLimit": 8000000
}

Action: set-trusted-chains

set-trusted-chains tx: 0x1b546d6c4e4c5fa9c172154a0527183c8c0f19ccab5f07e11e667365e2ad4815

Chain: Fraxtal

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 0.8877810960706572

Wallet nonce: 62

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {
"gasLimit": 8000000
}

Action: set-trusted-chains

set-trusted-chains tx: 0x4010174e5426725570f6453c9c19d71e4d1e39424b556fc5a06dfc3bf9d46152

Chain: Mantle-Sepolia

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 75.251040302839

Wallet nonce: 58

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {
"gasPrice": 25000000,
"gasLimit": 100000000000
}

Action: set-trusted-chains

set-trusted-chains tx: 0x36cdc1c59905dee07864ab64d4497aa2f62783e24e00e09d53ce1c7509d04c36

Chain: Optimism-Sepolia

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 1.028910076327865

Wallet nonce: 53

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {
"gasLimit": 8000000
}

Action: set-trusted-chains

set-trusted-chains tx: 0xaa5528d96db47c907f4d6a5cdf6ade12957e1737c6b8923df34125bb5f65f338

Chain: Flow

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 560.5131539901339

Wallet nonce: 244

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {
"gasLimit": 8000000
}

Action: set-trusted-chains

set-trusted-chains tx: 0x20bf5a7581c474b5efe783df0ae1941ad2ab8083c9a025cb623d2cdc8332b256

Chain: Hedera

No InterchainTokenService address found for chain Hedera

Chain: XRPL EVM

Wallet address: 0xBeF25f4733b9d451072416360609e5A4c115293E

Wallet balance: 5.2431230945

Wallet nonce: 41

Contract name: InterchainTokenService

Contract address: 0x0FCb262571be50815627C16Eca1f5F3D342FF5a5

Gas options: {}

Action: set-trusted-chains

set-trusted-chains tx: 0x84e1fd84e12f4aed6786fcea960b120510e7fdc79a9080648cd81f51a698cf59

```

</details>
