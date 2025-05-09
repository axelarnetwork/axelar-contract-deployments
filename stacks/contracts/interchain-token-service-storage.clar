;; title: interchain-token-service-storage
(define-constant SERVICE-PROXY .interchain-token-service)
(define-constant FACTORY-PROXY .interchain-token-factory)

(define-constant ERR-NOT-AUTHORIZED (err u130000))
(define-constant ERR-NON-STANDARD-ADDRESS (err u130001))

(define-constant NULL-ADDRESS (unwrap-panic (principal-construct? (if (is-eq chain-id u1) 0x16 0x1a) 0x0000000000000000000000000000000000000000)))


(define-private (is-service-proxy) (is-eq contract-caller SERVICE-PROXY))
(define-private (is-service-impl) (is-eq contract-caller (var-get service-impl)))

(define-private (is-proxy-or-service-impl) (or (is-service-proxy) (is-service-impl)))



;; ######################
;; ######################
;; ####### Storage ######
;; ######################
;; ######################

;; Constructor flag
(define-data-var is-started bool false)

(define-read-only (get-is-started) (var-get is-started))

(define-public (start)
    (begin
        (asserts! (is-service-proxy) ERR-NOT-AUTHORIZED)
        (ok (var-set is-started true))
    )
)



;; ITS implementation contract address
(define-data-var service-impl principal .interchain-token-service-impl)

(define-read-only (get-service-impl) (var-get service-impl))
(define-read-only (get-service-proxy) SERVICE-PROXY)

;; Only proxy through gov will be able to update this, the only check i see here is if it's the same as the old one
;; #[allow(unchecked_data)]
(define-public (set-service-impl (new-service-impl principal))
    (begin
        (asserts! (is-service-proxy) ERR-NOT-AUTHORIZED)
        (asserts! (is-standard new-service-impl) ERR-NON-STANDARD-ADDRESS)
        (ok (var-set service-impl new-service-impl))
    )
)



;; ITF implementation contract address
(define-data-var factory-impl principal .interchain-token-factory-impl)

(define-read-only (get-factory-impl) (var-get factory-impl))
(define-read-only (get-factory-proxy) FACTORY-PROXY)

(define-private (is-factory-proxy) (is-eq contract-caller FACTORY-PROXY))
(define-private (is-factory-impl) (is-eq contract-caller (var-get factory-impl)))
(define-private (is-proxy-or-factory-impl) (or (is-factory-proxy) (is-factory-impl)))


;; Only proxy through gov will be able to update this, the only check i see here is if it's the same as the old one
;; #[allow(unchecked_data)]
(define-public (set-factory-impl (new-factory-impl principal))
    (begin
        (asserts! (is-factory-proxy) ERR-NOT-AUTHORIZED)
        (asserts! (is-standard new-factory-impl) ERR-NON-STANDARD-ADDRESS)
        (ok (var-set factory-impl new-factory-impl))
    )
)

;; ITS owner
(define-data-var owner principal contract-caller)

(define-read-only (get-owner) (var-get owner))

;; logic for write guards will be in the calling context (proxy, impl)
;; #[allow(unchecked_data)]
(define-public (set-owner (new-owner principal))
    (begin
        (asserts! (is-proxy-or-service-impl) ERR-NOT-AUTHORIZED)
        (asserts! (is-standard new-owner) ERR-NON-STANDARD-ADDRESS)
        (ok (var-set owner new-owner))
    )
)


;; ITS operator
(define-data-var operator principal contract-caller)

(define-read-only (get-operator) (var-get operator))

;; logic for write guards will be in the calling context (proxy, impl)
;; #[allow(unchecked_data)]
(define-public (set-operator (new-operator principal))
    (begin
        (asserts! (is-proxy-or-service-impl) ERR-NOT-AUTHORIZED)
        (asserts! (is-standard new-operator) ERR-NON-STANDARD-ADDRESS)
        (ok (var-set operator new-operator))
    )
)

(define-data-var is-paused bool false)

(define-public (set-paused (status bool))
    (begin
        (asserts! (is-proxy-or-service-impl) ERR-NOT-AUTHORIZED)
        (ok (var-set is-paused status))))

(define-read-only (get-is-paused)
    (ok (var-get is-paused)))




;; ####################
;; ####################
;; ### address tracking ###
;; ####################
;; ####################

(define-map trusted-chain-address (string-ascii 19) (string-ascii 128))


;; Gets the trusted address at a remote chain
;; @param chain Chain name of the remote chain
;; @return trustedAddress_ The trusted address for the chain. Returns '' if the chain is untrusted
(define-read-only (get-trusted-address (chain (string-ascii 19)))
    (map-get? trusted-chain-address chain))

(define-read-only (is-trusted-chain (chain-name (string-ascii 19)))
    (is-some (map-get? trusted-chain-address chain-name)))

