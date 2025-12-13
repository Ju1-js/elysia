use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
    pub mb_s: f32,
    pub part_index: usize,
    pub parts_total: usize,
    pub status: String,
    pub is_busy: bool,
}

impl Progress {
    pub fn preparing() -> Self {
        Self {
            downloaded: 0,
            total: 0,
            mb_s: 0.0,
            part_index: 0,
            parts_total: 0,
            status: "Preparing download...".to_string(),
            is_busy: false,
        }
    }

    pub fn resuming(downloaded: u64, part: usize, parts_total: usize) -> Self {
        Self {
            downloaded,
            total: 0,
            mb_s: 0.0,
            part_index: part,
            parts_total,
            status: format!("Resuming from part {}/{}", part + 1, parts_total),
            is_busy: false,
        }
    }

    pub fn downloading(downloaded: u64, mb_s: f32, parts_total: usize) -> Self {
        Self {
            downloaded,
            total: 0,
            mb_s,
            part_index: 0,
            parts_total,
            status: "Downloading...".to_string(),
            is_busy: false,
        }
    }

    pub fn complete(total_downloaded: u64) -> Self {
        Self {
            downloaded: total_downloaded,
            total: total_downloaded,
            mb_s: 0.0,
            part_index: 0,
            parts_total: 0,
            status: "Complete".to_string(),
            is_busy: false,
        }
    }
}
