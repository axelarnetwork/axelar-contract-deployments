use super::*;

pub fn axelar_dummy_verifier_is_verified(
    request_body: Message,
    verifier_addr: String,
    rpc_addr: String,
) {
    let request_json = json!({
        "is_verified": {
            "messages": [{
                "cc_id": {
                    "chain": request_body.cc_id.chain,
                    "id": request_body.cc_id.id,
                },
                "source_address": request_body.source_address,
                "destination_chain": request_body.destination_chain,
                "destination_address": request_body.destination_address,
                "payload_hash": request_body.payload_hash,
            }],
        },
    });

    let request_json =
        serde_json::to_string_pretty(&request_json).expect("Failed to convert to JSON string");

    let output = Command::new("axelard")
        .arg("query")
        .arg("wasm")
        .arg("contract-state")
        .arg("smart")
        .arg(verifier_addr)
        .arg(request_json)
        .arg("--node")
        .arg(rpc_addr)
        .arg("--output")
        .arg("json")
        .output()
        .unwrap();

    if output.status.success() {
        let output = String::from_utf8_lossy(&output.stdout);
        let output: Value = serde_json::from_str(&output).unwrap();

        info!("axelar_dummy_verifier_is_verified | output: {:?}", output);
    } else {
        error!(
            "axelar_dummy_verifier_is_verified | error: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
