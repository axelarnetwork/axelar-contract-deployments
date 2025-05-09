;; clarity-stacks
;; Check if a Stacks transaction has been mined.
;; Only works for Nakamoto blocks.

(define-constant err-invalid-length-version (err u10000))
(define-constant err-invalid-length-chain-length (err u10001))
(define-constant err-invalid-length-burn-spent (err u10002))
(define-constant err-invalid-length-consensus-hash (err u10003))
(define-constant err-invalid-length-parent-block-id (err u10004))
(define-constant err-invalid-length-tx-merkle-root (err u10005))
(define-constant err-invalid-length-state-index-root (err u10006))
(define-constant err-invalid-length-timestamp (err u10007))
(define-constant err-invalid-length-miner-signature (err u10008))
(define-constant err-invalid-length-signer-bitvec (err u10009))
(define-constant err-invalid-length-block-hash (err u10010))
(define-constant err-invalid-length-txid (err u10011))

(define-constant err-proof-too-short (err u10012))

(define-constant err-block-header-too-short (err u10013))
(define-constant err-invalid-block-height (err u10014))
(define-constant err-block-height-header-mismatch (err u10015))
(define-constant err-merkle-proof-invalid (err u10016))

(define-constant merkle-path-leaf-tag 0x00)
(define-constant merkle-path-node-tag 0x01)

(define-map debug-block-header-hashes uint (buff 32))
(define-constant debug-mode false)

(define-read-only (valid-signer-bitvec (bitvec (buff 506)))
	(let ((byte-length (buff-to-uint-be (unwrap-panic (as-max-len? (unwrap! (slice? bitvec u2 u6) false) u4)))))
		(is-eq (len bitvec) (+ byte-length u6))
	)
)

(define-read-only (block-header-hash-buff
	(version (buff 1))
	(chain-length (buff 8))
	(burn-spent (buff 8))
	(consensus-hash (buff 20))
	(parent-block-id (buff 32))
	(tx-merkle-root (buff 32))
	(state-index-root (buff 32))
	(timestamp (buff 8))
	(miner-signature (buff 65))
	(signer-bitvec (buff 506))
	)
	(begin
		(asserts! (is-eq (len version) u1) err-invalid-length-version)
		(asserts! (is-eq (len chain-length) u8) err-invalid-length-chain-length)
		(asserts! (is-eq (len burn-spent) u8) err-invalid-length-burn-spent)
		(asserts! (is-eq (len consensus-hash) u20) err-invalid-length-consensus-hash)
		(asserts! (is-eq (len parent-block-id) u32) err-invalid-length-parent-block-id)
		(asserts! (is-eq (len tx-merkle-root) u32) err-invalid-length-tx-merkle-root)
		(asserts! (is-eq (len state-index-root) u32) err-invalid-length-state-index-root)
		(asserts! (is-eq (len timestamp) u8) err-invalid-length-timestamp)
		(asserts! (is-eq (len miner-signature) u65) err-invalid-length-miner-signature)
		(asserts! (valid-signer-bitvec signer-bitvec) err-invalid-length-signer-bitvec)
		(ok
			(sha512/256
				(concat
				version
				(concat
				chain-length
				(concat
				burn-spent
				(concat
				consensus-hash
				(concat
				parent-block-id
				(concat
				tx-merkle-root
				(concat
				state-index-root
				(concat
				timestamp
				(concat
				miner-signature
				signer-bitvec)))))))))
			)
		)
	)
)

(define-read-only (block-id-header-hash (block-hash (buff 32)) (consensus-hash (buff 20)))
	(begin
		(asserts! (is-eq (len block-hash) u32) err-invalid-length-block-hash)
		(asserts! (is-eq (len consensus-hash) u20) err-invalid-length-consensus-hash)
		(ok (sha512/256 (concat block-hash consensus-hash)))
	)
)

(define-read-only (tagged-hash (tag (buff 1)) (data (buff 64)))
	(sha512/256 (concat tag data))
)

(define-read-only (is-bit-set (val uint) (bit uint))
	(> (bit-and val (bit-shift-left u1 bit)) u0)
)

