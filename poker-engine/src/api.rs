use std::fmt;
use crate::{Action, ErrorType, Game, GameConfig, GameError, MoveEvent, PlayerType, Round, poker::{ExpandedCard, Hand}, special::{self, Modifier, SpecialCard}};
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
    pub special_card_limit: usize,
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
    pub max_bet: Option<i32>,
    pub winner_draft_options: Option<Vec<String>>,
    pub winner_draft_pending_players: Vec<usize>,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConsoleCommandResponse {
    pub ok: bool,
    pub message: String,
    pub events: Vec<MoveEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardTarget {
    Board,
    Player(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsoleCommand {
    Help,
    SetChips {
        player_id: usize,
        chips: i32,
    },
    GiveCard {
        target: CardTarget,
        card: ExpandedCard,
        count: usize,
    },
    GiveSpecial {
        player_id: usize,
        card: SpecialCard,
        count: usize,
    },
    DealSpecial {
        player_id: usize,
        count: usize,
    },
    SkipStreet {
        count: usize,
    },
    SkipHand {
        count: usize,
    },
    CardDraft {
        player_id: usize,
        simulated_chips: Option<i32>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ConsoleCommandOutcome {
    Help,
    ChipsSet {
        player_id: usize,
        chips: i32,
    },
    CardGiven {
        target: CardTarget,
        card: ExpandedCard,
        count: usize,
    },
    SpecialGiven {
        player_id: usize,
        card: SpecialCard,
        count: usize,
    },
    SpecialDealt {
        player_id: usize,
        cards: Vec<SpecialCard>,
    },
    StreetsSkipped {
        skipped: usize,
        round: Round,
    },
    HandsSkipped {
        skipped: usize,
        games_played: i32,
    },
    BotCardDrafted {
        player_id: usize,
        simulated_chips: i32,
        card: SpecialCard,
    },
    HumanCardDraftStarted {
        player_id: usize,
        simulated_chips: i32,
        pot_ratio: f64,
    },
}

impl fmt::Display for ConsoleCommandOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Help => f.write_str(concat!(
                "Commands:\n",
                "/set_chips [player_id] <count>\n",
                "/give_card [player_id|board] <card> [count]\n",
                "/give_special [player_id] <special_id> [count]\n",
                "/deal_special [player_id] [count]\n",
                "/skip_street [count]\n",
                "/skip_hand [count]\n",
                "/card_draft [player_id] [chips]",
            )),
            Self::ChipsSet { player_id, chips } => {
                write!(f, "Set player {} chips to {}", player_id, chips)
            }
            Self::CardGiven {
                target: CardTarget::Board,
                card,
                count,
            } => {
                write!(f, "Dealt {}x {} to the community board", count, card)
            }
            Self::CardGiven {
                target: CardTarget::Player(player_id),
                card,
                count,
            } => {
                write!(f, "Gave {}x {} to player {}", count, card, player_id)
            }
            Self::SpecialGiven { player_id, card, count } => {
                write!(f, "Gave {}x special '{}' to player {}", count, card, player_id)
            }
            Self::SpecialDealt { player_id, cards } => {
                if cards.len() == 1 {
                    write!(f, "Dealt special '{}' to player {}", cards[0], player_id)
                } else {
                    let list = cards
                        .iter()
                        .map(|c| format!("'{}'", c))
                        .collect::<Vec<_>>()
                        .join(", ");
                    write!(
                        f,
                        "Dealt {}x specials ({}) to player {}",
                        cards.len(),
                        list,
                        player_id
                    )
                }
            }
            Self::StreetsSkipped { skipped, round } => {
                write!(f, "Successfully skipped {} street{}. New round: {}", skipped, if *skipped != 1 { "s" } else { "" }, round)
            }
            Self::HandsSkipped {
                skipped,
                games_played,
            } => write!(
                f,
                "Successfully skipped {} hand{}. New hand: {}",
                skipped, if *skipped != 1 { "s" } else { "" }, games_played
            ),
            Self::BotCardDrafted {
                player_id,
                simulated_chips,
                card,
            } => write!(
                f,
                "Simulated card draft for bot {} with {} chips. Granted '{}'",
                player_id, simulated_chips, card
            ),
            Self::HumanCardDraftStarted {
                player_id,
                simulated_chips,
                pot_ratio,
            } => write!(
                f,
                "Started card draft for player {} simulating {} chips (pot ratio of {:.2}).",
                player_id, simulated_chips, pot_ratio
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConsoleCommandError {
    EmptyCommand,
    UnknownCommand(String),
    InvalidUsage(&'static str),
    InvalidPlayerId(String),
    InvalidNumber(String),
    NegativeChips,
    ZeroCount,
    InvalidCard(String),
    UnknownSpecialCard(String),
    Engine {
        command: Option<ConsoleCommand>,
        round: Round,
        error: GameError,
    },
}

impl fmt::Display for ConsoleCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCommand => f.write_str("Invalid command. Type /help to display available commands"),
            Self::UnknownCommand(cmd) => {
                write!(f, "Invalid command. Type /help to display available commands")
            }
            Self::InvalidUsage(usage) => write!(f, "Usage: {}", usage),
            Self::InvalidPlayerId(raw) => write!(f, "Invalid player id"),
            Self::InvalidNumber(raw) => write!(f, "Invalid number"),
            Self::NegativeChips => write!(f, "Cannot set negative chips"),
            Self::ZeroCount => write!(f, "Count must be >= 1"),
            Self::InvalidCard(raw) => write!(f, "Invalid card string. Use compact notation (i.e. 10h, 5d, joker, unmarked)"),
            Self::UnknownSpecialCard(raw) => write!(f, "Invalid special card ID"),
            Self::Engine {
                command,
                round,
                error: GameError::InvalidRound,
            } => {
                if matches!(round, Round::Room | Round::GameEnd) {
                    write!(f, "Command not valid outside round")
                } else if matches!(command, Some(ConsoleCommand::GiveCard { .. })) && *round == Round::Preround {
                    write!(f, "Playing cards must be dealt in hand")
                } else if matches!(command, Some(ConsoleCommand::SkipStreet { .. })) {
                    write!(f, "Command only valid during an active street (preflop - river)")
                } else {
                    write!(f, "{}", &GameError::InvalidRound.to_string())
                }
            }
            Self::Engine { error, .. } => write!(f, "{}", &error.to_string()),
        }
    }
}

pub(crate) fn parse_compact_card(raw: &str) -> Result<ExpandedCard, ConsoleCommandError> {
    let normalised = raw.trim().to_ascii_lowercase();
    if normalised.is_empty() || normalised.contains(':') {
        return Err(ConsoleCommandError::InvalidCard(raw.to_string()));
    }

    if normalised == "joker" {
        return Ok(ExpandedCard::get_joker());
    }
    if normalised == "unmarked" || normalised == "null" {
        return Ok(ExpandedCard::Unmarked);
    }

    if normalised.len() < 2 {
        return Err(ConsoleCommandError::InvalidCard(raw.to_string()));
    }

    let split_at = normalised.len() - 1;
    let (rank_str, suit_str) = normalised.split_at(split_at);
    let valid_rank = matches!(rank_str, "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "10" | "j" | "q" | "k" | "a");
    let valid_suit = matches!(suit_str, "s" | "d" | "c" | "h");

    if !valid_rank || !valid_suit {
        return Err(ConsoleCommandError::InvalidCard(raw.to_string()));
    }

    Ok(ExpandedCard::get_card(rank_str, suit_str))
}

pub(crate) fn parse_console_command(
    caller_id: usize,
    raw_command: &str,
) -> Result<ConsoleCommand, ConsoleCommandError> {
    let trimmed = raw_command.trim();
    let without_prefix = trimmed.strip_prefix('/').unwrap_or(trimmed).trim();
    let parts: Vec<&str> = without_prefix.split_whitespace().collect();
    if parts.is_empty() {
        return Err(ConsoleCommandError::EmptyCommand);
    }

    let cmd = parts[0].to_ascii_lowercase();
    let args = &parts[1..];

    let parse_pid = |raw: &str| {
        raw.parse::<usize>()
            .map_err(|_| ConsoleCommandError::InvalidPlayerId(raw.to_string()))
    };
    let parse_count = |raw: &str| {
        let count = raw
            .parse::<usize>()
            .map_err(|_| ConsoleCommandError::InvalidNumber(raw.to_string()))?;
        if count == 0 {
            return Err(ConsoleCommandError::ZeroCount);
        }
        Ok(count)
    };
    let parse_card_target = |raw: &str| {
        if raw.eq_ignore_ascii_case("board") || raw.eq_ignore_ascii_case("community") {
            Ok(CardTarget::Board)
        } else {
            Ok(CardTarget::Player(parse_pid(raw)?))
        }
    };
    let parse_special = |raw: &str| {
        let special_name = raw.trim().to_ascii_lowercase();
        SpecialCard::from_str(&special_name)
            .ok_or_else(|| ConsoleCommandError::UnknownSpecialCard(raw.to_string()))
    };

    match cmd.as_str() {
        "help" => Ok(ConsoleCommand::Help),
        "set_chips" => {
            let (player_id, chips_str) = match args.len() {
                1 => (caller_id, args[0]),
                2 => (parse_pid(args[0])?, args[1]),
                _ => return Err(ConsoleCommandError::InvalidUsage("/set_chips [player_id] <count>")),
            };
            let chips = chips_str
                .parse::<i32>()
                .map_err(|_| ConsoleCommandError::InvalidNumber(chips_str.to_string()))?;
            if chips < 0 {
                return Err(ConsoleCommandError::NegativeChips);
            }
            Ok(ConsoleCommand::SetChips { player_id, chips })
        }
        "give_card" => {
            let (target, card, count) = match args.len() {
                1 => (CardTarget::Player(caller_id), parse_compact_card(args[0])?, 1usize),
                2 => {
                    if let Ok(card) = parse_compact_card(args[0]) {
                        (CardTarget::Player(caller_id), card, parse_count(args[1])?)
                    } else if args[0].eq_ignore_ascii_case("board")
                        || args[0].eq_ignore_ascii_case("community")
                        || args[0].parse::<usize>().is_ok()
                    {
                        (parse_card_target(args[0])?, parse_compact_card(args[1])?, 1usize)
                    } else if parse_compact_card(args[1]).is_ok() {
                        return Err(ConsoleCommandError::InvalidPlayerId(args[0].to_string()));
                    } else {
                        return Err(ConsoleCommandError::InvalidCard(args[0].to_string()));
                    }
                }
                3 => (
                    parse_card_target(args[0])?,
                    parse_compact_card(args[1])?,
                    parse_count(args[2])?,
                ),
                _ => return Err(ConsoleCommandError::InvalidUsage("/give_card [player_id|board] <card> [count]")),
            };
            Ok(ConsoleCommand::GiveCard { target, card, count })
        }
        "give_special" => {
            let (player_id, card, count) = match args.len() {
                1 => (caller_id, parse_special(args[0])?, 1usize),
                2 => {
                    if let Ok(card) = parse_special(args[0]) {
                        (caller_id, card, parse_count(args[1])?)
                    } else if let Ok(pid) = parse_pid(args[0]) {
                        (pid, parse_special(args[1])?, 1usize)
                    } else if parse_special(args[1]).is_ok() {
                        return Err(ConsoleCommandError::InvalidPlayerId(args[0].to_string()));
                    } else {
                        return Err(ConsoleCommandError::UnknownSpecialCard(args[0].to_string()));
                    }
                }
                3 => (parse_pid(args[0])?, parse_special(args[1])?, parse_count(args[2])?),
                _ => return Err(ConsoleCommandError::InvalidUsage("/give_special [player_id] <special_id> [count]")),
            };
            Ok(ConsoleCommand::GiveSpecial { player_id, card, count })
        }
        "deal_special" => {
            let (player_id, count) = match args.len() {
                0 => (caller_id, 1usize),
                1 => (caller_id, parse_count(args[0])?),
                2 => (parse_pid(args[0])?, parse_count(args[1])?),
                _ => return Err(ConsoleCommandError::InvalidUsage("/deal_special [player_id] [count]")),
            };
            Ok(ConsoleCommand::DealSpecial { player_id, count })
        }
        "skip_street" => {
            let count = match args.len() {
                0 => 1usize,
                1 => args[0]
                    .parse::<usize>()
                    .map_err(|_| ConsoleCommandError::InvalidNumber(args[0].to_string()))?,
                _ => return Err(ConsoleCommandError::InvalidUsage("/skip_street [count]")),
            };
            if count == 0 {
                return Err(ConsoleCommandError::ZeroCount);
            }
            Ok(ConsoleCommand::SkipStreet { count })
        }
        "skip_hand" => {
            let count = match args.len() {
                0 => 1usize,
                1 => args[0]
                    .parse::<usize>()
                    .map_err(|_| ConsoleCommandError::InvalidNumber(args[0].to_string()))?,
                _ => return Err(ConsoleCommandError::InvalidUsage("/skip_hand [count]")),
            };
            if count == 0 {
                return Err(ConsoleCommandError::ZeroCount);
            }
            Ok(ConsoleCommand::SkipHand { count })
        }
        "card_draft" => {
            let parse_chips = |raw: &str| {
                let chips = raw
                    .parse::<i32>()
                    .map_err(|_| ConsoleCommandError::InvalidNumber(raw.to_string()))?;
                if chips < 0 {
                    return Err(ConsoleCommandError::NegativeChips);
                }
                Ok(chips)
            };

            let (player_id, simulated_chips) = match args.len() {
                0 => (caller_id, None),
                1 => (caller_id, Some(parse_chips(args[0])?)),
                2 => (parse_pid(args[0])?, Some(parse_chips(args[1])?)),
                _ => return Err(ConsoleCommandError::InvalidUsage("/card_draft [player_id] [chips]")),
            };
            Ok(ConsoleCommand::CardDraft {
                player_id,
                simulated_chips,
            })
        }
        _ => Err(ConsoleCommandError::UnknownCommand(cmd)),
    }
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

pub(crate) fn console_ok_json(outcome: &ConsoleCommandOutcome, events: Vec<MoveEvent>) -> String {
    ok_json(&ConsoleCommandResponse {
        ok: true,
        message: outcome.to_string(),
        events,
    })
}

pub(crate) fn console_err_json(error: &ConsoleCommandError) -> String {
    ok_json(&ConsoleCommandResponse {
        ok: false,
        message: error.to_string(),
        events: Vec::new(),
    })
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
    let is_showdown = game.round == Round::Showdown || game.round == Round::WinnerDraft;
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
            if game.special.blackjack_active() {
                let score = special::evaluate_blackjack_hand(&p.cards);
                format!("{}", if score == -1 { "Bust".to_string() } else { score.to_string() })
            } else {
                let hand_eval = Hand::new(&full_cards, &game.ctx.hand_fns, &game.special.invalidated_hands(), game.ctx.hand_size);
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
            special_card_limit: game.player_special_card_limit(i),
            hand_type: hand_type_str,
        }
    }).collect();

    let pots: Vec<i32> = game.pots.iter().map(|p| p.amount).collect();
    let total_pot: i32 = pots.iter().sum::<i32>() + game.round_pool;
    let visible_modifiers = game.special.active_modifiers()
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
            pot_mult: game.special.pot_multiplier(), 
            ante_mult: game.special.ante_multiplier(), 
            gamble_success_chance: game.special.gamble_success_chance(), 
        },
        games_played: game.games_played,
        max_bet: game.get_current_max_bet(),
        winner_draft_options: game
            .pending_drafts
            .iter()
            .find(|draft| draft.player_id == viewer_id)
            .map(|draft| draft.options.iter().map(|c| c.to_string()).collect()),
        winner_draft_pending_players: game
            .pending_drafts
            .iter()
            .map(|draft| draft.player_id)
            .collect(),
    }
}