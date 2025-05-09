
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
(use-trait its-trait .traits.interchain-token-service-trait)
(use-trait gas-service-trait .traits.gas-service-impl-trait)
(impl-trait .traits.interchain-token-factory-trait)
;; token definitions
;;

;; constants
;;


(define-constant PROXY .interchain-token-factory)

(define-private (is-proxy) (is-eq contract-caller PROXY))
;; This type is reserved for interchain tokens deployed by ITS, and can't be used by custom token managers.
;; @notice same as mint burn in functionality will be custom tokens made by us
;; that are deployed outside of the contracts but registered by the ITS contract
;; The token will be locked/unlocked at the token manager.
(define-constant TOKEN-TYPE-LOCK-UNLOCK u2)

(define-constant ERR-TOKEN-NOT-ENABLED (err u100001))
(define-constant ERR-INVALID-MINTER (err u100002))
(define-constant ERR-NOT-MINTER (err u100003))
(define-constant ERR-SERVICE-NOT-DEPLOYED (err u100004)) 
(define-constant ERR-TOKEN-NOT-DEPLOYED (err u100005))
(define-constant ERR-MANAGER-NOT-DEPLOYED (err u100006))
(define-constant ERR-NOT-PROXY (err u100007))
(define-constant ERR-TOKEN-NOT-FOUND (err u100008))
(define-constant ERR-TOKEN-MISMATCH (err u100009))
(define-constant ERR-INVALID-CHAIN-NAME (err u100010))
(define-constant ERR-REMOTE-DEPLOYMENT-NOT-APPROVED (err u100011))
(define-constant ERR-CANNOT-DELETE-DEPLOY-APPROVAL (err u100012))
(define-constant ERR-PAUSED (err u100013))
(define-constant ERR-INVALID-PARAMS (err u100014))
(define-constant ERR-NOT-TOKEN-DEPLOYER (err u100015))
(define-constant ERR-NOT-IMPLEMENTED (err u100016))




(define-constant CONTRACT-ID (keccak256 (unwrap-panic (to-consensus-buff? "interchain-token-factory"))))
(define-constant PREFIX-CANONICAL-TOKEN-SALT (keccak256 (unwrap-panic (to-consensus-buff? "canonical-token-salt"))))
(define-constant PREFIX-INTERCHAIN-TOKEN-SALT (keccak256 (unwrap-panic (to-consensus-buff? "interchain-token-salt"))))
(define-constant NULL-BYTES 0x)
(define-constant NULL-ADDRESS (unwrap-panic (principal-construct? (if (is-eq chain-id u1) 0x16 0x1a) 0x0000000000000000000000000000000000000000)))
(define-constant TOKEN-FACTORY-DEPLOYER NULL-ADDRESS)
(define-constant CHAIN-NAME "stacks")
(define-constant CHAIN-NAME-HASH (keccak256 (unwrap-panic (to-consensus-buff? CHAIN-NAME))))
;; The address of the interchain token service.

;; Getter for the contract id.
;; @return (buff 32) The contract id of this contract.
(define-read-only (get-contract-id)
    (ok CONTRACT-ID))

;; Calculates the salt for an interchain token.
;; @param deployer The address of the deployer.
;; @param salt A unique identifier to generate the salt.
(define-read-only (get-interchain-token-deploy-salt (deployer principal) (salt (buff 32)))
        (keccak256
            (concat
                (concat PREFIX-INTERCHAIN-TOKEN-SALT CHAIN-NAME-HASH)
                (concat
                    (unwrap-panic (to-consensus-buff? deployer))
                    salt))))

;; Calculates the salt for a canonical interchain token.
;; @param token-address The address of the token.
;; @return (buff 32) The calculated salt for the interchain token.
(define-read-only (get-canonical-interchain-token-deploy-salt (token-address principal))
        (keccak256
            (concat
                (concat PREFIX-CANONICAL-TOKEN-SALT CHAIN-NAME-HASH)
                (unwrap-panic (to-consensus-buff? token-address)))))

