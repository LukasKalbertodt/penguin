use std::{env, path::Path};

use penguin::{Config, Server};
use structopt::StructOpt;

use crate::args::{Args, Command};

mod args;


// A single thread runtime is plenty enough for a webserver purpose.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments.
    let args = Args::from_args();

    init_logger();

    // Build Penguin configuration from arguments.
    let bind_addr = (args.bind, args.port).into();
    let mut builder = Server::bind(bind_addr);
    for mount in &args.mounts {
        builder = builder.add_mount(&mount.uri_path, &mount.fs_path)?;
    }
    if let Some(control_path) = args.control_path {
        builder = builder.set_control_path(control_path);
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

    let config = builder.validate()?;
    let (server, _controller) = Server::build(config.clone());

    // Nice output of what is being done
    bunt::println!(
        "{$bold}Penguin started!{/$} Listening on {$yellow+intense+bold}http://{}{/$}",
        bind_addr,
    );
    pretty_print_config(&config);

    server.await?;

    Ok(())
}

fn init_logger() {
    if env::var("RUST_LOG") == Err(env::VarError::NotPresent) {
        env::set_var("RUST_LOG", "penguin=warn");
    }

    pretty_env_logger::init();
}

fn pretty_print_config(config: &Config) {
    println!();
    bunt::println!("   {$cyan+bold}▸ Routing:{/$}");
    bunt::println!(
        "     ├╴ Requests to {[blue+intense]} are handled internally by penguin",
        config.control_path(),
    );

    for mount in config.mounts() {
        let fs_path = env::current_dir()
            .as_deref()
            .unwrap_or(Path::new("."))
            .join(&mount.fs_path);

        bunt::println!(
            "     ├╴ Requests to {[blue+intense]} are served from the directory {[green]}",
            mount.uri_path,
            fs_path.display(),
        );
    }

    if let Some(proxy) = config.proxy() {
        bunt::println!("     ╰╴ All remaining requests are forwarded to {[green+intense]}", proxy);
    } else {
        bunt::println!("     ╰╴ All remaining requests will be responded to with 404");
    }
    println!();
}
