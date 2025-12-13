pub mod file;
pub mod sophon;
pub mod zip_stream;

use anyhow::Result;
use reqwest::Client;
use tokio::sync::mpsc;

use crate::task_manager::{TaskStatus, steps::zip_stream::ZipStreamStep};

pub enum Step {
    DownloadFiles,
    DownloadFromArchives(ZipStreamStep),
}

impl Step {
    pub async fn run(&self, status_tx: mpsc::Sender<TaskStatus>, client: &Client) -> Result<()> {
        match self {
            Step::DownloadFiles => {
                println!("Running File step");
                Ok(())
            }
            Step::DownloadFromArchives(step) => {
                println!("Running Archive step");
                step.run(status_tx, client).await?;
                Ok(())
            }
        }
    }
}
