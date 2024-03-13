use std::io::Read;

/// Encodes a Bitcoin p2p message as bytes
pub trait Encode {
    fn encode(&self) -> anyhow::Result<Vec<u8>>;
}

/// Decodes a bytes into a Bitoin p2p message
pub trait Decode: Sized {
    fn decode(bytes: &mut impl Read) -> anyhow::Result<Self>;
}
