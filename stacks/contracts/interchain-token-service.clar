
;; title: interchain-token-service
;; version:
;; summary:
;; description:
(use-trait its-trait .traits.interchain-token-service-trait)
(use-trait gas-service-trait .traits.gas-service-impl-trait)
(impl-trait .traits.proxy-trait)

;; ######################
;; ######################
;; ### Proxy Calls ######
;; ######################
;; ######################

(define-constant ERR-INVALID-IMPL (err u140001))
(define-constant ERR-UNTRUSTED-CHAIN (err u140002))
(define-constant ERR-HUB-TRUSTED-ADDRESS-MISSING (err u140003))
(define-constant ERR-ZERO-AMOUNT (err u140004))
(define-constant ERR-NOT-IMPLEMENTED (err u140005))
(define-constant ERR-STARTED (err u140006))

(define-constant MESSAGE-TYPE-SEND-TO-HUB u3)

(define-private (is-correct-impl-raw (impl principal))
    (is-eq
        (contract-call? .interchain-token-service-storage get-service-impl)
        impl))

(define-private (is-correct-impl (interchain-token-service-impl <its-trait>))
    (is-correct-impl-raw
        (contract-of interchain-token-service-impl)))

;; traits
;;
(use-trait sip-010-trait .traits.sip-010-trait)
(use-trait token-manager-trait .traits.token-manager-trait)
(use-trait interchain-token-executable-trait .traits.interchain-token-executable-trait)
(use-trait native-interchain-token-trait .traits.native-interchain-token-trait)
(use-trait gateway-trait .traits.gateway-trait)

;; token definitions
;;

;; constants
;;
(define-constant ERR-NOT-AUTHORIZED (err u21051))

(define-constant DEPLOYER tx-sender)

(define-public (set-paused (its-impl <its-trait>) (status bool))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl set-paused status contract-caller)))


;; ####################
;; ####################
;; ### Operatorship ###
;; ####################
;; ####################

;; Transfers operatorship to a new account
(define-public (transfer-operatorship (its-impl <its-trait>) (new-operator principal))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl transfer-operatorship new-operator contract-caller)
    )
)

(define-public (transfer-ownership (its-impl <its-trait>) (new-owner principal))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl transfer-ownership new-owner contract-caller)
    )
)

;; ####################
;; ####################
;; ### address tracking ###
;; ####################
;; ####################

;; Sets the trusted address and its hash for a remote chain
;; @param its-impl The implementation of the Interchain Token Service
;; @param chain-name Chain name of the remote chain
;; @param address the string representation of the trusted address
;; #[allow(unchecked_data)]
(define-public (set-trusted-address (its-impl <its-trait>) (chain-name (string-ascii 19)) (address (string-ascii 128)))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl set-trusted-address chain-name address contract-caller)))

;; Remove the trusted address of the chain.
;; @param its-impl The implementation of the Interchain Token Service
;; @param chain-name Chain name that should be made untrusted
;; #[allow(unchecked_data)]
(define-public (remove-trusted-address (its-impl <its-trait>) (chain-name  (string-ascii 19)))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl remove-trusted-address chain-name contract-caller)))

(define-read-only (get-its-hub-chain)
    (contract-call? .interchain-token-service-storage get-its-hub-chain))
(define-read-only (get-trusted-address (chain (string-ascii 19)))
    (contract-call? .interchain-token-service-storage get-trusted-address chain))


(define-private (get-call-params (destination-chain (string-ascii 19)) (payload (buff 63000)))
    (let (
            (destination-address (unwrap! (get-trusted-address destination-chain) ERR-UNTRUSTED-CHAIN))
            (destination-address-hash (keccak256 (unwrap-panic (to-consensus-buff? destination-address))))
            (hub-chain (get-its-hub-chain)))
        ;; Prevent sending directly to the ITS Hub chain. This is not supported yet,
        ;; so fail early to prevent the user from having their funds stuck.
        (asserts! (not (is-eq destination-chain hub-chain)) ERR-UNTRUSTED-CHAIN)
        (ok
            {
                ;; Wrap ITS message in an ITS Hub message
                destination-address: (unwrap! (get-trusted-address hub-chain) ERR-HUB-TRUSTED-ADDRESS-MISSING),
                destination-chain: hub-chain,
                payload: (unwrap-panic (to-consensus-buff? {
                    type: MESSAGE-TYPE-SEND-TO-HUB,
                    destination-chain: destination-chain,
                    payload: payload,
                })),
            })))

(define-private (pay-native-gas-for-contract-call
        (gas-service-impl <gas-service-trait>)
        (amount uint)
        (refund-address principal)
        (destination-chain (string-ascii 19))
        (destination-address (string-ascii 128))
        (payload (buff 64000)))
            (contract-call? .gas-service pay-native-gas-for-contract-call
                gas-service-impl
                amount
                (as-contract tx-sender)
                destination-chain
                destination-address
                payload
                refund-address))



