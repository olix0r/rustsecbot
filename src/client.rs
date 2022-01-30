use crate::GitHubRepo;
use anyhow::{Context, Result};
use hubcaps::{issues::*, Credentials, Github};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone, Debug)]
pub struct Client(Arc<Semaphore>, Github);

impl Client {
    /// Create a client that watches its rate limit. The client delays work instead of violating the
    /// hinted limit.
    pub async fn spawn_rate_limited(token: String) -> Result<Self> {
        let gh = Github::new(Self::user_agent(), Credentials::Token(token))?;
        let semaphore = Arc::new(Semaphore::new(0));
        let (init_tx, init_rx) = tokio::sync::oneshot::channel();

        tokio::spawn({
            let gh = gh.clone();
            let mut init = Some(init_tx);
            let handle = Arc::downgrade(&semaphore);
            async move {
                while let Some(semaphore) = handle.upgrade() {
                    let result = gh.rate_limit().get().await;
                    if let Some(tx) = init.take() {
                        if let Err(e) = result {
                            let _ = tx.send(Err(e));
                            return Ok(());
                        }
                        if tx.send(Ok(())).is_err() {
                            return Ok(());
                        }
                    }
                    let rate_limit = result?;

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

        init_rx
            .await
            .expect("sender must not be dropped")
            .context("failed to initialize GitHub client")?;

        Ok(Self(semaphore, gh))
    }

    fn user_agent() -> String {
        format!("{}/{}", clap::crate_name!(), clap::crate_version!())
    }

    pub async fn list_issues(&self, repo: &GitHubRepo) -> Result<Vec<Issue>> {
        let opts = IssueListOptions::builder()
            .state(State::Open)
            .labels(vec!["rust", "security"])
            .build();
        let gh = self.acquire(1).await?;
        let issues = gh
            .repo(&repo.owner, &repo.name)
            .issues()
            .list(&opts)
            .await?;
        Ok(issues)
    }

    pub async fn create_issues(
        &self,
        repo: &GitHubRepo,
        advisories: Vec<crate::Advisory>,
    ) -> Result<Vec<(hubcaps::issues::Issue, crate::Advisory)>> {
        let gh = self
            .acquire(advisories.len() as u32)
            .await?
            .repo(&repo.owner, &repo.name)
            .issues();

        // Ensure that we have enough rate limit remaining to create issues. If we
        let mut created = Vec::with_capacity(advisories.len());
        for advisory in advisories.into_iter() {
            let opts = IssueOptions {
                title: advisory.title.clone(),
                body: Some(advisory.body.clone()),
                assignee: None,
                milestone: None,
                labels: vec!["rust".to_string(), "security".to_string()],
            };
            let issue = gh.create(&opts).await?;
            created.push((issue, advisory));
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
