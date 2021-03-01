use std::env;

use penguin::Mount;
use structopt::StructOpt;

use crate::args::{Args, Command};

mod args;
mod server;


// A single thread runtime is plenty enough for a webserver purpose.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments.
    let args = Args::from_args();

    init_logger();

    match &args.cmd {
        Command::Proxy { target, options } => {
            server::run(Some(target), options.mounts.iter(), options, &args).await?;
        }
        Command::Serve { path, options } => {
            let root_mount = path.clone().map(|p| Mount { uri_path: "/".into(), fs_path: p });
            let mounts = options.mounts.iter().chain(&root_mount);
            server::run(None, mounts, options, &args).await?;
        }
    }

    Ok(())
}

fn init_logger() {
    if env::var("RUST_LOG") == Err(env::VarError::NotPresent) {
        env::set_var("RUST_LOG", "penguin=warn");
    }

    pretty_env_logger::init();
}
