use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Config path
    pub path: String,

    /// Api url
    pub api_url: String,

    /// Docker socket path
    #[clap(long, short, default_value = "unix:///var/run/docker.sock")]
    pub socket: String,
}
