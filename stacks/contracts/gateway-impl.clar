(impl-trait .traits.gateway-trait)

(define-constant NULL-PUB 0x00)

(define-constant PROXY .gateway)

(define-private (is-proxy) (is-eq contract-caller PROXY))

(define-read-only (get-is-started) (contract-call? .gateway-storage get-is-started))

(define-constant ERR-NOT-STARTED (err u50000))
(define-constant ERR-UNAUTHORIZED (err u50001))
(define-constant ERR-NOT-IMPLEMENTED (err u50002))
(define-constant ERR-MESSAGES-DATA (err u50003))
(define-constant ERR-MESSAGE-NOT-FOUND (err u50004))
(define-constant ERR-MESSAGE-INSERT (err u50005))
(define-constant ERR-ONLY-OPERATOR (err u50006))
(define-constant ERR-SIGNERS-LEN (err u50006))
(define-constant ERR-SIGNER-WEIGHT (err u50007))
(define-constant ERR-SIGNERS-ORDER (err u50008))
(define-constant ERR-SIGNERS-THRESHOLD (err u50009))
(define-constant ERR-SIGNERS-THRESHOLD-MISMATCH (err u50010))
(define-constant ERR-INVALID-SIGNATURE-DATA (err u50011))
(define-constant ERR-LOW-SIGNATURES-WEIGHT (err u50012))
(define-constant ERR-INVALID-SIGNERS (err u50013))
(define-constant ERR-INSUFFICIENT-ROTATION-DELAY (err u50014))
(define-constant ERR-SIGNERS-DATA (err u50015))
(define-constant ERR-PROOF-DATA (err u50016))
(define-constant ERR-DUPLICATE-SIGNERS (err u50017))
(define-constant ERR-NOT-LATEST-SIGNERS (err u50018))


;; Sends a message to the specified destination chain and address with a given payload.
;; This function is the entry point for general message passing between chains.
;; @param destination-chain; The chain where the destination contract exists. A registered chain name on Axelar must be used here
;; @param destination-contract-address; The address of the contract to call on the destination chain
;; @param payload; The payload to be sent to the destination contract, usually representing an encoded function call with arguments
;; @param sender the proxy contract-caller passed from the proxy
(define-public (call-contract
    (destination-chain (string-ascii 19))
    (destination-contract-address (string-ascii 128))
    (payload (buff 64000))
    (sender principal)
)
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (contract-call? .gateway-storage emit-contract-call sender destination-chain destination-contract-address payload (keccak256 payload)))
        (ok true)
    )
)

;; ######################
;; ######################
;; ##### Messaging ######
;; ######################
;; ######################

(define-constant MESSAGE-EXECUTED 0x01)

;; Compute the command-id for a message.
;; @param source-chain The name of the source chain as registered on Axelar.
;; @param message-id The unique message id for the message.
;; @returns (buff 32) the command-id.
(define-read-only (message-to-command-id (source-chain (string-ascii 19)) (message-id (string-ascii 128)))
    ;; Axelar doesn't allow `sourceChain` to contain '_', hence this encoding is unambiguous
    (keccak256 (unwrap-panic (to-consensus-buff? (concat (concat source-chain "_") message-id)))))


;; For backwards compatibility with `validateContractCall`, `commandId` is used here instead of `messageId`.
;; @returns (buff 32) the message hash
(define-private (get-message-hash (message {
        message-id: (string-ascii 128),
        source-chain: (string-ascii 19),
        source-address: (string-ascii 128),
        contract-address: principal,
        payload-hash: (buff 32)
    }))
    (keccak256 (unwrap-panic (to-consensus-buff? message)))
)

;; Helper function to build keccak256 data-hash from messages
;; @param messages;
;; @returns (response (buff 32))
(define-read-only (data-hash-from-messages (messages (list 10 {
                source-chain: (string-ascii 19),
                message-id: (string-ascii 128),
                source-address: (string-ascii 128),
                contract-address: principal,
                payload-hash: (buff 32)
        })))
    (keccak256 (unwrap-panic (to-consensus-buff? (merge {data: messages} { type: "approve-messages" }))))
)

