(impl-trait .traits.proxy-trait)
(use-trait gas-impl-trait .traits.gas-service-impl-trait)

;; ######################
;; ######################
;; ### Proxy Calls ######
;; ######################
;; ######################

(define-constant ERR-INVALID-IMPL (err u30000))
(define-constant ERR-STARTED (err u30001))
(define-constant ERR-UNAUTHORIZED (err u30002))
(define-constant ERR-NOT-IMPLEMENTED (err u30003))

(define-constant DEPLOYER tx-sender)

(define-private (is-correct-impl (gas-impl <gas-impl-trait>)) (is-eq (contract-call? .gas-storage get-impl) (contract-of gas-impl)))

;; Proxy all gas service functions to implementation
(define-public (pay-native-gas-for-contract-call
    (gas-impl <gas-impl-trait>)
    (amount uint)
    (sender principal)
    (destination-chain (string-ascii 19))
    (destination-address (string-ascii 128))
    (payload (buff 64000))
    (refund-address principal))
    (begin
        (asserts! (is-correct-impl gas-impl) ERR-INVALID-IMPL)
        (contract-call? gas-impl
            pay-native-gas-for-contract-call
            amount
            sender
            destination-chain
            destination-address
            payload
            refund-address))
)

(define-public (add-native-gas
    (gas-impl <gas-impl-trait>)
    (amount uint)
    (tx-hash (buff 32))
    (log-index uint)
    (refund-address principal))
    (begin
        (asserts! (is-correct-impl gas-impl) ERR-INVALID-IMPL)
        (contract-call? gas-impl
            add-native-gas
            amount
            tx-hash
            log-index
            refund-address))
)

(define-public (refund
    (gas-impl <gas-impl-trait>)
    (tx-hash (buff 32))
    (log-index uint)
    (receiver principal)
    (amount uint))
    (begin
        (asserts! (is-correct-impl gas-impl) ERR-INVALID-IMPL)
        (contract-call? gas-impl
            refund
            tx-hash
            log-index
            receiver
            amount))
)

(define-public (collect-fees
    (gas-impl <gas-impl-trait>)
    (receiver principal)
    (amount uint))
    (begin
        (asserts! (is-correct-impl gas-impl) ERR-INVALID-IMPL)
        (try! (contract-call? gas-impl
            collect-fees
            receiver
            amount))
        (contract-call? .gas-storage emit-fees-collected-event receiver amount))
)

;; Read-only functions
(define-public (get-balance (gas-impl <gas-impl-trait>))
    (begin
        (asserts! (is-correct-impl gas-impl) ERR-INVALID-IMPL)
        (contract-call? gas-impl get-balance))
)

(define-public (transfer-ownership (gas-impl <gas-impl-trait>) (new-owner principal))
    (begin
        (asserts! (is-correct-impl gas-impl) ERR-INVALID-IMPL)
        (contract-call? gas-impl transfer-ownership new-owner)
    )
)

(define-public (transfer-gas-collector (gas-impl <gas-impl-trait>) (new-gas-collector principal))
    (begin
        (asserts! (is-correct-impl gas-impl) ERR-INVALID-IMPL)
        (contract-call? gas-impl transfer-gas-collector new-gas-collector)
    )
)

;; Add unimplemented functions from the trait
(define-public (pay-gas-for-contract-call
    (gas-impl <gas-impl-trait>)
    (amount uint)
    (sender principal)
    (destination-chain (string-ascii 19))
    (destination-address (string-ascii 128))
    (payload (buff 64000))
    (refund-address principal))
    (begin
        (asserts! (is-correct-impl gas-impl) ERR-INVALID-IMPL)
        (contract-call? gas-impl pay-gas-for-contract-call amount sender destination-chain destination-address payload refund-address)
    )
)

(define-public (add-gas
    (gas-impl <gas-impl-trait>)
    (amount uint)
    (sender principal)
    (tx-hash (buff 32))
    (log-index uint)
    (refund-address principal))
    (begin
        (asserts! (is-correct-impl gas-impl) ERR-INVALID-IMPL)
        (contract-call? gas-impl add-gas amount sender tx-hash log-index refund-address)
    )
)

(define-public (pay-native-gas-for-express-call
    (gas-impl <gas-impl-trait>)
    (amount uint)
    (sender principal)
    (destination-chain (string-ascii 19))
    (destination-address (string-ascii 128))
    (payload (buff 64000))
    (refund-address principal))
    (begin
        (asserts! (is-correct-impl gas-impl) ERR-INVALID-IMPL)
        (contract-call? gas-impl pay-native-gas-for-express-call amount sender destination-chain destination-address payload refund-address)
    )
)

(define-public (add-native-express-gas
    (gas-impl <gas-impl-trait>)
    (amount uint)
    (sender principal)
    (tx-hash (buff 32))
    (log-index uint)
    (refund-address principal))
    (begin
        (asserts! (is-correct-impl gas-impl) ERR-INVALID-IMPL)
        (contract-call? gas-impl add-native-express-gas amount sender tx-hash log-index refund-address)
    )
)


;; ######################
;; ######################
;; ### Upgradability ####
;; ######################
;; ######################



(define-public (set-impl (gas-impl principal))
    (let
        (
            (governance-impl (contract-call? .gateway-storage get-governance))
            (prev (contract-call? .gas-storage get-impl))
        )
        (asserts! (is-eq contract-caller governance-impl) ERR-UNAUTHORIZED)

        ;; Set new implementation
        (try! (contract-call? .gas-storage set-impl gas-impl))
        (print {
            type: "gas-impl-upgraded",
            prev: prev,
            new: gas-impl,
        })
        (ok true)
    )
)

(define-public (set-governance (governance principal))
    ERR-NOT-IMPLEMENTED)

;; Constructor function
;; @param gas-collector
;; @returns (response true) or reverts
(define-public (setup
    (gas-collector principal)
)
    (begin
        (asserts! (not (contract-call? .gas-storage get-is-started)) ERR-STARTED)
        (asserts! (is-eq contract-caller DEPLOYER) ERR-UNAUTHORIZED)
        (try! (contract-call? .gas-storage set-gas-collector gas-collector))
        (try! (contract-call? .gas-storage start))
        (ok true)
    )
)