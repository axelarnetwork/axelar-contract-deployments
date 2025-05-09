(define-constant PROXY .gateway)

(define-constant ERR-UNAUTHORIZED (err u60000))
(define-constant ERR-NON-STANDARD-ADDRESS (err u60001))

(define-private (is-proxy-or-impl) (or (is-eq contract-caller PROXY) (is-eq contract-caller (var-get impl))))

(define-private (is-proxy) (is-eq contract-caller PROXY))

(define-private (is-impl) (is-eq contract-caller (var-get impl)))

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


;; Gateway implementation contract address
(define-data-var impl principal .gateway-impl)

(define-read-only (get-impl) (var-get impl))

(define-public (set-impl (new-impl principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-standard new-impl) ERR-NON-STANDARD-ADDRESS)
        (ok (var-set impl new-impl))
    )
)

;; Governance contract address
(define-data-var governance principal .governance)

(define-read-only (get-governance) (var-get governance))

(define-public (set-governance (new principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (is-standard new) ERR-NON-STANDARD-ADDRESS)
        (ok (var-set governance new))
    )
)


;; Gateway operator
(define-data-var operator principal contract-caller)

(define-read-only (get-operator) (var-get operator))

(define-public (set-operator (new-operator principal))
    (begin
        (asserts! (is-proxy-or-impl) ERR-UNAUTHORIZED)
        (asserts! (is-standard new-operator) ERR-NON-STANDARD-ADDRESS)
        (ok (var-set operator new-operator))
    )
)

;; Current signers epoch
(define-data-var epoch uint u0)

(define-read-only (get-epoch) (var-get epoch))

(define-public (set-epoch (epoch- uint))
    (begin
        (asserts! (is-proxy-or-impl) ERR-UNAUTHORIZED)
        (ok (var-set epoch epoch-))
    )
)

;; The timestamp for the last signer rotation
(define-data-var last-rotation-timestamp uint u0)

(define-read-only (get-last-rotation-timestamp) (var-get last-rotation-timestamp))

(define-public (set-last-rotation-timestamp (new-timestamp uint))
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (ok (var-set last-rotation-timestamp new-timestamp))
    )
)

;; The map of signer hash by epoch
(define-map signer-hash-by-epoch uint (buff 32))

(define-read-only (get-signer-hash-by-epoch (signer-epoch uint)) (map-get? signer-hash-by-epoch signer-epoch))

(define-public (set-signer-hash-by-epoch (epoch- uint) (signers-hash (buff 32)))
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (ok (map-set signer-hash-by-epoch epoch- signers-hash))
    )
)

;; The map of epoch by signer hash
(define-map epoch-by-signer-hash (buff 32) uint)

(define-read-only (get-epoch-by-signer-hash (signer-hash (buff 32))) (map-get? epoch-by-signer-hash signer-hash))

(define-public (set-epoch-by-signer-hash (signers-hash (buff 32)) (epoch- uint) )
    (begin
        (asserts! (is-proxy-or-impl) ERR-UNAUTHORIZED)
        (ok (map-set epoch-by-signer-hash signers-hash epoch-))
    )
)

;; Previous signers retention. 0 means only the current signers are valid
(define-data-var previous-signers-retention uint u0)

(define-read-only (get-previous-signers-retention) (var-get previous-signers-retention))

(define-public (set-previous-signers-retention (retention uint))
    (begin
        (asserts! (is-proxy-or-impl) ERR-UNAUTHORIZED)
        (ok (var-set previous-signers-retention retention))
    )
)

;; The domain separator for the signer proof
(define-data-var domain-separator (buff 32) 0x00)

(define-read-only (get-domain-separator) (var-get domain-separator))

(define-public (set-domain-separator (separator (buff 32)))
    (begin
        (asserts! (is-proxy-or-impl) ERR-UNAUTHORIZED)
        (ok (var-set domain-separator separator))
    )
)


;; The minimum delay required between rotations
(define-data-var minimum-rotation-delay uint u0)

(define-read-only (get-minimum-rotation-delay) (var-get minimum-rotation-delay))

(define-public (set-minimum-rotation-delay (delay uint))
    (begin
        (asserts! (is-proxy-or-impl) ERR-UNAUTHORIZED)
        (ok (var-set minimum-rotation-delay delay))
    )
)

;; Messages map
(define-map messages (buff 32) (buff 32))

(define-read-only (get-message (command-id (buff 32))) (map-get? messages command-id))

(define-public (insert-message (command-id (buff 32)) (message-hash (buff 32)) )
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (ok (map-insert messages command-id message-hash))
    )
)

(define-public (set-message (command-id (buff 32)) (message-hash (buff 32)) )
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (ok (map-set messages command-id message-hash))
    )
)

;; ######################
;; ######################
;; ####### Events #######
;; ######################
;; ######################

(define-public (emit-contract-call
        (sender principal)
        (destination-chain (string-ascii 19))
        (destination-contract-address (string-ascii 128))
        (payload (buff 64000))
        (payload-hash (buff 32))
)
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (print {
            type: "contract-call",
            sender: sender,
            destination-chain: destination-chain,
            destination-contract-address: destination-contract-address,
            payload-hash: payload-hash,
            payload: payload
        })
        (ok true)
    )
)

(define-public (emit-message-approved
        (command-id (buff 32))
        (message {
                source-chain: (string-ascii 19),
                message-id: (string-ascii 128),
                source-address: (string-ascii 128),
                contract-address: principal,
                payload-hash: (buff 32)
        })
)
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (print (merge message {
            type: "message-approved",
            command-id: command-id
        }))
        (ok true)
    )
)

(define-public (emit-message-executed
        (command-id (buff 32))
        (source-chain (string-ascii 19))
        (message-id (string-ascii 128))
)
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (print {
          type: "message-executed",
            command-id: command-id,
            source-chain: source-chain,
            message-id: message-id,
        })
        (ok true)
    )
)

(define-public (emit-signers-rotated
        (new-epoch uint)
        (new-signers {
                signers: (list 100 {signer: (buff 33), weight: uint}),
                threshold: uint,
                nonce: (buff 32)
        })
        (new-signers-hash (buff 32))
)
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (print {
            type: "signers-rotated",
            epoch: new-epoch,
            signers-hash: new-signers-hash,
            signers: new-signers
        })
        (ok true)
    )
)

(define-public (emit-transfer-operatorship
        (new-operator principal)
)
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (print {type: "transfer-operatorship", new-operator: new-operator})
        (ok true)
    )
)

;; General purpose event emitter for future
(define-public (emit-str (o (string-ascii 4096)))
    (begin
        (asserts! (is-impl) ERR-UNAUTHORIZED)
        (print o)
        (ok true)
    )
)
