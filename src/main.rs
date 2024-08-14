mod tailscale;

use anyhow::Context;
use clap::Parser;
use std::{path::PathBuf, time::Duration};
use tailscale::TailscaleStatus;
use tokio::{select, time::sleep};
use tracing::{error, info};
use zenoh::prelude::r#async::*;

/// Selected with a random dice roll
const TCP_DISCOVERY_PORT: u16 = 7436;

#[derive(Parser, Debug)]
#[command(version, author, about)]
struct Cli {
    #[arg(long)]
    zenoh_config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();
    let args = Cli::parse();

    let mut tailscale_info = TailscaleStatus::read_from_command().await?;

    let zenoh_config = build_zenoh_config(&tailscale_info, args.zenoh_config.clone())?;
    let mut zenoh_session = zenoh::open(zenoh_config)
        .res()
        .await
        .map_err(ErrorWrapper::ZenohError)?;
    print_session_info(&zenoh_session).await;
    loop {
        select! {
            _ = sleep(Duration::from_secs(10)) => {
                // nothing
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl+C detected. Exiting");
                break;
            }
        }

        let new_tailscale_info = TailscaleStatus::read_from_command().await?;
        if new_tailscale_info != tailscale_info {
            info!("Tailscale config changed. Reloading");

            // close old session
            zenoh_session
                .close()
                .res()
                .await
                .map_err(ErrorWrapper::ZenohError)?;

            tailscale_info = new_tailscale_info;

            let zenoh_config = build_zenoh_config(&tailscale_info, args.zenoh_config.clone())?;
            zenoh_session = zenoh::open(zenoh_config)
                .res()
                .await
                .map_err(ErrorWrapper::ZenohError)?;

            print_session_info(&zenoh_session).await;
        }
    }
    zenoh_session
        .close()
        .res()
        .await
        .map_err(ErrorWrapper::ZenohError)?;

    Ok(())
}

async fn print_session_info(zenoh_session: &Session) {
    let info = zenoh_session.info();
    info!("Session zid: {:?}", info.zid().res().await);
    info!(
        "Routers zid: {:?}",
        info.routers_zid().res().await.collect::<Vec<ZenohId>>()
    );
    info!(
        "Peers zid: {:?}",
        info.peers_zid().res().await.collect::<Vec<ZenohId>>()
    );
}

fn build_zenoh_config(
    tailscale_status: &TailscaleStatus,
    path: Option<PathBuf>,
) -> anyhow::Result<zenoh::config::Config> {
    let mut config = if let Some(conf_file) = path {
        zenoh::config::Config::from_file(conf_file).map_err(ErrorWrapper::ZenohError)?
    } else {
        zenoh::config::Config::default()
    };

    if config.scouting.gossip.set_multihop(Some(true)).is_err() {
        error!("Failed to set multihop");
        anyhow::bail!("Failed to set multihop")
    }

    let mut listening_addresses = vec![];
    for local_address in &tailscale_status.tailscale_ip_list {
        let address: std::net::IpAddr = local_address.parse().context("Failed to parse address")?;
        if !address.is_ipv4() {
            // skip IPv6 because pain
            continue;
        }
        let tcp = zenoh_config::EndPoint::new(
            "tcp",
            format!("{}:{}", local_address, TCP_DISCOVERY_PORT),
            "",
            "",
        )
        .map_err(ErrorWrapper::ZenohError)?;

        listening_addresses.push(tcp);
    }

    info!("Listening addresses {listening_addresses:?}");

    config.listen.endpoints.extend(listening_addresses);

    let mut peer_addresses = vec![];
    for peer in tailscale_status.peers.values() {
        for peer_address in &peer.tailscale_ip_list {
            let tmp = build_peer_endpoints_for_address(peer_address)?;
            peer_addresses.extend(tmp);
        }
    }

    info!("Peer addresses {peer_addresses:?}");

    config.connect.endpoints.extend(peer_addresses);

    Ok(config)
}

fn build_peer_endpoints_for_address(address: &str) -> anyhow::Result<Vec<zenoh_config::EndPoint>> {
    let parsed_address: std::net::IpAddr = address.parse().context("Failed to parse address")?;
    if !parsed_address.is_ipv4() {
        // skip IPv6 because pain
        return Ok(vec![]);
    }
    let endpoints = vec![zenoh_config::EndPoint::new(
        "tcp",
        format!("{}:{}", address, TCP_DISCOVERY_PORT),
        "",
        "",
    )
    .map_err(ErrorWrapper::ZenohError)?];

    Ok(endpoints)
}

pub fn setup_tracing() {
    tracing_subscriber::fmt()
        .pretty()
        .with_thread_names(true)
        .with_max_level(tracing::Level::INFO)
        .init();
}

#[derive(thiserror::Error, Debug)]
pub enum ErrorWrapper {
    #[error("Zenoh error {0:?}")]
    ZenohError(#[from] zenoh::Error),
}