(define-read-only (merkle-leaf-hash (data (buff 32)))
	(tagged-hash merkle-path-leaf-tag data)
)

(define-private (inner-merkle-proof-verify (ctr uint) (state { path: uint, root-hash: (buff 32), proof-hashes: (list 14 (buff 32)), tree-depth: uint, cur-hash: (buff 32), verified: bool}))
  (let ((path (get path state))
        (is-left (is-bit-set path ctr))
        (proof-hashes (get proof-hashes state))
        (cur-hash (get cur-hash state))
        (root-hash (get root-hash state))
        (h1 (if is-left (unwrap-panic (element-at proof-hashes ctr)) cur-hash))
        (h2 (if is-left cur-hash (unwrap-panic (element-at proof-hashes ctr))))
        (next-hash (tagged-hash merkle-path-node-tag (concat h1 h2)))
        (is-verified (and (is-eq (+ u1 ctr) (len proof-hashes)) (is-eq next-hash root-hash)))
		)
    	(merge state { cur-hash: next-hash, verified: is-verified})
	)
)

;; Note that the hashes in the proof must be tagged hashes.
;; Do not put TXIDs in the proof directly, they must first be
;; hashed with (merkle-leaf-hash).
;; Returns (ok true) if the proof is valid, or an error if not.
(define-read-only (verify-merkle-proof (txid (buff 32)) (merkle-root (buff 32)) (proof { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint}))
	(if (> (get tree-depth proof) (len (get hashes proof)))
		err-proof-too-short
		(ok (asserts! (get verified
			(fold inner-merkle-proof-verify
				(unwrap-panic (slice? (list u0 u1 u2 u3 u4 u5 u6 u7 u8 u9 u10 u11 u12 u13) u0 (get tree-depth proof)))
				{
					path: (+ (pow u2 (get tree-depth proof)) (get tx-index proof)),
					root-hash: merkle-root, proof-hashes: (get hashes proof),
					cur-hash: (tagged-hash merkle-path-leaf-tag txid),
					tree-depth: (get tree-depth proof),
					verified: false
				}
			)) err-merkle-proof-invalid)
		)
	)
)

(define-public (debug-set-block-header-hash (stx-height uint) (header-hash (buff 32)))
	(begin
		(asserts! debug-mode (err u1))
		;; #[allow(unchecked_data)]
		(ok (map-set debug-block-header-hashes stx-height header-hash))
	)
)

(define-read-only (get-block-info-header-hash? (stx-height uint))
	(if debug-mode
		(map-get? debug-block-header-hashes stx-height)
		(get-stacks-block-info? header-hash stx-height)
	)
)

;; Expected structure for block-header-without-signer-signatures, take a block header and
;; remove the signer signature count and signer signatures. It should look like this:
;; version: 1 byte
;; chain_length: 8 bytes
;; burn_spent: 8 bytes
;; consensus_hash: 20 bytes
;; parent_block_id: 32 bytes
;; tx_merkle_root: 32 bytes
;; state_index_root: 32 bytes
;; timestamp: 8 bytes
;; miner_signature: 65 bytes
;; signer_bitvec: 2 bytes bitvec bit count + 4 bytes buffer length + bitvec buffer
(define-read-only (was-tx-mined-compact (txid (buff 32)) (proof { tx-index: uint, hashes: (list 14 (buff 32)), tree-depth: uint}) (tx-block-height uint) (block-header-without-signer-signatures (buff 800)))
	(let (
		(target-header-hash (unwrap! (get-block-info-header-hash? tx-block-height) err-invalid-block-height))
		(tx-merkle-root (unwrap-panic (as-max-len? (unwrap! (slice? block-header-without-signer-signatures u69 u101) err-block-header-too-short) u32)))
		(header-hash (sha512/256 block-header-without-signer-signatures))
		)
		(asserts! (is-eq (len txid) u32) err-invalid-length-txid)
		;; It is fine to compare header hash because the consensus hash is part
		;; of the header in Nakamoto.
		(asserts! (is-eq header-hash target-header-hash) err-block-height-header-mismatch)
		(verify-merkle-proof txid tx-merkle-root proof)
	)
)