(define-read-only (decode-contract-principal (contract-principal principal))
    (let (
        (data (unwrap! (principal-destruct? contract-principal) ERR-INVALID-PARAMS))
        (contract-name-str (unwrap! (get name data) ERR-INVALID-PARAMS))
        (contract-name-buff (unwrap-panic (to-consensus-buff? contract-name-str)))
        (contract-name (unwrap-panic (slice? contract-name-buff u5 (len contract-name-buff))))
    )
    (ok {
        contract-name: contract-name,
        deployer: (unwrap! (principal-construct? (get version data) (get hash-bytes data)) ERR-INVALID-PARAMS),
    })))


;; Computes the ID for an interchain token based on the deployer and a salt.
;; @param deployer The address that deployed the interchain token.
;; @param salt A unique identifier used in the deployment process.
;; @return (buff 32) The ID of the interchain token.
(define-private (get-interchain-token-id-raw (its-impl <its-trait>) (salt (buff 32)))
    (contract-call? .interchain-token-service interchain-token-id its-impl TOKEN-FACTORY-DEPLOYER salt))

;; Computes the ID for an interchain token based on the deployer and a salt.
;; @param deployer The address that deployed the interchain token.
;; @param salt A unique identifier used in the deployment process.
;; @return (buff 32) The ID of the interchain token.
(define-public (get-interchain-token-id (its-impl <its-trait>) (deployer principal) (salt (buff 32)))
    (let ((deploy-salt (get-interchain-token-deploy-salt deployer salt)))
    ;; this is assumed to be a read only operation
    ;; #[allow(unchecked_data)]
        (get-interchain-token-id-raw its-impl deploy-salt)))


;; Computes the ID for a canonical interchain token based on its address.
;; @param token-address The address of the canonical interchain token.
;; @return (buff 32) The ID of the canonical interchain token.
(define-public (get-canonical-interchain-token-id (its-impl <its-trait>) (token-address principal))
    ;; this is assumed to be a read only operation
    ;; #[allow(unchecked_data)]
    (get-interchain-token-id-raw its-impl (get-canonical-interchain-token-deploy-salt token-address)))


(define-constant LOCAL-DEPLOYMENT "")
;; Registers a canonical token as an interchain token and deploys its token manager.
;; @param token-address The address of the canonical token.
;; @param token-manager-address The address of the token manager.
;; @param verification-params The verification parameters for the token manager deployment.
(define-public (register-canonical-interchain-token
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
        (caller principal)
    )
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (is-eq (get deployer
            (try! (decode-contract-principal (contract-of token-manager)))
        ) caller) ERR-NOT-TOKEN-DEPLOYER)
        (asserts! (unwrap! (contract-call? token-manager get-is-started) ERR-TOKEN-NOT-ENABLED) ERR-TOKEN-NOT-ENABLED)
        (contract-call?
            .interchain-token-service
                deploy-token-manager
                gateway-impl
                gas-service-impl
                its-impl
                (get-canonical-interchain-token-deploy-salt (contract-of token))
                LOCAL-DEPLOYMENT
                TOKEN-TYPE-LOCK-UNLOCK
                (unwrap-panic (to-consensus-buff? {
                    operator: none,
                    token-address: (contract-of token)
                }))
                token-manager
                verification-params)
    ))


;; Deploys a canonical interchain token on a remote chain.
;; @param token The address of the original token on the original chain.
;; @param destination-chain The name of the chain where the token will be deployed.
;; @param gas-value The gas amount to be sent for deployment.
;; #[allow(unchecked_data)]
(define-public (deploy-remote-canonical-interchain-token
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (token <sip-010-trait>)
        (destination-chain (string-ascii 19))
        (gas-value uint)
        (caller principal))
    (let
        (
            (deploy-salt (get-canonical-interchain-token-deploy-salt (contract-of token)))
            (token-id (unwrap! (get-canonical-interchain-token-id its-impl (contract-of token)) ERR-SERVICE-NOT-DEPLOYED))
            ;; This ensures that the token manager has been deployed by this address, so it's safe to trust it.
            (minter NULL-BYTES)
        )
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (try! (contract-call? .interchain-token-service valid-token-address its-impl token-id))
        (deploy-remote-interchain-token-inner
            gateway-impl
            gas-service-impl
            its-impl
            deploy-salt
            minter
            destination-chain
            gas-value
            token)
    )
)

