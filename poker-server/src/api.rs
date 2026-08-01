use crate::{
    Action, ErrorType, Game, GameConfig, GameError, MoveEvent, MoveResult, PlayerType, SpecialCard,
};
use serde::Serialize;

#[derive(Serialize)]
struct PlayerInfo {
    humans: usize,
    bots: usize,
}

#[derive(Serialize)]
struct AddPlayerResponse {
    id: usize,
}

#[derive(Serialize)]
#[serde(rename_all="lowercase")]
enum ResponseType {
    Warning,
    Error,
}

#[derive(Serialize)]
struct ErrorResponse<'a> {
    response_type: ResponseType,
    message: &'a str,
}

pub(crate) fn parse_config(config_json: &str) -> Result<GameConfig, GameError> {
    serde_json::from_str(config_json).map_err(|_| GameError::InvalidConfig)
}

pub(crate) fn parse_action(action: &str) -> Result<Action, GameError> {
    let args: Vec<&str> = action.split_whitespace().collect();
    let name = args.first().ok_or(GameError::InvalidAction)?;

    match name.to_ascii_lowercase().as_str() {
        "raise" => {
            let amount = args
                .get(1)
                .ok_or(GameError::InvalidAction)?
                .parse::<i32>()
                .map_err(|_| GameError::InvalidAction)?;

            if amount <= 0 {
                return Err(GameError::InvalidAction);
            }

            Ok(Action::Raise(amount))
        }

        "fold" => Ok(Action::Fold),
        "call" => Ok(Action::Call),
        "timeout" => Ok(Action::Timeout),
        "endmove" => Ok(Action::EndMove),

        "special" => {
            let card_name = args.get(1).ok_or(GameError::InvalidAction)?;
            let card = SpecialCard::from_str(card_name)
                .ok_or(GameError::InvalidAction)?;

            let target_id = args
                .get(2)
                .map(|value| value.parse::<usize>())
                .transpose()
                .map_err(|_| GameError::InvalidAction)?;

            let card_index = args
                .get(3)
                .map(|value| value.parse::<usize>())
                .transpose()
                .map_err(|_| GameError::InvalidAction)?;

            Ok(Action::PlaySpecial {
                card,
                target_id,
                card_index,
            })
        }

        _ => Err(GameError::InvalidAction),
    }
}

pub(crate) fn parse_player_id(player_id: i32) -> Result<usize, GameError> {
    usize::try_from(player_id).map_err(|_| GameError::UnknownPlayer)
}

pub(crate) fn ok_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| r#"{"error":"Failed to serialize response"}"#.to_string())
}

pub(crate) fn error_json(error: &GameError) -> String {
    let message = error.to_string();

    let response = match error.error_type() {
        ErrorType::Error => ErrorResponse {
            response_type: ResponseType::Error,
            message: &message,
        },

        ErrorType::Warning => ErrorResponse {
            response_type: ResponseType::Warning,
            message: &message,
        },
    };

    serde_json::to_string(&response).unwrap_or_else(|_| {r#"{"response_type": "error", "message": "Failed to process response"}"#.to_string()})
}

pub(crate) fn state_json(game: &Game, viewer_id: usize) -> String {
    ok_json(&game.get_game_state(viewer_id))
}

pub(crate) fn move_json(
    game: &Game,
    viewer_id: usize,
    events: Vec<MoveEvent>,
) -> String {
    let response = MoveResult {
        events,
        game_state: game.get_game_state(viewer_id),
    };

    ok_json(&response)
}

pub(crate) fn player_info_json(game: &Game) -> String {
    let (humans, bots) = game.players.iter().fold(
        (0usize, 0usize),
        |(humans, bots), player| match player.player_type {
            PlayerType::Human => (humans + 1, bots),
            PlayerType::Computer(_) => (humans, bots + 1),
        },
    );

    ok_json(&PlayerInfo { humans, bots })
}

pub(crate) fn added_player_json(id: usize) -> String {
    ok_json(&AddPlayerResponse { id })
}