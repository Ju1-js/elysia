use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
    pub mb_s: f32,
    pub part_index: usize,
    pub parts_total: usize,
    pub status: String,
    pub is_busy: bool,
}