(define-public (deploy-interchain-token
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
        })
        (caller principal))
    (let
        (
            (deploy-salt (get-interchain-token-deploy-salt caller salt_))
        )
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (is-eq (get deployer
            (try! (decode-contract-principal (contract-of token)))
        ) caller) ERR-NOT-TOKEN-DEPLOYER)
        (asserts! (not (is-eq (contract-call? .interchain-token-service-storage get-service-impl) minter)) ERR-INVALID-MINTER)
    (contract-call? .interchain-token-service deploy-interchain-token
        gateway-impl
        gas-service-impl
        its-impl
        deploy-salt
        token
        initial-supply
        (some
        minter)
        verification-params)))

;; This will only be a risk if the user deploying the token remotely
;; is deploying an existing malicious token on stacks
;; basically getting themself rekt
;; #[allow(unchecked_data)]
(define-public (deploy-remote-interchain-token
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (salt_ (buff 32))
        (minter_ principal)
        (destination-chain (string-ascii 19))
        (gas-value uint)
        (token <sip-010-trait>)
        (token-manager <token-manager-trait>)
        (caller principal))
    (deploy-remote-interchain-token-with-minter
        gateway-impl
        gas-service-impl
        its-impl
        salt_
        minter_
        destination-chain
        none
        gas-value
        token
        token-manager
        caller)
)

;; Deploys a remote interchain token on a specified destination chain.
;; @param salt_ The unique salt for deploying the token.
;; @param minter_ The address to receive the minter and operator role of the token, in addition to ITS. If the address is `address(0)`,
;; no additional minter is set on the token. Reverts if the minter does not have mint permission for the token.
;; @param destination-chain The name of the destination chain.
;; @param destination-minter The minter address to set on the deployed token on the destination chain. This can be arbitrary bytes
;; @param gas-value The amount of gas to send for the deployment.
;; @param token The token to deploy.
;; @param token-manager The token manager to use for the token.
(define-public (deploy-remote-interchain-token-with-minter
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
        (caller principal)
)
    (let (
        (proxy-check (asserts! (is-proxy) ERR-NOT-PROXY))
        (deploy-salt (get-interchain-token-deploy-salt caller salt_))
        ;; #[allow(unchecked_data)]
        (deployed-token-id (unwrap-panic (get-interchain-token-id-raw its-impl deploy-salt)))
        (minter
            (if
                (not (is-eq NULL-ADDRESS minter_))
                (begin
                    ;; #[filter(token-manager)]
                    (try! (check-token-minter token-manager deployed-token-id minter_))
                    (asserts! (not (is-eq minter_ (contract-of its-impl))) ERR-INVALID-MINTER)
                        (match
                            destination-minter
                            destination-minter-unpacked
                            (begin
                                (try! (use-deploy-approval {
                                        minter: minter_,
                                        token-id: deployed-token-id,
                                        destination-chain: destination-chain,
                                    } destination-minter-unpacked))
                                destination-minter-unpacked
                            )
                            (unwrap! (as-max-len? (unwrap-panic (to-consensus-buff? minter_)) u128) ERR-INVALID-MINTER)))
                (begin
                    (asserts! (is-none destination-minter) ERR-INVALID-MINTER)
                    NULL-BYTES)
        ))
    )
        (deploy-remote-interchain-token-inner
            ;; #[allow(unchecked_data)]
            gateway-impl
            ;; #[allow(unchecked_data)]
            gas-service-impl
            its-impl
            ;; #[allow(unchecked_data)]
            deploy-salt
            minter
            ;; #[allow(unchecked_data)]
            destination-chain
            ;; #[allow(unchecked_data)]
            gas-value
            ;; #[allow(unchecked_data)]
            token
        )))

(define-constant PREFIX-DEPLOY-APPROVAL (keccak256 (unwrap-panic (to-consensus-buff? "deploy-approval"))))

