(use-trait gateway-trait .traits.gateway-trait)
(use-trait proxy-trait .traits.proxy-trait)

(define-constant NULL-ADDRESS (unwrap-panic (principal-construct? (if (is-eq chain-id u1) 0x16 0x1a) 0x0000000000000000000000000000000000000000)))

;; ######################
;; ######################
;; ###### Timelock ######
;; ######################
;; ######################

(define-constant ERR-TIMELOCK-EXISTS (err u80000))
(define-constant ERR-TIMELOCK-NOT-READY (err u80001))
(define-constant ERR-TIMELOCK-HASH (err u80002))
(define-constant ERR-TIMELOCK-MIN-ETA (err u80003))
(define-constant ERR-PAYLOAD-DATA (err u80004))
(define-constant ERR-INVALID-TYPE (err u80005))
(define-constant ERR-INVALID-PROXY (err u80006))
(define-constant ERR-UNAUTHORIZED (err u80007))
(define-constant ERR-NOT-STARTED (err u80008))
(define-constant ERR-STARTED (err u80009))


(define-constant MIN-TIMELOCK-DELAY u43200) ;; 12 hours

(define-map timelock-map (buff 32) {target: principal, proxy: principal, eta: uint, type: uint})

;; Returns the timestamp after which the timelock may be executed.
;; @params hash; The hash of the timelock
;; @returns uint
(define-read-only (get-timelock (hash (buff 32))) (default-to {target: NULL-ADDRESS, proxy: NULL-ADDRESS, eta: u0, type: u0} (map-get? timelock-map hash)))

;; Schedules a new timelock.
;; The timestamp will be set to the current block timestamp + minimum time delay, if the provided timestamp is less than that.
;; @params hash; The hash of the new timelock.
;; @params target; The target principal address to be interacted with.
;; @params eta; The proposed Unix timestamp (in secs) after which the new timelock can be executed.
;; @params type; Task type.
;; @returns (response true) or reverts
(define-private (schedule-timelock (hash (buff 32)) (target principal) (proxy principal) (eta uint) (type uint))
    (let
        (
            (current-ts (unwrap-panic (get-stacks-block-info? time (- stacks-block-height u1))))
            (min-eta (+ current-ts MIN-TIMELOCK-DELAY))
        )
        (asserts! (is-eq (get eta (get-timelock hash)) u0) ERR-TIMELOCK-EXISTS)
        (asserts! (>= eta min-eta) ERR-TIMELOCK-MIN-ETA)
        (ok (map-set timelock-map hash {target: target, proxy: proxy, eta: eta, type: type}))
    )
)

;; Cancels an existing timelock by setting its eta to zero.
;; @params hash; The hash of the timelock to cancel
;; @returns (response true) or reverts
(define-private (cancel-timelock (hash (buff 32)))
    (let
        (
            (eta (get eta (get-timelock hash)))
        )
        (asserts! (> eta u0) ERR-TIMELOCK-HASH)
        (ok (map-delete timelock-map hash))
    )
)

;; Finalizes an existing timelock and sets its eta back to zero.
;; To finalize, the timelock must currently exist and the required time delay must have passed.
;; @params hash; The hash of the timelock to finalize
;; @returns (response true) or reverts
(define-private (finalize-timelock (hash (buff 32)))
    (let
        (
            (current-ts (unwrap-panic (get-stacks-block-info? time (- stacks-block-height u1))))
            (eta (get eta (get-timelock hash)))
        )
        (asserts! (> eta u0) ERR-TIMELOCK-HASH)
        (asserts! (>= current-ts eta) ERR-TIMELOCK-NOT-READY)
        (ok (map-delete timelock-map hash))
    )
)

;; ######################
;; ######################
;; ##### Governance #####
;; ######################
;; ######################

(define-data-var governance-chain-hash (buff 32) 0x00)
(define-data-var governance-address-hash (buff 32) 0x00)


