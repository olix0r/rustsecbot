use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
#[clap(version)]
struct Args {
    #[clap(env, long, parse(from_os_str))]
    cargo_deny_path: PathBuf,
    #[clap(env, long)]
    github_token: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let Args {
        cargo_deny_path: _,
        github_token,
    } = Args::parse();

    let _github = hubcaps::Github::new(
        format!("{}/{}", clap::crate_name!(), clap::crate_version!()),
        hubcaps::Credentials::Token(github_token),
    );

    Ok(())
}
