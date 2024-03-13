//! Module contains the functionality related to Bitcoin p2p messages

use crate::{
    constants::{
        MAINNET_MAGIC_VALUE, NAMECOIN_MAGIC_VALUE, REGNET_MAGIC_VALUE, SIGNET_MAGIC_VALUE,
        TESTNET3_MAGIC_VALUE,
    },
    crypto::checksum,
};
use anyhow::anyhow;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::str::FromStr;
use std::{
    fmt::Debug,
    io::{Read, Write},
};

pub mod codec;
pub mod types;
use crate::constants::MAX_PAYLOAD_SIZE;
use codec::{Decode, Encode};

pub trait CommandName {
    fn command_name() -> [u8; 12];
}

/// Different Bitcoin Networks
///
/// Source: https://en.bitcoin.it/wiki/Protocol_documentation#M_structure
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Chain {
    Mainnet,
    Regnet,
    Testnet3,
    Signet,
    Namecoin,
}

impl FromStr for Chain {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mainnet" => Ok(Chain::Mainnet),
            "regnet" => Ok(Chain::Regnet),
            "testnet3" => Ok(Chain::Testnet3),
            "signet" => Ok(Chain::Signet),
            "namecoin" => Ok(Chain::Namecoin),
            _ => return Err(anyhow!("Cannot convert string to chain")),
        }
    }
}

impl Encode for Chain {
    /// Magic value indicating message origin network, and used to seek to next message when stream state is unknown
    ///
    /// Source: https://en.bitcoin.it/wiki/Protocol_documentation#M_structure
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        match self {
            Chain::Mainnet => Ok(MAINNET_MAGIC_VALUE.to_vec()),
            Chain::Regnet => Ok(REGNET_MAGIC_VALUE.to_vec()),
            Chain::Testnet3 => Ok(TESTNET3_MAGIC_VALUE.to_vec()),
            Chain::Signet => Ok(SIGNET_MAGIC_VALUE.to_vec()),
            Chain::Namecoin => Ok(NAMECOIN_MAGIC_VALUE.to_vec()),
        }
    }
}

impl Decode for Chain {
    fn decode(bytes: &mut impl Read) -> anyhow::Result<Self> {
        let mut magic_value = [0u8; 4];
        bytes.read_exact(&mut magic_value)?;
        match magic_value {
            MAINNET_MAGIC_VALUE => Ok(Chain::Mainnet),
            REGNET_MAGIC_VALUE => Ok(Chain::Regnet),
            TESTNET3_MAGIC_VALUE => Ok(Chain::Testnet3),
            SIGNET_MAGIC_VALUE => Ok(Chain::Signet),
            NAMECOIN_MAGIC_VALUE => Ok(Chain::Namecoin),
            _ => return Err(anyhow!("Unknown Magic Value: {:?}", magic_value)),
        }
    }
}

/// Struct represents a message on the Bitcoin p2p network protocol
pub struct Message<M: CommandName + Encode + Decode> {
    pub chain: Chain,
    pub message: M,
}

impl<M: CommandName + Encode + Decode> Message<M> {
    pub fn new(chain: Chain, message: M) -> Self {
        Self { chain, message }
    }
}

#[derive(Debug, thiserror::Error)]
enum MessageEncodeError {
    #[error("payload too big")]
    PayloadTooBig,
}
#[derive(Debug, thiserror::Error)]
enum MessageDecodeError {
    #[error("payload too big")]
    PayloadTooBig,
    #[error("command name unknown")]
    CommandNameUnkown,
    #[error("checksum is invalid")]
    CheksumIsInvalid,
}

impl<M: CommandName + Encode + Decode> Encode for Message<M> {
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        let encoded_message = self.message.encode()?;
        let encoded_message_len = encoded_message.len() as u32;
        if encoded_message_len > MAX_PAYLOAD_SIZE {
            Err(MessageEncodeError::PayloadTooBig)?
        }
        let checksum = checksum(&encoded_message);

        let mut buffer = Vec::with_capacity(24 + encoded_message.len());

        buffer.write_all(&self.chain.encode()?)?;
        buffer.write_all(&M::command_name())?;
        buffer.write_u32::<LittleEndian>(encoded_message_len)?;
        buffer.write_all(&checksum)?;
        buffer.write_all(&encoded_message)?;

        Ok(buffer)
    }
}

impl<M: CommandName + Encode + Decode> Decode for Message<M> {
    fn decode(bytes: &mut impl Read) -> anyhow::Result<Self> {
        let mut magic_number = [0u8; 4];
        bytes.read_exact(&mut magic_number)?;
        let chain = Chain::decode(&mut magic_number.as_slice())?;

        let mut command_name = [0u8; 12];
        bytes.read_exact(&mut command_name)?;
        if command_name != M::command_name() {
            Err(MessageDecodeError::CommandNameUnkown)?
        }

        let encoded_message_len = bytes.read_u32::<LittleEndian>()?;
        if encoded_message_len > MAX_PAYLOAD_SIZE {
            Err(MessageDecodeError::PayloadTooBig)?
        }

        let mut received_checksum = [0u8; 4];
        bytes.read_exact(&mut received_checksum)?;

        let mut encoded_message = vec![0u8; encoded_message_len as usize];
        bytes.read_exact(&mut encoded_message)?;

        if received_checksum != checksum(&encoded_message) {
            Err(MessageDecodeError::CheksumIsInvalid)?
        }

        let message = M::decode(&mut encoded_message.as_slice())?;

        Ok(Self { chain, message })
    }
}
