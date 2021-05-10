use std::{env, path::Path, thread, time::Duration};

use anyhow::{Context, Result};
use log::LevelFilter;
use penguin::{Config, Mount, ProxyTarget, Server};

use crate::args::{Args, DEFAULT_PORT, ServeOptions};



pub(crate) async fn run(
    proxy: Option<&ProxyTarget>,
    mounts: impl Iterator<Item = &Mount>,
    options: &ServeOptions,
    args: &Args,
) -> Result<()> {
    let bind_addr = (options.bind, args.port).into();
    let mut builder = Server::bind(bind_addr);

    for mount in mounts {
        builder = builder.add_mount(&mount.uri_path, &mount.fs_path)
            .context("failed to add mount")?;
    }
    if let Some(control_path) = &args.control_path {
        builder = builder.set_control_path(control_path);
    }
    if let Some(target) = proxy {
        builder = builder.proxy(target.clone())
    }


    let config = builder.validate().context("invalid penguin config")?;
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

    if args.open {
        // This is a bit hacky but it works and doing it properly is
        // surprisingly hard. We want to only open the browser if we were able
        // to start the server without problems (where 99% of anticipated
        // problems are: port is already in use). `hyper` doesn't quite allow us
        // to do that as far as I know. So we simply start a thread and wait a
        // bit. If starting the server errors, then the program (including this
        // thread) will be stopped quickly and the `open::that` call is never
        // executed.
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));

            let url = format!("http://{}", bind_addr);
            match open::that(url) {
                Ok(c) if c.success() => {}
                other => bunt::println!(
                    "{$yellow}Warning{/$}: couldn't open browser. Error: {:?}",
                    other,
                ),
            }
        });
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
