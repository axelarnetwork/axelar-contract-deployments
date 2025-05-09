
;; title: interchain-token-service
;; version:
;; summary:
;; description:
(use-trait interchain-token-service-trait .traits.interchain-token-service-trait)
(use-trait gas-service-trait .traits.gas-service-impl-trait)
(impl-trait .traits.interchain-token-service-trait)

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
(define-constant PROXY .interchain-token-service)

(define-private (is-proxy) (is-eq contract-caller PROXY))

(define-constant ERR-NOT-AUTHORIZED (err u120001))
(define-constant ERR-PAUSED (err u120002))
(define-constant ERR-NOT-PROXY (err u120003))
(define-constant ERR-UNTRUSTED-CHAIN (err u120004))
(define-constant ERR-TOKEN-NOT-FOUND (err u120005))
(define-constant ERR-TOKEN-EXISTS (err u120006))
(define-constant ERR-TOKEN-NOT-DEPLOYED (err u120007))
(define-constant ERR-TOKEN-MANAGER-NOT-DEPLOYED (err u120008))
(define-constant ERR-TOKEN-MANAGER-MISMATCH (err u120009))
(define-constant ERR-UNSUPPORTED-TOKEN-TYPE (err u120010))
(define-constant ERR-INVALID-PAYLOAD (err u120011))
(define-constant ERR-INVALID-DESTINATION-CHAIN (err u120012))
(define-constant ERR-INVALID-SOURCE-CHAIN (err u120013))
(define-constant ERR-INVALID-SOURCE-ADDRESS (err u120014))
(define-constant ERR-ZERO-AMOUNT (err u120015))
(define-constant ERR-INVALID-METADATA-VERSION (err u120016))
(define-constant ERR-INVALID-SALT (err u120017))
(define-constant ERR-INVALID-DESTINATION-ADDRESS (err u120018))
(define-constant ERR-EMPTY-DATA (err u120019))
(define-constant ERR-TOKEN-DEPLOYMENT-NOT-APPROVED (err u120020))
(define-constant ERR-INVALID-MESSAGE-TYPE (err u120021))
(define-constant ERR-CANNOT-DEPLOY-REMOTELY-TO-SELF (err u120022))
(define-constant ERR-NOT-REMOTE-SERVICE (err u120023))
(define-constant ERR-TOKEN-METADATA-NAME-INVALID (err u120024))
(define-constant ERR-TOKEN-METADATA-SYMBOL-INVALID (err u120025))
(define-constant ERR-TOKEN-METADATA-DECIMALS-INVALID (err u120026))
(define-constant ERR-TOKEN-METADATA-OPERATOR-INVALID (err u120027))
(define-constant ERR-TOKEN-METADATA-OPERATOR-ITS-INVALID (err u120028))
(define-constant ERR-TOKEN-METADATA-FLOW-LIMITER-ITS-INVALID (err u120029))
(define-constant ERR-TOKEN-METADATA-MINTER-ITS-INVALID (err u120030))
(define-constant ERR-TOKEN-METADATA-TOKEN-ID-INVALID (err u120031))
(define-constant ERR-TOKEN-METADATA-SUPPLY-INVALID (err u120032))
(define-constant ERR-TOKEN-METADATA-PASSED-MINTER-INVALID (err u120033))
(define-constant ERR-TOKEN-METADATA-PASSED-MINTER-NOT-NULL (err u120034))
(define-constant ERR-INVALID-PARAMS (err u120035))
(define-constant ERR-GATEWAY-NOT-DEPLOYED (err u120036))
(define-constant ERR-NOT-TOKEN-DEPLOYER (err u120037))
(define-constant ERR-NOT-IMPLEMENTED (err u120038))
(define-constant ERR-ONLY-OPERATOR (err u120039))
(define-constant ERR-ONLY-OWNER (err u120040))
(define-constant ERR-NOT-STARTED (err u120041))
(define-constant ERR-INVALID-MINTER (err u120042))


;; This type is reserved for interchain tokens deployed by ITS, and can't be used by custom token managers.
;; same as mint burn in functionality will be custom tokens made by us
;; that are deployed outside of the contracts but registered by the ITS contract
(define-constant TOKEN-TYPE-NATIVE-INTERCHAIN-TOKEN u0)
;; The token will be locked/unlocked at the token manager.
(define-constant TOKEN-TYPE-LOCK-UNLOCK u2)

