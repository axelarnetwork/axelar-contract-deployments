(impl-trait .traits.proxy-trait)
(use-trait gateway-trait .traits.gateway-trait)

;; ######################
;; ######################
;; ### Proxy Calls ######
;; ######################
;; ######################

(define-constant ERR-INVALID-IMPL (err u70000))
(define-constant ERR-UNAUTHORIZED (err u70001))
(define-constant ERR-SIGNERS-DATA (err u70002))
(define-constant ERR-STARTED (err u70003))

(define-private (is-correct-impl (gateway-impl <gateway-trait>)) (is-eq (contract-call? .gateway-storage get-impl) (contract-of gateway-impl)))

(define-public (call-contract
    (gateway-impl <gateway-trait>)
    (destination-chain (string-ascii 19))
    (destination-contract-address (string-ascii 128))
    (payload (buff 64000))
)
    (begin
        (asserts! (is-correct-impl gateway-impl) ERR-INVALID-IMPL)
        (contract-call? gateway-impl call-contract destination-chain destination-contract-address payload contract-caller)
    )
)

(define-public (approve-messages
    (gateway-impl <gateway-trait>)
    (messages (buff 4096))
    (proof (buff 16384))
)
    (begin
        (asserts! (is-correct-impl gateway-impl) ERR-INVALID-IMPL)
        (contract-call? gateway-impl approve-messages messages proof)
    )
)

(define-public (validate-message
    (gateway-impl <gateway-trait>)
    (source-chain (string-ascii 19))
    (message-id (string-ascii 128))
    (source-address (string-ascii 128))
    (payload-hash (buff 32))
)
    (begin
        (asserts! (is-correct-impl gateway-impl) ERR-INVALID-IMPL)
        (contract-call? gateway-impl validate-message source-chain message-id source-address payload-hash contract-caller)
    )
)

(define-public (rotate-signers
    (gateway-impl <gateway-trait>)
    (new-signers (buff 8192))
    (proof (buff 16384))
)
    (begin
        (asserts! (is-correct-impl gateway-impl) ERR-INVALID-IMPL)
        (contract-call? gateway-impl rotate-signers new-signers proof)
    )
)

(define-public (transfer-operatorship (gateway-impl <gateway-trait>) (new-operator principal))
    (begin
        (asserts! (is-correct-impl gateway-impl) ERR-INVALID-IMPL)
        (contract-call? gateway-impl transfer-operatorship new-operator contract-caller)
    )
)

(define-public (is-message-approved
    (gateway-impl <gateway-trait>)
    (source-chain (string-ascii 19))
    (message-id (string-ascii 128))
    (source-address (string-ascii 128))
    (contract-address principal)
    (payload-hash (buff 32))
)
    (begin
        (asserts! (is-correct-impl gateway-impl) ERR-INVALID-IMPL)
        (contract-call? gateway-impl is-message-approved source-chain message-id source-address contract-address payload-hash))
)

(define-public (is-message-executed
    (gateway-impl <gateway-trait>)
    (source-chain (string-ascii 19))
    (message-id (string-ascii 128))
)
    (begin
        (asserts! (is-correct-impl gateway-impl) ERR-INVALID-IMPL)
        (contract-call? gateway-impl is-message-executed source-chain message-id))
)

;; General purpose proxy call
(define-public (call (gateway-impl <gateway-trait>) (fn (string-ascii 32)) (data (buff 65000)))
    (begin
        (asserts! (is-correct-impl gateway-impl) ERR-INVALID-IMPL)
        (contract-call? gateway-impl dispatch fn data)
    )
)

;; ######################
;; ######################
;; ### Upgradability ####
;; ######################
;; ######################


(define-private (is-governance) (is-eq contract-caller (contract-call? .gateway-storage get-governance)))

(define-public (set-impl (new principal))
    (let
        (
            (prev (contract-call? .gateway-storage get-impl))
        )
        (asserts! (is-governance) ERR-UNAUTHORIZED)
        (try! (contract-call? .gateway-storage set-impl new))
        (print {
            type: "gateway-impl-updated",
            prev: prev,
            new: new
        })
        (ok true)
    )
)

(define-public (set-governance (new principal))
    (let
        (
            (prev (contract-call? .gateway-storage get-governance))
        )
        (asserts! (is-governance) ERR-UNAUTHORIZED)
        (try! (contract-call? .gateway-storage set-governance new))
        (print {
            type: "gateway-governance-updated",
            prev: prev,
            new: new
        })
        (ok true)
    )
)

;; ######################
;; ######################
;; ### Initialization ###
;; ######################
;; ######################


(define-constant DEPLOYER tx-sender)

;; Initial setup function should be only called once to start the protocol
;; @param signers The initial gateway signer set
;; @param operator_ The initial gateway operator
;; @param domain-separator_ The initial gateway domain separator used to create messages for signing
;; @minimum-rotation-delay_ The minimum delay between two rotations of the gateway signers
;; @previous-signers-retention_ How many epochs should previous signers remain valid (when reverting to an old set by the operator)
;; @returns (response true) or reverts
(define-public (setup
    (signers (buff 8192))
    (operator_ principal)
    (domain-separator_ (buff 32))
    (minimum-rotation-delay_ uint)
    (previous-signers-retention_ uint)
)
    (let
        (
            (signers_ (unwrap! (from-consensus-buff? {
                signers: (list 100 {signer: (buff 33), weight: uint}),
                threshold: uint,
                nonce: (buff 32)
            } signers) ERR-SIGNERS-DATA))
        )
        (asserts! (is-eq (contract-call? .gateway-storage get-is-started) false) ERR-STARTED)
        (asserts! (is-eq contract-caller DEPLOYER) ERR-UNAUTHORIZED)
        (try! (contract-call? .gateway-impl rotate-signers-inner signers_ false))
        (try! (contract-call? .gateway-storage set-operator operator_))
        (try! (contract-call? .gateway-storage set-domain-separator domain-separator_))
        (try! (contract-call? .gateway-storage set-minimum-rotation-delay minimum-rotation-delay_))
        (try! (contract-call? .gateway-storage set-previous-signers-retention previous-signers-retention_))
        (try! (contract-call? .gateway-storage start))
        (ok true)
    )
)
