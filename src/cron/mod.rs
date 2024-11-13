mod collect_hypernet_task;

use crate::context::{AppContext, CronAppContext};
use crate::cron::collect_hypernet_task::CollectHypernetTask;
use async_trait::async_trait;
use tokio::task::JoinSet;
use tokio::{select, time};

pub async fn start_cron(ctx: CronAppContext) -> anyhow::Result<()> {
    let tasks: Vec<Box<dyn CronTask>> = vec![Box::new(CollectHypernetTask)];

    let mut join_set = JoinSet::new();

    for task in tasks {
        let cloned_ctx = ctx.clone();

        join_set.spawn(async move {
            let task = task;
            let mut interval = time::interval(task.interval());
            let timeout = task.timeout();

            loop {
                interval.tick().await;
                let now = chrono::Utc::now();
                select! {
                    res = task.run(cloned_ctx.clone()) => {
                        if let Err(e) = res {
                            log::error!("Task {} failed: {:?}", task.name(), e);
                        } else {
                            log::info!("Task {} ran successfully in {}", task.name(), chrono::Utc::now() - now);
                        }
                    },
                    _ = time::sleep(timeout) => {
                        log::error!("Task {} timed out", task.name());
                    }
                }
            }
        });
    }

    join_set.join_all().await;

    Ok(())
}

#[async_trait]
pub trait CronTask: Send {
    /// Name of the task
    fn name(&self) -> &'static str;

    /// Duration between runs
    fn interval(&self) -> std::time::Duration;

    /// Timeout in seconds
    fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(60)
    }
    async fn run(&self, ctx: CronAppContext) -> anyhow::Result<()>;
}
