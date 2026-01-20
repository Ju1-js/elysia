use crate::game_providers::Progress;
use crate::game_providers::installer::GameInstaller;
use crate::settings::InstalledGame;
use async_trait::async_trait;
use serde_json::json;
use std::path::PathBuf;

pub struct EndfieldInstaller {
    pub appcode: String,
    pub game_id: String,
    pub temp_dir: PathBuf,
    pub games_dir: PathBuf,
    pub biz_name: String,
}

impl EndfieldInstaller {
    #[must_use] 
    pub fn new(game_id: String, temp_dir: PathBuf, games_dir: PathBuf, biz_name: String) -> Self {
        Self {
            appcode: game_id.clone(),
            game_id,
            temp_dir,
            games_dir,
            biz_name,
        }
    }
}

#[async_trait]
impl GameInstaller for EndfieldInstaller {
    fn progress_key(&self) -> String {
        format!("{}_streaming", self.game_id)
    }

    fn clear_progress(&self) {
        super::clear_progress(&self.progress_key());
    }

    fn get_progress(&self, key: &str) -> Option<crate::game_providers::Progress> {
        super::get_progress(key).map(|p| crate::game_providers::Progress {
            downloaded: p.downloaded,
            total: p.total,
            mb_s: p.mb_s,
            part_index: p.part_index,
            parts_total: p.parts_total,
            status: p.status,
            is_busy: p.is_busy,
        })
    }

    fn get_install_path(&self) -> PathBuf {
        self.games_dir.join("endfield")
    }

    fn get_executable_name(&self) -> &'static str {
        "Endfield.exe"
    }

    async fn install(&self) -> Result<InstalledGame, String> {
        println!("Starting Endfield installer for appcode={}", self.appcode);
        self.clear_progress();

        let body = json!({
            "proxy_reqs": [{
                "kind": "get_latest_game",
                "get_latest_game_req": {
                    "appcode": self.appcode,
                    "channel": "6",
                    "sub_channel": "6",
                    "version": ""
                }
            }]
        });

        let v = super::batch_proxy_post(&body)
            .await
            .map_err(|e| format!("batch proxy error: {e}"))?;

        let appcode = self.appcode.clone();
        let games_dir = self.games_dir.clone();
        let temp_dir = self.temp_dir.clone();

        let dest = tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(super::install_from_batch_body_value(
                v,
                &appcode,
                games_dir.as_path(),
                temp_dir.as_path(),
            ))
        })
        .await
        .map_err(|e| format!("Task error: {e}"))?
        .map_err(|e| format!("Install error: {e}"))?;

        println!("Endfield game installed at: {}", dest.display());
        self.clear_progress();

        Ok(InstalledGame {
            id: self.game_id.clone(),
            biz_name: self.biz_name.clone(),
            executable_path: dest.join("Endfield.exe"),
            install_path: dest,
            ..Default::default()
        })
    }
}
