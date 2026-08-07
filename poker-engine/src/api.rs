use crate::{
    Action, ErrorType, Game, GameConfig, GameError, MoveEvent, PlayerType, Round, SpecialCard, poker::{ExpandedCard, Hand}, special::{self, Modifier},
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
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    response_type: ResponseType,
    message: String,
}

#[derive(Serialize)]
#[serde(tag = "visibility", rename_all = "camelCase")]
pub(crate) enum CardView {
    Hidden,
    Visible { value: String },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlayerState {
    pub id: usize,
    pub chips: i32,
    pub total_bet: i32,
    pub round_bet: i32,
    pub folded: bool,
    pub acted: bool,
    pub is_turn: bool,
    pub is_dealer: bool,
    pub hole_cards: Vec<CardView>,
    pub special_cards: Vec<CardView>,
    pub hand_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameState {
    pub round_name: String,
    pub community_cards: Vec<CardView>,
    pub pots: Vec<i32>,
    pub round_bet_sum: i32,
    pub overall_sum: i32,
    pub highest_bet: i32,
    pub deck_cards: usize,
    pub winning_hand_type: String,
    pub players: Vec<PlayerState>,
    pub modifiers: Vec<Modifier>,
    pub modifier_vars: ModifierDisplayValues,
    pub games_played: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModifierDisplayValues {
    pub pot_mult: f64,
    pub ante_mult: f64,
    pub gamble_success_chance: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoveResult {
    pub events: Vec<MoveEvent>,
    pub game_state: GameState,
}

pub(crate) fn parse_config(config_json: &str) -> Result<GameConfig, GameError> {
    serde_json::from_str(config_json).map_err(|error| {
        GameError::InvalidConfig(format!("{}", error))
    })
}

pub(crate) fn parse_action(action: &str) -> Result<Action, GameError> {
    if let Some(act) = Action::from_str(action) {
        Ok(act)
    } else {
        Err(GameError::InvalidAction)
    }
}

pub(crate) fn parse_player_id(player_id: i32) -> Result<usize, GameError> {
    usize::try_from(player_id).map_err(|_| GameError::UnknownPlayer)
}

pub(crate) fn ok_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| r#"{"error":"Failed to serialize response"}"#.to_string())
}

pub(crate) fn error_json(error: &GameError) -> String {
    let message = error.to_string();

    let response = match error.error_type() {
        ErrorType::Error => ErrorResponse {
            response_type: ResponseType::Error,
            message,
        },

        ErrorType::Warning => ErrorResponse {
            response_type: ResponseType::Warning,
            message,
        },
    };

    serde_json::to_string(&response).unwrap_or_else(|_| {r#"{"response_type": "error", "message": "Failed to process response"}"#.to_string()})
}

pub(crate) fn state_json(game: &Game, viewer_id: usize) -> String {
    move_result_json(game, viewer_id, Vec::new())
}

pub(crate) fn move_result_json(game: &Game, viewer_id: usize, events: Vec<MoveEvent>) -> String {
    let response = MoveResult {
        // filtered server side
        events: events,
        game_state: view_game_state(game, viewer_id),
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

fn convert_to_cardview(c: &ExpandedCard) -> CardView {
    match c {
        ExpandedCard::PlayingCard(_) | ExpandedCard::Joker(_) => CardView::Visible { value: c.to_string() },
        ExpandedCard::Unmarked => CardView::Hidden,
    }
}

/**
 * Gets the current game state, as seen by the specified player, as a GameState struct
 */
pub(crate) fn view_game_state(game: &Game, viewer_id: usize) -> GameState {
    let is_showdown = game.round == Round::Showdown;
    let player_states: Vec<PlayerState> = game.players.iter().enumerate().map(|(i, p)| {
        let hole_cards: Vec<CardView> = {
            let forced_visibility = is_showdown || p.id == viewer_id;

            p.cards.iter().enumerate().map(|(card_idx, c)| {
                if forced_visibility || p.card_visibility.get(card_idx).map_or(false, |vis| vis.contains(&viewer_id)) {
                    convert_to_cardview(c)
                } else {
                    CardView::Hidden
                }
            }).collect()
        };

        let special_cards: Vec<CardView> = if p.id == viewer_id {
            p.special_cards.iter().map(|c| CardView::Visible { value: c.to_string() }).collect()
        } else {
            p.special_cards.iter().map(|_| CardView::Hidden).collect()
        };

        let full_cards = [&game.community[..], &p.cards[..]].concat();
        let hand_type_str = if !p.folded && !full_cards.is_empty() && (p.id == viewer_id || is_showdown) {
            if game.modifiers.blackjack_active() {
                let score = special::evaluate_blackjack_hand(&p.cards);
                format!("{}", if score == -1 { "Bust".to_string() } else { score.to_string() })
            } else {
                let hand_eval = Hand::new(&full_cards, &game.ctx.hand_fns, &game.modifiers.invalidated_hands(), game.ctx.hand_size);
                hand_eval.hand_type.to_string()
            }
        } else {
            String::new()
        };

        PlayerState {
            id: p.id,
            chips: p.chips,
            round_bet: p.round_bet,
            total_bet: p.total_bet,
            folded: p.folded,
            acted: p.acted,
            is_turn: game.turn_index == i && !is_showdown && game.round != Round::Preround,
            is_dealer: game.dealer_index == i,
            hole_cards,
            special_cards,
            hand_type: hand_type_str,
        }
    }).collect();

    let pots: Vec<i32> = game.pots.iter().map(|p| p.amount).collect();
    let total_pot: i32 = pots.iter().sum::<i32>() + game.round_pool;
    let visible_modifiers = game.modifiers.active
        .iter()
        .filter_map(|m| {
            if m.effect.is_visible(viewer_id) {
                Some((*m).clone())
            } else {
                None
            }
        }).collect();

    GameState {
        round_name: game.round.to_string(),
        community_cards: game.community.iter().map(|c| convert_to_cardview(c)).collect(),
        pots,
        round_bet_sum: game.round_pool,
        overall_sum: total_pot,
        highest_bet: game.bet,
        deck_cards: game.deck.len(),
        winning_hand_type: game.winning_hand_type.clone(),
        players: player_states,
        modifiers: visible_modifiers,
        modifier_vars: ModifierDisplayValues { 
            pot_mult: game.modifiers.vars.pot_multiplier, 
            ante_mult: game.modifiers.vars.ante_multiplier, 
            gamble_success_chance: game.modifiers.vars.gamble_success_chance,
        },
        games_played: game.games_played,
    }
}