;; Sets the trusted address and its hash for a remote chain
;; @param chain-name Chain name of the remote chain
;; @param address the string representation of the trusted address
;; #[allow(unchecked_data)]
(define-public (set-trusted-address (chain-name (string-ascii 19)) (address (string-ascii 128)))
    (begin
        (asserts! (is-proxy-or-service-impl) ERR-NOT-AUTHORIZED)
        (ok (map-set trusted-chain-address chain-name address))))

;; Remove the trusted address of the chain.
;; @param chain-name Chain name that should be made untrusted
;; #[allow(unchecked_data)]
(define-public (remove-trusted-address  (chain-name (string-ascii 19)))
    (begin
        (asserts! (is-proxy-or-service-impl) ERR-NOT-AUTHORIZED)
        (ok (map-delete trusted-chain-address chain-name))))

(define-private (extract-and-set-trusted-address
    (entry {chain-name: (string-ascii 19), address: (string-ascii 128)}))
        (map-set trusted-chain-address (get chain-name entry) (get address entry)))

(define-public (set-trusted-addresses (trusted-chain-names-addresses (list 50 {chain-name: (string-ascii 19), address: (string-ascii 128)})))
    (begin
        (asserts! (is-service-proxy) ERR-NOT-AUTHORIZED)
        (map extract-and-set-trusted-address trusted-chain-names-addresses)
        (ok true))
)


;; Token managers


(define-map token-managers (buff 32)
    {
        manager-address: principal,
        token-type: uint,
    })

(define-map used-token-managers principal bool)

(define-read-only (is-manager-address-used (manager-address principal))
    (default-to false (map-get? used-token-managers manager-address)))

(define-read-only (get-token-info (token-id (buff 32)))
    (map-get? token-managers token-id))

;; logic for write guards will be in the calling context (proxy, impl)
;; #[allow(unchecked_data)]
(define-public (insert-token-manager (token-id (buff 32)) (manager-address principal) (token-type uint))
    (begin
        (asserts! (is-proxy-or-service-impl) ERR-NOT-AUTHORIZED)
        (map-insert used-token-managers manager-address true)
        (asserts! (is-standard manager-address) ERR-NON-STANDARD-ADDRESS)
        (ok (map-insert token-managers token-id {
            manager-address: manager-address,
            token-type: token-type
        }))
    ))


(define-data-var gas-service principal NULL-ADDRESS)
(define-data-var its-contract-name (string-ascii 128) "")

;; @dev Chain name where ITS Hub exists. This is used for routing ITS calls via ITS hub.
;; This is set as a constant, since the ITS Hub will exist on Axelar.
(define-data-var its-hub-chain (string-ascii 19) "axelarnet")


(define-read-only (get-token-factory-impl)
    (var-get factory-impl))

(define-read-only (get-gas-service)
    (var-get gas-service))

;; logic for write guards will be in the calling context (proxy, impl)
;; #[allow(unchecked_data)]
(define-public (set-gas-service (address principal))
    (begin
        (asserts! (is-proxy-or-service-impl) ERR-NOT-AUTHORIZED)
        (asserts! (is-standard address) ERR-NON-STANDARD-ADDRESS)
        (var-set gas-service address)
        (ok true)))


(define-read-only (get-its-hub-chain)
    (var-get its-hub-chain))

;; logic for write guards will be in the calling context (proxy, impl)
;; #[allow(unchecked_data)]
(define-public (set-its-hub-chain (chain-name (string-ascii 19)))
    (begin
        (asserts! (is-proxy-or-service-impl) ERR-NOT-AUTHORIZED)
        (var-set its-hub-chain chain-name)
        (ok true)))

(define-read-only (get-its-contract-name)
    (var-get its-contract-name))

;; logic for write guards will be in the calling context (proxy, impl)
;; #[allow(unchecked_data)]
(define-public (set-its-contract-name (contract-name (string-ascii 128)))
    (begin
        (asserts! (is-proxy-or-service-impl) ERR-NOT-AUTHORIZED)
        (var-set its-contract-name contract-name)
        (ok true)))

(define-map approved-destination-minters (buff 32) (buff 32))

;; logic for write guards will be in the calling context (proxy, impl)
;; #[allow(unchecked_data)]
(define-public (set-approved-destination-minter (approval-key (buff 32)) (hashed-destination-minter (buff 32)))
    (begin
        (asserts! (is-proxy-or-factory-impl) ERR-NOT-AUTHORIZED)
        (ok (map-set approved-destination-minters approval-key hashed-destination-minter))
    ))

;; logic for write guards will be in the calling context (proxy, impl)
;; #[allow(unchecked_data)]
(define-public (remove-approved-destination-minter (approval-key (buff 32)))
    (begin
        (asserts! (is-proxy-or-factory-impl) ERR-NOT-AUTHORIZED)
        (ok (map-delete approved-destination-minters approval-key))
    ))

(define-read-only (get-approved-destination-minter (approval-key (buff 32)))
    (map-get? approved-destination-minters approval-key))

;; EVENTS