;; Schedules a new task
;; @gateway-impl; Trait reference of the current gateway implementation.
;; @param source-chain; The name of the source chain.
;; @param message-id; The unique identifier of the message.
;; @param source-address; The address of the sender on the source chain.
;; @param payload; The payload that contains the new impl address and eta.
;; @returns (response true) or reverts
(define-public (execute
    (gateway-impl <gateway-trait>)
    (source-chain (string-ascii 19))
    (message-id (string-ascii 128))
    (source-address (string-ascii 128))
    (payload (buff 64000))
)
    (let
        (
            (data (unwrap! (from-consensus-buff? {
                proxy: principal,
                target: principal,
                eta: uint,
                type: uint
            } payload) ERR-PAYLOAD-DATA))
            (payload-hash (keccak256 payload))
            (source-chain-hash (keccak256 (unwrap-panic (to-consensus-buff? source-chain))))
            (source-address-hash (keccak256 (unwrap-panic (to-consensus-buff? source-address))))
        )
        (asserts! (var-get is-started) ERR-NOT-STARTED)
        (asserts! (and (is-eq source-chain-hash (var-get governance-chain-hash)) (is-eq source-address-hash (var-get governance-address-hash))) ERR-UNAUTHORIZED)
        (try! (contract-call? .gateway validate-message gateway-impl source-chain message-id source-address payload-hash))
        (schedule-timelock payload-hash (get target data) (get proxy data) (get eta data) (get type data))
    )
)

(define-constant ACTION-SET-IMPLEMENTATION u1)
(define-constant ACTION-SET-GOVERNANCE u2)
(define-constant ACTION-CANCEL-TASK u3)

;; Finalizes a scheduled task
;; @proxy; Proxy trait reference to run task with.
;; @payload; Hash to find the scheduled task. This is the hash passed while scheduling the task.
;; @returns (response true) or reverts
(define-public (finalize
    (proxy <proxy-trait>)
    (payload (buff 64000))
)
    (let
        (
            (payload-hash (keccak256 payload))
            (timelock (get-timelock payload-hash))
            (target (get target timelock))
            (type (get type timelock))
        )
        (try! (finalize-timelock payload-hash))
        (asserts! (is-eq (contract-of proxy) (get proxy timelock)) ERR-INVALID-PROXY)
        (asserts! (is-eq
            (if (is-eq type ACTION-SET-IMPLEMENTATION)
                (try! (contract-call? proxy set-impl target))
                (if (is-eq type ACTION-SET-GOVERNANCE)
                    (try! (contract-call? proxy set-governance target))
                    false
            )
        ) true) ERR-INVALID-TYPE)
        (ok true)
    )
)

;; Cancels a scheduled task
;; @gateway-impl; Trait reference of the current gateway implementation.
;; @param source-chain; The name of the source chain.
;; @param message-id; The unique identifier of the message.
;; @param source-address; The address of the sender on the source chain.
;; @param payload; The payload that contains the new impl address and eta.
;; @returns (response true) or reverts
(define-public (cancel
    (gateway-impl <gateway-trait>)
    (source-chain (string-ascii 19))
    (message-id (string-ascii 128))
    (source-address (string-ascii 128))
    (payload (buff 64000))
)
    (let
        (
            (data (unwrap! (from-consensus-buff? {
                hash: (buff 32),
                type: uint
            } payload) ERR-PAYLOAD-DATA))
        )
        (asserts! (is-eq (get type data) ACTION-CANCEL-TASK) ERR-INVALID-TYPE)
        (try! (contract-call? .gateway validate-message gateway-impl source-chain message-id source-address (keccak256 payload)))
        (cancel-timelock (get hash data))
    )
)

;; ######################
;; ######################
;; ### Initialization ###
;; ######################
;; ######################

(define-data-var is-started bool false)


(define-constant DEPLOYER tx-sender)

;; Constructor function
;; @param governance-chain; The name of the governance chain
;; @param governance-address; The address of the governance contract
;; @returns (response true) or reverts
(define-public (setup
    (governance-chain (string-ascii 19))
    (governance-address (string-ascii 128))
)
    (begin
        (asserts! (is-eq (var-get is-started) false) ERR-STARTED)
        (asserts! (is-eq contract-caller DEPLOYER) ERR-UNAUTHORIZED)
        (var-set governance-chain-hash (keccak256 (unwrap-panic (to-consensus-buff? governance-chain))))
        (var-set governance-address-hash (keccak256 (unwrap-panic (to-consensus-buff? governance-address))))
        (var-set is-started true)
        (ok true)
    )
)