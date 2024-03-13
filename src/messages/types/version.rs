use crate::messages::{CommandName, Decode, Encode};
use anyhow::anyhow;
use bitflags::bitflags;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv6Addr, SocketAddr},
};

bitflags! {
    /// Services supported by a node (encoded as a bitfield)
    ///
    /// Source: https://en.bitcoin.it/wiki/Protocol_documentation#version
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct Services: u64 {
        /// This node is not a full node. It may not be able to provide any data except for the transactions it originates.
        const UNNAMED = 0;
        /// This node can be asked for full blocks instead of just headers.
        const NODE_NETWORK = 1;
        /// This is a full node capable of responding to the getutxo protocol request. This is not supported by any currently-maintained Bitcoin node.
        const NODE_GETUTXO = 2;
        /// This is a full node capable and willing to handle bloom-filtered connections
        const NODE_BLOOM = 4;
        /// This is a full node that can be asked for blocks and transactions including witness data
        const NODE_WITNESS = 8;
        /// This is a full node that supports Xtreme Thinblocks. This is not supported by any currently-maintained Bitcoin node.
        const NODE_XTHIN = 16;
        /// See [BIP 0157](https://github.com/bitcoin/bips/blob/master/bip-0157.mediawiki)
        const NODE_COMPACT_FILTERS = 64;
        /// This is the same as NODE_NETWORK but the node has at least the last 288 blocks (last 2 days)
        const NODE_NETWORK_LIMITED = 1024;
    }
}

/// Network address of a node
///
/// Source: https://en.bitcoin.it/wiki/Protocol_documentation#version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkAddress {
    /// Services supported by the node encoded as a bitfield
    pub services: Services,
    /// IP address of the node
    pub ip_address: Ipv6Addr,
    /// Port number of the node
    pub port: u16,
}

impl NetworkAddress {
    fn new(services: Services, socket_address: SocketAddr) -> Self {
        Self {
            services,
            ip_address: match socket_address.ip() {
                IpAddr::V4(addr) => addr.to_ipv6_mapped(),
                IpAddr::V6(addr) => addr,
            },
            port: socket_address.port(),
        }
    }
}

impl Encode for NetworkAddress {
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(26);
        buffer.write_u64::<LittleEndian>(self.services.bits())?;
        buffer.write_all(&self.ip_address.octets()[..])?;
        buffer.write_u16::<BigEndian>(self.port)?;
        Ok(buffer)
    }
}

impl Decode for NetworkAddress {
    fn decode(bytes: &mut impl Read) -> anyhow::Result<Self> {
        let services = Services::from_bits_truncate(bytes.read_u64::<LittleEndian>()?);
        let ip_address = Ipv6Addr::from(bytes.read_u128::<BigEndian>()?);
        let port = bytes.read_u16::<BigEndian>()?;

        Ok(Self {
            services,
            ip_address,
            port,
        })
    }
}

/// The “version” message provides information about the transmitting node to the receiving node at the beginning of a connection.
/// Until both peers have exchanged “version” messages, no other messages will be accepted.
///
/// Source: https://developer.bitcoin.org/reference/p2p_networking.html#version
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionMessage {
    /// Highest protocol version understood by the transmitting node
    pub version: i32,
    /// Services supported by the transmitting node encoded as a bitfield
    pub services: Services,
    /// Current Unix time according to the transmitting node’s clock
    pub timestamp: i64,
    /// Receiving node as perceived by the transmitting node
    pub receiving_node: NetworkAddress,
    /// Transmitting Node
    pub transmitting_node: NetworkAddress,
    /// Random nonce which can help a node detect a connection to itself
    ///
    /// If the nonce is 0, the nonce field is ignored.
    /// If the nonce is anything else, a node should terminate the connection on receipt of a “version” message with a nonce it previously sent.
    pub nonce: u64,
    /// User agent as defined by [BIP14](https://github.com/bitcoin/bips/blob/master/bip-0014.mediawiki)
    pub user_agent: String,
    /// Height of the transmitting node’s best block
    pub start_height: i32,
    /// Whether the remote peer should announce relayed transactions or not, see (BIP 0037)[https://github.com/bitcoin/bips/blob/master/bip-0037.mediawiki]
    pub relay: bool,
}

impl VersionMessage {
    pub fn new(
        version: i32,
        services: Services,
        timestamp: i64,
        receiving_node_services: Services,
        receiving_node_address: SocketAddr,
        transmitting_address: SocketAddr,
        transmitting_node_services: Services,
        nonce: u64,
        user_agent: String,
        start_height: i32,
        relay: bool,
    ) -> Self {
        Self {
            version,
            services,
            timestamp,
            receiving_node: NetworkAddress::new(receiving_node_services, receiving_node_address),
            transmitting_node: NetworkAddress::new(
                transmitting_node_services,
                transmitting_address,
            ),
            nonce,
            user_agent,
            start_height,
            relay,
        }
    }
}