;; Compute the key for the deploy approval mapping.
(define-private (get-deploy-approval-key (approval {
        minter: principal,
        token-id: (buff 32),
        destination-chain: (string-ascii 19),
    }))
    (keccak256 (concat PREFIX-DEPLOY-APPROVAL
        (unwrap-panic (to-consensus-buff? approval))
    )))


;; Allow the minter to approve the deployer for a remote interchain token deployment that uses a custom destinationMinter address.
;; This ensures that a token deployer can't choose the destinationMinter itself, and requires the approval of the minter to reduce trust assumptions on the deployer.
;; @param its-impl The implementation of the InterChainTokenService contract.
;; @param deployer The address of the deployer.
;; @param salt_ The unique salt for deploying the token.
;; @param destination-chain The name of the destination chain.
;; @param destination-minter The minter address to set on the deployed token on the destination chain. This can be arbitrary bytes
;; @param token The token to deploy.
;; @param caller the proxy contract-caller passed from the proxy
;; since the encoding of the account is dependent on the destination chain.
(define-public (approve-deploy-remote-interchain-token
    (its-impl <its-trait>)
    (deployer principal)
    (salt_ (buff 32))
    (destination-chain (string-ascii 19))
    (destination-minter (buff 128))
    (token <native-interchain-token-trait>)
    (caller principal)
)
    (let (
        (proxy-check (asserts! (is-proxy) ERR-NOT-PROXY))
        (minter caller)
        ;; #[allow(unchecked_data)]
        (token-id (unwrap-panic (get-interchain-token-id its-impl deployer salt_)))
        (token-info (unwrap! (contract-call? .interchain-token-service-storage get-token-info token-id) ERR-TOKEN-NOT-FOUND))
        (approval {
            minter: minter,
            token-id: token-id,
            destination-chain: destination-chain,
        })
        (approval-key (get-deploy-approval-key approval))
    )
    (try! (require-not-paused))
    (asserts! (is-eq (contract-of token) (get manager-address token-info)) ERR-TOKEN-MISMATCH)
    ;; #[filter(minter)]
    (try! (check-token-minter token token-id minter))
    (asserts! (contract-call? .interchain-token-service-storage is-trusted-chain destination-chain) ERR-INVALID-CHAIN-NAME)
    (try! (contract-call? .interchain-token-service-storage emit-deploy-remote-interchain-token-approval
        minter
        deployer
        token-id
        destination-chain
        destination-minter
    ))
    (contract-call? .interchain-token-service-storage set-approved-destination-minter approval-key (keccak256 destination-minter))))


;; Allows the minter to revoke a deployer's approval for a remote interchain token deployment that uses a custom destination-minter address.
;; @param its-impl The implementation of the InterChainTokenService contract.
;; @param deployer The address of the deployer.
;; @param salt_ The unique salt for deploying the token.
;; @param destination-chain The name of the destination chain.
;; @param caller the contract-caller passed from the proxy
(define-public (revoke-deploy-remote-interchain-token
        (its-impl <its-trait>)
        (deployer principal)
        (salt_ (buff 32))
        (destination-chain (string-ascii 19))
        (caller principal))
    (let (
        (proxy-check (asserts! (is-proxy) ERR-NOT-PROXY))
        (minter caller)
        ;; #[allow(unchecked_data)]
        (token-id (unwrap-panic (get-interchain-token-id its-impl deployer salt_)))
        (approval {
            minter: minter,
            token-id: token-id,
            destination-chain: destination-chain,
        })
        (key (get-deploy-approval-key approval))
        (is-deleted (unwrap! (contract-call? .interchain-token-service-storage remove-approved-destination-minter key) ERR-CANNOT-DELETE-DEPLOY-APPROVAL))
    )
        (try! (require-not-paused))
        (asserts! is-deleted ERR-REMOTE-DEPLOYMENT-NOT-APPROVED)
        (try! (contract-call? .interchain-token-service-storage emit-revoked-deploy-remote-interchain-token-approval
            minter
            deployer
            token-id
            destination-chain))
        (ok true)))