(define-constant CHAIN-NAME "stacks")
(define-constant CHAIN-NAME-HASH (keccak256 (unwrap-panic (to-consensus-buff? CHAIN-NAME))))
;; (define-constant CONTRACT-ID (keccak256 (unwrap-panic (to-consensus-buff? "interchain-token-service"))))
(define-constant PREFIX-INTERCHAIN-TOKEN-ID (keccak256 (unwrap-panic (to-consensus-buff? "its-interchain-token-id"))))


(define-constant METADATA-VERSION {
    contract-call: u0,
    express-call: u1
})

(define-constant LATEST-METADATA-VERSION u1)

(define-constant EMPTY-32-BYTES 0x0000000000000000000000000000000000000000000000000000000000000000)


(define-constant CA (as-contract tx-sender))

;; @dev Special identifier that the trusted address for a chain should be set to, which indicates if the ITS call
;; for that chain should be routed via the ITS hub.
(define-constant ITS-HUB-ROUTING-IDENTIFIER "hub")

(define-constant MESSAGE-TYPE-INTERCHAIN-TRANSFER u0)
(define-constant MESSAGE-TYPE-DEPLOY-INTERCHAIN-TOKEN u1)
(define-constant MESSAGE-TYPE-DEPLOY-TOKEN-MANAGER u2)
(define-constant NULL-ADDRESS (unwrap-panic (principal-construct? (if (is-eq chain-id u1) 0x16 0x1a) 0x0000000000000000000000000000000000000000)))


(define-read-only (get-token-info (token-id (buff 32)))
    (contract-call? .interchain-token-service-storage get-token-info token-id))

(define-private (insert-token-manager (token-id (buff 32)) (manager-address principal) (token-type uint))
    (contract-call? .interchain-token-service-storage insert-token-manager token-id manager-address token-type))

(define-public (set-paused (status bool) (caller principal))
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (asserts! (is-eq caller (get-owner)) ERR-NOT-AUTHORIZED)
        (contract-call? .interchain-token-service-storage set-paused status)))

(define-read-only (get-is-paused)
    (contract-call? .interchain-token-service-storage get-is-paused))

(define-private (require-not-paused)
    (ok (asserts! (not (unwrap-panic (get-is-paused))) ERR-PAUSED)))

(define-read-only (get-its-hub-chain)
    (contract-call? .interchain-token-service-storage get-its-hub-chain))

(define-read-only (get-token-factory-impl)
    (contract-call? .interchain-token-service-storage get-token-factory-impl))

(define-read-only (get-its-contract-name)
    (contract-call? .interchain-token-service-storage get-its-contract-name))

(define-read-only (is-valid-token-type (token-type uint))
    (is-eq token-type TOKEN-TYPE-LOCK-UNLOCK))

;;  Calculates the token-id that would correspond to a link for a given deployer with a specified salt.
;;  @param sender The address of the TokenManager deployer.
;;  @param salt The salt that the deployer uses for the deployment.
;;  @return (buff 32) The token-id that the custom TokenManager would get (or has gotten).
(define-read-only (interchain-token-id-raw (sender principal) (salt (buff 32)))
    (keccak256 (concat
        (concat PREFIX-INTERCHAIN-TOKEN-ID (unwrap-panic (to-consensus-buff? sender)))
    salt)))

(define-read-only (interchain-token-id (sender principal) (salt (buff 32)))
    (ok (interchain-token-id-raw sender salt)))

;; ####################
;; ####################
;; ### Operatorship ###
;; ####################
;; ####################


(define-read-only (get-operator) (contract-call? .interchain-token-service-storage get-operator))
(define-read-only (is-operator (address principal))
    (ok (is-eq address (get-operator))))

;; Transfers operatorship to a new account
(define-public (transfer-operatorship (new-operator principal) (caller principal))
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (require-not-paused))
        (asserts! (is-eq caller (get-operator)) ERR-ONLY-OPERATOR)
        ;; #[allow(unchecked_data)]
        (try! (contract-call? .interchain-token-service-storage emit-transfer-operatorship new-operator))
        (contract-call? .interchain-token-service-storage set-operator new-operator)
    )
)

(define-read-only (get-owner) (contract-call? .interchain-token-service-storage get-owner))