impl CommandName for VersionMessage {
    fn command_name() -> [u8; 12] {
        *b"version\x00\x00\x00\x00\x00"
    }
}
impl Encode for VersionMessage {
    fn encode(&self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(85 + self.user_agent.len());

        buffer.write_i32::<LittleEndian>(self.version)?;
        buffer.write_u64::<LittleEndian>(self.services.bits())?;
        buffer.write_i64::<LittleEndian>(self.timestamp)?;
        buffer.write_all(&self.receiving_node.encode()?)?;
        buffer.write_all(&self.transmitting_node.encode()?)?;
        buffer.write_u64::<LittleEndian>(self.nonce)?;
        buffer.write_u8(self.user_agent.len() as u8)?;
        buffer.write_all(&self.user_agent.as_bytes())?;
        buffer.write_i32::<LittleEndian>(self.start_height)?;
        buffer.write_u8(self.relay.into())?;

        Ok(buffer)
    }
}
impl Decode for VersionMessage {
    fn decode(bytes: &mut impl Read) -> anyhow::Result<Self> {
        let version = bytes.read_i32::<LittleEndian>()?;
        let services = Services::from_bits_truncate(bytes.read_u64::<LittleEndian>()?);
        let timestamp = bytes.read_i64::<LittleEndian>()?;

        let mut encoded_receiving_node = [0u8; 26];
        bytes.read_exact(&mut encoded_receiving_node)?;
        let receiving_node = NetworkAddress::decode(&mut encoded_receiving_node.as_slice())?;

        let mut encoded_transmitting_node = [0u8; 26];
        bytes.read_exact(&mut encoded_transmitting_node)?;
        let transmitting_node = NetworkAddress::decode(&mut encoded_transmitting_node.as_slice())?;

        let nonce = bytes.read_u64::<LittleEndian>()?;

        let user_agent_len = bytes.read_u8()?;
        let mut user_agent_bytes = vec![0u8; user_agent_len as usize];
        bytes.read_exact(&mut user_agent_bytes)?;
        let user_agent = String::from_utf8(user_agent_bytes)?;

        let start_height = bytes.read_i32::<LittleEndian>()?;
        let relay: bool = match bytes.read_u8()? {
            0 => false,
            1 => true,
            _ => return Err(anyhow!("Invalid relay encoding")),
        };

        Ok(Self {
            version,
            services,
            timestamp,
            receiving_node,
            transmitting_node,
            nonce,
            user_agent,
            start_height,
            relay,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_should_work() {
        let verack_message = VersionMessage {
            version: 60002,
            services: Services::NODE_NETWORK,
            timestamp: 1355854353,
            receiving_node: NetworkAddress {
                services: Services::NODE_NETWORK,
                ip_address: Ipv6Addr::from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0]),
                port: 0,
            },
            transmitting_node: NetworkAddress {
                services: Services::NODE_NETWORK,
                ip_address: Ipv6Addr::from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0]),
                port: 0,
            },
            nonce: 0x6517E68C5DB32E3B as u64,
            user_agent: "/Satoshi:0.7.2/".to_string(),
            start_height: 212672,
            relay: false,
        };

        assert_eq!(
            verack_message.encode().unwrap(),
            // Hexdump example of version message taken from https://en.bitcoin.it/wiki/Protocol_documentation#version
            hex::decode(
                "62EA0000010000000000000011B2D05000000000010000000000000000000000000000000000FFFF000000000000010000000000000000000000000000000000FFFF0000000000003B2EB35D8CE617650F2F5361746F7368693A302E372E322FC03E030000"
            ).unwrap()
        )
    }

    #[test]
    fn decode_should_work() {
        // Hexdump example of version message taken from https://developer.bitcoin.org/reference/p2p_networking.html#version
        let hex_string = "721101000100000000000000bc8f5e5400000000010000000000000000000000000000000000ffffc61b6409208d010000000000000000000000000000000000ffffcb0071c0208d128035cbc97953f80f2f5361746f7368693a302e392e332fcf05050001";
        let bytes = hex::decode(hex_string).unwrap();

        assert_eq!(
            VersionMessage::decode(&mut bytes.as_slice()).unwrap(),
            VersionMessage {
                version: 70002,
                services: Services::NODE_NETWORK,
                timestamp: 1415483324,
                receiving_node: NetworkAddress {
                    services: Services::NODE_NETWORK,
                    ip_address: Ipv6Addr::from([
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 198, 27, 100, 9
                    ]),
                    port: 8333,
                },
                transmitting_node: NetworkAddress {
                    services: Services::NODE_NETWORK,
                    ip_address: Ipv6Addr::from([
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 203, 0, 113, 192
                    ]),
                    port: 8333,
                },
                nonce: 0xf85379c9cb358012,
                user_agent: "/Satoshi:0.9.3/".to_string(),
                start_height: 329167,
                relay: true,
            }
        );
    }
}
