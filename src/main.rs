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
    ensure_file(directory.join("Cargo.lock")).await?;

    // Before checking advisories get the list of already-opened issues with the expected labels.,
    let open_issues = github
        .list_issues(&github_organization, &github_repository)
        .await?;

    // Run cargo-deny to determine the advisories for the given crate.pen_issues
    let mut advisories = deny::advisories(cargo_deny_path, directory).await?;

    // Remove any advisories that have already been reported by comparing issue titles.
    advisories.retain(|a| {
        let title = a.title();
        !open_issues.iter().any(|i| i.title == title)
    });

    // Create a new issue for each advisory that hasn't previously been reported.
    github
        .create_issues(&github_organization, &github_repository, advisories)
        .await?;

    Ok(())
}

// Errors if the specified path is not a file.
async fn ensure_file(path: PathBuf) -> Result<()> {
    match tokio::fs::metadata(&path).await {
        Ok(m) if m.is_file() => Ok(()),
        Ok(_) => anyhow::bail!("{} is not a file", path.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!("{} not found", path.display());
        }
        Err(e) => anyhow::bail!("failed to read {}: {}", path.display(), e),
    }
}