(define-public (transfer-ownership (new-owner principal) (caller principal))
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (require-not-paused))
        (asserts! (is-eq caller (get-owner)) ERR-ONLY-OWNER)
        ;; #[allow(unchecked_data)]
        (try! (contract-call? .interchain-token-service-storage emit-transfer-ownership new-owner))
        (contract-call? .interchain-token-service-storage set-owner new-owner)
    )
)

;; ####################
;; ####################
;; ### address tracking ###
;; ####################
;; ####################

;; Gets the name of the chain this is deployed at
(define-read-only (get-chain-name)
    (ok CHAIN-NAME))


;; Gets the trusted address at a remote chain
;; @param chain Chain name of the remote chain
;; @return The trusted address for the chain. Returns none if the chain is untrusted
(define-read-only (get-trusted-address (chain (string-ascii 19)))
    (contract-call? .interchain-token-service-storage get-trusted-address chain))
;; Checks whether the interchain sender is a trusted address
;; @param chain Chain name of the sender
;; @param address Address of the sender
;; @return bool true if the sender chain/address are trusted, false otherwise
(define-read-only (is-trusted-address (chain-name (string-ascii 19)) (address (string-ascii 128)))
    (is-eq address (default-to "" (get-trusted-address chain-name))))

;; Sets the trusted address and its hash for a remote chain
;; @param chain Chain name of the remote chain
;; @param address the string representation of the trusted address
(define-public (set-trusted-address (chain-name (string-ascii 19)) (address (string-ascii 128)) (caller principal))
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (asserts!  (is-eq caller (get-owner)) ERR-NOT-AUTHORIZED)
        (asserts!
            (or
                (is-eq (get-its-hub-chain) chain-name)
                (is-eq address ITS-HUB-ROUTING-IDENTIFIER)
                ) ERR-INVALID-DESTINATION-ADDRESS)
        (try! (contract-call? .interchain-token-service-storage emit-trusted-address-set chain-name address))
        (contract-call? .interchain-token-service-storage set-trusted-address chain-name address)))

;; Remove the trusted address of the chain.
;; @param chain Chain name that should be made untrusted
(define-public (remove-trusted-address  (chain-name (string-ascii 19)) (caller principal))
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (asserts!  (is-eq caller (get-owner)) ERR-NOT-AUTHORIZED)
        (try! (contract-call? .interchain-token-service-storage emit-trusted-address-removed chain-name))
        (contract-call? .interchain-token-service-storage remove-trusted-address chain-name)))

;; Check if the chain is trusted
;; @param chain Chain name that should be checked for trust
;; @return true, if the chain is trusted
(define-read-only (is-trusted-chain (chain (string-ascii 19)))
    (contract-call? .interchain-token-service-storage is-trusted-chain chain))



