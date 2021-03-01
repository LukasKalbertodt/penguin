use std::{net::IpAddr, path::{Path, PathBuf}};

use structopt::StructOpt;
use penguin::{Mount, ProxyTarget};


#[derive(Debug, StructOpt)]
#[structopt(
    name = "Penguin",
    about = "Language-agnostic dev server that can serve directories and forward \
        requests to a proxy.",
    setting(structopt::clap::AppSettings::VersionlessSubcommands),
)]
pub(crate) struct Args {
    /// The port that the Penguin server listens on.
    #[structopt(short, long, default_value = "4090", global = true)]
    pub(crate) port: u16,

    /// Overrides the default control path '/~~penguin' with a custom path.
    ///
    /// Only useful you need to use '/~~penguin' in your own application.
    #[structopt(long, global = true)]
    pub(crate) control_path: Option<String>,

    #[structopt(subcommand)]
    pub(crate) cmd: Command,
}

#[derive(Debug, StructOpt)]
pub(crate) enum Command {
    /// Serve the specified directory as file server.
    ///
    /// You can mount more directories via '--mount'. If you don't specify a
    /// main directory for this subcommand, you have to mount at least one
    /// directory via '--mount'.
    Serve {
        #[structopt(parse(from_os_str))]
        path: Option<PathBuf>,

        #[structopt(flatten)]
        options: ServeOptions,
    },

    /// Starts a server forwarding all request to the specified target address.
    Proxy {
        target: ProxyTarget,

        #[structopt(flatten)]
        options: ServeOptions,
    },
}

#[derive(Debug, StructOpt)]
pub(crate) struct ServeOptions {
    /// Address to bind to.
    ///
    /// Mostly useful to set to "0.0.0.0" to let other
    /// devices in your network access the server.
    #[structopt(long, default_value = "127.0.0.1")]
    pub(crate) bind: IpAddr,

    /// Mount a directory on an URI path: '--mount <uri>:<path>'.
    ///
    /// Example: '--mount assets:/home/peter/images'. Can be specified multiple
    /// times. If you only want to mount one directory in the root, rather use
    /// the `penguin serve` subcommand.
    #[structopt(
        short,
        long = "--mount",
        number_of_values = 1,
        parse(try_from_str = parse_mount),
    )]
    pub(crate) mounts: Vec<Mount>,
}

fn parse_mount(s: &str) -> Result<Mount, &'static str> {
    let colon_pos = s.find(':').ok_or("does not contain a colon")?;
    let fs_path = Path::new(&s[colon_pos + 1..]).to_owned();

    let mut uri_path = s[..colon_pos].to_owned();
    if !uri_path.starts_with('/') {
        uri_path.insert(0, '/');
    }
    if uri_path.ends_with('/') && uri_path.len() > 1 {
        uri_path.pop();
    }

    Ok(Mount { uri_path, fs_path})
}