;; Approves a message if it hasn't been approved before. The message status is set to approved.
;; @params message;
;; @returns (some message) or none
(define-private (approve-message (message {
                source-chain: (string-ascii 19),
                message-id: (string-ascii 128),
                source-address: (string-ascii 128),
                contract-address: principal,
                payload-hash: (buff 32)
            }))
            (let (
                    (command-id (message-to-command-id (get source-chain message) (get message-id message)))
                    (inserted (unwrap! (contract-call? .gateway-storage insert-message command-id (get-message-hash {
                        message-id: (get message-id message),
                        source-chain: (get source-chain message),
                        source-address: (get source-address message),
                        contract-address: (get contract-address message),
                        payload-hash: (get payload-hash message)
                    })) ERR-MESSAGE-INSERT))
                )
                (if inserted (some (contract-call? .gateway-storage emit-message-approved command-id message)) none)
                (ok inserted)
            )
)

;; @notice Approves an array of messages, signed by the Axelar signers.
;; @param messages; The list of messages to verify.
;; @param proof; The proof signed by the Axelar signers for this command.
;; @returns (response true) or err
(define-public (approve-messages
    (messages (buff 4096))
    (proof (buff 16384)))
    (let (
        (proof_ (unwrap! (from-consensus-buff? {
                signers: {
                    signers: (list 100 {signer: (buff 33), weight: uint}),
                    threshold: uint,
                    nonce: (buff 32)
                },
                signatures: (list 100 (buff 65))
            } proof) ERR-SIGNERS-DATA))
        (messages_ (unwrap! (from-consensus-buff?
            (list 10 {
                source-chain: (string-ascii 19),
                message-id: (string-ascii 128),
                source-address: (string-ascii 128),
                contract-address: principal,
                payload-hash: (buff 32)
            })
            messages) ERR-MESSAGES-DATA))
             (data-hash (data-hash-from-messages messages_)
        ))
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (validate-proof data-hash proof_))
        (ok (map approve-message messages_))
    )
)

;; Validates if a message is approved. If message was in approved status, status is updated to executed to avoid replay.
;; @param source-chain; The name of the source chain.
;; @param message-id; The unique identifier of the message.
;; @param source-address; The address of the sender on the source chain.
;; @param payload-hash The keccak256 hash of the payload data.
;; @param sender the proxy contract-caller passed from the proxy
;; @returns (response true) or reverts
(define-public (validate-message
    (source-chain (string-ascii 19))
    (message-id (string-ascii 128))
    (source-address (string-ascii 128))
    (payload-hash (buff 32))
    (sender principal)
)
    (let (
        (command-id (message-to-command-id source-chain message-id))
        (message-hash (get-message-hash {
                message-id: message-id,
                source-chain: source-chain,
                source-address: source-address,
                contract-address: sender,
                payload-hash: payload-hash
            }))
    )
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (asserts! (is-eq (get-message command-id) message-hash) ERR-MESSAGE-NOT-FOUND)
        (try! (contract-call? .gateway-storage set-message command-id MESSAGE-EXECUTED))
        (try! (contract-call? .gateway-storage emit-message-executed command-id source-chain message-id))
        (ok true)
    )
)

;; Checks if a message is approved.
;; Determines whether a given message, identified by the source-chain and message-id, is approved.
;; @param source-chain; The name of the source chain.
;; @param message-id; The unique identifier of the message.
;; @param source-address; The address of the sender on the source chain.
;; @param contract-address; The address of the contract where the call will be executed.
;; @param payload-hash; The keccak256 hash of the payload data.
;; @returns (response bool)
(define-read-only (is-message-approved
    (source-chain (string-ascii 19))
    (message-id (string-ascii 128))
    (source-address (string-ascii 128))
    (contract-address principal)
    (payload-hash (buff 32))
)
    (let (
            (command-id (message-to-command-id source-chain message-id))
            (message-hash (get-message-hash {
                message-id: message-id,
                source-chain: source-chain,
                source-address: source-address,
                contract-address: contract-address,
                payload-hash: payload-hash
            }))
        )
        (ok (is-eq message-hash (get-message command-id))))
)

