use crate::messages::{CommandName, Decode, Encode};
use std::io::Read;

#[derive(Debug, PartialEq, Eq)]
pub struct VerackMessage;

impl CommandName for VerackMessage {
    fn command_name() -> [u8; 12] {
        *b"verack\x00\x00\x00\x00\x00\x00"
    }
}

impl Encode for VerackMessage {
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(vec![])
    }
}
impl Decode for VerackMessage {
    fn decode(bytes: &mut impl Read) -> anyhow::Result<Self> {
        let mut buffer = [0u8; 1];
        if bytes.read(&mut buffer)? != 0 {
            return Err(anyhow::anyhow!("Invalid Encoding"));
        }
        Ok(VerackMessage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_should_work() {
        let verack_message = VerackMessage;
        assert_eq!(verack_message.encode().unwrap(), vec![])
    }

    #[test]
    fn decode_should_work() {
        assert_eq!(
            VerackMessage::decode(&mut vec![].as_slice()).unwrap(),
            VerackMessage
        );
    }
}
