
;; title: interchain-token-factory
;; version:
;; summary:
;; description:
;; This contract is responsible for deploying new interchain tokens and managing their token managers.

;; traits
;;
(use-trait sip-010-trait .traits.sip-010-trait)
(use-trait token-manager-trait .traits.token-manager-trait)
(use-trait native-interchain-token-trait .traits.native-interchain-token-trait)
(use-trait gateway-trait .traits.gateway-trait)
(use-trait gas-service-trait .traits.gas-service-impl-trait)
(use-trait itf-trait .traits.interchain-token-factory-trait)
(use-trait its-trait .traits.interchain-token-service-trait)
(impl-trait .traits.proxy-trait)
;; token definitions
;;

(define-constant ERR-INVALID-IMPL (err u110000))
(define-constant ERR-NOT-AUTHORIZED (err u110001))
(define-constant ERR-NOT-IMPLEMENTED (err u110002))

(define-private (is-correct-impl (interchain-token-factory-impl <itf-trait>))
    (is-eq
        (contract-call? .interchain-token-service-storage get-factory-impl)
        (contract-of interchain-token-factory-impl)))

;; Registers a canonical token as an interchain token and deploys its token manager.
;; @param itf-impl The address of the current InterChainTokenFactory implementation contract
;; @param gateway-impl The address of the current Gateway implementation contract
;; @param gas-service-impl The address of the current GasService implementation contract
;; @param its-impl The address of the current InterchainTransferService implementation contract
;; @param token-address The address of the canonical token.
;; @param token-manager-address The address of the token manager.
;; @param verification-params The verification parameters for the canonical token.
(define-public (register-canonical-interchain-token
        (itf-impl <itf-trait>)
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (token <sip-010-trait>)
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
        (asserts! (is-correct-impl itf-impl) ERR-INVALID-IMPL)
        (contract-call?
            itf-impl
                register-canonical-interchain-token
                gateway-impl
                gas-service-impl
                its-impl
                token
                token-manager
                verification-params
                contract-caller)
    ))


;; Deploys a canonical interchain token on a remote chain.
;; @param itf-impl The address of the current InterChainTokenFactory implementation contract
;; @param gateway-impl The address of the current Gateway implementation contract
;; @param gas-service-impl The address of the current GasService implementation contract
;; @param its-impl The address of the current InterchainTransferService implementation contract
;; @param token The address of the original token on the original chain.
;; @param destination-chain The name of the chain where the token will be deployed.
;; @param gas-value The gas amount to be sent for deployment.
;; #[allow(unchecked_data)]
(define-public (deploy-remote-canonical-interchain-token
        (itf-impl <itf-trait>)
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (token <sip-010-trait>)
        (destination-chain (string-ascii 19))
        (gas-value uint))
    (begin
        (asserts! (is-correct-impl itf-impl) ERR-INVALID-IMPL)
        (contract-call? itf-impl deploy-remote-canonical-interchain-token
            gateway-impl
            gas-service-impl
            its-impl
            token
            destination-chain
            gas-value
            contract-caller)
    )
)

(define-public (deploy-interchain-token
        (itf-impl <itf-trait>)
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (salt_ (buff 32))
        (token <native-interchain-token-trait>)
        (initial-supply uint)
        (minter principal)
        (verification-params {
            nonce: (buff 8),
            fee-rate: (buff 8),
            signature: (buff 65),
            proof: { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint},
            tx-block-height: uint,
            block-header-without-signer-signatures: (buff 800),
        }))
    (begin
        (asserts! (is-correct-impl itf-impl) ERR-INVALID-IMPL)
        (contract-call? itf-impl deploy-interchain-token
            gateway-impl
            gas-service-impl
            its-impl
            salt_
            token
            initial-supply
            minter
            verification-params
            contract-caller)))

;; This will only be a risk if the user deploying the token remotely
;; is deploying an existing malicious token on stacks
;; basically getting themself rekt
;; #[allow(unchecked_data)]

