use anchor_client::ClientError;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use regex::Regex;
use solana_program::pubkey::Pubkey;
use solana_transaction_status::option_serializer::OptionSerializer;

const PROGRAM_LOG: &str = "Program log: ";
const PROGRAM_DATA: &str = "Program data: ";

pub struct Execution {
    stack: Vec<String>,
}

impl Execution {
    pub fn new(logs: &mut &[String]) -> Result<Self, ClientError> {
        let l = &logs[0];
        *logs = &logs[1..];

        let re = Regex::new(r"^Program (.*) invoke.*$").unwrap();
        let c = re
            .captures(l)
            .ok_or_else(|| ClientError::LogParseError(l.to_string()))?;
        let program = c
            .get(1)
            .ok_or_else(|| ClientError::LogParseError(l.to_string()))?
            .as_str()
            .to_string();
        Ok(Self {
            stack: vec![program],
        })
    }

    pub fn program(&self) -> String {
        assert!(!self.stack.is_empty());
        self.stack[self.stack.len() - 1].clone()
    }

    pub fn push(&mut self, new_program: String) {
        self.stack.push(new_program);
    }

    pub fn pop(&mut self) {
        assert!(!self.stack.is_empty());
        self.stack.pop().unwrap();
    }
}

pub fn parse_logs_from_contract_call_event(
    tx_body: solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
    contract_id: &Pubkey,
) -> Vec<Vec<Vec<u8>>> {
    let tx_meta = tx_body.transaction.meta.unwrap();
    let tx_meta_log_messages = tx_meta.log_messages;
    if let OptionSerializer::Some(meta) = &tx_meta_log_messages {
        let parsed_events: Vec<Vec<Vec<u8>>> =
            parse_logs_response(meta.clone(), &contract_id.to_string());

        parsed_events
    } else {
        // hack
        Vec::new()
    }
}

pub fn parse_logs_response(logs: Vec<String>, program_id_str: &str) -> Vec<Vec<Vec<u8>>> {
    let mut logs = &logs[..];
    let mut events = vec![];
    if !logs.is_empty() {
        if let Ok(mut execution) = Execution::new(&mut logs) {
            for l in logs {
                // Parse the log.
                let (event, new_program, did_pop) = {
                    if program_id_str == execution.program() {
                        handle_program_log(program_id_str, l).unwrap_or_else(|e| {
                            println!("Unable to parse log: {e}");
                            std::process::exit(1);
                        })
                    } else {
                        let (program, did_pop) = handle_system_log(program_id_str, l);
                        (None, program, did_pop)
                    }
                };
                // Emit the event.
                if let Some(e) = event {
                    events.push(e);
                }
                // Switch program context on CPI.
                if let Some(new_program) = new_program {
                    execution.push(new_program);
                }
                // Program returned.
                if did_pop {
                    execution.pop();
                }
            }
        }
    }
    events
}

fn handle_program_log(
    self_program_str: &str,
    l: &str,
) -> Result<(Option<Vec<Vec<u8>>>, Option<String>, bool), ClientError> {
    // Log emitted from the current program.
    if let Some(_log) = l
        .strip_prefix(PROGRAM_LOG)
        .or_else(|| l.strip_prefix(PROGRAM_DATA))
    {
        let b64_split: Vec<&str> = _log.split_whitespace().collect();
        let decoded: Vec<Vec<u8>> = b64_split
            .iter()
            .map(|v| STANDARD.decode(v).unwrap())
            .collect();
        Ok((Some(decoded), None, false))
    }
    // System log.
    else {
        let (program, did_pop) = handle_system_log(self_program_str, l);
        Ok((None, program, did_pop))
    }
}

fn handle_system_log(this_program_str: &str, log: &str) -> (Option<String>, bool) {
    if log.starts_with(&format!("Program {this_program_str} log:")) {
        (Some(this_program_str.to_string()), false)
    } else if log.contains("invoke") {
        (Some("cpi".to_string()), false) // Any string will do.
    } else {
        let re = Regex::new(r"^Program (.*) success*$").unwrap();
        if re.is_match(log) {
            (None, true)
        } else {
            (None, false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::panic;

    #[test]
    fn test_new_handle_program_log() {
        let expected = "Program data: WNc6VBv8x5ppS4doXxDnQjRIQGsc1P1e53w0IttK5+M= ZXRoZXJldW0= MHgyRjQzRERGZjU2NEZiMjYwZGJENzgzRDU1ZmM2RTRjNzBCZTE4ODYy 80GFIxykuDhKsGtEKTFpCYAqEx02iSp17xUDApLliYUBriCN9AdcpsNMJE5gbIqo5UwxMf8n9mptOHOqfegKgI5W7/krz0FtFvmcDJn7MIO5cheYmbcD5Djcc73wX7y3MmSvaA== 3UVacVf+vox/rNMZ+/Ei99QlGuIvX3B9a7+o6ny12Rc=".to_owned();
        let not_expected = "Program data: WNc6VBv8x5ppS4doXxDnQjRIQGsc1P1e53w0IttK5+M= cG90YXRv MHgyRjQzRERGZjU2NEZiMjYwZGJENzgzRDU1ZmM2RTRjNzBCZTE4ODYy 80GFIxykuDhKsGtEKTFpCYAqEx02iSp17xUDApLliYUBriCN9AdcpsNMJE5gbIqo5UwxMf8n9mptOHOqfegKgI5W7/krz0FtFvmcDJn7MIO5cheYmbcD5Djcc73wX7y3MmSvaA== 3UVacVf+vox/rNMZ+/Ei99QlGuIvX3B9a7+o6ny12Rc=".to_owned();
        let actual = match handle_program_log("self_program_str", expected.as_str())
            .unwrap()
            .0
        {
            Some(v) => v,
            None => panic!("Ups!"),
        };
        assert_eq!(prepare_from_base64(&expected), actual);
        assert_ne!(prepare_from_base64(&not_expected), actual);
    }

    fn prepare_from_base64(expected: &String) -> Vec<Vec<u8>> {
        let expected_no_prefix = expected.strip_prefix(PROGRAM_DATA).unwrap();
        let expected_split: Vec<&str> = expected_no_prefix.split_whitespace().collect();
        let expected_split_decoded: Vec<Vec<u8>> = expected_split
            .iter()
            .map(|v| STANDARD.decode(v).unwrap())
            .collect();
        expected_split_decoded
    }
}
