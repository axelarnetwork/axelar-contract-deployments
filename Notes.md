Notes:
--- ON EXT CHAIN(S)
1. Deploy create3 deployer 
2. Predicate Ext. Gateway
-> Predicate not deploy because 
    1. Core amplifier contracts require address of ext gateway (as we pass in verifier set into contract? -> *UNCLEAR I CAN PASS IN VERIFIER SET WITHOUT THEM SUPPORTING ME, NO?*)
    2. But ext gateway requires verifiers to 
- `node evm/deploy-amplifier-gateway.js \
        -m create \
        --minimumRotationDelay 300 \
        --predictOnly \
-n ethereum-sepolia`



-- ON AMPLIFIER
1. Deploy the voting verifier `node cosmwasm/deploy-contract.js \ -c VotingVerifier \ -n avalanche-fuji  \ -s saltOne \  -m "hint pause black nerve govern embody fade gesture fluid arrange soldier outdoor front risk scorpion narrow flower modify boat social theory real pluck lunch" -e devnet-amplifier -a artifacts -y --admin axelar199g24qmzg4znysvnwfknqrmlupazxmfxjq7vsf`

2. Deploy the Gateway `node.js v20.12.2
❯ `node cosmwasm/deploy-contract.js \
        -c Gateway \
        -n avalanche-fuji \
        -s saltFour \
         --fetchCodeId -m "hint pause black nerve govern embody fade gesture fluid arrange soldier outdoor front risk scorpion narrow flower modify boat social theory real pluck lunch" -e devnet-amplifier -y --codeId 616`

3. Deploy the Prover `node cosmwasm/deploy-contract.js \
             -c MultisigProver \
             -n avalanche-fuji \
             -s saltThreeTwo \
               -m "hint pause black nerve govern embody fade gesture fluid arrange soldier outdoor front risk scorpion narrow flower modify boat social theory real pluck lunch" -e devnet-amplifier -y --codeId 618`