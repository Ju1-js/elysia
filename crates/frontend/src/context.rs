use backend::game_providers::hoyoplay::proto::{Content, Game, GameBasicInfo};
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Context {
    pub api_games: Vec<Game>,
    pub api_news: HashMap<String, Content>,
    pub api_game_basic_info: Vec<GameBasicInfo>,
}
