use anyhow::anyhow;
use bitcoin_p2p::{
    constants::{MAINNET_PORT_NUMBER, PROTOCOL_VERSION},
    messages::{
        codec::{Decode, Encode},
        types::{
            verack::VerackMessage,
            version::{Services, VersionMessage},
        },
        Chain, Message,
    },
};
use clap::Parser;
use futures::{stream::FuturesUnordered, StreamExt};
use std::str::FromStr;
use std::{
    net::SocketAddr,
    time::{Duration, SystemTime},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{lookup_host, TcpStream},
    time::timeout,
};

fn parse_services(services_bits: &str) -> anyhow::Result<Services> {
    let services_bits: u64 = services_bits.parse()?;
    Ok(Services::from_bits_truncate(services_bits))
}
fn parse_timeout(seconds: &str) -> anyhow::Result<Duration> {
    Ok(Duration::from_secs(seconds.parse()?))
}
fn parse_chain(chain: &str) -> anyhow::Result<Chain> {
    Chain::from_str(chain)
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct HandshakeCli {
    /// Bitcoin DNS Seed that is queried
    pub dns_seed: String,
    /// The Bitcoin Network to connect to
    #[arg(short, long, value_parser = parse_chain, default_value = "mainnet")]
    pub chain: Chain,
    /// Port Number of the Receiving Node
    #[arg(short, long, default_value_t = MAINNET_PORT_NUMBER)]
    pub port: u16,
    /// Services supported by the transmitting node encoded as a 64-bit bitfield
    #[arg(short, long, value_parser = parse_services, default_value = "0")]
    pub services: Services,
    /// Services supported by the receiving node encoded as a 64-bit bitfield
    #[arg(short, long, value_parser = parse_services, default_value = "0")]
    pub receiving_services: Services,
    /// Maximum duration (in seconds) to perform the handshake in
    #[arg(short, long, value_parser = parse_timeout, default_value = "10")]
    pub timeout: Duration,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .init();

    let cli: HandshakeCli = HandshakeCli::parse();

    let socket_addresses: Vec<SocketAddr> = lookup_host((cli.dns_seed, cli.port)).await?.collect();

    let (mut success, mut failure) = (0u32, 0u32);

    // Since we do need the output of the list of futures to be in-order, it is more efficient to use `FuturesUnordered` than `futures::futures::future::join_all()`
    let mut timeout_futures: FuturesUnordered<_> = socket_addresses
        .into_iter()
        .map(|addr| {
            timeout(
                cli.timeout,
                perform_handshake(cli.chain, cli.services, cli.receiving_services, addr),
            )
        })
        .collect();

    while let Some(result) = timeout_futures.next().await {
        match result {
            Ok(Ok(_)) => {
                tracing::info!("Handshake succeeded!");
                success += 1;
            }
            Ok(Err(e)) => {
                tracing::info!("Handshake failed with error: {}", e);
                failure += 1;
            }
            Err(e) => {
                tracing::info!("Handshake timed out with error: {}", e);
                failure += 1;
            }
        }
    }

    tracing::info!("Handshake Success Count: {success}");
    tracing::info!("Handshake Failure Count: {failure}");

    Ok(())
}

async fn perform_handshake(
    chain: Chain,
    services: Services,
    receiving_services: Services,
    socket_address: SocketAddr,
) -> anyhow::Result<()> {
    let mut tcp_stream = TcpStream::connect(socket_address).await?;
    exchange_version_message(chain, services, receiving_services, &mut tcp_stream).await?;
    exchange_verack_message(chain, &mut tcp_stream).await?;
    Ok(())
}

async fn exchange_version_message(
    chain: Chain,
    services: Services,
    receiving_services: Services,
    tcp_stream: &mut TcpStream,
) -> anyhow::Result<()> {
    let version_message = VersionMessage::new(
        PROTOCOL_VERSION,
        services,
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs() as i64,
        receiving_services,
        tcp_stream.peer_addr()?,
        tcp_stream.local_addr()?,
        services,
        rand::random(),
        format!(""),
        0,
        false,
    );
    let message = Message::<VersionMessage>::new(chain, version_message);
    tcp_stream.write_all(&message.encode()?).await?;

    let mut buffer_reader = BufReader::new(tcp_stream);
    let mut bytes = buffer_reader.fill_buf().await?;
    let received_message = Message::<VersionMessage>::decode(&mut bytes)?;
    let bytes_len = bytes.len();
    buffer_reader.consume(bytes_len);

    if received_message.chain != chain {
        return Err(anyhow!("Invalid Bitcoin Network"));
    }

    Ok(())
}

async fn exchange_verack_message(chain: Chain, tcp_stream: &mut TcpStream) -> anyhow::Result<()> {
    let verack_message = VerackMessage;
    let message = Message::<VerackMessage>::new(chain, verack_message);
    tcp_stream.write_all(&message.encode()?).await?;

    let mut buffer_reader = BufReader::new(tcp_stream);
    let mut bytes = buffer_reader.fill_buf().await?;
    let bytes_len = bytes.len();
    if bytes_len == 0 {
        tracing::info!("VERACK message was not exchanged by peer");
        return Ok(());
    }
    let received_message = Message::<VerackMessage>::decode(&mut bytes)?;
    buffer_reader.consume(bytes_len);

    if received_message.chain != chain {
        return Err(anyhow!("Invalid Bitcoin Network!"));
    }

    Ok(())
}
