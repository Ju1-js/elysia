use std::path::PathBuf;

use anyhow::Result;
use reqwest::Client;
pub use stream_unpacker::Archive;
use tokio::sync::mpsc;

use crate::task_manager::TaskStatus;

pub struct ZipStreamStep {
    pub dest: PathBuf,
    pub archives: Vec<Archive>,
}

impl ZipStreamStep {
    pub fn new(dest: PathBuf, archives: Vec<Archive>) -> Self {
        Self { dest, archives }
    }

    pub async fn run(&self, status_tx: mpsc::Sender<TaskStatus>, client: &Client) -> Result<()> {
        if self.archives.is_empty() {
            return Ok(());
        }

        let _ = status_tx.send(TaskStatus::Started {}).await;
        println!("Archive step dest: {}", self.dest.display());

        let (tx, mut rx) = mpsc::channel::<stream_unpacker::Progress>(100);

        let status_tx_clone = status_tx.clone();
        tokio::spawn(async move {
            while let Some(progress) = rx.recv().await {
                if progress.total == 0 {
                    continue;
                }

                status_tx_clone
                    .send(TaskStatus::Progress {
                        current: progress.downloaded,
                        total: progress.total,
                        mb_s: progress.mb_s,
                    })
                    .await
                    .unwrap();
            }
        });

        let res = stream_unpacker::stream_unpack(
            tx,
            client.to_owned(),
            self.archives.to_owned(),
            self.dest.to_owned(),
        )
        .await;

        if let Err(err) = res {
            println!("Unpacker error: {:?}", err);
        } else {
            println!("Unpacker done");
            // use crate::game_providers::installer::InstallationManifest;

            // let manifest = InstallationManifest {
            //     game_id: "12".to_string(),
            // };

            // let marker_path = self.dest.join(".elysia_installed");
            // let json = serde_json::to_string_pretty(&manifest)?;

            // tokio::fs::write(&marker_path, json).await?;

            // eprintln!("[INFO] Created installation marker at {:?}", marker_path);
        }

        Ok(())
    }
}
