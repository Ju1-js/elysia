use std::path::PathBuf;

use anyhow::Result;
use reqwest::Client;
use stream_unpacker::Archive;
use tokio::sync::mpsc;

use crate::task_manager::TaskStatus;

pub enum ArchiveKind {
    Zip,
    //TarGz,
}

pub struct ArchiveStep {
    pub dest: PathBuf,
    pub archives: Vec<Archive>,
    status_tx: mpsc::Sender<TaskStatus>,
    archive_kind: ArchiveKind,
}

impl ArchiveStep {
    pub fn new(
        status_tx: mpsc::Sender<TaskStatus>,
        dest: PathBuf,
        archives: Vec<Archive>,
        archive_kind: ArchiveKind,
    ) -> Self {
        Self {
            dest,
            archives,
            status_tx,
            archive_kind,
        }
    }

    pub async fn run(&self, client: &Client) -> Result<()> {
        if self.archives.is_empty() {
            return Ok(());
        }

        match self.archive_kind {
            ArchiveKind::Zip => self.run_zip(client).await,
            //ArchiveKind::TarGz => self.run_tar_gz(client).await,
        }
    }

    async fn run_zip(&self, client: &Client) -> Result<()> {
        let _ = self.status_tx.send(TaskStatus::Started {}).await;

        let (tx, mut rx) = mpsc::channel::<stream_unpacker::Progress>(100);

        let status_tx_clone = self.status_tx.clone();
        tokio::spawn(async move {
            while let Some(progress) = rx.recv().await {
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

        if res.is_ok() {
            use crate::game_providers::installer::InstallationManifest;

            let manifest = InstallationManifest {
                game_id: "12".to_string(),
            };

            let marker_path = self.dest.join(".elysia_installed");
            let json = serde_json::to_string_pretty(&manifest)?;

            tokio::fs::write(&marker_path, json).await?;

            eprintln!("[INFO] Created installation marker at {:?}", marker_path);
        } else {
            //clear_progress(progress_key);
        }

        let _ = self
            .status_tx
            .send(TaskStatus::Progress {
                current: 0u64,
                total: 1u64,
                mb_s: 0.0,
            })
            .await;

        Ok(())
    }
}
