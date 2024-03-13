/// Bitcoin p2p protocol version used in this implementation
pub const PROTOCOL_VERSION: i32 = 70015;

pub const MAINNET_MAGIC_VALUE: [u8; 4] = [0xf9, 0xbe, 0xb4, 0xd9];
pub const REGNET_MAGIC_VALUE: [u8; 4] = [0xfa, 0xbf, 0xb5, 0xda];
pub const TESTNET3_MAGIC_VALUE: [u8; 4] = [0x0b, 0x11, 0x09, 0x07];
pub const SIGNET_MAGIC_VALUE: [u8; 4] = [0x0a, 0x0c, 0xcf, 0x40];
pub const NAMECOIN_MAGIC_VALUE: [u8; 4] = [0xf9, 0xbe, 0xb4, 0xfe];

/// Default port number for peers on the Bitcoin Mainnet (https://developer.bitcoin.org/reference/p2p_networking.html#constants-and-defaults)
pub const MAINNET_PORT_NUMBER: u16 = 8333;

/// Maximum allowed Payload size (https://developer.bitcoin.org/reference/p2p_networking.html#message-headers)
pub const MAX_PAYLOAD_SIZE: u32 = 32 * 1024 * 1024;