;; Use the deploy approval to check that the destination minter is valid and then delete the approval.
(define-private (use-deploy-approval
    (approval {
        minter: principal,
        token-id: (buff 32),
        destination-chain: (string-ascii 19),
    })
    (destination-minter (buff 128))
)
    (let (
        (key (get-deploy-approval-key approval))
        (destination-minter-hash (keccak256 destination-minter))
        (approved-minter-hash (default-to NULL-BYTES (contract-call? .interchain-token-service-storage get-approved-destination-minter key)))
    )
    (asserts!
        (is-eq approved-minter-hash destination-minter-hash)
    ERR-REMOTE-DEPLOYMENT-NOT-APPROVED)
    (contract-call? .interchain-token-service-storage remove-approved-destination-minter key)))

;; Checks that the minter is registered for the token on the current chain and not the ITS address.
;; @param token-manager The token manager to check. The token must be an interchain token deployed via ITS.
;; @param token-id The unique identifier for the token.
;; @param minter The address to be checked as a minter for the interchain token.
(define-private (check-token-minter
    (token-manager <token-manager-trait>)
    (token-id (buff 32))
    (minter principal))
    (let (
            (token-info (unwrap! (contract-call? .interchain-token-service-storage get-token-info token-id) ERR-TOKEN-NOT-FOUND))
            (manager (get manager-address token-info))
            (current-its-impl (contract-call? .interchain-token-service-storage get-service-impl))
    )
        (asserts! (is-eq manager (contract-of token-manager)) ERR-TOKEN-MISMATCH)
        (asserts! (unwrap! (contract-call? token-manager is-minter minter) ERR-TOKEN-NOT-DEPLOYED) ERR-NOT-MINTER)
        (asserts! (not (is-eq current-its-impl minter)) ERR-INVALID-MINTER)
        (asserts! (not (is-eq .interchain-token-service minter)) ERR-INVALID-MINTER)
        (ok true)))


;; Deploys a remote interchain token on a specified destination chain.
;; @param gateway-impl The address of the current gateway contract implementation
;; @param gas-service-impl The address of the current gas service contract implementation
;; @param its-impl The address of the current Interchain Token Service contract implementation
;; @param deploy-salt The salt used for the deployment.
;; @param minter The address to receive the minter and operator role of the token, in addition to ITS.
;; @param destination-chain The name of the destination chain.
;; @param gas-value The amount of gas to send for the deployment.
;; @param token The token to deploy remotely.
(define-private (deploy-remote-interchain-token-inner
        ;; #[allow(unchecked_params)]
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (its-impl <its-trait>)
        (deploy-salt (buff 32))
        (minter (buff 128))
        (destination-chain (string-ascii 19))
        (gas-value uint)
        (token <sip-010-trait>))
        (let (
            (proxy-check (asserts! (is-proxy) ERR-NOT-PROXY))
            ;; #[allow(unchecked_data)]
            (name (unwrap! (contract-call? token get-name) ERR-TOKEN-NOT-DEPLOYED))
            ;; #[allow(unchecked_data)]
            (symbol (unwrap! (contract-call? token get-symbol) ERR-TOKEN-NOT-DEPLOYED))
            ;; #[allow(unchecked_data)]
            (decimals (unwrap!  (contract-call? token get-decimals) ERR-TOKEN-NOT-DEPLOYED))
    )
        (contract-call? .interchain-token-service deploy-remote-interchain-token
            gateway-impl
            gas-service-impl
            its-impl
            deploy-salt
            destination-chain
            name
            symbol
            decimals
            minter
            gas-value
        )))

;; #########################
;; #########################
;; #### Dynamic Dispatch ###
;; #########################
;; #########################

(define-public (dispatch (fn (string-ascii 32)) (data (buff 65000)) (caller principal))
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        ERR-NOT-IMPLEMENTED
    )
)


(define-read-only (get-is-paused)
    (contract-call? .interchain-token-service-storage get-is-paused))

(define-private (require-not-paused)
    (ok (asserts! (not (unwrap-panic (get-is-paused))) ERR-PAUSED)))