;; Checks if a message is executed.
;; Determines whether a given message, identified by the source-chain and message-id is executed.
;; @param source-chain; The name of the source chain.
;; @param message-id; The unique identifier of the message.
;; @returns (response bool)
(define-read-only (is-message-executed
    (source-chain (string-ascii 19))
    (message-id (string-ascii 128))
)
    (ok (is-eq MESSAGE-EXECUTED (get-message (message-to-command-id source-chain message-id))))
)

;; Message getter with the command-id. Returns an empty buffer if no message matched.
;; @param command-id
;; @returns (buff 32) or (buff 1)
(define-read-only (get-message
    (command-id (buff 32))
)
    (default-to 0x00 (contract-call? .gateway-storage get-message command-id))
)

;; ####################
;; ####################
;; ### Operatorship ###
;; ####################
;; ####################


(define-read-only (get-operator) (contract-call? .gateway-storage get-operator))

;; Transfers operatorship to a new account
(define-public (transfer-operatorship (new-operator principal) (caller principal))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (asserts! (is-eq caller (get-operator)) ERR-ONLY-OPERATOR)
        (try! (contract-call? .gateway-storage set-operator new-operator))
        (try! (contract-call? .gateway-storage emit-transfer-operatorship new-operator))
        (ok true)
    )
)

;; #########################
;; #########################
;; ### Weighted Multisig ###
;; #########################
;; #########################

;; Current signers epoch
(define-read-only (get-epoch) (contract-call? .gateway-storage get-epoch))

;; The timestamp for the last signer rotation
(define-read-only (get-last-rotation-timestamp) (contract-call? .gateway-storage get-last-rotation-timestamp))

;; The map of signer hash by epoch
(define-read-only (get-signer-hash-by-epoch (signer-epoch uint)) (contract-call? .gateway-storage get-signer-hash-by-epoch signer-epoch))

;; The map of epoch by signer hash
(define-read-only (get-epoch-by-signer-hash (signer-hash (buff 32))) (contract-call? .gateway-storage get-epoch-by-signer-hash signer-hash))

;; Previous signers retention. 0 means only the current signers are valid
(define-read-only (get-previous-signers-retention) (contract-call? .gateway-storage get-previous-signers-retention))

;; The domain separator for the signer proof
(define-read-only (get-domain-separator) (contract-call? .gateway-storage get-domain-separator))

;; The minimum delay required between rotations
(define-read-only (get-minimum-rotation-delay) (contract-call? .gateway-storage get-minimum-rotation-delay))

;; Compute the message hash that is signed by the weighted signers
;; Returns an Stacks Signed Message, created from `domain-separator`, `signers-hash`, and `data-hash`.
;; @param signers-hash; The hash of the weighted signers that sign off on the data
;; @param data-hash; The hash of the data
;; @returns (buff 32); The message hash to be signed
(define-read-only (message-hash-to-sign (signers-hash (buff 32)) (data-hash (buff 32)))
    (keccak256
        (concat
            (unwrap-panic (to-consensus-buff? "Stacks Signed Message"))
            (concat
                (get-domain-separator)
                (concat
                    signers-hash
                    data-hash
                )
            )
        )
    )
)

;; Helper function to build keccak256 data-hash from signers
;; @param signers;
;; @returns (response (buff 32))
(define-read-only (data-hash-from-signers (signers {
                signers: (list 100 {signer: (buff 33), weight: uint}),
                threshold: uint,
                nonce: (buff 32)
            })
)
    (keccak256 (unwrap-panic (to-consensus-buff? (merge {data: signers} { type: "rotate-signers" }))))
)

;; Helper function to build keccak256 of signers
;; @param signers;
;; @returns (response (buff 32))
(define-read-only (get-signers-hash (signers {
                signers: (list 100 {signer: (buff 33), weight: uint}),
                threshold: uint,
                nonce: (buff 32)
            })
)
    (keccak256 (unwrap-panic (to-consensus-buff? signers)))
)


;; ##########################
;; ### Signers validation ###
;; ##########################


;; Returns weight of a signer
;; @param signer; Signer to validate
;; @returns uint
(define-private (get-signer-weight (signer {signer: (buff 33), weight: uint})) (get weight signer))

