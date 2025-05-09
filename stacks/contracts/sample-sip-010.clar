
;; title: sample-sip-010
;; version:
;; summary:
;; description:

;; traits
;;

;; token definitions
;;

;; constants
;;

;; data vars
;;

;; data maps
;;

;; public functions
;;

;; read only functions
;;

;; private functions
;;


(define-constant ERR-INSUFFICIENT-BALANCE (err u160000))
(define-constant ERR-INVALID-PARAMS (err u160001))
(define-constant ERR-NOT-AUTHORIZED (err u160002))

(define-fungible-token itscoin)

(define-read-only (get-balance (address principal))
    (ok (ft-get-balance itscoin address)))

(define-read-only (get-decimals)
    (ok u6)
)

(define-read-only (get-total-supply)
    (ok (ft-get-supply itscoin)))

(define-read-only (get-token-uri)
    (ok none))

(define-read-only (get-name)
    (ok "sample"))

(define-read-only (get-symbol)
    (ok "SMPL"))

(define-public (transfer (amount uint) (from principal) (to principal) (memo (optional (buff 34))))
    (begin
        (asserts! (or (is-eq from tx-sender) (is-eq from contract-caller)) ERR-NOT-AUTHORIZED)
        (asserts! (not (is-eq to tx-sender)) ERR-INVALID-PARAMS)
        (asserts! (>= (ft-get-balance itscoin from) amount) ERR-INSUFFICIENT-BALANCE)
        (match memo m
            (print m) 0x)
        (ft-transfer? itscoin amount from to)))

(ft-mint? itscoin u1000000000 tx-sender)