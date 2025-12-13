pub mod archive;
pub mod file;
pub mod sophon;

use anyhow::Result;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::task_manager::{TaskStatus, steps::archive::ArchiveStep};

pub enum Step {
    DownloadFiles,
    DownloadFromArchives(ArchiveStep),
}

impl Step {
    pub async fn run(&self, _status_tx: mpsc::Sender<TaskStatus>, client: &Client) -> Result<()> {
        match self {
            Step::DownloadFiles => {
                println!("Running File step");
                Ok(())
            }
            Step::DownloadFromArchives(step) => {
                println!("Running Archive step");
                step.run(client).await?;
                Ok(())
            }
        }
    }
}
