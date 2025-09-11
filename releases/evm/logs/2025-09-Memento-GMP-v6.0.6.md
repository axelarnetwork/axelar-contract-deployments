1. Fund the following addresses with native tokens on chain:

1. Deploy `ConstAddrDeployer`:

    ```bash
    ➜  axelar-contract-deployments git:(release/memento-devnet) ts-node evm/deploy-contract.js -c ConstAddressDeployer -m create --artifactPath ../evm/legacy/ConstAddressDeployer.json -n $CHAIN
    Environment: devnet-amplifier

    Chain: Memento

    Wallet address: 0xba76c6980428A0b10CFC5d8ccb61949677A61233

    Wallet balance: 1.0527789425098335 

    Wallet nonce: 536

    Contract name: ConstAddressDeployer

    Pre-deploy Contract bytecode hash: 0x8fda47a596dfba923270da84e0c32a2d0312f1c03389f83e16f2b5a35ed37fbe

    Gas options: {}

    Constructor args for chain Memento: []

    Deployment method: create

    Deployer contract

    ConstAddressDeployer will be deployed to: 0x0C0e6F34F4a68264842BD347b59E19F85F512B5a

    Predicted address 0x0C0e6F34F4a68264842BD347b59E19F85F512B5a does not match existing deployment 0x98B2920D53612483F91F12Ed7754E51b4A77919e in chain configs.

    For official deployment, recheck the deployer, salt, args, or contract bytecode.

    This is NOT required if the deployments are done by different integrators

    Proceed with deployment on Memento? (y/n) y

    Deployed Contract bytecode hash: 0x8fda47a596dfba923270da84e0c32a2d0312f1c03389f83e16f2b5a35ed37fbe

    Memento | ConstAddressDeployer: 0x0C0e6F34F4a68264842BD347b59E19F85F512B5a

    Succeeded chains: [
    "Memento"
    ]

    Failed chains: []
    ```

1. Deploy `Create3Deployer`:

```bash

```
