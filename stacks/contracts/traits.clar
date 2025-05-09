(define-trait proxy-trait
  (
    (set-impl (principal) (response bool uint))
    (set-governance (principal) (response bool uint))
  )
)

(define-trait gateway-trait
	(
		(call-contract ((string-ascii 19) (string-ascii 128) (buff 64000) principal) (response bool uint))
        (approve-messages ((buff 4096) (buff 16384)) (response (list 10 (response bool uint)) uint))
        (validate-message ((string-ascii 19) (string-ascii 128) (string-ascii 128) (buff 32) principal) (response bool uint))
        (is-message-approved  ((string-ascii 19) (string-ascii 128) (string-ascii 128) principal (buff 32)) (response bool uint))
        (is-message-executed  ((string-ascii 19) (string-ascii 128)) (response bool uint))
        (rotate-signers ((buff 8192) (buff 16384)) (response bool uint))
        (transfer-operatorship (principal principal) (response bool uint))
        (dispatch ((string-ascii 32) (buff 65000)) (response bool uint))
	)
)

(define-trait interchain-token-executable-trait (
  ;; MUST check that the caller is the ITS and only the ITS in contracts that impl this trait
    (execute-with-interchain-token (
        ;; sourceChain,
        ;; messageId,
        ;; sourceAddress,
        ;; data,
        ;; tokenId,
        ;; tokenAddress,
        ;; amount
        (string-ascii 19)
        (string-ascii 128)
        (buff 128)
        (buff 64000)
        (buff 32)
        principal
        uint
        ;; must return keccak256('its-execute-success')
    ) (response (buff 32) uint))
))

(define-trait interchain-token-service-trait (
    (set-paused (bool principal) (response bool uint))
    (transfer-operatorship (principal principal) (response bool uint))
    (transfer-ownership (principal principal) (response bool uint))
    (set-trusted-address ((string-ascii 19) (string-ascii 128) principal) (response bool uint))
    (remove-trusted-address  ((string-ascii 19) principal) (response bool uint))
    (deploy-token-manager
        (<gateway-trait>
        <gas-service-impl-trait>
        (buff 32)
        (string-ascii 19)
        uint
        (buff 62000)
        <token-manager-trait>
        {
            nonce: (buff 8),
            fee-rate: (buff 8),
            signature: (buff 65),
            proof: { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint},
            tx-block-height: uint,
            block-header-without-signer-signatures: (buff 800),
        }
        principal)
    (response bool uint))
    (deploy-remote-interchain-token
        (<gateway-trait>
        <gas-service-impl-trait>
        (buff 32)
        (string-ascii 19)
        (string-ascii 32)
        (string-ascii 32)
        uint
        (buff 128)
        uint
        principal)
        (response bool uint))
    (deploy-interchain-token
        (<gateway-trait>
        <gas-service-impl-trait>
        (buff 32)
        <native-interchain-token-trait>
        uint
        (optional principal)
        {
            nonce: (buff 8),
            fee-rate: (buff 8),
            signature: (buff 65),
            proof: { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint},
            tx-block-height: uint,
            block-header-without-signer-signatures: (buff 800),
        }
        principal)
        (response bool uint))
    (interchain-transfer
        (<gateway-trait>
        <gas-service-impl-trait>
        <token-manager-trait>
        <sip-010-trait>
        (buff 32)
        (string-ascii 19)
        (buff 128)
        uint
        {
            version: uint,
            data: (buff 62000)
        }
        uint
        principal)
        (response bool uint)
    )
    (call-contract-with-interchain-token
        (<gateway-trait>
        <gas-service-impl-trait>
        <token-manager-trait>
        <sip-010-trait>
        (buff 32)
        (string-ascii 19)
        (buff 128)
        uint
        {
            version: uint,
            data: (buff 62000)
        }
        uint
        principal)
        (response bool uint))
    (execute-deploy-interchain-token
        (<gateway-trait>
        <gas-service-impl-trait>
        (string-ascii 19)
        (string-ascii 128)
        (string-ascii 128)
        <native-interchain-token-trait>
        (buff 62000)
        {
            nonce: (buff 8),
            fee-rate: (buff 8),
            signature: (buff 65),
            proof: { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint},
            tx-block-height: uint,
            block-header-without-signer-signatures: (buff 800),
        }
        principal)
        (response bool uint))
    (execute-receive-interchain-token
        (<gateway-trait>
        (string-ascii 19)
        (string-ascii 128)
        (string-ascii 128)
        <token-manager-trait>
        <sip-010-trait>
        (buff 64000)
        (optional <interchain-token-executable-trait>)
        principal)
        (response (buff 32) uint)
    )
    (set-flow-limit ((buff 32) <token-manager-trait> uint principal) (response bool uint))
    (valid-token-address ((buff 32)) (response {
        manager-address: principal,
        token-type: uint,
    } uint))
    (dispatch ((string-ascii 32) (buff 65000) principal) (response bool uint))
    (interchain-token-id (principal (buff 32)) (response (buff 32) uint))
))