;; Used to deploy local and remote custom TokenManagers.
;; @dev At least the `gas-value` amount of native token must be passed to the function call. `gas-value` exists because validators
;; would check the contract code for validity before deployment locally
;; @param gateway-impl The implementation of the gateway contract
;; @param gas-service-impl The implementation of the GasService contract
;; @param salt The salt to be used during deployment.
;; @param destination-chain The name of the chain to deploy the TokenManager and standardized token to.
;; @param token-manager-type The type of token manager to be deployed. Cannot be NATIVE_INTERCHAIN_TOKEN.
;; @param params The params that will be used to initialize the TokenManager.
;; @param verification-params The params that will be used to verify the token manager local deployment
;; @param caller the contract caller passed by the proxy
(define-public (deploy-token-manager
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
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
        (caller principal)
    )
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (require-not-paused))
        (let (
                (deployer (if (is-eq caller (get-token-factory-impl)) NULL-ADDRESS caller))
                (token-id (interchain-token-id-raw deployer salt))
                (token-manager-address (contract-of token-manager))
                (contract-principal (try! (decode-contract-principal token-manager-address)))
                (managed-token (unwrap! (contract-call? token-manager get-token-address) ERR-TOKEN-MANAGER-NOT-DEPLOYED))
                (data (unwrap! (from-consensus-buff? {
                    operator: (optional principal),
                    token-address: principal
                } params) ERR-INVALID-PARAMS))
                (operator (default-to NULL-ADDRESS (get operator data)))
            )

            (asserts! (is-valid-token-type token-manager-type) ERR-UNSUPPORTED-TOKEN-TYPE)
            (asserts! (or
                (is-eq deployer NULL-ADDRESS)
                (is-eq (get deployer contract-principal) deployer)) ERR-NOT-TOKEN-DEPLOYER)
            (asserts! (is-eq u32 (len salt)) ERR-INVALID-SALT)
            (try! (contract-call? .interchain-token-service-storage emit-interchain-token-id-claimed token-id deployer salt))
            (asserts! (is-eq (len destination-chain) u0) ERR-INVALID-DESTINATION-CHAIN)
            (asserts! (is-none (get-token-info token-id)) ERR-TOKEN-EXISTS)
            (try! (contract-call? .verify-onchain verify-token-manager-deployment
                    (get nonce verification-params)
                    (get fee-rate verification-params)
                    (get signature verification-params)
                    (get contract-name contract-principal)
                    (get deployer contract-principal)
                    (get proof verification-params)
                    (get tx-block-height verification-params)
                    (get block-header-without-signer-signatures verification-params)))
            (asserts! (is-eq
                token-manager-type
                (unwrap! (contract-call? token-manager get-token-type) ERR-TOKEN-MANAGER-NOT-DEPLOYED)
            ) ERR-TOKEN-MANAGER-MISMATCH)
            (asserts! (is-eq
                managed-token
                (get token-address data)
            ) ERR-TOKEN-MANAGER-MISMATCH)
            (asserts! (unwrap!
                (contract-call? token-manager is-operator operator) ERR-TOKEN-NOT-DEPLOYED) ERR-TOKEN-METADATA-OPERATOR-INVALID)
            (asserts! (unwrap! (contract-call? token-manager is-operator CA) ERR-TOKEN-NOT-DEPLOYED) ERR-TOKEN-METADATA-OPERATOR-ITS-INVALID)
            (asserts! (unwrap! (contract-call? token-manager is-flow-limiter CA) ERR-TOKEN-NOT-DEPLOYED) ERR-TOKEN-METADATA-FLOW-LIMITER-ITS-INVALID)
            (asserts! (not (contract-call? .interchain-token-service-storage is-manager-address-used token-manager-address)) ERR-TOKEN-EXISTS)
            (asserts!
                (unwrap! (insert-token-manager token-id token-manager-address token-manager-type) ERR-NOT-AUTHORIZED)
            ERR-TOKEN-EXISTS)
            (contract-call? .interchain-token-service-storage emit-token-manager-deployed token-id token-manager-address token-manager-type))))



;; Deploys an interchain token on a destination chain.
;; @param gateway-impl the gateway implementation contract address.
;; @param gas-service-impl the gas service implementation contract address.
;; @param salt The salt to be used during deployment.
;; @param destination-chain the destination chain name.
;; @param name The name of the token.
;; @param symbol The symbol of the token.
;; @param decimals The number of decimals of the token.
;; @param minter The minter address for the token.
;; @param gas-value The amount of gas to be paid for the transaction.
;; @param caller the contract caller passed by the proxy
(define-public (deploy-remote-interchain-token
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (salt (buff 32))
        (destination-chain (string-ascii 19))
        (name (string-ascii 32))
        (symbol (string-ascii 32))
        (decimals uint)
        (minter (buff 128))
        (gas-value uint)
        (caller principal))
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (require-not-paused))
        (let (
            (deployer (if (is-eq caller (get-token-factory-impl)) NULL-ADDRESS caller))
            (token-id (interchain-token-id-raw deployer salt))
            (payload (unwrap-panic (to-consensus-buff? {
                type: MESSAGE-TYPE-DEPLOY-INTERCHAIN-TOKEN,
                token-id: token-id,
                name: name,
                symbol: symbol,
                decimals: decimals,
                minter: minter
            })))
            (token-info (unwrap! (get-token-info token-id) ERR-TOKEN-NOT-FOUND))
        )
        (asserts! (and
                (not (is-eq destination-chain CHAIN-NAME))
                (> (len destination-chain) u0))
            ERR-INVALID-DESTINATION-CHAIN)
        (try! (contract-call? .interchain-token-service-storage emit-interchain-token-deployment-started
            token-id
            destination-chain
            name
            symbol
            decimals
            minter))
        ;; #[filter(gateway-impl, gas-value)]
        (contract-call? .interchain-token-service its-hub-call-contract gateway-impl gas-service-impl destination-chain payload (get contract-call METADATA-VERSION) gas-value))))

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

