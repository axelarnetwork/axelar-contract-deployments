1. Predict the [External Gateway](../evm/2025-09-Memento-GMP-v6.0.6.md) address, as `VotingVerifier` needs the `sourceGatewayAddress` which is the External Gateway address.

    ```bash
    ➜  axelar-contract-deployments git:(release/memento-devnet) ✗ ts-node evm/deploy-amplifier-gateway.js -m create3 --minimumRotationDelay 0 --predictOnly -n $CHAIN
    Environment: devnet-amplifier

    Chain: Memento

    Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233

    Wallet balance: 1.052441451166225

    Wallet nonce: 539

    Owner address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233

    Gas options: {}

    Predicted gateway proxy address: 0x974ea3BFbb51Fe73a7BB981ae744d5C124898c78

    Predicted address 0x974ea3BFbb51Fe73a7BB981ae744d5C124898c78 does not match existing deployment 0xb7879887ec7e85a5C757D7ccF5E3AB15007152e2 in chain configs.

    For official deployment, recheck the deployer, salt, args, or contract bytecode.

    This is NOT required if the deployments are done by different integrators

    Succeeded chains: [
    "Memento"
    ]

    Failed chains: []
    ```

1. Instantiate `VotingVerifier`

```bash
➜  axelar-contract-deployments git:(release/memento-devnet) ✗ ts-node ./cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2 --admin axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9 -m $MNEMONIC
Fetched code id 1331 from the network

Using code id: 1331

Proceed with instantiation on axelar? (y/n) y

Instantiated memento VotingVerifier. Address: axelar1eq4zqxah6ftzvzarvvqfdmarvaea3az2v2algqa0lemjcdz7duyqnmxfkf
```

1. Instantiate `Gateway`

```bash
➜  axelar-contract-deployments git:(release/memento-devnet) ✗ ts-node ./cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2 --admin axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9 -m $MNEMONIC
Fetched code id 1332 from the network

Using code id: 1332

Proceed with instantiation on axelar? (y/n) y

Instantiated memento Gateway. Address: axelar1wrt986xtyevgr4pskh05mgp5746ds4lq5affdetgknkmzh9003csqagk4z
```

1. Instantiate `MultisigProver`

```bash
➜  axelar-contract-deployments git:(release/memento-devnet) ✗ ts-node ./cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2 --admin axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9 -m $MNEMONIC
Fetched code id 1333 from the network

Using code id: 1333

Proceed with instantiation on axelar? (y/n) y

Instantiated memento MultisigProver. Address: axelar1yqpc6pevjl9e7cw0mvc8yfczx0mgacufdf7uy87pr3u6t9rekvyqjdt5nj
```