;; Validates a particular signer's weight
;; @param signer; Signer to validate
;; @returns bool
(define-private (validate-signer-weight (signer {signer: (buff 33), weight: uint}))
    (> (get weight signer) u0) ;; signer weight must be bigger than zero
)

;; Validates public key order accumulating error inside the state provided
;; @param pub; The public key
;; @param state; State to accumulate next public key and errors
;; @returns {pub: (buff 33), failed: bool}
(define-private (validate-pub-order (pub (buff 33)) (state {pub: (buff 33), failed: bool}))
    (if (> pub (get pub state)) (merge state {pub: pub}) {pub: pub, failed: true})
)

;; This function checks if the provided signers are valid, i.e sorted and contain no duplicates, with valid weights and threshold
;; @param signers; Signers to validate
;; @returns (response true) or reverts
(define-private (validate-signers (signers {
            signers: (list 100 {signer: (buff 33), weight: uint}),
            threshold: uint,
            nonce: (buff 32)
        }))
    (let
        (
            (signers- (get signers signers))
            (threshold (get threshold signers))
            (total-weight (fold + (map get-signer-weight signers-) u0))
        )
        ;; signers list must have at least one item
        (asserts! (> (len signers-) u0) ERR-SIGNERS-LEN)
        ;; threshold must be bigger than zero
        (asserts! (> threshold u0) ERR-SIGNERS-THRESHOLD)
        ;; total weight of signers must be bigger than the threshold
        (asserts! (>= total-weight threshold) ERR-SIGNERS-THRESHOLD-MISMATCH)
        ;; signer weights need to be > 0
        (asserts! (is-eq (len (filter not (map validate-signer-weight signers-))) u0) ERR-SIGNER-WEIGHT)
        ;; signers need to be in strictly increasing order
        (asserts! (not (get failed (fold validate-pub-order (map get-signer-pub signers-) {pub: 0x00, failed: false}))) ERR-SIGNERS-ORDER)
        (ok true)
    )
)

;; ############################
;; ### Signature validation ###
;; ############################


;; Accumulates weight of signers
;; @param signer
;; @accumulator
(define-private (accumulate-weights (signer {signer: (buff 33), weight: uint}) (accumulator uint))
    (+ accumulator (get weight signer))
)

;; Returns public key of a signer
;; @param signer
;; @returns (buff 33)
(define-private (get-signer-pub (signer {signer: (buff 33), weight: uint})) (get signer signer))

;; Recovers ECDSA signature with the message hash provided
;; @param signature
;; @param message-hash
;; @returns (response (buff 33) uint)
(define-private (recover-signature (signature (buff 65)) (message-hash (buff 32)))
     (secp256k1-recover? message-hash signature)
)

;; Returns true if the provided response is an error
;; @param signer
;; @returns bool
(define-private (is-error-or-pub (signer (response (buff 33) uint)))
  (is-err signer)
)

;; Helper function to unwrap pubkey from response
;; @param pub
;; @returns (buff 33)
(define-private (unwrap-pub (pub (response (buff 33) uint))) (unwrap-panic pub))

;; Helper function to iterate pubkeys along with signers and return signer
;; @param pub
;; @param signer
;; @returns {signer: (buff 33), weight: uint}
(define-private (pub-to-signer (pub (buff 33)) (signer {signer: (buff 33), weight: uint})) signer)

;; Helper function to repeat the same messages hash in a list
;; @param signature
;; @param state
;; @returns (list 100 (buff 32))
(define-private (repeat-message-hash (signature (buff 65)) (state (list 100 (buff 32))) )
    (unwrap-panic (as-max-len? (append state (unwrap-panic (element-at? state u0))) u100))
)

(define-data-var signers-temp (list 100 {signer: (buff 33), weight: uint}) (list))
;; Marker value used to identify that no singer was found, it must be larger
;; than any valid index (99), so anything big works
(define-constant SIGNER-NOT-FOUND u12345)

