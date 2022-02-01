use crate::{Config, GitHubRepo};
use anyhow::{Context, Result};
use hubcaps::{issues::*, Credentials, Github};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone, Debug)]
pub struct Client {
    api: Github,
    config: Config,
    rate_limit: Arc<Semaphore>,
}

impl Client {
    /// Create a client that watches its rate limit. The client delays work instead of violating the
    /// hinted limit.
    pub async fn spawn_rate_limited(config: Config, token: String) -> Result<Self> {
        let api = Github::new(Self::user_agent(), Credentials::Token(token))?;
        let rate_limit = Arc::new(Semaphore::new(0));
        let (init_tx, init_rx) = tokio::sync::oneshot::channel();

        tokio::spawn({
            let api = api.clone();
            let mut init = Some(init_tx);
            let handle = Arc::downgrade(&rate_limit);
            async move {
                while let Some(semaphore) = handle.upgrade() {
                    let result = api.rate_limit().get().await;
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

        Ok(Self {
            rate_limit,
            api,
            config,
        })
    }

    fn user_agent() -> String {
        format!("{}/{}", clap::crate_name!(), clap::crate_version!())
    }

    pub async fn list_issues(&self, repo: &GitHubRepo) -> Result<Vec<Issue>> {
        let opts = IssueListOptions::builder()
            .state(State::Open)
            .labels(self.config.labels.clone())
            .build();
        let api = self.acquire(1).await?;
        let issues = api
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
        let api = self
            .acquire(advisories.len() as u32)
            .await?
            .repo(&repo.owner, &repo.name)
            .issues();

        // Ensure that we have enouapi rate limit remaining to create issues. If we
        let mut created = Vec::with_capacity(advisories.len());
        for advisory in advisories.into_iter() {
            let labels = {
                let base_labels = self.config.labels.iter().cloned();
                let crate_labels = advisory
                    .crate_name
                    .as_ref()
                    .map(|cn| {
                        self.config
                            .crates
                            .get(cn)
                            .as_ref()
                            .map(|c| c.labels.clone())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                base_labels.into_iter().chain(crate_labels).collect()
            };
            let opts = IssueOptions {
                title: advisory.title.clone(),
                body: Some(advisory.body.clone()),
                labels,
                assignee: None,
                milestone: None,
            };
            let issue = api.create(&opts).await?;
            created.push((issue, advisory));
        }

        Ok(created)
    }

    async fn acquire(&self, n: u32) -> Result<hubcaps::Github> {
        self.rate_limit
            .clone()
            .acquire_many_owned(n)
            .await
            .context("failed to acquire permit")?
            .forget();
        Ok(self.api.clone())
    }
}
