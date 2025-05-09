(define-constant PROXY .gas-service)

(define-constant ERR-UNAUTHORIZED (err u40000))
(define-constant ERR-OWNER-CANNOT-BE-COLLECTOR (err u40001))
(define-constant ERR-NON-STANDARD-ADDRESS (err u40002))

(define-private (is-proxy-or-impl) (or (is-eq contract-caller PROXY) (is-eq contract-caller (var-get impl))))

(define-private (is-proxy) (is-eq contract-caller PROXY))

(define-private (is-impl) (is-eq contract-caller (var-get impl)))

(define-private (is-gas-collector) (is-eq contract-caller (var-get gas-collector)))


;; ######################
;; ######################
;; ####### Storage ######
;; ######################
;; ######################

;; Constructor flag
(define-data-var is-started bool false)

(define-read-only (get-is-started) (var-get is-started))

(define-public (start)
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (ok (var-set is-started true))
    )
)

;; Gas Service implementation contract address
(define-data-var impl principal .gas-impl)

(define-read-only (get-impl) (var-get impl))

(define-public (set-impl (new-impl principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-standard new-impl) ERR-NON-STANDARD-ADDRESS)
        (ok (var-set impl new-impl))
    )
)

;; Gas Collector
(define-data-var gas-collector principal contract-caller)

(define-read-only (get-gas-collector) (var-get gas-collector))

(define-public (set-gas-collector (new-gas-collector principal))
    (begin
        (asserts! (is-proxy-or-impl) ERR-UNAUTHORIZED)
        (asserts! (is-standard new-gas-collector) ERR-NON-STANDARD-ADDRESS)
        (asserts! (not (is-eq new-gas-collector (get-owner))) ERR-OWNER-CANNOT-BE-COLLECTOR)
        (ok (var-set gas-collector new-gas-collector))
    )
)

;; Gas owner
(define-data-var owner principal contract-caller)

(define-read-only (get-owner) (var-get owner))

(define-public (set-owner (new-owner principal))
    (begin
        (asserts! (is-proxy-or-impl) ERR-UNAUTHORIZED)
        (asserts! (is-standard new-owner) ERR-NON-STANDARD-ADDRESS)
        (asserts! (not (is-eq new-owner (get-gas-collector))) ERR-OWNER-CANNOT-BE-COLLECTOR)
        (ok (var-set owner new-owner))
    )
)

;; ######################
;; ######################
;; ####### Events #######
;; ######################
;; ######################
(define-public (emit-gas-paid-event
    (sender principal)
    (amount uint)
    (refund-address principal)
    (destination-chain (string-ascii 19))
    (destination-address (string-ascii 128))
    (payload-hash (buff 32)))
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (print {
            type: "native-gas-paid-for-contract-call",
            sender: sender,
            amount: amount,
            refund-address: refund-address,
            destination-chain: destination-chain,
            destination-address: destination-address,
            payload-hash: payload-hash
        })
        (ok true)))

(define-public (emit-gas-added-event
    (amount uint)
    (refund-address principal)
    (tx-hash (buff 32))
    (log-index uint))
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (print {
            type: "native-gas-added",
            amount: amount,
            refund-address: refund-address,
            tx-hash: tx-hash,
            log-index: log-index
        })
        (ok true)))

(define-public (emit-refund-event
    (tx-hash (buff 32))
    (log-index uint)
    (receiver principal)
    (amount uint))
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (print {
            type: "refunded",
            tx-hash: tx-hash,
            log-index: log-index,
            receiver: receiver,
            amount: amount
        })
        (ok true)))

(define-public (emit-fees-collected-event
    (receiver principal)
    (amount uint))
    (begin
        (asserts! (is-proxy-or-impl) ERR-UNAUTHORIZED)
        (print {
            type: "fees-collected",
            receiver: receiver,
            amount: amount
        })
        (ok true)))

(define-public (emit-transfer-ownership
        (new-owner principal)
)
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (print {type: "transfer-ownership", new-owner: new-owner})
        (ok true)
    )
)
(define-public (emit-transfer-gas-collector
        (new-gas-collector principal)
)
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (print {type: "transfer-gas-collector", new-gas-collector: new-gas-collector})
        (ok true)
    )
)