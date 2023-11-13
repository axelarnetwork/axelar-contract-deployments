use super::*;

pub fn gateway_contract_call_event_listener<C: Deref<Target = impl Signer> + Clone>(
    client: &Client<C>,
    program_id: Pubkey,
) -> Result<ContractCallEvent, ClientError> {
    let program = client.program(program_id)?;

    let (sender, receiver) = std::sync::mpsc::channel();
    let event_unsubscriber = program.on(move |_, event: ContractCallEvent| {
        if sender.send(event).is_err() {
            error!("Error while transferring the event.")
        }
    })?;

    let event = receiver.recv().unwrap();

    event_unsubscriber.unsubscribe();

    Ok(event)
}
