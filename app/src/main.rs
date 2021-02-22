use penguin::Config;
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
    let mut config = Config::new(bind_addr);
    match args.cmd {
        Command::Proxy { target } => config = config.proxy(target),
        Command::Serve { path } => {
            if let Some(path) = path {
                config = config.add_serve_dir("/", &path);
            } else if args.mounts.is_empty() {
                bunt::eprintln!(
                    "{$red+bold}error:{/$} neither serve path nor '--mount' arguments \
                        given, but at least one path has to be specified!"
                );
                std::process::exit(1);
            }

            for mount in &args.mounts {
                config = config.add_serve_dir(&mount.uri_path, &mount.fs_path);
            }
        },
    }

    let (_controller, serve) = penguin::serve(config)?;

    bunt::println!("Penguin 🐧 is listening on {$yellow+intense+bold}http://{}{/$}", bind_addr);
    serve.await?;

    Ok(())
}
