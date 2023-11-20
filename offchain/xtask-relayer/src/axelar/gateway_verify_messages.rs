use super::*;

pub fn axelar_gateway_verify_messages(
    request_body: Message,
    fees: &str,
    fees_ratio: &str,
    payer: &str,
    gateway_addr: &str,
    rpc_addr: &str,
) {
    let request_json = json!({
        "verify_messages": [
            {
                "cc_id": {
                    "chain": request_body.cc_id.chain,
                    "id": request_body.cc_id.id,
                },
                "source_address": request_body.source_address,
                "destination_chain": request_body.destination_chain,
                "destination_address": request_body.destination_address,
                "payload_hash": request_body.payload_hash,
            }
        ]
    });

    let request_json =
        serde_json::to_string_pretty(&request_json).expect("Failed to convert to JSON string");

    let output = Command::new("axelard")
        .arg("tx")
        .arg("wasm")
        .arg("execute")
        .arg(gateway_addr)
        .arg("--from") // Payer
        .arg(payer)
        .arg("--gas-prices")
        .arg(fees)
        .arg("--gas")
        .arg("auto")
        .arg("--gas-adjustment")
        .arg(fees_ratio)
        .arg("-y")
        .arg(request_json)
        .arg("--node")
        .arg(rpc_addr)
        .output()
        .unwrap();

    if output.status.success() {
        let output = String::from_utf8_lossy(&output.stdout);
        let output: Value = serde_json::from_str(&output).unwrap();

        info!("output: {:#?}", output)
    } else {
        error!("error: {:#?}", String::from_utf8_lossy(&output.stderr));
    }
}
