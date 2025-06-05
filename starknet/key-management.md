# **Key Management Scripts Guidelines**

## **Key Considerations**

Many blockchains require certain transaction elements—such as **nonce management, blockhash freshness, or sequence numbers**—to ensure valid execution. To enhance security and flexibility, the recommended approach is to **generate unsigned transactions first**, then sign them in an environment appropriate to the target network.

This process is specifically designed for **multisig workflows**, where each signer performs signing on an **offline device**, after which the signatures must be **shared and coordinated** to produce a valid combined transaction.

Key handling and signing flows vary by environment:

* **Mainnet**: Keys are held in **hardware wallets (e.g., Ledger)** and must be accessed in secure, **offline** environments. All signing dependencies must be bundled and portable. **Each signer independently signs the transaction offline**, and the signatures must be **collected and combined** before broadcasting.  
* **Testnet / Stagenet**: Use **directory-based keystores**, and both transaction generation and signing can happen on the same machine — **no offline system or packaging is needed**. Multisig signing can also be simulated in this environment.

---

## **Recommended Transaction Flow**

### **Step 1: Generate an Unsigned Transaction and Package Dependencies**

* Takes user-defined inputs (sender, recipient, gas fees, payload, etc.).  
* Includes required chain metadata (e.g., nonce, recent blockhash, sequence).  
* Produces:  
  * A serialized unsigned transaction.  
  * A **packaged archive** (e.g., `.tar.gz` or `.zip`) that includes:  
    * The unsigned transaction.  
    * All scripts or CLI tools needed for signing.  
    * Any runtime dependencies (e.g., compiled binaries, config files, ABI definitions).

Mainnet:

This bundle is intended for **offline use by multiple signers**. Each signer uses this bundle on their **own secure, offline device** to generate their individual signature. The environment is **fully self-contained** with no internet access or external tooling required. The packaging approach should follow a model similar to [`package.sh`](https://github.com/axelarnetwork/axelar-contract-deployments) in the [Axelar repo](https://github.com/axelarnetwork/axelar-contract-deployments).

Testnet / Stagenet:

No need for packaging or offline transport. Transactions can be generated and signed directly on the same machine using local key directories. Multisig signing (if simulated) can also be done locally.

---

### **Step 2: Signing the Transaction**

* The signing script must support **automatic environment detection** and select the key source accordingly:  
  * **Mainnet** → Interface with a **Ledger** or other hardware wallet.  
  * **Testnet / Stagenet** → Use a **flat key directory**.  
* **Mainnet Multisig**:  
  * Each signer unpacks the bundle in their **secure offline environment**.  
  * Each signer executes the provided script to sign the transaction using their Ledger.  
  * Signatures are exported (e.g., to USB or QR) for coordination.  
  * All partial signatures must be **shared among signers or with a coordinator**, who will **combine them into a final multisigned transaction**.  
* **Testnet / Stagenet**:  
  * Run the signing script directly; it will use the directory-based keys.  
  * No network isolation or manual packaging required.  
  * Multisig signing and combination steps can be executed on the same system.

---

### **Step 3: Combining and Broadcasting the Signed Transaction**

* After all signers have provided their individual signatures:  
  * Use the combination script or utility to merge all signatures into a single multisigned transaction.  
* For mainnet:  
  * Transfer the combined signed output back to an online system.  
* Submit the fully signed transaction to the blockchain network using the broadcast script.  
* Monitor the transaction for confirmation and potential errors.

---

## **Infrastructure Components**

| Component | Purpose |
| ----- | ----- |
| **Unsigned TX \+ Packager** | Generates the unsigned transaction and bundles all required scripts/tools into an offline-ready archive for use by each signer. |
| **Offline Signing Script** | Allows each signer to produce a signature independently, using **Ledger for mainnet**, or **key directory for testnet/stagenet**. |
| **Signature Combiner** | Collects and merges individual signatures into a single valid multisigned transaction. |
| **Broadcast Script** | Submits the combined transaction to the blockchain and handles result reporting. |

---

## **Additional Considerations**

### **Transaction File Format**

* Support serialization formats like JSON, RLP, or raw binary.  
* JSON is preferred for test environments; use compact formats for production if supported.

### **Chain Metadata Handling**

* Always ensure nonces, sequence numbers, and block references are current.  
* For delayed or multisig scenarios, provide a metadata refresh mechanism before signing(for solana like chains that require recent blockhash).

### **Security Considerations**

* **Mainnet**: Signing should occur **entirely offline** on **each signer's device**, and **only inside trusted, reproducible environments**. Use packaging to deliver everything needed for each signer to operate independently and securely.  
* **Testnet / Stagenet**: Simpler flows are allowed, but take care not to accidentally expose mainnet-like credentials or behaviors.