(define-private (native-interchain-token-checks
    (token <native-interchain-token-trait>)
    (minter principal)
    (token-id (buff 32))
    (supply uint)
    (verification-params {
        nonce: (buff 8),
        fee-rate: (buff 8),
        signature: (buff 65),
        proof: { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint},
        tx-block-height: uint,
        block-header-without-signer-signatures: (buff 800),
    })
    (deployer principal)
)
    (let (
            (token-address (contract-of token))
            (contract-principal (try! (decode-contract-principal token-address)))

    )
        (asserts! (or
            (is-eq deployer NULL-ADDRESS)
            (is-eq (get deployer contract-principal) deployer)) ERR-NOT-TOKEN-DEPLOYER)
        (try! (contract-call? .verify-onchain verify-nit-deployment
            (get nonce verification-params)
            (get fee-rate verification-params)
            (get signature verification-params)
            (get contract-name contract-principal)
            (get deployer contract-principal)
            (get proof verification-params)
            (get tx-block-height verification-params)
            (get block-header-without-signer-signatures verification-params)))
        (asserts! (unwrap!
            (contract-call? token is-operator minter) ERR-TOKEN-NOT-DEPLOYED) ERR-TOKEN-METADATA-OPERATOR-INVALID)
        (asserts! (unwrap! (contract-call? token is-operator CA) ERR-TOKEN-NOT-DEPLOYED) ERR-TOKEN-METADATA-OPERATOR-ITS-INVALID)
        (asserts! (unwrap! (contract-call? token is-flow-limiter CA) ERR-TOKEN-NOT-DEPLOYED) ERR-TOKEN-METADATA-FLOW-LIMITER-ITS-INVALID)
        (asserts! (unwrap! (contract-call? token is-minter CA) ERR-TOKEN-NOT-DEPLOYED) ERR-TOKEN-METADATA-MINTER-ITS-INVALID)
        (asserts! (unwrap! (contract-call? token is-minter minter) ERR-TOKEN-NOT-DEPLOYED) ERR-TOKEN-METADATA-PASSED-MINTER-INVALID)
        (asserts! (is-eq
            TOKEN-TYPE-NATIVE-INTERCHAIN-TOKEN
            (unwrap! (contract-call? token get-token-type) ERR-TOKEN-NOT-DEPLOYED)
        ) ERR-UNSUPPORTED-TOKEN-TYPE)
        (asserts! (is-eq
            token-id
            (unwrap! (contract-call? token get-token-id) ERR-TOKEN-NOT-DEPLOYED)
        ) ERR-TOKEN-METADATA-TOKEN-ID-INVALID)
        (asserts! (is-eq
            supply
            (unwrap! (contract-call? token get-total-supply) ERR-TOKEN-NOT-DEPLOYED)
        ) ERR-TOKEN-METADATA-SUPPLY-INVALID)
        (ok true))
)
;; Used to deploy a native interchain token on stacks
;; @dev At least the `gas-value` amount of native token must be passed to the function call. `gas-value` exists because
;; validators will need to verify the contract code and parameters
;; If minter is none, no additional minter is set on the token, only ITS is allowed to mint.
;; @param gateway-impl the gateway implementation contract address.
;; @param gas-service-impl the gas service implementation contract address.
;; @param salt The salt to be used during deployment.
;; @param token the deployed native interchain token contract address
;; @param supply The already minted supply of the deployed token.
;; @param minter The address that will be able to mint and burn the deployed token.
;; @param verification-params The verification parameters for the deployed token.
;; @param caller the contract caller passed by the proxy
(define-public (deploy-interchain-token
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
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
        })
        (caller principal))
    (let (
            (deployer (if (is-eq caller (get-token-factory-impl)) NULL-ADDRESS caller))
            (token-id (interchain-token-id-raw deployer salt))
            (token-address (contract-of token))
            (minter-unpacked (default-to NULL-ADDRESS minter)))
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (require-not-paused))
        (asserts! (not (contract-call? .interchain-token-service-storage is-manager-address-used token-address)) ERR-TOKEN-EXISTS)
        (asserts! (is-none (get-token-info token-id)) ERR-TOKEN-EXISTS)
        (try! (contract-call? .interchain-token-service-storage emit-interchain-token-id-claimed token-id deployer salt))
        ;; #[filter(verification-params, minter-unpacked, supply)]
        (try! (native-interchain-token-checks token minter-unpacked token-id supply verification-params deployer))
        (asserts!
            (unwrap! (insert-token-manager token-id token-address TOKEN-TYPE-NATIVE-INTERCHAIN-TOKEN) ERR-NOT-AUTHORIZED)
            ERR-TOKEN-EXISTS)
        (try! (contract-call? .interchain-token-service-storage emit-token-manager-deployed
            token-id
            token-address
            TOKEN-TYPE-NATIVE-INTERCHAIN-TOKEN))
        (ok true)))