(define-public (emit-transfer-operatorship (new-operator principal))
    (begin
        (asserts! (is-service-impl) ERR-NOT-AUTHORIZED)
        (print {action: "transfer-operatorship", new-operator: new-operator})
        (ok true)))

(define-public (emit-transfer-ownership (new-owner principal))
    (begin
        (asserts! (is-service-impl) ERR-NOT-AUTHORIZED)
        (print {action: "transfer-ownership", new-owner: new-owner})
        (ok true)))


(define-public (emit-trusted-address-set
        (chain-name (string-ascii 19))
        (address (string-ascii 128)))
    (begin
        (asserts! (is-service-impl) ERR-NOT-AUTHORIZED)
        (print {
                    type: "trusted-address-set",
                    chain: chain-name,
                    address: address
                })
        (ok true)))

(define-public (emit-trusted-address-removed
        (chain-name (string-ascii 19)))
    (begin
        (asserts! (is-service-impl) ERR-NOT-AUTHORIZED)
        (print {
                    type: "trusted-address-removed",
                    chain: chain-name,
                })
        (ok true)))

(define-public (emit-interchain-token-id-claimed
        (token-id (buff 32))
        (deployer principal)
        (salt (buff 32)))
    (begin
        (asserts! (is-service-impl) ERR-NOT-AUTHORIZED)
        (print {
            type: "interchain-token-id-claimed",
            token-id: token-id,
            deployer: deployer,
            salt: salt,
        })
        (ok true)))

(define-public (emit-token-manager-deployment-started
        (token-id (buff 32))
        (destination-chain (string-ascii 19))
        (token-manager-type uint)
        (params (buff 62000)))
    (begin
        (asserts! (is-service-impl) ERR-NOT-AUTHORIZED)
        (print {
                type: "token-manager-deployment-started",
                token-id: token-id,
                destination-chain: destination-chain,
                token-manager-type: token-manager-type,
                params: params,
            })
        (ok true)))

(define-public (emit-token-manager-deployed
        (token-id  (buff 32))
        (token-manager-address principal)
        (token-type uint)
        )
    (begin
        (asserts! (is-service-impl) ERR-NOT-AUTHORIZED)
        (print {
            type: "token-manager-deployed",
            token-id: token-id,
            token-manager: token-manager-address,
            token-type: token-type,
        })
        (ok true)))

(define-public (emit-interchain-token-deployment-started
        (token-id  (buff 32))
        (destination-chain (string-ascii 19))
        (name (string-ascii 32))
        (symbol (string-ascii 32))
        (decimals uint)
        (minter (buff 128))
        )
    (begin
        (asserts! (is-service-impl) ERR-NOT-AUTHORIZED)
        (print {
            type:"interchain-token-deployment-started",
            destination-chain: destination-chain,
            token-id: token-id,
            name: name,
            symbol: symbol,
            decimals: decimals,
            minter: minter,
        })
        (ok true)))


(define-public (emit-interchain-transfer
        (token-id  (buff 32))
        (source-address principal)
        (destination-chain (string-ascii 19))
        (destination-address (buff 128))
        (amount uint)
        (data (buff 32))
        )
    (begin
        (asserts! (is-service-impl) ERR-NOT-AUTHORIZED)
        (print {
            type: "interchain-transfer",
            token-id: token-id,
            source-address: source-address,
            destination-chain: destination-chain,
            destination-address: destination-address,
            amount: amount,
            data: data
        })
        (ok true)))

(define-public (emit-interchain-transfer-received
        (token-id  (buff 32))
        (source-chain (string-ascii 19))
        (source-address (buff 128))
        (destination-address principal)
        (amount uint)
        (data (buff 32))
        )
    (begin
        (asserts! (is-service-impl) ERR-NOT-AUTHORIZED)
        (print {
            type: "interchain-transfer-received",
            token-id: token-id,
            source-chain: source-chain,
            source-address: source-address,
            destination-address: destination-address,
            amount: amount,
            data: data,
        })
        (ok true)))

(define-public (emit-deploy-remote-interchain-token-approval
        (minter principal)
        (deployer principal)
        (token-id (buff 32))
        (destination-chain (string-ascii 19))
        (destination-minter (buff 128))
        )
    (begin
        (asserts! (is-factory-impl) ERR-NOT-AUTHORIZED)
        (print {
            type: "deploy-remote-interchain-token-approval",
            minter: minter,
            deployer: deployer,
            token-id: token-id,
            destination-chain: destination-chain,
            destination-minter: destination-minter,
        })
        (ok true)))

(define-public (emit-revoked-deploy-remote-interchain-token-approval
        (minter principal)
        (deployer principal)
        (token-id (buff 32))
        (destination-chain (string-ascii 19))
        )
    (begin
        (asserts! (is-factory-impl) ERR-NOT-AUTHORIZED)
        (print {
            type: "revoked-deploy-remote-interchain-token-approval",
            minter: minter,
            deployer: deployer,
            token-id: token-id,
            destination-chain: destination-chain,
        })
        (ok true)))
