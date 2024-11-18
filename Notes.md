Notes:
--- ON EXT CHAIN(S)
1. Deploy create3 deployer 
2. Predicate Ext. Gateway
-> Predicate not deploy because 
    1. Core amplifier contracts require address of ext gateway (as we pass in verifier set into contract? -> *UNCLEAR I CAN PASS IN VERIFIER SET WITHOUT THEM SUPPORTING ME, NO?*)
    2. But ext gateway requires verifiers to 
- `node evm/deploy-contract.js -c InterchainGovernance -m create2 -n ethereum-sepolia`
- `node evm/deploy-contract.js -c InterchainGovernance -m create2 -n MY_CHAIN`

-- ON AMPLIFIER