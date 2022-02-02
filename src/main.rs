#![deny(warnings, rust_2018_idioms)]

use anyhow::Result;
use clap::Parser;
use rustsecbot::{deny, Client, GitHubRepo, Labels};
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
#[clap(version)]
struct Args {
    #[clap(default_value = "cargo-deny", env, long, parse(from_os_str))]
    cargo_deny_path: PathBuf,

    /// Print advisories and exit without creating issues.
    #[clap(default_value = "report", long)]
    mode: Mode,

    #[clap(default_value = ".", long, parse(from_os_str), short = 'd')]
    directory: PathBuf,

    /// Labels that apply to issues created by this tool.
    #[clap(default_value = "rust,security", long)]
    labels: Labels,

    #[clap(env, long, short = 'r')]
    github_repository: GitHubRepo,

    #[clap(env, long, short = 't')]
    github_token: String,
}

#[derive(Copy, Clone, Debug)]
enum Mode {
    Check,
    Report,
}

impl std::str::FromStr for Mode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match &*s.to_ascii_lowercase() {
            "check" => Ok(Self::Check),
            "report" => Ok(Self::Report),
            _ => Err(anyhow::anyhow!("invalid mode: {}", s)),
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let Args {
        cargo_deny_path,
        directory,
        github_repository,
        github_token,
        labels,
        mode,
    } = Args::parse();

    // Build a rate-limited GitHub API client.
    let github = Client::spawn_rate_limited(github_token).await?;

    // Before checking advisories get the list of already-opened issues with the expected labels.,
    let open_issues = github
        .list_issues(&github_repository, labels.clone())
        .await?;
    println!("{} open issues", open_issues.len());
    for i in &open_issues {
        println!("  {}: {}", i.id, i.title);
    }

    // Run cargo-deny to determine the advisories for the given crate.pen_issues
    let mut advisories = deny::advisories(cargo_deny_path, directory).await?;
    println!("found {} active advisories", advisories.len());

    // Remove any advisories that have already been reported by comparing issue titles.
    advisories.retain(|a| !open_issues.iter().any(|i| i.title == a.title));

    if let Mode::Check = mode {
        // Skip creating issues.
        if advisories.is_empty() {
            return Ok(());
        }

        eprintln!("{} new advisories", advisories.len());
        for a in &advisories {
            eprintln!("  {}", a.title);
        }
        std::process::exit(1);
    }

    println!("{} new advisories", advisories.len());
    for a in &advisories {
        println!("  {}", a.title);
    }

    // Create a new issue for each advisory that hasn't previously been reported.
    let opened = github
        .create_issues(&github_repository, advisories, labels)
        .await?;

    println!("::group::{} new issues", opened.len());
    for (i, _) in &opened {
        println!("Opened {}: {}", i.id, i.title);
    }
    println!("::endgroup");

    println!(
        "::set-output name=opened::{}",
        opened
            .iter()
            .map(|(i, a)| format!("{}:{}", i.number, a.id))
            .collect::<Vec<_>>()
            .join(",")
    );

    // We do not try to close issues that are no longer relevant, since we may acknowledge open
    // issues by adding them to deny.toml (which removes them from the report); but we wouldn't want
    // to close these issues until they're removed from deny.toml.

    Ok(())
}
