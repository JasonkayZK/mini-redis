use clap::Parser;
use dotenv::dotenv;
use log::debug;
use mini_redis::client::cmd::Command;

use mini_redis::consts::DEFAULT_PORT;
use mini_redis::logger;

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

fn main() {
    dotenv().ok();
    logger::init();
    debug!("get cli: ");

    // Parse command line arguments
    let cli = Cli::parse();

    // Get the remote address to connect to
    let addr = format!("{}:{}", cli.host, cli.port);
}
