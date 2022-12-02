use clap::Parser;
use dotenv::dotenv;
use log::debug;

use mini_redis::client::cmd::Command;
use mini_redis::consts::DEFAULT_PORT;
use mini_redis::error::{MiniRedisClientError, MiniRedisConnectionError};
use mini_redis::{client, logger};

#[derive(Parser, Debug)]
#[clap(
    name = "mini-redis-cli",
    version,
    author,
    about = "Issue Redis commands"
)]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    #[clap(name = "hostname", long, default_value = "127.0.0.1")]
    host: String,

    #[clap(long, default_value_t = DEFAULT_PORT)]
    port: u16,
}

/// Entry point for CLI tool.
///
/// The `[tokio::main]` annotation signals that the Tokio runtime should be
/// started when the function is called. The body of the function is executed
/// within the newly spawned runtime.
///
/// `flavor = "current_thread"` is used here to avoid spawning background
/// threads. The CLI tool use case benefits more by being lighter instead of
/// multi-threaded.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), MiniRedisClientError> {
    dotenv().ok();
    logger::init();

    // Parse command line arguments
    let cli = Cli::parse();
    debug!("get cli: {:?}", cli);

    // Get the remote address to connect to
    let addr = format!("{}:{}", cli.host, cli.port);

    // Establish a connection
    let mut client = client::connect(&addr).await?;

    // Process the requested command
    match cli.command {
        Command::Ping { msg } => {
            let value = client.ping(msg).await?;
            if let Ok(string) = std::str::from_utf8(&value) {
                println!("\"{}\"", string);
            } else {
                println!("{:?}", value);
            }
        }
        Command::Get { key } => {
            if let Some(value) = client.get(&key).await? {
                if let Ok(string) = std::str::from_utf8(&value) {
                    println!("\"{}\"", string);
                } else {
                    println!("{:?}", value);
                }
            } else {
                println!("(nil)");
            }
        }
        Command::Set {
            key,
            value,
            expires: None,
        } => {
            client.set(&key, value).await?;
            println!("OK");
        }
        Command::Set {
            key,
            value,
            expires: Some(expires),
        } => {
            client.set_expires(&key, value, expires).await?;
            println!("OK");
        }
        Command::Publish { channel, message } => {
            client.publish(&channel, message).await?;
            println!("Publish OK");
        }
        Command::Subscribe { channels } => {
            if channels.is_empty() {
                return Err(MiniRedisConnectionError::InvalidArgument(
                    "channel(s) must be provided".into(),
                )
                .into());
            }
            let mut subscriber = client.subscribe(channels).await?;

            // await messages on channels
            while let Some(msg) = subscriber.next_message().await? {
                println!(
                    "got message from the channel: {}; message = {:?}",
                    msg.channel, msg.content
                );
            }
        }
    }

    Ok(())
}
