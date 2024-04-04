use crate::amplifier_api::SubscribeToApprovalsResponse;

pub struct BlockMessages {
    height: u64,
    pub messages: Vec<SubscribeToApprovalsResponse>,
}

#[allow(dead_code)]
impl BlockMessages {
    pub fn new(height: u64) -> Self {
        Self {
            height,
            messages: Vec::new(),
        }
    }

    pub fn indicate_height(&mut self, height: u64) -> Option<Vec<SubscribeToApprovalsResponse>> {
        if self.height != height {
            let messages = std::mem::take(&mut self.messages);
            self.height = height;
            Some(messages)
        } else {
            None
        }
    }

    pub fn push(&mut self, axl_proof: SubscribeToApprovalsResponse) {
        self.messages.push(axl_proof);
    }
}