;; Calls a contract on a specific destination chain with the given payload
;; @dev This method also determines whether the ITS call should be routed via the ITS Hub.
;; If the `(is-eq (get-trusted-address destination-chain) "hub")`, then the call is wrapped and routed to the ITS Hub destination.
;; Right now only ITS hub payloads are supported
;; @param gateway-impl The implementation of the Gateway contract
;; @param gas-service-impl The implementation of the Gas Service contract
;; @param destination-chain The target chain where the contract will be called.
;; @param payload The data payload for the transaction.
;; @param metadata-version The version of the metadata to be used, currently only contract-call is supported.
;; @param gas-value The amount of gas to be paid for the transaction.
(define-public (its-hub-call-contract
    (gateway-impl <gateway-trait>)
    (gas-service-impl <gas-service-trait>)
    (destination-chain (string-ascii 19))
    (payload (buff 63000))
    (metadata-version uint)
    (gas-value uint))
    (let
        (
            ;; payload can be any arbitrary bytes doesn't need to be checked
            ;; #[filter(destination-chain, payload)]
            (params (try! (get-call-params destination-chain payload)))
            (destination-chain_ (get destination-chain params))
            (destination-address_ (get destination-address params))
            (payload_ (get payload params))
        )
        (asserts! (is-correct-impl-raw contract-caller) ERR-INVALID-IMPL)
        (asserts! (> gas-value u0) ERR-ZERO-AMOUNT)
        (try! (pay-native-gas-for-contract-call gas-service-impl gas-value tx-sender destination-chain_ destination-address_ payload_))
        (as-contract (contract-call? .gateway call-contract gateway-impl destination-chain_ destination-address_ payload_))
    )
)

(define-public (gateway-validate-message
    (gateway-impl <gateway-trait>)
    (source-chain (string-ascii 19))
    (message-id (string-ascii 128))
    (source-address (string-ascii 128))
    (payload-hash (buff 32)))
    (begin
        (asserts! (is-correct-impl-raw contract-caller) ERR-INVALID-IMPL)
        (as-contract (contract-call? .gateway validate-message gateway-impl source-chain message-id source-address payload-hash))))


(define-public (deploy-token-manager
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (salt (buff 32))
        (destination-chain (string-ascii 19))
        (token-manager-type uint)
        (params (buff 62000))
        (token-manager <token-manager-trait>)
        (verification-params {
            nonce: (buff 8),
            fee-rate: (buff 8),
            signature: (buff 65),
            proof: { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint},
            tx-block-height: uint,
            block-header-without-signer-signatures: (buff 800),
        })
    )
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl deploy-token-manager
            gateway-impl
            gas-service-impl
            salt
            destination-chain
            token-manager-type
            params
            token-manager
            verification-params
            contract-caller)))

;; Deploys an interchain token on a destination chain.
;; @param gateway-impl The implementation of the Gateway contract
;; @param gas-service-impl The implementation of the GasService contract
;; @param its-impl The implementation of the InterchainTokenService contract
;; @param salt The salt to be used during deployment.
;; @param destination-chain The destination chain where the token will be deployed.
;; @param name The name of the token.
;; @param symbol The symbol of the token.
;; @param decimals The number of decimals of the token.
;; @param minter The minter address for the token.
;; @param gas-value The amount of gas to be paid for the transaction.
(define-public (deploy-remote-interchain-token
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (salt (buff 32))
        (destination-chain (string-ascii 19))
        (name (string-ascii 32))
        (symbol (string-ascii 32))
        (decimals uint)
        (minter (buff 128))
        (gas-value uint))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl deploy-remote-interchain-token
            gateway-impl
            gas-service-impl
            salt
            destination-chain
            name
            symbol
            decimals
            minter
            gas-value
            contract-caller)))

(define-public (deploy-interchain-token
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (salt (buff 32))
        (token <native-interchain-token-trait>)
        (supply uint)
        (minter (optional principal))
        (verification-params {
            nonce: (buff 8),
            fee-rate: (buff 8),
            signature: (buff 65),
            proof: { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint},
            tx-block-height: uint,
            block-header-without-signer-signatures: (buff 800),
        }))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl deploy-interchain-token
            gateway-impl
            gas-service-impl
            salt
            token
            supply
            minter
            verification-params
            contract-caller)))

;; Initiates an interchain transfer of a specified token to a destination chain.
;; @param gateway-impl The implementation of the Gateway contract
;; @param gas-service-impl The implementation of the GasService contract
;; @param its-impl The implementation of the InterchainTokenService contract
;; @param token-manager The TokenManager contract associated with the token being transferred
;; @param token The token to be transferred.
;; @param token-id The token ID of the token to be transferred
;; @param destination-chain The destination chain to send the tokens to.
;; @param destination-address The address on the destination chain to send the tokens to.
;; @param amount The amount of tokens to be transferred.
;; @param metadata Optional metadata for the call for additional effects (such as calling a destination contract).
;; @param gas-value The amount of gas to be sent with the transfer.
(define-public (interchain-transfer
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (token-manager <token-manager-trait>)
        (token <sip-010-trait>)
        (token-id (buff 32))
        (destination-chain (string-ascii 19))
        (destination-address (buff 128))
        (amount uint)
        (metadata {
            version: uint,
            data: (buff 62000)
        })
        (gas-value uint)
    )
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl interchain-transfer
            gateway-impl
            gas-service-impl
            token-manager
            token
            token-id
            destination-chain
            destination-address
            amount
            metadata
            gas-value
            contract-caller)))

