(use-trait gateway-trait .traits.gateway-trait)
(use-trait gas-service-trait .traits.gas-service-impl-trait)
(impl-trait .traits.interchain-token-executable-trait)
(impl-trait .traits.axelar-executable)
(define-constant ERR-NOT-AUTHORIZED (err u90000))

(define-data-var value
    {
        source-chain: (string-ascii 19),
        message-id: (string-ascii 128),
        source-address: (string-ascii 128),
        source-address-its: (buff 128),
        payload: (buff 64000),
    } {
        source-chain: "",
        message-id: "",
        source-address: "",
        payload: 0x00,
        source-address-its: 0x00,
    }
)

(define-read-only (get-value) (var-get value))

(define-public (set-remote-value
    (destination-chain (string-ascii 19))
    (destination-contract-address (string-ascii 128))
    (payload (buff 64000))
    (gas-amount uint)
    (gateway-impl <gateway-trait>)
    (gas-service <gas-service-trait>)
)
    (begin
        (try! (stx-transfer? gas-amount contract-caller (as-contract tx-sender)))
        (try!
            (contract-call? .gas-service pay-native-gas-for-contract-call
                gas-service
                gas-amount
                (as-contract tx-sender)
                destination-chain
                destination-contract-address
                payload
                contract-caller
            )
        )
        (try! (contract-call? .gateway call-contract gateway-impl destination-chain destination-contract-address payload))
        (ok true)
    )
)

(define-public (execute
    (source-chain (string-ascii 19))
    (message-id (string-ascii 128))
    (source-address (string-ascii 128))
    (payload (buff 64000))
    (gateway-impl <gateway-trait>)
)
    (begin
        (try! (contract-call? .gateway validate-message gateway-impl source-chain message-id source-address (keccak256 payload)))
        (var-set value (merge (var-get value) {
            source-chain: source-chain,
            message-id: message-id,
            source-address: source-address,
            payload: payload
        }))
        (ok true)
    )
)

(define-public (execute-with-interchain-token
        (source-chain (string-ascii 19))
        (message-id (string-ascii 128))
        (source-address (buff 128))
        (payload (buff 64000))
        (token-id (buff 32))
        (tokenAddress principal)
        (amount uint))
    (begin
        (asserts! (is-eq contract-caller .interchain-token-service-impl) ERR-NOT-AUTHORIZED)
        (var-set value (merge (var-get value) {
            source-chain: source-chain,
            message-id: message-id,
            source-address-its: source-address,
            payload: payload
        }))
        (ok (keccak256 (unwrap-panic (to-consensus-buff? "its-execute-success"))))))
