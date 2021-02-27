use penguin::Server;
use structopt::StructOpt;

use crate::args::{Args, Command};

mod args;


// A single thread runtime is plenty enough for a webserver purpose.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments.
    let args = Args::from_args();

    // Build Penguin configuration from arguments.
    let bind_addr = (args.bind, args.port).into();
    let mut builder = Server::bind(bind_addr);
    for mount in &args.mounts {
        builder = builder.add_mount(&mount.uri_path, &mount.fs_path)?;
    }
    match args.cmd {
        Command::Proxy { target } => builder = builder.proxy(target),
        Command::Serve { path: Some(path) } => builder = builder.add_mount("/", &path)?,
        Command::Serve { path: None } => {
            if args.mounts.is_empty() {
                bunt::eprintln!(
                    "{$red+bold}error:{/$} neither serve path nor '--mount' arguments \
                        given, but at least one path has to be specified!"
                );
                std::process::exit(1);
            }
        },
    }

    let (server, _controller) = builder.build()?;

    bunt::println!("Penguin 🐧 is listening on {$yellow+intense+bold}http://{}{/$}", bind_addr);
    server.await?;

    Ok(())
}