(define-public (execute-deploy-interchain-token
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
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
        })
        (caller principal))
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (require-not-paused))
        (asserts! (or
            (and
                (is-eq source-chain CHAIN-NAME)
                (is-eq source-address (get-its-contract-name)))
        (is-trusted-address source-chain source-address)) ERR-NOT-REMOTE-SERVICE)
        (let (
            (payload-decoded (unwrap! (from-consensus-buff? {
                type: uint,
                source-chain: (string-ascii 19),
                token-id: (buff 32),
                name: (string-ascii 32),
                symbol: (string-ascii 32),
                decimals: uint,
                minter-bytes: (buff 20),
            } payload) ERR-INVALID-PAYLOAD))
            (token-address (contract-of token))
            (contract-principal (try! (decode-contract-principal token-address)))
            (token-id (get token-id payload-decoded))
            (wrapped-source-chain (get source-chain payload-decoded))
            (minter-bytes (get minter-bytes payload-decoded))
            (minter (if (is-eq (len minter-bytes) u20) (unwrap! (principal-construct? (if (is-eq chain-id u1) 0x16 0x1a) minter-bytes) ERR-INVALID-MINTER) NULL-ADDRESS))
        )
        (asserts! (not (is-eq wrapped-source-chain (get-its-hub-chain))) ERR-UNTRUSTED-CHAIN)
        (asserts! (is-trusted-chain wrapped-source-chain) ERR-UNTRUSTED-CHAIN)
        (asserts! (is-trusted-chain source-chain) ERR-UNTRUSTED-CHAIN)
        (asserts! (is-trusted-address source-chain source-address) ERR-NOT-REMOTE-SERVICE)
        (asserts! (is-trusted-address wrapped-source-chain ITS-HUB-ROUTING-IDENTIFIER) ERR-NOT-REMOTE-SERVICE)
        (asserts! (is-eq MESSAGE-TYPE-DEPLOY-INTERCHAIN-TOKEN (get type payload-decoded)) ERR-INVALID-MESSAGE-TYPE)
        ;; #[filter(verification-params, caller)]
        (try! (native-interchain-token-checks token minter token-id u0 verification-params caller))
        (asserts! (not (contract-call? .interchain-token-service-storage is-manager-address-used token-address)) ERR-TOKEN-EXISTS)
        (asserts!
            (unwrap! (insert-token-manager token-id token-address TOKEN-TYPE-NATIVE-INTERCHAIN-TOKEN) ERR-NOT-AUTHORIZED)
            ERR-TOKEN-EXISTS)
        (try! (contract-call? .interchain-token-service-storage emit-token-manager-deployed
            token-id
            token-address
            TOKEN-TYPE-NATIVE-INTERCHAIN-TOKEN))
        (try! (as-contract (contract-call? .interchain-token-service gateway-validate-message
            gateway-impl
            source-chain
            message-id
            source-address
            (keccak256 payload))))
        (ok true))))

(define-read-only (valid-token-address (token-id (buff 32)))
    (ok (unwrap! (get-token-info token-id) ERR-TOKEN-NOT-FOUND)))


;; Initiates an interchain transfer of a specified token to a destination chain.
;; @dev The function retrieves the TokenManager associated with the token-id.
;; @param gateway-impl the gateway implementation contract address.
;; @param gas-service-impl the gas service implementation contract address.
;; @param token-manager the token manager contract address.
;; @param token the token contract address
;; @param token-id The unique identifier of the token to be transferred.
;; @param destination-chain The destination chain to send the tokens to.
;; @param destination-address The address on the destination chain to send the tokens to.
;; @param amount The amount of tokens to be transferred.
;; @param metadata Optional metadata for the call for additional effects (such as calling a destination contract).
;; @param gas-value The amount of native tokens to be used to pay for gas for the remote transfer.
;; @param caller the contract caller passed by the proxy
(define-public (interchain-transfer
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
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
        (caller principal)
    )
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (require-not-paused))
        ;; #[filter(token-manager,token,token-id,destination-chain,destination-address,amount,metadata,gas-value)]
        (try! (check-interchain-transfer-params token-manager token token-id destination-chain destination-address amount metadata gas-value))
        (try! (contract-call? token-manager take-token token caller amount))
        ;; Proxy is trusted to always pass the correct data
        ;; #[allow(unchecked_data)]
        (transmit-interchain-transfer
            gateway-impl
            gas-service-impl
            token-id
            caller
            destination-chain
            destination-address
            amount
            (get version metadata)
            (get data metadata)
            gas-value)))