(define-private (get-valid-signers
            (signers (list 100 {signer: (buff 33), weight: uint}))
            (pubs (list 100 (buff 33))))
    (let (
            (set-result (var-set signers-temp signers))
            (validated-signers (get result (fold fold-validate-signers pubs {
                just-signers: (map get-signer-pub signers),
                result: (list)
            }))))
        (asserts! (is-eq (len validated-signers) (len pubs)) ERR-INVALID-SIGNERS)
        (var-set signers-temp (list))
        (ok validated-signers)
))

(define-private (fold-validate-signers
        (pub-to-check (buff 33))
        (acc { just-signers: (list 100 (buff 33)), result: (list 100 {signer: (buff 33), weight: uint}) } ))
    (let (
        (just-signers (get just-signers acc))
        (signer-index (default-to SIGNER-NOT-FOUND (index-of? just-signers pub-to-check)))
    )
    (if (is-eq signer-index SIGNER-NOT-FOUND)
        acc
        {
            just-signers: just-signers,
            result: (unwrap-panic (as-max-len? (append
                (get result acc)
                (unwrap-panic (element-at? (var-get signers-temp) signer-index))
            ) u100))
        }
    )
  )
)

;; This function takes message-hash and proof data and reverts if proof is invalid
;; The signers and signatures should be sorted by signer address in ascending order
;; @param message-hash; The hash of the message that was signed
;; @param signers; The weighted signers
;; @param signatures The sorted signatures data
(define-private (validate-signatures
                (message-hash (buff 32))
                (signers {
                    signers: (list 100 {signer: (buff 33), weight: uint}),
                    threshold: uint,
                    nonce: (buff 32)
                })
                (signatures (list 100 (buff 65))
))
    (let
        (
            (message-hash-repeated (fold repeat-message-hash signatures (list message-hash)))
            (recovered (map recover-signature signatures message-hash-repeated))
            (recover-err (element-at? (filter is-error-or-pub recovered) u0))
            (recover-err-check (asserts! (is-none recover-err) ERR-INVALID-SIGNATURE-DATA))
            (pubs (map unwrap-pub recovered))
            ;; the signers and signatures should be sorted by signer address in ascending order
            (pubs-order-check (asserts! (not (get failed (fold validate-pub-order pubs {pub: 0x00, failed: false}))) ERR-SIGNERS-ORDER))
            (signers- (get signers signers))
            ;; the signers and signatures should be sorted by signer address in ascending order
            (signers-order-check (asserts! (not (get failed (fold validate-pub-order (map get-signer-pub signers-) {pub: 0x00, failed: false}))) ERR-SIGNERS-ORDER))
            (validated-signers (try! (get-valid-signers signers- pubs)))
            (total-weight (fold accumulate-weights validated-signers u0))
            (weight-check (asserts! (>= total-weight (get threshold signers)) ERR-LOW-SIGNATURES-WEIGHT))
        )
        (ok true)
    )
)


;; ########################
;; ### Proof validation ###
;; ########################


;; This function takes data-hash and proof data and reverts if proof is invalid
;; @param data-hash; The hash of the message that was signed
;; @param proof; The multisig proof data
;; @returns (response true) or reverts
(define-private (validate-proof (data-hash (buff 32)) (proof {
                signers: {
                    signers: (list 100 {signer: (buff 33), weight: uint}),
                    threshold: uint,
                    nonce: (buff 32)
                },
                signatures: (list 100 (buff 65))
            }))
    (let
        (
            (signers (get signers proof))
            (signers-hash (get-signers-hash signers))
            (signer-epoch (default-to u0 (get-epoch-by-signer-hash signers-hash)))
            (current-epoch (get-epoch))
            ;; True if the proof is from the latest signer set
            (is-latest-signers (is-eq signer-epoch current-epoch))
            (message-hash (message-hash-to-sign signers-hash data-hash))
        )

        (asserts! (is-eq (or (is-eq signer-epoch u0) (> (- current-epoch signer-epoch) (get-previous-signers-retention))) false) ERR-INVALID-SIGNERS)

        (try! (validate-signatures message-hash signers (get signatures proof)))

        (ok is-latest-signers)
    )
)

;; ########################
;; ### Signer rotation ####
;; ########################