(define-trait sip-010-trait
  (
    ;; Transfer from the caller to a new principal
    (transfer (uint principal principal (optional (buff 34))) (response bool uint))
    ;; the human readable name of the token
    (get-name () (response (string-ascii 32) uint))
    ;; the ticker symbol, or empty if none
    (get-symbol () (response (string-ascii 32) uint))
    ;; the number of decimals used, e.g. 6 would mean 1_000_000 represents 1 token
    (get-decimals () (response uint uint))
    ;; the balance of the passed principal
    (get-balance (principal) (response uint uint))
    ;; the current total supply (which does not need to be a constant)
    (get-total-supply () (response uint uint))
    ;; an optional URI that represents metadata of this token
    (get-token-uri () (response (optional (string-utf8 256)) uint))
  )
)

(define-trait native-interchain-token-trait
  (
    ;; Transfer from the caller to a new principal
    (transfer (uint principal principal (optional (buff 34))) (response bool uint))

    ;; the human readable name of the token
    (get-name () (response (string-ascii 32) uint))

    ;; the ticker symbol, or empty if none
    (get-symbol () (response (string-ascii 32) uint))

    ;; the number of decimals used, e.g. 6 would mean 1_000_000 represents 1 token
    (get-decimals () (response uint uint))

    ;; the balance of the passed principal
    (get-balance (principal) (response uint uint))

    ;; the current total supply (which does not need to be a constant)
    (get-total-supply () (response uint uint))

    ;; an optional URI that represents metadata of this token

    (get-token-uri () (response (optional (string-utf8 256)) uint))
    (add-flow-limiter (principal) (response bool uint))
    (remove-flow-limiter ( principal) (response bool uint))
    (is-flow-limiter (principal) (response bool uint))
    (get-flow-limit () (response uint uint))
    (set-flow-limit (uint) (response bool uint))
    (get-flow-out-amount () (response uint uint))
    (get-flow-in-amount ()  (response uint uint))
    (take-token (<sip-010-trait> principal uint) (response bool uint))
    (give-token (<sip-010-trait> principal uint) (response bool uint))
    (get-token-address () (response principal uint))
    (get-token-type () (response uint uint))
    (is-minter (principal) (response bool uint))
    (get-operators () (response (list 2 principal) principal))
    (is-operator (principal) (response bool uint))
    (get-token-id () (response (buff 32) uint))
    (mint (principal uint) (response bool uint))
    (burn (principal uint) (response bool uint))
    (get-is-started () (response bool uint))
  )
)

(define-trait token-manager-trait (
    (add-flow-limiter (principal) (response bool uint))
    (remove-flow-limiter ( principal) (response bool uint))
    (is-flow-limiter (principal) (response bool uint))
    (get-flow-limit () (response uint uint))
    (set-flow-limit (uint) (response bool uint))
    (get-flow-out-amount () (response uint uint))
    (get-flow-in-amount ()  (response uint uint))
    (take-token (<sip-010-trait> principal uint) (response bool uint))
    (give-token (<sip-010-trait> principal uint) (response bool uint))
    (get-token-address () (response principal uint))
    (get-token-type () (response uint uint))
    (is-minter (principal) (response bool uint))
    (get-operators () (response (list 2 principal) principal))
    (is-operator (principal) (response bool uint))
    (get-is-started () (response bool uint))
))