;; Initiates an interchain call contract with interchain token to a destination chain.
;; @param gateway-impl the gateway implementation contract address.
;; @param gas-service-impl the gas service implementation contract address.
;; @param token-manager the token manager contract address.
;; @param token the token contract address
;; @param token-id The unique identifier of the token to be transferred.
;; @param destination-chain The destination chain to send the tokens to.
;; @param destination-address The address on the destination chain to send the tokens to.
;; @param amount The amount of tokens to be transferred.
;; @param metadata Additional data to be passed along with the transfer.
;; @param gas-value The amount of native tokens to be used to pay for gas for the remote transfer.
;; @param caller the contract caller passed by the proxy
(define-public (call-contract-with-interchain-token
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
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
        (caller principal))
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (require-not-paused))
        ;; #[filter(token-manager,token,token-id,destination-chain,destination-address,amount,metadata,gas-value)]
        (try! (check-interchain-transfer-params token-manager token token-id destination-chain destination-address amount metadata gas-value))
        (asserts! (> (len (get data metadata)) u0) ERR-EMPTY-DATA)
        (try! (contract-call? token-manager take-token token caller amount))
        ;; Caller is trusted since it will always be passed in the proxy and the gateway impl cannot access the storage
        ;; without the proxy upgrading to use a new impl
        ;; #[allow(unchecked_data)]
        (transmit-interchain-transfer
            gateway-impl
            gas-service-impl
            token-id
            caller
            destination-chain
            destination-address
            amount
            (get contract-call METADATA-VERSION)
            (get data metadata)
            gas-value)))

