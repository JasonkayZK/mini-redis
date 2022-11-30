//! mini-redis server.
//!
//! This file is the entry point for the server implemented in the library. It
//! performs command line parsing and passes the arguments on to
//! `mini_redis::server`.
//!
//! The `clap` crate is used for parsing arguments.

use clap::Parser;
use dotenv::dotenv;
use tokio::net::TcpListener;
use tokio::signal;

use mini_redis::{logger, server};
use mini_redis::consts::DEFAULT_PORT;
use mini_redis::error::MiniRedisServerError;

#[derive(Parser, Debug)]
#[clap(
name = "mini-redis-server",
version,
author,
about = "A mini redis server"
)]
struct Cli {
    #[clap(long)]
    port: Option<u16>,
}

#[tokio::main]
pub async fn main() -> Result<(), MiniRedisServerError> {
    let cli = init();
    let port = cli.port.unwrap_or(DEFAULT_PORT);

    // Bind a TCP listener
    let listener = TcpListener::bind(&format!("0.0.0.0:{}", port)).await?;

    server::run(listener, signal::ctrl_c()).await;

    Ok(())
}

fn init() -> Cli {
    dotenv().ok();
    logger::init();
    Cli::parse()
}