;; Updates the last rotation timestamp, and enforces the minimum rotation delay if specified
;; @params enforce-rotation-delay
;; @returns (response true) or reverts
(define-private (update-rotation-timestamp (enforce-rotation-delay bool))
    (let
        (
            (last-rotation-timestamp_ (get-last-rotation-timestamp))
            (current-ts (unwrap-panic (get-stacks-block-info? time  (- stacks-block-height u1))))
        )
        (asserts! (is-eq (and (is-eq enforce-rotation-delay true) (< (- current-ts last-rotation-timestamp_) (get-minimum-rotation-delay))) false) ERR-INSUFFICIENT-ROTATION-DELAY)
        (try! (contract-call? .gateway-storage set-last-rotation-timestamp current-ts))
        (ok true)
    )
)

;; This function rotates the current signers with a new set of signers
;; @param new-signers The new weighted signers data
;; @param enforce-rotation-delay If true, the minimum rotation delay will be enforced
;; @returns (response true) or reverts
(define-public (rotate-signers-inner (new-signers {
                signers: (list 100 {signer: (buff 33), weight: uint}),
                threshold: uint,
                nonce: (buff 32)
            }) (enforce-rotation-delay bool)
)
    (let
            (
                (new-signers-hash (get-signers-hash new-signers))
                (new-epoch (+ (get-epoch) u1))
            )
            (asserts! (is-proxy) ERR-UNAUTHORIZED)
            (asserts! (is-none (get-epoch-by-signer-hash new-signers-hash)) ERR-DUPLICATE-SIGNERS)
            (try! (validate-signers new-signers))
            (try! (update-rotation-timestamp enforce-rotation-delay))
            (try! (contract-call? .gateway-storage set-epoch new-epoch))
            (try! (contract-call? .gateway-storage set-signer-hash-by-epoch new-epoch new-signers-hash))
            (try! (contract-call? .gateway-storage set-epoch-by-signer-hash new-signers-hash new-epoch))
            (try! (contract-call? .gateway-storage emit-signers-rotated new-epoch new-signers new-signers-hash))
            (ok true)
        )
)

;; Rotate the weighted signers, signed off by the latest Axelar signers.
;; The minimum rotation delay is enforced by default, unless the caller is the gateway operator.
;; The gateway operator allows recovery in case of an incorrect/malicious rotation, while still requiring a valid proof from a recent signer set.
;; Rotation to duplicate signers is rejected.
;; @param new-signers; The data for the new signers.
;; @param proof; The proof signed by the Axelar verifiers for this command.
;; @returns (response true) or reverts
(define-public (rotate-signers
    (new-signers (buff 8192))
    (proof (buff 16384))
)
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (let
            (
                (new-signers_ (unwrap! (from-consensus-buff? {
                    signers: (list 100 {signer: (buff 33), weight: uint}),
                    threshold: uint,
                    nonce: (buff 32)
                } new-signers) ERR-SIGNERS-DATA))
                (proof_ (unwrap! (from-consensus-buff? {
                    signers: {
                        signers: (list 100 {signer: (buff 33), weight: uint}),
                        threshold: uint,
                        nonce: (buff 32)
                    },
                    signatures: (list 100 (buff 65))
                } proof) ERR-PROOF-DATA))
                (data-hash (data-hash-from-signers new-signers_))
                (enforce-rotation-delay (not (is-eq tx-sender (get-operator))))
                (is-latest-signers (try! (validate-proof data-hash proof_)))
            )
            ;; if the caller is not the operator the signer set provided in proof must be the latest
            (asserts! (is-eq (and (is-eq enforce-rotation-delay true) (is-eq is-latest-signers false)) false) ERR-NOT-LATEST-SIGNERS)
            (try! (rotate-signers-inner new-signers_ enforce-rotation-delay))
            (ok true)
        )
    )
)

;; #########################
;; #########################
;; #### Dynamic Dispatch ###
;; #########################
;; #########################

(define-public (dispatch (fn (string-ascii 32)) (data (buff 65000)))
    (begin
        (asserts! (is-proxy) ERR-UNAUTHORIZED)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        ERR-NOT-IMPLEMENTED
    )
)