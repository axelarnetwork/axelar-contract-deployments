
;; title: native-interchain-token
;; version:
;; summary:
;; description:

;; traits
;;
(impl-trait 'STWXYJW1C758HRJR2Y12YN6MNXMY2WVGH144WHAZ.traits.native-interchain-token-trait)
(use-trait sip-010-trait 'STWXYJW1C758HRJR2Y12YN6MNXMY2WVGH144WHAZ.traits.sip-010-trait)

(define-constant ERR-NOT-AUTHORIZED (err u150000))
(define-constant ERR-NON-STANDARD-ADDRESS (err u150001))
(define-constant ERR-INSUFFICIENT-BALANCE (err u150002))
(define-constant ERR-INVALID-PARAMS (err u150003))
(define-constant ERR-ZERO-AMOUNT (err u150004))
(define-constant ERR-NOT-MANAGED-TOKEN (err u150005))
(define-constant ERR-FLOW-LIMIT-EXCEEDED (err u150006))
(define-constant ERR-STARTED (err u150007))
(define-constant ERR-NOT-STARTED (err u150008))
(define-constant ERR-UNSUPPORTED-TOKEN-TYPE (err u150009))
(define-constant ERR-ONLY-OPERATOR (err u150010))

;; ##########################
;; ##########################
;; ######  SIP-010  #########
;; ##########################
;; ##########################


(define-fungible-token itscoin)

(define-data-var decimals uint u0)
(define-data-var token-uri (optional (string-utf8 256)) none)
(define-data-var name (string-ascii 32) "not-initialized")
(define-data-var symbol (string-ascii 32) "not-initialized")
(define-data-var token-id (buff 32) 0x)
(define-data-var minter principal NULL-ADDRESS)

(define-read-only (get-balance (address principal))
    (ok (ft-get-balance itscoin address)))

(define-read-only (get-decimals)
    (ok (var-get decimals))
)

(define-read-only (get-total-supply)
    (ok (ft-get-supply itscoin)))

(define-read-only (get-token-uri)
    (ok (var-get token-uri)))

(define-read-only (get-name)
    (ok (var-get name)))

(define-read-only (get-symbol)
    (ok (var-get symbol)))

(define-public (transfer (amount uint) (from principal) (to principal) (memo (optional (buff 34))))
    (begin
        (asserts! (var-get is-started) ERR-NOT-STARTED)
        (asserts! (or (is-eq from tx-sender) (is-eq from contract-caller)) (err u4))

        (try! (ft-transfer? itscoin amount from to))
        (match memo to-print (print to-print) 0x)
        (ok true)))

;; constants
;;
(define-constant TOKEN-TYPE-NATIVE-INTERCHAIN-TOKEN u0)




;; ##########################
;; ##########################
;; ####  token manager  #####
;; ##########################
;; ##########################

(define-read-only (get-token-id)
    (ok (var-get token-id)))

(define-public (burn (from principal) (amount uint))
    (begin
        (asserts! (var-get is-started) ERR-NOT-STARTED)
        (asserts! (> amount u0) ERR-ZERO-AMOUNT)
        (asserts! (not (is-eq from (as-contract tx-sender))) ERR-INVALID-PARAMS)
        (asserts! (is-minter-raw contract-caller) ERR-NOT-AUTHORIZED)
        (ft-burn? itscoin amount from))
)

(define-public (mint (to principal) (amount uint))
    (begin
        (asserts! (var-get is-started) ERR-NOT-STARTED)
        (asserts! (> amount u0) ERR-ZERO-AMOUNT)
        (asserts! (not (is-eq to (as-contract tx-sender))) ERR-INVALID-PARAMS)
        (asserts! (is-minter-raw contract-caller) ERR-NOT-AUTHORIZED)
        (ft-mint? itscoin amount to))
)

;; Reads the managed token address
;; @return principal The address of the token.
(define-read-only (get-token-address)
    (ok (as-contract tx-sender)))

(define-read-only (get-token-type)
    (ok TOKEN-TYPE-NATIVE-INTERCHAIN-TOKEN))

;; @notice mint burn give/take will be handled in the token mintable-burnable itself
;; the flow would still be handled by the ITS
;; subject to change
(define-public (take-token (token <sip-010-trait>) (from principal) (amount uint))
    (begin
        ;; #[filter(amount)]
        (try! (add-flow-out amount))
        (burn from amount))
)

(define-public (give-token (token <sip-010-trait>) (to principal) (amount uint))
    (begin
        ;; #[filter(amount)]
        (try! (add-flow-in amount))
        (mint to amount)))


(define-read-only (is-minter-raw (address principal))
    (or
        (is-eq address (var-get minter))
        (is-eq address (get-its-impl))))

(define-read-only (is-minter (address principal))
    (ok (is-minter-raw address)))

(define-map roles principal {
    flow-limiter: bool,
})


;; ######################
;; ######################
;; ##### Flow Limit #####
;; ######################
;; ######################

;; 6 BTC hours
(define-constant EPOCH-TIME u36)


(define-map flows uint {
    flow-in: uint,
    flow-out: uint,
})
(define-data-var flow-limit uint u0)

;; This function adds a flow limiter for this TokenManager.
;; Can only be called by the operator.
;; @param address the address of the new flow limiter.
;; #[allow(unchecked_data)]
(define-public (add-flow-limiter (address principal))
    (begin
        (asserts! (var-get is-started) ERR-NOT-STARTED)
        (asserts! (is-operator-raw contract-caller) ERR-NOT-AUTHORIZED)
        (asserts! (is-standard address) ERR-NON-STANDARD-ADDRESS)
        (ok (map-set roles address  {flow-limiter: true}))))

;; This function removes a flow limiter for this TokenManager.
;; Can only be called by the operator.
;; @param address the address of an existing flow limiter.
;; #[allow(unchecked_data)]
(define-public (remove-flow-limiter (address principal))
    (begin
        (asserts! (var-get is-started) ERR-NOT-STARTED)
        (asserts! (is-operator-raw contract-caller) ERR-NOT-AUTHORIZED)
        (match (map-get? roles address)
            ;; no need to check limiter if they don't exist it will be a noop
            limiter-roles (ok (map-set roles address (merge limiter-roles {flow-limiter: false})))
            (ok true))))

;; Query if an address is a flow limiter.
;; @param addr The address to query for.
;; @return bool Boolean value representing whether or not the address is a flow limiter.
(define-read-only (is-flow-limiter (addr principal))
    (ok (is-flow-limiter-raw addr)))

(define-read-only (is-flow-limiter-raw (addr principal))
    (or
        (is-eq addr (get-its-impl))
        (default-to false (get flow-limiter (map-get? roles addr)))))

;;
;; Returns the current flow limit.
;; @return The current flow limit value.
;;
(define-read-only (get-flow-limit)
    (ok (var-get flow-limit)))

;; This function sets the flow limit for this TokenManager.
;; Can only be called by the flow limiters.
;; @param limit The maximum difference between the tokens
;; flowing in and/or out at any given interval of time (6h).
;; #[allow(unchecked_data)]
(define-public (set-flow-limit (limit uint))
    (begin
        (asserts! (var-get is-started) ERR-NOT-STARTED)
        (asserts! (is-flow-limiter-raw contract-caller) ERR-NOT-AUTHORIZED)
        ;; no need to check can be set to 0 to practically makes it unlimited
        (var-set flow-limit limit)
        (ok true))
)

;; Returns the current flow out amount.
;; @return flow-out-amount The current flow out amount.
(define-read-only (get-flow-out-amount)
    (let (
            (epoch (/ burn-block-height EPOCH-TIME))
        )
        (ok (default-to u0 (get flow-out (map-get? flows epoch))))))

;; Returns the current flow in amount.
;; @return flow-in-amount The current flow in amount.
(define-read-only (get-flow-in-amount)
    (let ((epoch (/ burn-block-height EPOCH-TIME)))
        (ok (default-to u0 (get flow-in (map-get? flows epoch))))))

;; Adds a flow out amount while ensuring it does not exceed the flow limit.
;; @param flow-amount The flow out amount to add.
(define-private (add-flow-out (flow-amount uint))
    (let (
            (limit  (var-get flow-limit))
            (epoch  (/ burn-block-height EPOCH-TIME))
            (current-flow-out   (unwrap-panic (get-flow-out-amount)))
            (current-flow-in  (unwrap-panic (get-flow-in-amount)))
            (new-flow-out (+ current-flow-out flow-amount))
        )
        (if (is-eq limit u0)
            (ok true)
            (begin
                (asserts! (> flow-amount u0) ERR-ZERO-AMOUNT)
                (asserts! (<= new-flow-out (+ current-flow-in limit)) ERR-FLOW-LIMIT-EXCEEDED)
                (asserts! (<= flow-amount limit) ERR-FLOW-LIMIT-EXCEEDED)
                (map-set flows epoch {
                    flow-out: new-flow-out,
                    flow-in: current-flow-in
                })
                (ok true)))))

;; Adds a flow in amount while ensuring it does not exceed the flow limit.
;; @param flow-amount The flow out amount to add.
(define-private  (add-flow-in  (flow-amount uint))
    (let (
            (limit   (var-get flow-limit))
            (epoch   (/ burn-block-height EPOCH-TIME))
            (current-flow-out    (unwrap-panic  (get-flow-out-amount)))
            (current-flow-in (unwrap-panic (get-flow-in-amount)))
            (new-flow-in (+ current-flow-in flow-amount)))
        (if  (is-eq limit u0)
            (ok true)
            (begin
                (asserts! (> flow-amount u0) ERR-ZERO-AMOUNT)
                (asserts!  (<= new-flow-in (+ current-flow-out limit)) ERR-FLOW-LIMIT-EXCEEDED)
                (asserts!  (<= flow-amount limit) ERR-FLOW-LIMIT-EXCEEDED)
                (map-set flows epoch {
                    flow-out: current-flow-out,
                    flow-in: new-flow-in
                })
                (ok true)))))


;; ######################
;; ######################
;; ### Initialization ###
;; ######################
;; ######################

(define-constant DEPLOYER tx-sender)

(define-data-var token-type (optional uint) none)

(define-data-var is-started bool false)
(define-read-only (get-is-started) (ok (var-get is-started)))

(define-public (setup
    (token-id_ (buff 32))
    (token-type_ uint)
    (operator-address (optional principal))
    (name_ (string-ascii 32))
    (symbol_ (string-ascii 32))
    (decimals_ uint)
    (token-uri_ (optional (string-utf8 256)))
    (minter_ (optional principal))
)
    (let
        (
            (minter-unpacked (default-to NULL-ADDRESS minter_))
        )
        (asserts! (is-eq contract-caller DEPLOYER) ERR-NOT-AUTHORIZED)
        (asserts! (not (var-get is-started)) ERR-STARTED)
        (asserts! (is-eq token-type_ TOKEN-TYPE-NATIVE-INTERCHAIN-TOKEN) ERR-UNSUPPORTED-TOKEN-TYPE)
        (asserts! (> (len token-id_) u0) ERR-INVALID-PARAMS)
        (asserts! (> (len name_) u0) ERR-INVALID-PARAMS)
        (asserts! (> (len symbol_) u0) ERR-INVALID-PARAMS)
        (asserts! (not (is-eq minter-unpacked (get-its-impl))) ERR-INVALID-PARAMS)
        (var-set is-started true)
        ;; #[allow(unchecked_data)]
        (var-set token-type (some token-type_))
        ;; #[allow(unchecked_data)]
        (match operator-address op
            (begin 
                (asserts! (is-standard op) ERR-NON-STANDARD-ADDRESS)
                (map-set roles op {
                    flow-limiter: true,
                }))
            true)
        ;; #[allow(unchecked_data)]
        (var-set operator (default-to NULL-ADDRESS operator-address))
        ;; #[allow(unchecked_data)]
        (var-set decimals decimals_)
        (var-set name name_)
        (var-set symbol symbol_)
        ;; #[allow(unchecked_data)]
        (var-set token-uri token-uri_)
        (var-set token-id token-id_)
        (asserts! (is-standard minter-unpacked) ERR-NON-STANDARD-ADDRESS)
        ;; #[allow(unchecked_data)]
        (var-set minter minter-unpacked)
        (print
            {
                notification: "token-metadata-update",
                payload: {
                    token-class: "ft",
                    contract-id: (as-contract tx-sender)
                }
            })
        (ok true)
    )
)
(define-constant NULL-ADDRESS (unwrap-panic (principal-construct? (if (is-eq chain-id u1) 0x16 0x1a) 0x0000000000000000000000000000000000000000)))
(define-data-var operator principal NULL-ADDRESS)
(define-read-only (get-its-impl)
    (contract-call? 'STWXYJW1C758HRJR2Y12YN6MNXMY2WVGH144WHAZ.interchain-token-service-storage get-service-impl))

(define-read-only (is-operator-raw (address principal))
    (or
        (is-eq address (var-get operator))
        (is-eq address (get-its-impl))
    ))

(define-read-only (is-operator (address principal))
    (ok (is-operator-raw address)))

(define-read-only (get-operators)
    (ok (list
            (get-its-impl)
            (var-get operator))))

;; Transfers operatorship to a new account
(define-public (transfer-operatorship (new-operator principal))
    (begin
        (asserts! (var-get is-started) ERR-NOT-STARTED)
        (asserts! (is-operator-raw contract-caller) ERR-ONLY-OPERATOR)
        (asserts! (is-standard new-operator) ERR-NON-STANDARD-ADDRESS)
        ;; #[allow(unchecked_data)]
        (var-set operator new-operator)
        (print {action: "transfer-operatorship", new-operator: new-operator})
        (ok true)
    )
)

(define-public (transfer-mintership (new-minter principal))
    (begin
        (asserts! (var-get is-started) ERR-NOT-STARTED)
        (asserts! (is-eq (var-get minter) contract-caller) ERR-NOT-AUTHORIZED)
        (asserts! (not (is-eq (get-its-impl) new-minter)) ERR-NOT-AUTHORIZED)
        (asserts! (is-standard new-minter) ERR-NON-STANDARD-ADDRESS)
        (var-set minter new-minter)
        (print {action: "transfer-mintership", new-minter: new-minter})
        (ok true)))
