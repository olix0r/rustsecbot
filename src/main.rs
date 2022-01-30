#![deny(warnings, rust_2018_idioms)]

use anyhow::Result;
use clap::Parser;
use rustsec_issues::{deny, Client};
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
#[clap(version)]
struct Args {
    #[clap(default_value = "cargo-deny", env, long, parse(from_os_str))]
    cargo_deny_path: PathBuf,

    #[clap(default_value = ".", long, parse(from_os_str), short = 'd')]
    directory: PathBuf,

    #[clap(env, long, short = 'o')]
    github_organization: String,

    #[clap(env, long, short = 'r')]
    github_repository: String,

    #[clap(env, long, short = 't')]
    github_token: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let Args {
        cargo_deny_path,
        directory,
        github_organization,
        github_repository,
        github_token,
    } = Args::parse();

    // Build a rate-limited GitHub API client.
    let github = Client::spawn_rate_limited(github_token)?;

    // Ensure that the target directory contains a Cargo.lock. Otherwise there's no point in running
    // cargo-deny.
    ensure_exists(directory.join("Cargo.lock")).await?;

    let (advisories, open_issues) = tokio::try_join!(
        // Run cargo-deny to determine the advisories for the given crate.
        deny::advisories(cargo_deny_path, directory),
        // Get the list of already-oopened issues with the expected labels.
        github.list_issues(&github_organization, &github_repository)
    )?;

    github
        .create_issues(
            &github_organization,
            &github_repository,
            open_issues,
            advisories,
        )
        .await?;

    Ok(())
}

async fn ensure_exists(path: PathBuf) -> Result<()> {
    if let Err(e) = tokio::fs::metadata(&path).await {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow::bail!("{} not found", path.display());
        } else {
            anyhow::bail!("failed to read {}: {}", path.display(), e);
        }
    }

    Ok(())
}
