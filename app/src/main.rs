use std::{env, iter};

use anyhow::{Context, Result};
use log::LevelFilter;
use penguin::{Mount, hyper::{Body, Client, Request}};
use structopt::StructOpt;

use crate::args::{Args, Command};

mod args;
mod server;


// A single thread runtime is plenty enough for a webserver purpose.
#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = run().await {
        let header = "An error occured :-(";
        let line = iter::repeat('━').take(header.len() + 4).collect::<String>();

        eprintln!();
        bunt::eprintln!(" {$yellow+intense}┏{}┓{/$}", line);
        bunt::eprintln!(" {$yellow+intense}┃{/$}  {[red+bold]}  {$yellow+intense}┃{/$}", header);
        bunt::eprintln!(" {$yellow+intense}┗{}┛{/$}", line);
        eprintln!();

        bunt::eprintln!("{[red+intense]}", e);
        if e.chain().count() > 1 {
            eprintln!();
            eprintln!("Caused by:");
            for cause in e.chain().skip(1) {
                bunt::eprintln!("   ‣ {}", cause);
            }
        }
    }
}

async fn run() -> Result<()> {
    if env::args().count() == 1 {
        print_welcome_message();
        return Ok(());
    }

    // Parse CLI arguments.
    let args = Args::from_args();

    init_logger(args.log_level);

    match &args.cmd {
        Command::Proxy { target, options } => {
            server::run(Some(target), options.mounts.iter(), options, &args)
                .await
                .context("failed to run server")?;
        }
        Command::Serve { path, options } => {
            let root_mount = path.clone().map(|p| Mount { uri_path: "/".into(), fs_path: p });
            let mounts = options.mounts.iter().chain(&root_mount);
            server::run(None, mounts, options, &args).await.context("failed to run server")?;
        }
        Command::Reload => reload(&args).await.context("failed to send reload request")?,
    }

    Ok(())
}

fn init_logger(level: LevelFilter) {
    if env::var("RUST_LOG") == Err(env::VarError::NotPresent) {
        env::set_var("RUST_LOG", format!("penguin={}", level));
    }

    pretty_env_logger::init();
}

async fn reload(args: &Args) -> Result<()> {
    // TODO: is '127.0.0.1' always valid? 'localhost' is not necessarily always
    // defined, right?
    let uri = format!(
        "http://127.0.0.1:{}{}/reload",
        args.port,
        args.control_path.as_deref().unwrap_or(penguin::DEFAULT_CONTROL_PATH),
    );

    let req = Request::builder()
        .method("POST")
        .uri(&uri)
        .body(Body::empty())
        .expect("bug: failed to build request");

    if !args.is_quiet() {
        bunt::println!("Sending POST request to {[green]}", uri);
    }

    let client = Client::new();
    client.request(req).await
        .with_context(|| format!("failed to send request to '{}'", uri))?;

    if !args.is_quiet() {
        bunt::println!("{$green+bold}✔ done{/$}");
    }

    Ok(())
}

fn print_welcome_message() {
    bunt::println!("{$blue+bold+intense}Penguin 🐧{/$}");
    println!();
    println!("You have to specify a subcommand. Example usages:");
    bunt::println!("   ‣ Serve a directory: {$yellow}penguin serve ./target{/$}");
    bunt::println!("   ‣ Reload all browser sessions: {$yellow}penguin reload{/$}");
    bunt::println!("   ‣ Forward requests to proxy and serve one directory on a subpath:");
    bunt::println!("         {$yellow}penguin proxy localhost:8000 -m /assets:frontend/dist{/$}");

    println!();
    bunt::println!("For more information, run {$yellow}penguin -h{/$} for a short CLI overview");
    bunt::println!("or {$yellow}penguin --help{/$} for a detailed description.");
}