(define-private (check-interchain-transfer-params
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
    (let (
        (token-info (unwrap! (get-token-info token-id) ERR-TOKEN-NOT-FOUND))
    )
        (asserts! (> amount u0) ERR-ZERO-AMOUNT)
        (asserts! (is-eq (contract-of token-manager) (get manager-address token-info)) ERR-TOKEN-MANAGER-MISMATCH)
        (asserts! (<= (get version metadata) LATEST-METADATA-VERSION) ERR-INVALID-METADATA-VERSION)
        (asserts! (> gas-value u0) ERR-ZERO-AMOUNT)
        (asserts! (> (len destination-chain) u0) ERR-INVALID-DESTINATION-CHAIN)
        (asserts! (> (len destination-address) u0) ERR-INVALID-DESTINATION-ADDRESS)
        (ok true)))

;; Transmit a callContractWithInterchainToken for the given token-id.
;; @param gateway-impl The gateway implementation.
;; @param gas-service-impl the gas service implementation contract address.
;; @param token-id The token-id of the TokenManager (which must be the msg.sender).
;; @param source-address The address where the token is coming from, which will also be used for gas reimbursement.
;; @param destination-chain The name of the chain to send tokens to.
;; @param destination-address The destination-address for the interchain-transfer.
;; @param amount The amount of tokens to send.
;; @param metadata-version The version of the metadata.
;; @param data The data to be passed with the token transfer.
;; @param gas-value The amount of native tokens to be used to pay for gas for the remote transfer.
(define-private (transmit-interchain-transfer
        (gateway-impl <gateway-trait>)
        (gas-service-impl <gas-service-trait>)
        (token-id (buff 32))
        (source-address principal)
        (destination-chain (string-ascii 19))
        (destination-address (buff 128))
        (amount uint)
        (metadata-version uint)
        (data (buff 62000))
        (gas-value uint))
    (let
        (
            (payload (unwrap-panic (to-consensus-buff? {
                type: MESSAGE-TYPE-INTERCHAIN-TRANSFER,
                token-id: token-id,
                source-address: source-address,
                destination-address: destination-address,
                amount: amount,
                data: data
            })))
        )
        (asserts! (> amount u0) ERR-ZERO-AMOUNT)
        (try! (contract-call? .interchain-token-service-storage emit-interchain-transfer
            token-id
            source-address
            destination-chain
            destination-address
            amount
            (if (is-eq u0 (len data)) EMPTY-32-BYTES (keccak256 data))))
        (contract-call? .interchain-token-service its-hub-call-contract gateway-impl gas-service-impl destination-chain payload metadata-version gas-value)
    ))



(define-public (execute-receive-interchain-token
        (gateway-impl <gateway-trait>)
        (source-chain (string-ascii 19))
        (message-id (string-ascii 128))
        (source-address (string-ascii 128))
        (token-manager <token-manager-trait>)
        (token <sip-010-trait>)
        (payload (buff 64000))
        (destination-contract (optional <interchain-token-executable-trait>))
        (caller principal)
    )
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (require-not-paused))
        (let (
            (payload-decoded (unwrap! (from-consensus-buff? {
                type: uint,
                source-chain: (string-ascii 19),
                token-id: (buff 32),
                source-address: (buff 128),
                destination-address: (buff 128),
                amount: uint,
                data: (buff 63000),
            } payload) ERR-INVALID-PAYLOAD))
            (token-id (get token-id payload-decoded))
            (sender-address (get source-address payload-decoded))
            (recipient (unwrap-panic (from-consensus-buff? principal (get destination-address payload-decoded))))
            (amount (get amount payload-decoded))
            (data (get data payload-decoded))
            (token-info (unwrap! (get-token-info token-id) ERR-TOKEN-NOT-FOUND))
            (data-is-empty (is-eq (len data) u0))
            (wrapped-source-chain (get source-chain payload-decoded))
        )
        (asserts! (not (is-eq wrapped-source-chain (get-its-hub-chain))) ERR-UNTRUSTED-CHAIN)
        (asserts! (is-trusted-chain wrapped-source-chain) ERR-UNTRUSTED-CHAIN)
        (asserts! (is-trusted-chain source-chain) ERR-UNTRUSTED-CHAIN)
        (asserts! (is-trusted-address source-chain source-address) ERR-NOT-REMOTE-SERVICE)
        (asserts! (is-trusted-address wrapped-source-chain ITS-HUB-ROUTING-IDENTIFIER) ERR-NOT-REMOTE-SERVICE)
        (asserts! (is-eq (get manager-address token-info) (contract-of token-manager)) ERR-TOKEN-MANAGER-MISMATCH)
        (try! (as-contract
            (contract-call? .interchain-token-service gateway-validate-message gateway-impl source-chain message-id source-address (keccak256 payload))
        ))
        (try! (as-contract (contract-call? token-manager give-token token recipient amount)))
        (try! (contract-call? .interchain-token-service-storage emit-interchain-transfer-received
            token-id
            wrapped-source-chain
            sender-address
            recipient
            amount
            (if data-is-empty EMPTY-32-BYTES (keccak256 data))))
        (if data-is-empty
            (ok 0x)
            (let (
                (destination-contract-unwrapped (unwrap! destination-contract ERR-INVALID-DESTINATION-ADDRESS))
            )
                (asserts! (is-eq (contract-of destination-contract-unwrapped) recipient) ERR-INVALID-DESTINATION-ADDRESS)
                (as-contract
                    (contract-call? destination-contract-unwrapped execute-with-interchain-token
                        wrapped-source-chain message-id sender-address data token-id (contract-of token) amount)))))))


;; ######################
;; ######################
;; ### Initialization ###
;; ######################
;; ######################



(define-read-only (get-is-started)
    (contract-call? .interchain-token-service-storage get-is-started))


;; Constructor function
;; @returns (response true) or reverts


(define-public (set-flow-limit (token-id (buff 32)) (token-manager <token-manager-trait>) (limit uint) (caller principal))
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        (try! (require-not-paused))
        (asserts! (is-eq (get-operator) caller) ERR-ONLY-OPERATOR)
        (asserts! (is-eq
            (get manager-address (unwrap! (get-token-info token-id) ERR-TOKEN-NOT-FOUND))
            (contract-of token-manager)) ERR-TOKEN-MANAGER-MISMATCH)
        (as-contract (contract-call? token-manager set-flow-limit limit))))



;; #########################
;; #########################
;; #### Dynamic Dispatch ###
;; #########################
;; #########################

(define-public (dispatch (fn (string-ascii 32)) (data (buff 65000)) (caller principal))
    (begin
        (asserts! (is-proxy) ERR-NOT-PROXY)
        (asserts! (get-is-started) ERR-NOT-STARTED)
        ERR-NOT-IMPLEMENTED
    )
)
