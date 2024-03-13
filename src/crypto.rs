//! Module that contains cryptographic operations

use sha2::{Digest, Sha256};

/// Computes the checksum (of the payload `payload`) that will be added to a message's header
///
/// Source: https://developer.bitcoin.org/reference/p2p_networking.html#message-headers
pub fn checksum(payload: &[u8]) -> [u8; 4] {
    let hash = Sha256::digest(Sha256::digest(payload));
    let mut buffer = [0u8; 4];
    buffer.copy_from_slice(&hash[..4]);
    buffer
}
