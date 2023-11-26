use std::{net::IpAddr, path::{Path, PathBuf}, time::Duration};

use log::LevelFilter;

use structopt::StructOpt;
use penguin::{Mount, ProxyTarget};

pub(crate) const DEFAULT_PORT: u16 = 4090;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Penguin",
    about = "Language-agnostic dev server that can serve directories and forward \
        requests to a proxy.",
    setting(structopt::clap::AppSettings::VersionlessSubcommands),
)]
pub(crate) struct Args {
    /// Port of the Penguin server.
    #[structopt(short, long, default_value = "4090", global = true)]
    pub(crate) port: u16,

    /// Address to bind to.
    ///
    /// Mostly useful to set to "0.0.0.0" to let other devices in your network
    /// access the server.
    #[structopt(long, default_value = "127.0.0.1", global = true)]
    pub(crate) bind: IpAddr,

    /// Overrides the default control path '/~~penguin' with a custom path.
    ///
    /// Only useful you need to use '/~~penguin' in your own application.
    #[structopt(long, global = true)]
    pub(crate) control_path: Option<String>,

    /// Quiet: `-q` for less output, `-qq` for no output.
    #[structopt(short, global = true, parse(from_occurrences))]
    pub(crate) quiet: u8,

    /// Sets the log level: trace, debug, info, warn, error or off.
    ///
    /// This value is only used if `RUST_LOG` is NOT set. If it is, the
    /// environment variable controls everything.
    #[structopt(short, long, global = true, default_value = "warn")]
    pub(crate) log_level: LevelFilter,

    /// Automatically opens the browser with the URL of this server.
    #[structopt(long, global = true)]
    pub(crate) open: bool,

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
    ///
    /// Like with `--mount`, the directory specified here will be watched for
    /// file changes to automatically reload browser sessions. You can disable
    /// that with `--no-auto-watch`.
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

    /// Reloads all browser sessions.
    ///
    /// This sends a reload request to a locally running penguin server. The
    /// port and control path can be specified, if they are non-standard.
    Reload,
}

#[derive(Debug, Clone, StructOpt)]
pub(crate) struct ServeOptions {
    /// Mount a directory on an URI path: '--mount <uri>:<path>'.
    ///
    /// Example: '--mount assets:/home/peter/images'. Can be specified multiple
    /// times. If you only want to mount one directory in the root, rather use
    /// the `penguin serve` subcommand.
    ///
    /// By default, directories specified here will be watched for file changes
    /// to automatically reload browser sessions. You can disable that with
    /// `--no-auto-watch`.
    #[structopt(
        short,
        long = "--mount",
        number_of_values = 1,
        parse(try_from_str = parse_mount),
    )]
    pub(crate) mounts: Vec<Mount>,

    /// When specified, penguin will not automatically watch the mounted paths.
    #[structopt(long)]
    pub(crate) no_auto_watch: bool,

    /// Watch a path for file system changes, triggering a reload.
    ///
    /// Note that mounted paths are already watched by default.
    #[structopt(short, long = "--watch", number_of_values = 1)]
    pub(crate) watched_paths: Vec<PathBuf>,

    /// The debounce duration (in ms) for watching paths.
    ///
    /// Debouncing means that if a watch-event arrived, we are not immediately
    /// triggering a reload. Instead we wait for this duration and see if any
    /// other events arrive during this period. Whenever an event arrives, we
    /// reset the timer (so we could wait indefinitely).
    #[structopt(
        long = "--debounce",
        default_value = "200",
        parse(try_from_str = parse_duration)
    )]
    pub(crate) debounce_duration: Duration,

    /// The debounce duration (in ms) for when a file in a watched path was
    /// removed.
    ///
    /// Like `--debounce`, but for "remove" file system events. This is treated
    /// separately as usually reloading quickly on deletion is not useful: the
    /// reload would result in a 404 page. And in many situation (e.g. cargo
    /// doc) the watched directory is first wiped by a build process and then
    /// populated again after a while. So this default is much higher.
    #[structopt(
        long = "--removal-debounce",
        default_value = "3000",
        parse(try_from_str = parse_duration)
    )]
    pub(crate) removal_debounce_duration: Duration,
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

fn parse_duration(s: &str) -> Result<Duration, &'static str> {
    let ms = s.parse::<u64>().map_err(|_| "failed to parse as positive integer")?;
    Ok(Duration::from_millis(ms))
}

impl Args {
    pub(crate) fn is_quiet(&self) -> bool {
        self.quiet > 0
    }

    pub(crate) fn is_muted(&self) -> bool {
        self.quiet == 2
    }
}