;; TODO: write an axelar executable trait and contracts which interact with axelar should

(define-trait axelar-executable (
  ;; the gateway validate-message pub fn MUST be called in contracts that implements of this trait
  (execute (
    (string-ascii 19)
    (string-ascii 128)
    (string-ascii 128)
    (buff 64000)
    <gateway-trait>
  ) (response bool uint)))
)

(define-trait interchain-token-factory-trait (
    (register-canonical-interchain-token
            (
                <gateway-trait>
                <gas-service-impl-trait>
                <interchain-token-service-trait>
                <sip-010-trait>
                <token-manager-trait>
                {
                    nonce: (buff 8),
                    fee-rate: (buff 8),
                    signature: (buff 65),
                    proof: { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint},
                    tx-block-height: uint,
                    block-header-without-signer-signatures: (buff 800),
                }
                principal
            )
            (response bool uint))
    (deploy-remote-canonical-interchain-token
            (
                <gateway-trait>
                <gas-service-impl-trait>
                <interchain-token-service-trait>
                <sip-010-trait>
                (string-ascii 19)
                uint
                principal
            )
            (response bool uint))
    (deploy-interchain-token
            (
                <gateway-trait>
                <gas-service-impl-trait>
                <interchain-token-service-trait>
                (buff 32)
                <native-interchain-token-trait>
                uint
                principal
                {
                    nonce: (buff 8),
                    fee-rate: (buff 8),
                    signature: (buff 65),
                    proof: { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint},
                    tx-block-height: uint,
                    block-header-without-signer-signatures: (buff 800),
                }
                principal
            )
            (response bool uint))
    (deploy-remote-interchain-token
            (
                <gateway-trait>
                <gas-service-impl-trait>
                <interchain-token-service-trait>
                (buff 32)
                principal
                (string-ascii 19)
                uint
                <sip-010-trait>
                <token-manager-trait>
                principal
            )
            (response bool uint))
    (dispatch ((string-ascii 32) (buff 65000) principal) (response bool uint))
    (deploy-remote-interchain-token-with-minter
        (
                <gateway-trait>
                <gas-service-impl-trait>
                <interchain-token-service-trait>
                (buff 32)
                principal
                (string-ascii 19)
                (optional (buff 128))
                uint
                <sip-010-trait>
                <token-manager-trait>
                principal
            )
            (response bool uint))
    (approve-deploy-remote-interchain-token (
        <interchain-token-service-trait>
        principal
        (buff 32)
        (string-ascii 19)
        (buff 128)
        <native-interchain-token-trait>
        principal
    ) (response bool uint))


    (revoke-deploy-remote-interchain-token (
        <interchain-token-service-trait>
        principal
        (buff 32)
        (string-ascii 19)
        principal
        ) (response bool uint))
))

;; Add this new trait for gas service implementation
(define-trait gas-service-impl-trait
    (
        (pay-native-gas-for-contract-call (uint principal (string-ascii 19) (string-ascii 128) (buff 64000) principal) (response bool uint))
        (add-native-gas (uint (buff 32) uint principal) (response bool uint))
        (refund ((buff 32) uint principal uint) (response bool uint))
        (collect-fees (principal uint) (response bool uint))
        (get-balance () (response uint uint))
        (transfer-ownership (principal) (response bool uint))
        (transfer-gas-collector (principal) (response bool uint))
        (pay-gas-for-contract-call (uint principal (string-ascii 19) (string-ascii 128) (buff 64000) principal) (response bool uint))
        (add-gas (uint principal (buff 32) uint principal) (response bool uint))
        (pay-native-gas-for-express-call (uint principal (string-ascii 19) (string-ascii 128) (buff 64000) principal) (response bool uint))
        (add-native-express-gas (uint principal (buff 32) uint principal) (response bool uint))
    )
)