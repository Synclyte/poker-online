use wasm_bindgen::prelude::*;

use crate::{api, Game, GameError, PlayerType};

#[wasm_bindgen]
pub struct GameAPI {
    game: Game,
}

#[wasm_bindgen]
impl GameAPI {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            game: Game::new(),
        }
    }

    pub fn init_panic_hook(&self) {
        console_error_panic_hook::set_once();
    }

    pub fn update_config(&mut self, config_json: String) -> String {
        let result = api::parse_config(&config_json)
            .and_then(|config| self.game.update_config(config));

        match result {
            Ok(()) => String::new(),
            Err(error) => api::error_json(&error),
        }
    }

    pub fn try_add_player(&mut self, player_type: String) -> String {
        let player_type = match PlayerType::from_str(&player_type) {
            Some(player_type) => player_type,
            None => return api::error_json(&GameError::InvalidPlayerType),
        };

        match self.game.try_add_player(player_type) {
            Ok(id) => api::added_player_json(id),
            Err(error) => api::error_json(&error),
        }
    }

    pub fn initialise(&mut self) -> String {
        match self.game.initialise() {
            Ok(()) => api::state_json(&self.game, usize::MAX),
            Err(error) => api::error_json(&error),
        }
    }

    pub fn start(&mut self) -> String {
        match self.game.start() {
            Ok(events) => api::move_json(&self.game, usize::MAX, events),
            Err(error) => api::error_json(&error),
        }
    }

    pub fn player_move(&mut self, player_id: i32, action: String) -> String {
        let result = (|| {
            let player_id = api::parse_player_id(player_id)?;
            let action = api::parse_action(&action)?;
            let events = self.game.player_move(player_id, action)?;
            Ok::<_, GameError>((player_id, events))
        })();

        match result {
            Ok((viewer_id, events)) => api::move_json(&self.game, viewer_id, events),
            Err(error) => api::error_json(&error),
        }
    }

    pub fn queue_remove_player(&mut self, player_id: i32) -> String {
        let result = api::parse_player_id(player_id)
            .and_then(|player_id| self.game.queue_remove_player(player_id));

        match result {
            Ok(()) => String::new(),
            Err(error) => api::error_json(&error),
        }
    }

    pub fn force_remove_player(&mut self, player_id: i32) -> String {
        let result = api::parse_player_id(player_id)
            .and_then(|player_id| self.game.force_remove_player(player_id));

        match result {
            Ok(()) => String::new(),
            Err(error) => api::error_json(&error),
        }
    }

    pub fn get_player_info(&self) -> String {
        api::player_info_json(&self.game)
    }

    pub fn get_round(&self) -> String {
        self.game.round.to_string()
    }

    pub fn get_current_turn_player(&self) -> i32 {
        let id = self.game.get_current_turn_player_id();
        i32::try_from(id).unwrap_or(0)
    }

    pub fn get_game_state(&self, player_id: i32) -> String {
        match api::parse_player_id(player_id) {
            Ok(player_id) => api::state_json(&self.game, player_id),
            Err(error) => api::error_json(&error),
        }
    }

    pub fn toggle_id_with_bot(&mut self, player_id: i32, bot: bool) -> String {
        let result = api::parse_player_id(player_id)
            .and_then(|player_id| self.game.toggle_id_with_bot(player_id, bot));

        match result {
            Ok(events) => api::move_json(&self.game, usize::MAX, events),
            Err(error) => api::error_json(&error),
        }
    }
}