(define-public (deploy-remote-interchain-token
        (itf-impl <itf-trait>)
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (salt_ (buff 32))
        (minter_ principal)
        (destination-chain (string-ascii 19))
        (gas-value uint)
        (token <sip-010-trait>)
        (token-manager <token-manager-trait>)
)
    (begin
        (asserts! (is-correct-impl itf-impl) ERR-INVALID-IMPL)
        (contract-call? itf-impl deploy-remote-interchain-token
            gateway-impl
            gas-service-impl
            its-impl
            salt_
            minter_
            destination-chain
            gas-value
            token
            token-manager
            contract-caller)))

(define-public (deploy-remote-interchain-token-with-minter
        (itf-impl <itf-trait>)
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (salt_ (buff 32))
        (minter_ principal)
        (destination-chain (string-ascii 19))
        (destination-minter (optional (buff 128)))
        (gas-value uint)
        (token <sip-010-trait>)
        (token-manager <token-manager-trait>)
)
    (begin
        (asserts! (is-correct-impl itf-impl) ERR-INVALID-IMPL)
        (contract-call? itf-impl deploy-remote-interchain-token-with-minter
            gateway-impl
            gas-service-impl
            its-impl
            salt_
            minter_
            destination-chain
            destination-minter
            gas-value
            token
            token-manager
            contract-caller)))

;; Allow the minter to approve the deployer for a remote interchain token deployment that uses a custom destinationMinter address.
;; This ensures that a token deployer can't choose the destinationMinter itself, and requires the approval of the minter to reduce trust assumptions on the deployer.
(define-public (approve-deploy-remote-interchain-token
    (itf-impl <itf-trait>)
    (its-impl <its-trait>)
    (deployer principal)
    (salt_ (buff 32))
    (destination-chain (string-ascii 19))
    (destination-minter (buff 128))
    (token <native-interchain-token-trait>)
)
    (begin
        (asserts!  (is-correct-impl itf-impl) ERR-INVALID-IMPL)
        (contract-call? itf-impl approve-deploy-remote-interchain-token
            its-impl
            deployer
            salt_
            destination-chain
            destination-minter
            token
            contract-caller)
    ))


;; Allows the minter to revoke a deployer's approval for a remote interchain token deployment that uses a custom destinationMinter address.
(define-public (revoke-deploy-remote-interchain-token
        (itf-impl <itf-trait>)
        (its-impl <its-trait>)
        (deployer principal)
        (salt_ (buff 32))
        (destination-chain (string-ascii 19)))
    (begin
        (asserts! (is-correct-impl itf-impl) ERR-INVALID-IMPL)
        (contract-call? itf-impl revoke-deploy-remote-interchain-token
            its-impl
            deployer
            salt_
            destination-chain
            contract-caller
        )))


;; ######################
;; ######################
;; ### Upgradability ####
;; ######################
;; ######################

(define-public (set-impl (itf-impl principal))
    (let
        (
            (governance-impl (contract-call? .gateway-storage get-governance))
            (prev (contract-call? .interchain-token-service-storage get-factory-impl))
        )
        (asserts! (is-eq contract-caller governance-impl) ERR-NOT-AUTHORIZED)
        (try! (contract-call? .interchain-token-service-storage set-factory-impl itf-impl))
        (print {
            type: "interchain-token-factory-impl-upgraded",
            prev: prev,
            new: itf-impl
        })
        (ok true)
    )
)

(define-public (set-governance (governance principal))
    ERR-NOT-IMPLEMENTED)

;; General purpose proxy call
(define-public (call (itf-impl <itf-trait>) (fn (string-ascii 32)) (data (buff 65000)))
    (begin
        (asserts! (is-correct-impl itf-impl) ERR-INVALID-IMPL)
        (contract-call? itf-impl dispatch fn data contract-caller)
    )
)
