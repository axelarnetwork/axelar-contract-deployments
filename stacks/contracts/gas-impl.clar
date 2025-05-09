(impl-trait .traits.gas-service-impl-trait)

;; Define constants
(define-constant ERR-INSUFFICIENT-BALANCE (err u20000))
(define-constant ERR-INVALID-AMOUNT (err u20001))
(define-constant ERR-INVALID-PRINCIPAL (err u20002))
(define-constant ERR-UNAUTHORIZED (err u20003))
(define-constant ERR-NOT-IMPLEMENTED (err u20004))
(define-constant ERR-ONLY-OWNER (err u20005))
(define-constant ERR-ONLY-GAS-COLLECTOR (err u20006))
(define-constant ERR-NOT-STARTED (err u20007))
;; Proxy contract reference
(define-constant PROXY .gas-service)

(define-private (is-proxy) (is-eq contract-caller PROXY))
(define-private (is-started) (contract-call? .gas-storage get-is-started))

;; ####################
;; ####################
;; ### Ownership ###
;; ####################
;; ####################



(define-read-only (get-owner) (contract-call? .gas-storage get-owner))

(define-read-only (get-gas-collector) (contract-call? .gas-storage get-gas-collector))

;; Transfers ownership to a new account
(define-public (transfer-ownership (new-owner principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-started) ERR-NOT-STARTED)
        (asserts! (is-eq tx-sender (get-owner)) ERR-ONLY-OWNER)
        (try! (contract-call? .gas-storage set-owner new-owner))
        (try! (contract-call? .gas-storage emit-transfer-ownership new-owner))
        (ok true)
    )
)

;; Transfers gas collector to a new account
(define-public (transfer-gas-collector (new-gas-collector principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-started) ERR-NOT-STARTED)
        (asserts! (is-eq tx-sender (get-gas-collector)) ERR-ONLY-GAS-COLLECTOR)
        (try! (contract-call? .gas-storage set-gas-collector new-gas-collector))
        (try! (contract-call? .gas-storage emit-transfer-gas-collector new-gas-collector))
        (ok true)
    )
)

;; Public function for native gas payment for contract call
(define-public (pay-native-gas-for-contract-call
    (amount uint)
    (sender principal)
    (destination-chain (string-ascii 19))
    (destination-address (string-ascii 128))
    (payload (buff 64000))
    (refund-address principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-started) ERR-NOT-STARTED)
        (asserts! (> amount u0) ERR-INVALID-AMOUNT)
        (try! (stx-transfer? amount tx-sender (as-contract tx-sender)))
        (try! (contract-call? .gas-storage emit-gas-paid-event
            sender
            amount
            refund-address
            destination-chain
            destination-address
            (keccak256 payload)))
        (ok true)
    )
)

(define-public (add-native-gas
    (amount uint)
    (tx-hash (buff 32))
    (log-index uint)
    (refund-address principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-started) ERR-NOT-STARTED)
        (asserts! (> amount u0) ERR-INVALID-AMOUNT)
        (try! (stx-transfer? amount tx-sender (as-contract tx-sender)))
        (try! (contract-call? .gas-storage emit-gas-added-event
            amount
            refund-address
            tx-hash
            log-index))
        (ok true)
    )
)

(define-public (refund
    (tx-hash (buff 32))
    (log-index uint)
    (receiver principal)
    (amount uint))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-started) ERR-NOT-STARTED)
        (asserts! (is-eq tx-sender (get-gas-collector)) ERR-ONLY-GAS-COLLECTOR)
        (asserts! (> amount u0) ERR-INVALID-AMOUNT)
        (asserts! (<= amount (stx-get-balance (as-contract tx-sender))) ERR-INSUFFICIENT-BALANCE)
        (try! (as-contract (stx-transfer? amount tx-sender receiver)))
        (try! (contract-call? .gas-storage emit-refund-event
            tx-hash
            log-index
            receiver
            amount))
        (ok true)
    )
)

(define-public (collect-fees
    (receiver principal)
    (amount uint))
    (begin
        (asserts! (is-started) ERR-NOT-STARTED)
        (asserts! (is-eq tx-sender (get-gas-collector)) ERR-ONLY-GAS-COLLECTOR)
        (asserts! (> amount u0) ERR-INVALID-AMOUNT)
        (asserts! (<= amount (stx-get-balance (as-contract tx-sender))) ERR-INSUFFICIENT-BALANCE)
        (as-contract (stx-transfer? amount tx-sender receiver))
    )
)

(define-read-only (get-balance)
    (ok (stx-get-balance (as-contract tx-sender)))
)

;; Add unimplemented functions from the trait
(define-public (pay-gas-for-contract-call
    (amount uint)
    (sender principal)
    (destination-chain (string-ascii 19))
    (destination-address (string-ascii 128))
    (payload (buff 64000))
    (refund-address principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-started) ERR-NOT-STARTED)
        ERR-NOT-IMPLEMENTED)
)

(define-public (add-gas
    (amount uint)
    (sender principal)
    (tx-hash (buff 32))
    (log-index uint)
    (refund-address principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-started) ERR-NOT-STARTED)
        ERR-NOT-IMPLEMENTED)
)

(define-public (pay-native-gas-for-express-call
    (amount uint)
    (sender principal)
    (destination-chain (string-ascii 19))
    (destination-address (string-ascii 128))
    (payload (buff 64000))
    (refund-address principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-started) ERR-NOT-STARTED)
        ERR-NOT-IMPLEMENTED)
)

(define-public (add-native-express-gas
    (amount uint)
    (sender principal)
    (tx-hash (buff 32))
    (log-index uint)
    (refund-address principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-started) ERR-NOT-STARTED)
        ERR-NOT-IMPLEMENTED)
)