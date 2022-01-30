#![deny(warnings, rust_2018_idioms)]

use anyhow::{Context, Result};
use clap::Parser;
use rustsec_issues::deny;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;

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

    let advisories = deny::advisories(cargo_deny_path, directory)
        .await?
        .into_iter()
        .filter_map(|d| match d {
            deny::Object::Diagnostic(d) => Some(Advisory::from(d)),
            _ => None,
        })
        .collect::<Vec<Advisory>>();

    let client = RateLimited::spawned(github_token)?;

    let open_issues = client
        .list_issues(&github_organization, &github_repository)
        .await?;

    client
        .create_issues(
            &github_organization,
            &github_repository,
            open_issues,
            advisories,
        )
        .await?;

    Ok(())
}

#[derive(Clone, Debug)]
struct RateLimited(Arc<Semaphore>, hubcaps::Github);

impl RateLimited {
    fn spawned(token: String) -> Result<Self> {
        let gh = hubcaps::Github::new(
            format!("{}/{}", clap::crate_name!(), clap::crate_version!()),
            hubcaps::Credentials::Token(token),
        )?;
        let semaphore = Arc::new(Semaphore::new(0));

        tokio::spawn({
            let gh = gh.clone();
            let handle = Arc::downgrade(&semaphore);
            async move {
                while let Some(semaphore) = handle.upgrade() {
                    let rate_limit = gh.rate_limit().get().await?;
                    let new_perms = (rate_limit.resources.core.remaining as usize)
                        .saturating_sub(semaphore.available_permits());
                    semaphore.add_permits(new_perms);
                    let rate_limit_reset = std::time::SystemTime::UNIX_EPOCH
                        + std::time::Duration::from_secs(rate_limit.resources.core.reset as u64);
                    drop(semaphore);
                    if let Ok(t) = rate_limit_reset.duration_since(std::time::SystemTime::now()) {
                        tokio::time::sleep(t).await;
                    }
                }
                anyhow::Ok(())
            }
        });

        Ok(Self(semaphore, gh))
    }

    async fn list_issues(&self, org: &str, repo: &str) -> Result<Vec<hubcaps::issues::Issue>> {
        let opts = hubcaps::issues::IssueListOptions::builder()
            .state(hubcaps::issues::State::Open)
            .labels(vec!["rust", "security"])
            .build();
        let gh = self.acquire(1).await?;
        let issues = gh.repo(org, repo).issues().list(&opts).await?;
        Ok(issues)
    }

    async fn create_issues(
        &self,
        org: &str,
        repo: &str,
        open_issues: Vec<hubcaps::issues::Issue>,
        advisories: Vec<Advisory>,
    ) -> Result<Vec<hubcaps::issues::Issue>> {
        let gh = self
            .acquire(advisories.len() as u32)
            .await?
            .repo(org, repo)
            .issues();

        // Ensure that we have enough rate limit remaining to create issues. If we
        let mut created = Vec::with_capacity(advisories.len());
        for advisory in advisories.into_iter() {
            let title = advisory.title();
            if !open_issues.iter().any(|i| i.title == title) {
                let opts = hubcaps::issues::IssueOptions {
                    title,
                    body: Some(advisory.body.clone()),
                    assignee: None,
                    milestone: None,
                    labels: vec!["rust".to_string(), "security".to_string()],
                };
                let issue = gh.create(&opts).await?;
                created.push(issue);
            }
        }

        Ok(created)
    }

    async fn acquire(&self, n: u32) -> Result<hubcaps::Github> {
        self.0
            .clone()
            .acquire_many_owned(n)
            .await
            .context("failed to acquire permit")?
            .forget();
        Ok(self.1.clone())
    }
}

#[derive(Clone, Debug)]
struct Advisory {
    progenitor: Option<String>,
    id: String,
    message: String,
    body: String,
}

impl From<deny::Diagnostic> for Advisory {
    fn from(d: deny::Diagnostic) -> Self {
        let progenitor = Self::find_progenitor(d.graphs);
        Self {
            progenitor,
            id: d.advisory.id,
            message: d.message,
            body: d.advisory.description,
        }
    }
}

impl Advisory {
    fn title(&self) -> String {
        if let Some(progenitor) = &self.progenitor {
            format!("{}: [{}] {}", progenitor, self.id, self.message)
        } else {
            format!("[{}] {}", self.id, self.message)
        }
    }

    fn find_progenitor(graphs: Vec<deny::Graph>) -> Option<String> {
        fn find(g: &deny::Graph) -> (usize, String) {
            g.parents
                .iter()
                .map(find)
                .max_by_key(|(d, _)| *d)
                .unwrap_or_else(|| (0, g.name.clone()))
        }

        graphs
            .iter()
            .map(find)
            .max_by_key(|(d, _)| *d)
            .map(|(_, n)| n)
    }
}
