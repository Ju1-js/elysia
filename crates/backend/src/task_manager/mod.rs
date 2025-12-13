pub mod steps;

use std::path::PathBuf;

use anyhow::Result;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::task_manager::steps::Step;

pub enum TaskStatus {
    Preparing {},
    Started {},
    Progress { current: u64, total: u64, mb_s: f32 },
    Finalizing {},
    Done {},
    Failed { error: String },
}

pub struct Task {
    pub steps: Vec<Step>,
    pub dest: PathBuf,
}

pub struct TaskManager {
    pub submit_tx: mpsc::Sender<Task>,
    pub status_rx: mpsc::Receiver<TaskStatus>,
}

impl TaskManager {
    pub fn new(client: Client) -> Self {
        let (submit_tx, submit_rx) = mpsc::channel::<Task>(100);
        let (status_tx, status_rx) = mpsc::channel::<TaskStatus>(100);

        let local = tokio::task::LocalSet::new();

        {
            local.enter();
            tokio::task::spawn_local(async move {
                worker_loop(submit_rx, status_tx, client).await;
            });
        }

        Self {
            submit_tx,
            status_rx,
        }
    }
}

async fn worker_loop(
    mut job_rx: mpsc::Receiver<Task>,
    status_tx: mpsc::Sender<TaskStatus>,
    client: Client,
) -> () {
    loop {
        let job = match job_rx.recv().await {
            Some(j) => j,
            None => break, // channel closed => exit worker
        };

        match try_run_task(job, status_tx.clone(), &client).await {
            Ok(()) => {
                let _ = status_tx.send(TaskStatus::Done {}).await;
            }
            Err(e) => {
                let _ = status_tx
                    .send(TaskStatus::Failed {
                        error: e.to_string(),
                    })
                    .await;
            }
        }
    }
}

async fn try_run_task(
    task: Task,
    status_tx: mpsc::Sender<TaskStatus>,
    client: &Client,
) -> Result<()> {
    // TODO: Handle SendErrors
    let _ = status_tx.send(TaskStatus::Preparing {}).await;

    let _ = status_tx
        .send(TaskStatus::Progress {
            current: 0,
            total: 100,
            mb_s: 0.0,
        })
        .await;

    for step in task.steps {
        step.run(status_tx.clone(), client).await?;
    }

    Ok(())
}