(define-public (call-contract-with-interchain-token
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (token-manager <token-manager-trait>)
        (token <sip-010-trait>)
        (token-id (buff 32))
        (destination-chain (string-ascii 19))
        (destination-address (buff 128))
        (amount uint)
        (metadata {
            version: uint,
            data: (buff 62000)
        })
        (gas-value uint))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl call-contract-with-interchain-token
            gateway-impl
            gas-service-impl
            token-manager
            token
            token-id
            destination-chain
            destination-address
            amount
            metadata
            gas-value
            contract-caller)))




(define-public (execute-deploy-interchain-token
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (source-chain (string-ascii 19))
        (message-id (string-ascii 128))
        (source-address (string-ascii 128))
        (token <native-interchain-token-trait>)
        (payload (buff 62000))
        (verification-params {
            nonce: (buff 8),
            fee-rate: (buff 8),
            signature: (buff 65),
            proof: { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint},
            tx-block-height: uint,
            block-header-without-signer-signatures: (buff 800),
        }))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl execute-deploy-interchain-token
            gateway-impl
            gas-service-impl
            source-chain
            message-id
            source-address
            token
            payload
            verification-params
            contract-caller)))

(define-public (execute-receive-interchain-token
        (gateway-impl <gateway-trait>)
        (its-impl <its-trait>)
        (source-chain (string-ascii 19))
        (message-id (string-ascii 128))
        (source-address (string-ascii 128))
        (token-manager <token-manager-trait>)
        (token <sip-010-trait>)
        (payload (buff 64000))
        (destination-contract (optional <interchain-token-executable-trait>))
    )
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl execute-receive-interchain-token
            gateway-impl
            source-chain
            message-id
            source-address
            token-manager
            token
            payload
            destination-contract
            contract-caller)))


(define-public (set-flow-limit
    (its-impl <its-trait>)
    (token-id (buff 32))
    (token-manager <token-manager-trait>)
    (limit uint))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl set-flow-limit token-id token-manager limit contract-caller)))


(define-public (interchain-token-id
    (its-impl <its-trait>)
    (sender principal) (salt (buff 32)))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl interchain-token-id sender salt)))

(define-public (valid-token-address
    (its-impl <its-trait>)
    (token-id (buff 32)))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl valid-token-address token-id)))

;; ######################
;; ######################
;; ### Upgradability ####
;; ######################
;; ######################

(define-public (set-impl (its-impl principal))
    (let
        (
            (governance-impl (contract-call? .gateway-storage get-governance))
            (prev (contract-call? .interchain-token-service-storage get-service-impl))

        )
        (asserts! (is-eq contract-caller governance-impl) ERR-NOT-AUTHORIZED)
        (try! (contract-call? .interchain-token-service-storage set-service-impl its-impl))
        (print {
            type: "interchain-token-service-impl-upgraded",
            prev: prev,
            new: its-impl
        })
        (ok true)
    )
)

(define-public (set-governance (governance principal))
    ERR-NOT-IMPLEMENTED)



;; General purpose proxy call
(define-public (call (its-impl <its-trait>) (fn (string-ascii 32)) (data (buff 65000)))
    (begin
        (asserts! (is-correct-impl its-impl) ERR-INVALID-IMPL)
        (contract-call? its-impl dispatch fn data contract-caller)
    )
)

;; ######################
;; ######################
;; ### Initialization ###
;; ######################
;; ######################




;; Constructor function
;; @returns (response true) or reverts
(define-public (setup
    (its-contract-address-name (string-ascii 128))
    (gas-service-address principal)
    (operator-address principal)
    (trusted-chain-names-addresses (list 50 {chain-name: (string-ascii 19), address: (string-ascii 128)}))
    (hub-chain (string-ascii 19))
    (its-impl (optional principal))
)
    (begin
        (asserts! (not (contract-call? .interchain-token-service-storage get-is-started)) ERR-STARTED)
        (asserts! (is-eq contract-caller DEPLOYER) ERR-NOT-AUTHORIZED)
        (try! (contract-call? .interchain-token-service-storage set-its-contract-name its-contract-address-name))
        (try! (contract-call? .interchain-token-service-storage set-gas-service gas-service-address))
        (try! (contract-call? .interchain-token-service-storage set-operator operator-address))
        (try! (contract-call? .interchain-token-service-storage set-its-hub-chain hub-chain))
        (try! (contract-call? .interchain-token-service-storage set-trusted-addresses trusted-chain-names-addresses))
        (try! (match its-impl impl (contract-call? .interchain-token-service-storage set-service-impl impl) (ok true)))
        (try! (contract-call? .interchain-token-service-storage start))
        (ok true)
    )
)
