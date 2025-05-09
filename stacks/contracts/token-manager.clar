;;
;; @title TokenManager
;; This contract is responsible for managing tokens,
;; such as setting locking token balances,
;; or setting flow limits, for interchain transfers.
(impl-trait 'STWXYJW1C758HRJR2Y12YN6MNXMY2WVGH144WHAZ.traits.token-manager-trait)
(use-trait sip-010-trait 'STWXYJW1C758HRJR2Y12YN6MNXMY2WVGH144WHAZ.traits.sip-010-trait)
(define-constant CONTRACT-ID (keccak256 (unwrap-panic (to-consensus-buff? "token-manager"))))
(define-constant PREFIX_CANONICAL_TOKEN_SALT (keccak256 (unwrap-panic (to-consensus-buff? "canonical-token-salt"))))

(define-constant DEPLOYER tx-sender)

;; This type is reserved for interchain tokens deployed by ITS, and can't be used by custom token managers.
;; @notice same as mint burn in functionality will be custom tokens made by us
;; that are deployed outside of the contracts but registered by the ITS contract
(define-constant TOKEN-TYPE-NATIVE-INTERCHAIN-TOKEN u0)
;; The token will be locked/unlocked at the token manager.
(define-constant TOKEN-TYPE-LOCK-UNLOCK u2)

(define-constant ERR-ONLY-OPERATOR (err u170000))
(define-constant ERR-NOT-AUTHORIZED (err u170001))
(define-constant ERR-NON-STANDARD-ADDRESS (err u170002))
(define-constant ERR-FLOW-LIMIT-EXCEEDED (err u170003))
(define-constant ERR-NOT-MANAGED-TOKEN (err u170004))
(define-constant ERR-ZERO-AMOUNT (err u170005))
(define-constant ERR-STARTED (err u170006))
(define-constant ERR-NOT-STARTED (err u170007))
(define-constant ERR-UNSUPPORTED-TOKEN-TYPE (err u170008))
(define-constant ERR-INVALID-PARAMS (err u170009))




(define-data-var token-address (optional principal) none)
(define-data-var token-type (optional uint) none)



(define-map roles principal {
    flow-limiter: bool,
})
(define-read-only (get-its-impl)
    (contract-call? 'STWXYJW1C758HRJR2Y12YN6MNXMY2WVGH144WHAZ.interchain-token-service-storage get-service-impl))


;; Checks that the sender is the interchain-token-service contract
(define-read-only (is-its-sender)
    (is-eq contract-caller (get-its-impl)))

;; Getter for the contract id.
;; @return (buff 32) The contract id.
(define-read-only (contract-id)
    (ok CONTRACT-ID))

;; Reads the managed token address
;; @return principal The address of the token.
(define-read-only (get-token-address)
    (ok (unwrap! (var-get token-address) ERR-NOT-STARTED)))

(define-read-only (get-token-type)
    (ok (unwrap! (var-get token-type) ERR-NOT-STARTED)))

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
;; ### Token Manager ####
;; ######################
;; ######################

;; This function gives token to a specified address from the token manager.
;; @param sip-010-token The sip-010 interface of the token.
;; @param to The address to give tokens to.
;; @param amount The amount of tokens to give.
;; @return (response bool uint)
(define-public (give-token (sip-010-token <sip-010-trait>) (to principal) (amount uint))
    (begin
        (asserts! (> amount u0) ERR-ZERO-AMOUNT)
        (asserts! (is-its-sender) ERR-NOT-AUTHORIZED)
        (try! (add-flow-in amount))
        (as-contract (transfer-token-from sip-010-token contract-caller to amount))))

;; This function takes token from a specified address to the token manager.
;; @param sip-010-token The sip-010 interface of the token.
;; @param from The address to take tokens from.
;; @param amount The amount of token to take.
;; @return (response bool uint)
(define-public (take-token (sip-010-token <sip-010-trait>) (from principal) (amount uint))
    (begin
        (asserts! (> amount u0) ERR-ZERO-AMOUNT)
        (asserts! (is-its-sender) ERR-NOT-AUTHORIZED)
        (try! (add-flow-out amount))
        (transfer-token-from sip-010-token from (as-contract contract-caller) amount)))


(define-private (transfer-token-from (sip-010-token <sip-010-trait>) (from principal) (to principal) (amount uint))
    (begin
        (asserts! (var-get is-started) ERR-NOT-STARTED)
        (asserts! (is-eq (contract-of sip-010-token) (unwrap! (var-get token-address) ERR-NOT-STARTED)) ERR-NOT-MANAGED-TOKEN)
        (contract-call? sip-010-token transfer amount from to none)))

(define-read-only (is-minter (address principal))
    (ok false))


;; ######################
;; ######################
;; ### Initialization ###
;; ######################
;; ######################


(define-data-var is-started bool false)
(define-read-only (get-is-started) (ok (var-get is-started)))
;; Constructor function
;; @returns (response true) or reverts
;; #[allow(unchecked_data)]
(define-public (setup
    (token-address_ principal)
    (token-type_ uint)
    (operator-address (optional principal))
)
    (begin
        (asserts! (is-eq contract-caller DEPLOYER) ERR-NOT-AUTHORIZED)
        (asserts! (not (var-get is-started)) ERR-STARTED)
        (asserts! (is-eq token-type_ TOKEN-TYPE-LOCK-UNLOCK) ERR-UNSUPPORTED-TOKEN-TYPE)
        (var-set is-started true)
        (asserts! (is-standard token-address_) ERR-NON-STANDARD-ADDRESS)
        ;; #[allow(unchecked_data)]
        (var-set token-address (some token-address_))
        ;; #[allow(unchecked_data)]
        (var-set token-type (some token-type_))
        ;; #[allow(unchecked_data)]
        (var-set operator (default-to NULL-ADDRESS operator-address))
        (match operator-address op
            (begin 
                (asserts! (is-standard op) ERR-NON-STANDARD-ADDRESS)
                (map-set roles op {
                    flow-limiter: true,
                }))
            true)
        (ok true)
    )
)

;;  * @notice Getter function for the parameters of a lock/unlock TokenManager.
;;  * @dev This function will be mainly used by frontends.
;;  * @param operator_ The operator of the TokenManager.
;;  * @param token-address_ The token to be managed.
;;  * @return (buff 500) The resulting params to be passed to custom TokenManager deployments.
(define-read-only (get-params (operator_ (optional principal)) (token-address_ principal))
    (ok (unwrap-panic (to-consensus-buff? {
        operator: operator_,
        token-address: token-address_,
    }))))

;; ####################
;; ####################
;; ### Operatorship ###
;; ####################
;; ####################
(define-constant NULL-ADDRESS (unwrap-panic (principal-construct? (if (is-eq chain-id u1) 0x16 0x1a) 0x0000000000000000000000000000000000000000)))
(define-data-var operator principal NULL-ADDRESS)

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
        ;; #[allow(unchecked_data)]
        (var-set operator new-operator)
        (print {action: "transfer-operatorship", new-operator: new-operator})
        (ok true)
    )
)