use std::{env, path::Path};

use log::LevelFilter;
use penguin::{Config, Mount, ProxyTarget, Server};

use crate::args::{Args, DEFAULT_PORT, ServeOptions};



pub(crate) async fn run(
    proxy: Option<&ProxyTarget>,
    mounts: impl Iterator<Item = &Mount>,
    options: &ServeOptions,
    args: &Args,
) -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr = (options.bind, args.port).into();
    let mut builder = Server::bind(bind_addr);

    for mount in mounts {
        builder = builder.add_mount(&mount.uri_path, &mount.fs_path)?;
    }
    if let Some(control_path) = &args.control_path {
        builder = builder.set_control_path(control_path);
    }
    if let Some(target) = proxy {
        builder = builder.proxy(target.clone())
    }


    let config = builder.validate()?;
    let (server, _controller) = Server::build(config.clone());

    // Nice output of what is being done
    if !args.is_muted() {
        bunt::println!(
            "{$bold}Penguin started!{/$} Listening on {$yellow+intense+bold}http://{}{/$}",
            bind_addr,
        );

        if !args.is_quiet() {
            pretty_print_config(&config, args);
        }
    }

    server.await?;

    Ok(())
}

fn pretty_print_config(config: &Config, args: &Args) {
    // Routing description
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

    // Random hints
    println!();
    bunt::println!("   {$cyan+bold}▸ Hints:{/$}");
    bunt::println!(
        "     • To reload all browser sessions, run {$yellow}penguin reload{}{}{/$}",
        if args.port != DEFAULT_PORT { format!(" -p {}", args.port) } else { "".into() },
        args.control_path.as_ref()
            .map(|p| format!(" --control-path {}", p))
            .unwrap_or_default(),
    );
    if args.log_level == LevelFilter::Warn {
        bunt::println!(
            "     • For more log output use {$yellow}-l trace{/$} \
                or set the env variable {$yellow}RUST_LOG{/$}",
        );
    }

    println!();
}
