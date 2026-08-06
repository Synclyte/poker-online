/**
 * Main game logic file, handles game flow and server input
 */

pub mod poker;
pub mod ai;
pub mod special;
mod tests;
pub mod wasm;
pub(crate) mod api;

use poker::*;
use special::*;
use ai::*;

use std::collections::HashSet;
use std::fmt::{self, Debug};
use serde::{Serialize, Deserialize};
use rand::{SeedableRng, seq::{SliceRandom}};
use rand_chacha::ChaCha12Rng;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameError {
    InvalidConfig(String),
    ConfigLocked,
    InvalidRound,
    InvalidTurn,
    NoActiveTurnPlayer,
    UnknownPlayer,
    RoomFull,
    GameAlreadyStarted,
    InvalidPlayerType,
    InvalidAction,
    RaiseBelowMin,
    RaiseBlocked,
    SpecialCardsBlocked,
    SpecialCardNotOwned,
    InvalidTargetPlayer,
    CardIndexInvalid,
    TurnEndedBeforeAction,
    SpecialCardsFull,
    DuplicateConfigID,
    PrimaryMoveAlreadyMade,
    HandSwapFailed,
}
impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidConfig(reason) => return write!(f, "Failed to parse given config: {}", reason),
            Self::ConfigLocked => "Failed to apply config: round has already started",
            Self::InvalidRound => "Action is not valid in the current round",
            Self::InvalidTurn => "Invalid turn order",
            Self::NoActiveTurnPlayer => "No active turn player",
            Self::UnknownPlayer => "Could not find player with specified id",
            Self::RoomFull => "Room already at max capacity",
            Self::GameAlreadyStarted => "Game already started",
            Self::InvalidPlayerType => "Specified player type is invalid",
            Self::InvalidAction => "Invalid move",
            Self::RaiseBelowMin => "Raise below minimum threshold",
            Self::RaiseBlocked => "Raising is currently disabled",
            Self::SpecialCardsBlocked => "Special cards are currently disabled",
            Self::SpecialCardNotOwned => "Cannot play non-owned special card",
            Self::InvalidTargetPlayer => "Invalid target player",
            Self::CardIndexInvalid => "Card index is out of range",
            Self::TurnEndedBeforeAction => "Must act before ending move",
            Self::SpecialCardsFull => "Could not deal special card: already full",
            Self::DuplicateConfigID => "Provided configuration contained duplicate user IDs",
            Self::PrimaryMoveAlreadyMade => "Already performed primary action",
            Self::HandSwapFailed => "Hand swap failed: target has better hand",
        };

        f.write_str(message)
    }
}
impl GameError {
    pub fn error_type(&self) -> ErrorType {
        match self {
            Self::RaiseBlocked | Self::SpecialCardsBlocked | Self::SpecialCardsFull | Self::HandSwapFailed => ErrorType::Warning,
            _ => ErrorType::Error
        }
    }
}

pub enum ErrorType {
    Error,
    Warning,
}

#[derive(Clone, Copy)]
enum PlayerType {
    Human,
    Computer(AI),
}
impl PlayerType {
    fn from_str(player_type: &str) -> Option<Self> {
        match player_type.to_lowercase().as_str() {
            "human" => Some(PlayerType::Human),
            "ai_risky" => Some(PlayerType::Computer(AI::new(AIType::Risky))),
            "ai_safe" => Some(PlayerType::Computer(AI::new(AIType::Safe))),
            "ai_smart" => Some(PlayerType::Computer(AI::new(AIType::Smart))),
            "ai_random" => Some(PlayerType::Computer(AI::new(AIType::Random))),
            _ => None
        }
    }
}

#[derive(Clone)]
pub(crate) struct Player {
    player_type: PlayerType,
    chips: i32,
    total_bet: i32,
    round_bet: i32,
    cards: Vec<ExpandedCard>,
    card_visibility: Vec<Vec<usize>>,
    special_cards: Vec<SpecialCard>,
    id: usize,
    acted: bool,
    turn_ended: bool,
    folded: bool,
    remove: bool,
}
impl Player {
    fn new(player_type: PlayerType, id: usize) -> Self {
        Player { 
            player_type, 
            chips: 0, 
            total_bet: 0, 
            round_bet: 0, 
            cards: Vec::new(), 
            card_visibility: Vec::new(), 
            special_cards: Vec::new(), 
            id, 
            acted: false, 
            turn_ended: false, 
            folded: false, 
            remove: false 
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum Action {
    Raise { amount: i32 },
    Fold,
    Call,
    Timeout,
    EndMove,
    PlaySpecial { 
        card: SpecialCard, 
        target_id: Option<usize>, 
        card_index: Option<usize> 
    },
    DiscardSpecial { card_index: usize },
}
impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Action::Raise { amount } => return write!(f, "RAISE {}", amount),
            Action::Fold => "FOLD",
            Action::Call => "CALL",
            Action::Timeout => "TIMEOUT",
            Action::EndMove => "ENDMOVE",
            Action::PlaySpecial { 
                card, 
                target_id, 
                card_index 
            } => return write!(f, "SPECIAL {} {} {}", card, target_id.unwrap_or(0), card_index.unwrap_or(0)),
            Action::DiscardSpecial { card_index } => return write!(f, "DISCARD {}", card_index),
        };

        f.write_str(message)
    }
}

impl Action {
    fn from_str(action: &str) -> Option<Self> {
        let action_args: Vec<&str> = action.split(' ').collect();
        match action_args[0].to_lowercase().as_str() {
            "raise" => {
                let amount_str = action_args.get(1)?;
                let amount = amount_str.parse::<i32>().unwrap_or(0);
                if amount <= 0 { 
                    None 
                } else {
                    Some(Action::Raise { amount } )
                }
            }
            "fold" => Some(Action::Fold),
            "call" => Some(Action::Call),
            "timeout" => Some(Action::Timeout),
            "endmove" => Some(Action::EndMove),
            "special" => {
                let card_name = action_args.get(1)?;
                let card = SpecialCard::from_str(card_name)?;
                let target_id = action_args.get(2).and_then(|s| s.parse::<usize>().ok());
                let card_index = action_args.get(3).and_then(|s| s.parse::<usize>().ok());
                
                Some(Action::PlaySpecial { card, target_id, card_index })
            },
            "discardspecial" => {
                let index = action_args.get(1).and_then(|s| s.parse::<usize>().ok())?;

                Some(Action::DiscardSpecial { card_index: index })
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
enum Round {
    Room,
    Preround,
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
    GameEnd,
}
impl fmt::Display for Round {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Round::Room => "room",
            Round::Preround => "preround",
            Round::Preflop => "preflop",
            Round::Flop => "flop",
            Round::Turn => "turn",
            Round::River => "river",
            Round::Showdown => "showdown",
            Round::GameEnd => "gameover",
        };

        f.write_str(message)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeckType {
    Standard,
    Half,
    Double,
    Restricted,
    JokerStandard,
    TwoJokerStandard,
}
impl DeckType {
    pub fn from_str(deck_str: &str) -> Self {
        match deck_str.to_lowercase().as_str() {
            "half" => DeckType::Half,
            "double" => DeckType::Double,
            "restricted" => DeckType::Restricted,
            "1joker" => DeckType::JokerStandard,
            "2jokers" => DeckType::TwoJokerStandard,
            "standard" | _ => DeckType::Standard,
        }
    }

    /**
     * Gets a custom deck, with contents depending on the provided string
     */
    pub(crate) fn get_associated_deck(self) -> Vec<ExpandedCard> {
        match self {
            DeckType::Restricted => Self::get_deck(0, Some(
                |c| c.card().rank as usize >= 5
            )),
            DeckType::Double => { 
                let mut deck = Self::get_deck(0, None);
                deck.extend(Self::get_deck(0, None));
                deck
            },
            DeckType::Half => Self::get_deck(0,Some(
                |c| c.card().suit == Suit::Spades || c.card().suit == Suit::Hearts
            )),
            DeckType::JokerStandard => Self::get_deck(1, None),
            DeckType::TwoJokerStandard => Self::get_deck(2, None),
            _ => Self::get_deck(0, None),
        }
    }

    /**
     Produces a Vec<ExpandedCard> deck, given a number of jokers to be added to the standard deck and rules to filter cards out
    */
    fn get_deck(jokers: usize, rule: Option<fn(&ExpandedCard) -> bool>) -> Vec<ExpandedCard> {
        let default_rule: fn(&ExpandedCard) -> bool = |_: &ExpandedCard| true;
        let card_rule = rule.unwrap_or(default_rule);
        let mut deck: Vec<ExpandedCard> = Vec::new();
        for suit in poker::SUITS {
            for rank in poker::RANKS {
                let card = ExpandedCard::PlayingCard(Card {suit, rank});
                if card_rule(&card) {
                    deck.push(card);
                }
            }
        }
        deck.extend((0..jokers).into_iter().map(|_| ExpandedCard::Joker(Card {suit: Suit::Spades, rank: Rank::Ace})));
        deck
    }
}

struct GameContext {
    hand_size: usize,
    rng: ChaCha12Rng,
    hand_fns: Vec<(HandType, fn(&Vec<ExpandedCard>, usize) -> Option<Hand>)>,
    blind_size: i32,
    min_raise: i32,
    starting_chips: i32,
    special_card_limit: usize,
    deck_type: DeckType,
    max_players: usize,
    round_limit: i32,
    id: usize,
}
impl GameContext {
    fn new(
        hand_size: usize, 
        rng: ChaCha12Rng, 
        mut permitted_hands: Vec<HandType>, 
        blind_size: i32, 
        min_raise: i32, 
        starting_chips: i32,
        special_card_limit: usize,
        deck_type: DeckType, 
        max_players: usize,
        round_limit: i32,
    ) -> Self {
        permitted_hands.sort_by(|a, b| b.cmp(a));
        let hand_fns: Vec<(HandType, fn(&Vec<ExpandedCard>, usize) -> Option<Hand>)> = permitted_hands
            .iter()
            .map(|h| (*h, HandType::get_check_fn(*h))).collect();

        GameContext { 
            hand_size, 
            rng, 
            hand_fns, 
            blind_size, 
            min_raise, 
            starting_chips, 
            special_card_limit, 
            deck_type, 
            max_players, 
            round_limit,
            id: 0 
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum ConfigPlayer {
    Human { id: usize },
    Bot { id: usize, ai_type: AIType }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GameConfig {
    pub players: Vec<ConfigPlayer>,
    pub rng_seed: u64,
    pub blind_size: i32,
    pub min_raise: i32,
    pub starting_chips: i32,
    pub special_card_limit: usize,
    pub deck_type: DeckType,
    pub max_players: usize,
    pub round_limit: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pot {
    pub amount: i32,
    pub players: Vec<usize>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoveEvent {
    pub actor_id: Option<usize>,
    pub action: MoveAction,
    pub private: bool,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum MoveAction {
    Action { action: Action },
    DealHole { count: usize },
    DealCommunity { count: usize },
    DealSpecial { count: usize },
    RemoveHole { index: usize },
    RemoveCommunity { index: usize },
    ReplaceHole { index: usize },
    ReplaceCommunity { index: usize },
    RevealCard { target_id: usize, card_index: usize },
    SwapBot,
    SwapHuman,
}

pub struct Game {
    pub(crate) ctx: GameContext,
    pub(crate) round: Round,
    pub(crate) games_played: i32,
    pub(crate) players: Vec<Player>,
    pub(crate) turn_index: usize,
    pub(crate) dealer_index: usize,
    pub(crate) bet: i32,
    pub(crate) round_pool: i32,
    pub(crate) pots: Vec<Pot>,
    pub(crate) deck: Vec<ExpandedCard>,
    pub(crate) community: Vec<ExpandedCard>,
    pub(crate) winning_hand_type: String,
    pub(crate) modifiers: RoundModifiers,
    pub(crate) winner_ids: Vec<usize>,
}
impl Game {
    /**
     * Creates a game, given a valid JSON config string. Config is expected to be formed as a valid GameConfig struct
     */
    pub fn new() -> Self {
        let ctx = GameContext::new(
            5, 
            ChaCha12Rng::seed_from_u64(0), 
            HAND_TYPES.to_vec(), 
            20, 
            10, 
            1000, 
            0, 
            DeckType::Standard, 
            4, 
            30
        );
        let deck = DeckType::Standard.get_associated_deck();

        Game { 
            ctx, 
            round: Round::Room, 
            games_played: 0,
            players: Vec::new(), 
            turn_index: 0,
            dealer_index: 0,
            bet: 0,
            round_pool: 0,
            pots: Vec::new(),
            deck, 
            community: Vec::new(),
            winning_hand_type: String::new(),
            modifiers: RoundModifiers::default(),
            winner_ids: Vec::new(),
        }
    }

    /**
     * Updates the current game config with a newly provided one
     * Returns a JSON string containing an error if not possible
     */
    pub(crate) fn update_config(&mut self, config: GameConfig) -> Result<(), GameError> {
        if !matches!(self.round, Round::Room | Round::Preround | Round::GameEnd) {
            return Err(GameError::ConfigLocked); 
        }

        if config.players.len() > config.max_players {
            return Err(GameError::RoomFull);
        }

        let mut ctx = GameContext::new(
            5, 
            ChaCha12Rng::seed_from_u64(config.rng_seed), 
            HAND_TYPES.to_vec(), 
            config.blind_size, 
            config.min_raise, 
            config.starting_chips,
            config.special_card_limit,
            config.deck_type, 
            config.max_players,
            config.round_limit,
        );
        let deck = config.deck_type.get_associated_deck();

        let mut ids: HashSet<usize> = HashSet::new();

        let mut players: Vec<Player> = Vec::with_capacity(config.players.len());
        for config_player in config.players {
            let (player_type, id) = match config_player {
                ConfigPlayer::Human { id } => (PlayerType::Human, id),
                ConfigPlayer::Bot { id, ai_type } => (PlayerType::Computer(AI::new(ai_type)), id),
            };

            if ids.contains(&id) {
                return Err(GameError::DuplicateConfigID);
            }

            ctx.id = ctx.id.max(id.saturating_add(1));
            ids.insert(id);
            players.push(Player::new(player_type, id));
        }
    
        self.ctx = ctx;
        self.deck = deck;
        self.players = players;

        Ok(())
    }

    /**
     * Attempts to add a player to the room
     * Returns a JSON string containing either the ID of the new player or an error
     */
    pub(crate) fn try_add_player(&mut self, player_type: PlayerType) -> Result<usize, GameError> {
        if self.players.len() >= self.ctx.max_players {
            return Err(GameError::RoomFull);
        } else if self.round >= Round::Preflop {
            return Err(GameError::GameAlreadyStarted);
        }

        let mut new_player = Player::new(player_type, self.ctx.id);
        new_player.chips = self.ctx.starting_chips;
        self.players.push(new_player);
        self.ctx.id += 1;
        
        Ok(self.ctx.id - 1)
    }

    /**
     * Resets all player and game related data, progressing the round from Room to Preround
     */
    pub fn initialise(&mut self) -> Result<(), GameError> {
        if self.round != Round::Room && self.round != Round::GameEnd {
            return Err(GameError::InvalidRound);
        }

        self.games_played = 0;
        self.winner_ids.clear();

        let start_chips = if self.ctx.starting_chips > 0 { self.ctx.starting_chips } else { 1000 };
        self.players.iter_mut().for_each(|p| {
            p.chips = start_chips;
            p.folded = false;
            p.acted = false;
            p.turn_ended = false;
            p.cards = Vec::new();
            p.special_cards = Vec::new();
        });

        self.deck = self.ctx.deck_type.get_associated_deck();
        self.community = Vec::new();
        self.bet = 0;
        self.round = Round::Preround;
        self.pots = Vec::new();
        self.round_pool = 0;
        self.winning_hand_type.clear();
        self.dealer_index = 0;
        self.modifiers = RoundModifiers::default();

        Ok(())
    }

    /**
     * Progresses the round from the intermission to the Preflop
     */
    pub(crate) fn start(&mut self) -> Result<Vec<MoveEvent>, GameError> {
        let mut events = Vec::new();
        match self.round {
            Round::Preround => self.start_new_hand(&mut events),
            Round::Showdown => {
                self.end_hand();
                if self.round != Round::GameEnd && self.round != Round::Room {
                    self.start_new_hand(&mut events);
                }
            },
            _ => return Err(GameError::InvalidRound),
        }

        if self.round == Round::Room || self.round == Round::GameEnd {
            return Ok(events);
        }

        self.advance_game_loop(&mut events);
        Ok(events)
    }

    /**
     * Public API allowing players to make moves to modify the internal game state
     * Given a player id and action, handles the full process of validating, executing, and advancing the game loop
     * Returns a JSON string containing either an error or a response indicating the events which occurred and the complete new game state
     */
    pub(crate) fn player_move(&mut self, player_id: usize, action: Action) -> Result<Vec<MoveEvent>, GameError> {
        let current_turn_id = self.players
            .get(self.turn_index)
            .map(|player| player.id)
            .ok_or(GameError::NoActiveTurnPlayer)?;

        if current_turn_id != player_id {
            return Err(GameError::InvalidTurn);
        }

        let mut events = Vec::new();
        self.apply_action(self.turn_index, action, &mut events)?;
        self.advance_game_loop(&mut events);

        Ok(events)
    }

    /**
     * Queues a player for removal at the end of the current round
     * Returns a GameError if not possible
     */
    pub fn queue_remove_player(&mut self, player_id: usize) -> Result<(), GameError> {
        if let Ok(player_index) = self.get_player_index(player_id as usize) {
            self.players[player_index].remove = true;
            Ok(())
        } else {
            Err(GameError::InvalidTargetPlayer)
        }
    }

    /**
     * Immediately removes a player from the game
     * Returns a GameError if not possible
     */
    pub fn force_remove_player(&mut self, player_id: usize) -> Result<(), GameError> {
        if let Ok(player_index) = self.get_player_index(player_id) {
            self.players.remove(player_index);
            if self.players.is_empty() {
                self.turn_index = 0;
                self.dealer_index = 0;
                self.round = Round::Room;
                return Ok(());
            }

            if player_index < self.turn_index {
                self.turn_index -= 1;
            }
            if player_index < self.dealer_index {
                self.dealer_index -= 1;
            }

            self.turn_index %= self.players.len();
            self.dealer_index %= self.players.len();
            Ok(())
        } else {
            Err(GameError::UnknownPlayer)
        }
    }

    /**
     * Swaps a player with a given ID's type with a bot or human
     * Returns a JSON string including events and the new game state after swapping
     */
    pub(crate) fn toggle_id_with_bot(&mut self, player_id: usize, bot: bool) -> Result<Vec<MoveEvent>, GameError> {
        let index = match self.get_player_index(player_id) {
            Ok(index) => index,
            Err(err) => return Err(err),
        };

        let mut event_log = Vec::new();
        
        if bot {
            let pid = self.players[index].id;

            event_log.push(MoveEvent { 
                actor_id: Some(pid), 
                action: MoveAction::SwapBot, 
                private: false 
            });

            let new_ai = AI::new(AIType::Safe);
            self.players[index].player_type = PlayerType::Computer(new_ai);

            if self.turn_index == index && !self.players[index].folded && self.round >= Round::Preflop && self.round < Round::Showdown {
                self.advance_game_loop(&mut event_log);
            }
        } else {
            let pid = self.players[index].id;

            event_log.push(MoveEvent { 
                actor_id: Some(pid), 
                action: MoveAction::SwapHuman, 
                private: false 
            });

            self.players[index].player_type = PlayerType::Human;
        }

        Ok(event_log)
    }

    pub fn get_current_turn_player_id(&self) -> usize {
        self.players[self.turn_index].id
    }

    /**
     * Determines how the game should advance after a turn is completed
     */
    fn advance_game_loop(&mut self, events: &mut Vec<MoveEvent>) {
        loop {
            if self.round == Round::Room || self.round == Round::Preround {
                break;
            }

            // if all other players have folded, end the hand
            let unfolded_players: Vec<usize> = self.players.iter().filter(|p| !p.folded).map(|p| p.id).collect();
            if unfolded_players.len() <= 1 {
                if let Some(&winner_id) = unfolded_players.first() {
                    self.award_pot_to_single_winner(winner_id);
                }
                break;
            }

            // otherwise, if the current round is now complete, advance the round
            if self.is_round_complete() {
                self.advance_to_next_round(events);
                if self.round == Round::Preround || self.round == Round::Showdown {
                    break;
                }
                continue;
            }

            let turn_complete = self.current_turn_complete();
            let current_player = &self.players[self.turn_index];

            if !turn_complete {
                match current_player.player_type {
                    PlayerType::Computer(mut ai) => {
                        let mut ai_actions: Vec<Action> = ai.calculate_next_action(self.turn_index, self);
                        ai_actions.push(Action::EndMove);

                        for ai_action in ai_actions {
                            let _ = self.apply_action(self.turn_index, ai_action, events);
                        }
                        
                        if self.current_turn_complete() {
                            self.advance_turn_index();
                        }
                        continue;
                    }
                    PlayerType::Human => {
                        break;
                    }
                }
            } else if current_player.folded || current_player.round_bet == self.bet {
                self.advance_turn_index();
            } else {
                break;    
            }          
        }
    }

    /**
     * Given an action from a player at a given .players() index, executes that action
     * Modifies the internal game state and an event log, including additional intermediate bot moves
     * Returns an error if applicable
     */
    fn apply_action(&mut self, player_index: usize, action: Action, events: &mut Vec<MoveEvent>) -> Result<(), GameError> {
        let player = self.players.get(player_index).ok_or(GameError::NoActiveTurnPlayer)?;
        // prevent players from making a main action twice
        if player.acted && matches!(action, Action::Raise { amount: _ } | Action::Fold | Action::Call) {
            return Err(GameError::PrimaryMoveAlreadyMade);
        }

        if player.turn_ended || player.folded {
            return Err(GameError::InvalidAction);
        }

        let player_id = self.players[player_index].id;
        let mut action_events: Vec<MoveEvent> = Vec::new();

        match action {
            Action::Fold => {
                let p = &mut self.players[player_index];
                p.folded = true;
                p.acted = true;
            }
            Action::Call => {
                let p = &mut self.players[player_index];
                let call_amount = (self.bet - p.round_bet).min(p.chips);
                p.chips -= call_amount;
                p.round_bet += call_amount;
                p.total_bet += call_amount;
                self.round_pool += call_amount;
                p.acted = true;
            }
            Action::Raise { amount } => {
                if self.modifiers.raises_blocked() {
                    return Err(GameError::RaiseBlocked);
                }

                let p = &self.players[player_index];
                let max_raise = p.chips - (self.bet - p.round_bet).max(0);
                let true_raise = amount.min(max_raise);
                let raise_cost = ((self.bet + true_raise) - p.round_bet).min(p.chips);

                if amount < (self.ctx.min_raise as f64 * self.modifiers.vars.ante_multiplier) as i32 && p.chips > raise_cost {
                    return Err(GameError::RaiseBelowMin);
                }

                let p = &mut self.players[player_index];
                p.chips -= raise_cost;
                p.round_bet += raise_cost;
                p.total_bet += raise_cost;
                self.round_pool += raise_cost;
                self.bet = p.round_bet.max(self.bet);

                for (i, other) in self.players.iter_mut().enumerate() {
                    if i != player_index && !other.folded && other.chips > 0 {
                        other.acted = false;
                        other.turn_ended = false;
                    }
                }
                self.players[player_index].acted = true;
            }
            Action::Timeout => {
                let p = &mut self.players[player_index];
                if !p.acted {
                    p.folded = true;
                }
                p.acted = true;
                p.turn_ended = true;
            }
            Action::EndMove => {
                let p = &mut self.players[player_index];
                if !p.acted {
                    return Err(GameError::TurnEndedBeforeAction);
                }
                p.acted = true;
                p.turn_ended = true;
            }
            Action::PlaySpecial { card, target_id, card_index } => {
                if self.modifiers.specials_blocked() {
                    return Err(GameError::SpecialCardsBlocked);
                }

                // special card usage should be visible before its effects
                card.use_card(player_id, target_id, card_index, self, &mut action_events)?;
            },
            Action::DiscardSpecial { card_index } => {
                if card_index >= self.players[player_index].special_cards.len() {
                    return Err(GameError::CardIndexInvalid);
                }

                let card = self.players[player_index].special_cards[card_index];
                card.discard_card(player_id, self, &mut action_events)?;
                self.players[player_index].special_cards.remove(card_index);
            }
        }

        events.push(MoveEvent { 
            actor_id: Some(player_id), 
            action: MoveAction::Action { action }, 
            private: false 
        });

        // ensures events caused by an action are always queued after the action itself
        events.extend(action_events);

        Ok(())
    }

    fn current_turn_complete(&self) -> bool {
        let current_player = &self.players[self.turn_index];
        let can_play_specials = !current_player.special_cards.is_empty() && !self.modifiers.specials_blocked();

        current_player.folded || (current_player.acted || current_player.chips == 0) && (!can_play_specials || current_player.turn_ended)
    }

    fn advance_turn_index(&mut self) {
        self.turn_index = (self.turn_index + 1) % self.players.len();

        let next_player_id = self.players[self.turn_index].id;
        // clear expired blocks
        let expired = self.modifiers.player_turn(next_player_id);
        self.clear_modifier_effects(expired);
    }

    fn clear_modifier_effects(&mut self, expired: Vec<Modifier>) {
        for m in expired {
            m.remove_effect(self);
        }
    }

    /**
     * Gets .players() array index from player id
     */
    fn get_player_index(&self, player_id: usize) -> Result<usize, GameError> {
        if let Some(pos) = self.players.iter().position(|p| p.id == player_id) {
            Ok(pos)
        } else {
            Err(GameError::UnknownPlayer)
        }
    }

    fn process_pots(&mut self) {
        loop {
            let mut min_bet = i32::MAX;
            let mut active_bets = 0;

            for p in &self.players {
                if p.round_bet > 0 {
                    min_bet = min_bet.min(p.round_bet);
                    active_bets += 1;
                }
            }

            if active_bets == 0 || min_bet == i32::MAX {
                break;
            }

            if self.pots.is_empty() {
                self.pots.push(Pot { amount: 0, players: Vec::new() });
            }

            let pot_i = self.pots.len() - 1;
            
            for player in self.players.iter_mut() {
                if player.round_bet > 0 {
                    let contribution = player.round_bet.min(min_bet);
                    player.round_bet -= contribution;
                    
                    self.pots[pot_i].amount += contribution;
                    self.pots[pot_i].players.push(player.id);
                }
            }

            let has_remaining_bets = self.players.iter().any(|p| p.round_bet > 0);
            if has_remaining_bets {
                self.pots.push(Pot { amount: 0, players: Vec::new() });
            }
        }

        self.round_pool = 0;
    }

    fn is_round_complete(&self) -> bool {
        let active_players: Vec<&Player> = self.players.iter().filter(|p| !p.folded).collect();
        if active_players.len() <= 1 {
            return true;
        }
        active_players.iter().all(|p| p.chips == 0 || (p.acted && p.round_bet == self.bet))
    }

    fn advance_to_next_round(&mut self, events: &mut Vec<MoveEvent>) {
        self.process_pots();

        let expired = self.modifiers.round_ended();
        self.clear_modifier_effects(expired);

        for p in self.players.iter_mut() {
            p.round_bet = 0;
            p.acted = false;
            p.turn_ended = false;
        }
        self.bet = 0;
        self.round_pool = 0;

        let len = self.players.len();

        let start_index = if len == 2 { 
            self.dealer_index 
        } else { 
            (self.dealer_index + 1) % len 
        };
        
        let mut first_turn = start_index;
        for _ in 0..len {
            if !self.players[first_turn].folded && self.players[first_turn].chips > 0 {
                break;
            }
            first_turn = (first_turn + 1) % len;
        }
        self.turn_index = first_turn;

        match self.round {
            Round::Preflop => {
                self.deal_community_cards(3, events);
                self.round = Round::Flop;
            }
            Round::Flop => {
                self.deal_community_cards(1, events);
                self.deal_special_cards(1, events);
                self.round = Round::Turn;
            }
            Round::Turn => {
                self.deal_community_cards(1, events);
                self.round = Round::River;
            }
            Round::River => {
                self.round = Round::Showdown;
                self.resolve_showdown();
            }
            _ => {}
        }
    }

    fn start_new_hand(&mut self, events: &mut Vec<MoveEvent>) {
        let active_count = self.players.iter().filter(|p| p.chips > 0).count();
        if active_count <= 1 {
            self.round = Round::Room;
            self.pots.clear();
            self.community.clear();
            self.round_pool = 0;
            self.bet = 0;
            return;
        }

        self.round = Round::Preflop;
        self.bet = 0;
        self.round_pool = 0;
        self.pots.clear();
        self.community.clear();
        self.winning_hand_type.clear();

        self.deck = self.ctx.deck_type.get_associated_deck();
        self.deck.shuffle(&mut self.ctx.rng);

        for p in self.players.iter_mut() {
            // reset player params
            p.cards.clear();
            p.round_bet = 0;
            p.total_bet = 0;
            p.acted = false;
            p.turn_ended = false;
            p.folded = p.chips <= 0;

            // reset ai params
            if let PlayerType::Computer(ref mut ai) = p.player_type {
                ai.update_round_parameters();
            }
        }

        self.deal_hole_cards(2, events);
        let len = self.players.len();

        let mut make_player_pay_blind = |player_index: usize, blind_size: i32| {
            let blind_size = blind_size.min(self.players[player_index].chips);
            self.players[player_index].chips -= blind_size;
            self.players[player_index].total_bet = blind_size;
            self.players[player_index].round_bet = blind_size;

            return blind_size;
        };

        let mut pay_blinds = |sb_index: usize, end_index: usize| {
            let sb_index = sb_index;
            let bb_index = (sb_index + 1) % len;

            let blind_size = (self.ctx.blind_size as f64 * self.modifiers.vars.ante_multiplier) as i32;

            let sb = make_player_pay_blind(sb_index, blind_size / 2);
            let bb = make_player_pay_blind(bb_index, blind_size);

            self.round_pool = sb + bb;
            self.bet = bb;
            self.turn_index = end_index;
        };

        // heads up rules if 2 players
        if len == 2 {
            pay_blinds(self.dealer_index, self.dealer_index);
        // otherwise typical rules
        } else if len > 2 {
            pay_blinds((self.dealer_index + 1) % len, (self.dealer_index + 3) % len);
        } else {
            self.turn_index = 0;
        }
    }

    fn deal_hole_cards(&mut self, count: usize, events: &mut Vec<MoveEvent>) {
        for _ in 0..count {
            for i in 0..self.players.len() {
                self.deal_hole_card(self.players[i].id, events);
            }
        }
    }

    fn deal_hole_card(&mut self, player_id: usize, events: &mut Vec<MoveEvent>) {
        let player_index = self.get_player_index(player_id).unwrap();
        if self.players[player_index].folded {
            return;
        }

        let next_card = self.deck.pop().unwrap_or(ExpandedCard::Unmarked);
        self.players[player_index].cards.push(next_card);
        self.players[player_index].card_visibility.push(vec![]);

        events.push(MoveEvent { 
            actor_id: Some(player_id), 
            action: MoveAction::DealHole { count: 1 }, 
            private: false 
        });
    }

    fn remove_hole_card(&mut self, player_id: usize, card_idx: usize, events: &mut Vec<MoveEvent>) -> Result<(), GameError> {
        let player_index = self.get_player_index(player_id).unwrap();
        if self.players[player_index].cards.get(card_idx).is_none() {
            return Err(GameError::CardIndexInvalid);
        }

        self.players[player_index].cards.remove(card_idx);
        self.players[player_index].card_visibility.remove(card_idx);
        events.push(MoveEvent { 
            actor_id: Some(player_id), 
            action: MoveAction::RemoveHole { index: card_idx }, 
            private: false 
        
        });
        Ok(())
    }

    /**
     * Replaces a specified hole card of a specified player with a new hole card, populating an event Vector
     */
    fn replace_hole_card(&mut self, player_id: usize, card_idx: usize, replacement: ExpandedCard, events: &mut Vec<MoveEvent>) -> Result<(), GameError> {
        let player_index = self.get_player_index(player_id).unwrap();
        if self.players[player_index].cards.get(card_idx).is_none() {
            return Err(GameError::CardIndexInvalid);
        }

        self.players[player_index].cards[card_idx] = replacement;

        events.push(MoveEvent { 
            actor_id: Some(player_id), 
            action: MoveAction::ReplaceHole { index: card_idx }, 
            private: false 
        });

        Ok(())
    }

    /**
     * Replaces a specified community card with a provided replacement
     */
    fn replace_community_card(&mut self, card_idx: usize, replacement: ExpandedCard, events: &mut Vec<MoveEvent>) -> Result<(), GameError> {
        if self.community.get(card_idx).is_none() {
            return Err(GameError::CardIndexInvalid);
        }

        self.community[card_idx] = replacement;

        events.push(MoveEvent { 
            actor_id: None, 
            action: MoveAction::ReplaceCommunity { index: card_idx }, 
            private: false 
        });

        Ok(())
    }

    /**
     * Generic pot distribution function
     */
    fn distribute_pots<T: Ord>(&mut self, mut entries: Vec<(usize, T)>) {
        // sort in descending order
        entries.sort_by(|(_, a), (_, b)| b.cmp(a));

        // apply pot mult
        for pot in &mut self.pots {
            pot.amount = (pot.amount as f64 * self.modifiers.vars.pot_multiplier).round() as i32;
        }

        // iterates through all pots, finding winners for each and individually distributing chips
        for pot in &self.pots {
            let mut winners = Vec::new();
            if let Some(winning_entry) = entries.iter().find(|(id, _)| pot.players.contains(id)) {
                let winning_hand = &winning_entry.1;
                for (id, hand) in entries.iter() {
                    if pot.players.contains(id) && hand >= winning_hand {
                        winners.push(*id);
                    }
                }
            }

            if !winners.is_empty() {
                let split = pot.amount / winners.len() as i32;
                let mut remainder = pot.amount % winners.len() as i32;
                for winner_id in winners {
                    if let Ok(index) = self.get_player_index(winner_id) {
                        self.players[index].chips += split;
                        if remainder > 0 {
                            self.players[index].chips += 1;
                            remainder -= 1;
                        }
                    }
                }
            }
        }
    }

    fn resolve_showdown(&mut self) {
        self.process_pots();

        // this function structure allows for modifiers to overwrite the default scoring
        // so long as they can produce a Vec of Ord elements to sort players by
        if self.modifiers.blackjack_active() {
            let scores: Vec<(usize, i32)> = self.players.iter()
                .filter(|p| !p.folded)
                .map(|p| (p.id, special::evaluate_blackjack_hand(&p.cards)))
                .collect();

            if let Some((_, best_score)) = scores.iter().max_by_key(|(_, score)| score) {
                self.winning_hand_type = if *best_score == -1 {
                    "Bust".to_string()
                } else {
                    best_score.to_string()
                };
            }

            self.distribute_pots(scores);            
        } else {
            let invalid_hands = self.modifiers.invalidated_hands();
            let hands: Vec<(usize, Hand)> = self.players.iter()
                .filter(|p| !p.folded)
                .map(|p| {
                    let cards = [&self.community[..], &p.cards[..]].concat();
                    (p.id, Hand::new(&cards, &self.ctx.hand_fns, &invalid_hands, self.ctx.hand_size))
                }).collect();

            if let Some((_, winning_hand)) = hands.iter().max_by(|(_, a), (_, b)| a.cmp(b)) {
                self.winning_hand_type = winning_hand.hand_type.to_string();
            }

            self.distribute_pots(hands);
        }

        self.pots.clear();
        self.round_pool = 0;
    }

    fn award_pot_to_single_winner(&mut self, winner_id: usize) {
        self.process_pots();
        let raw_pot: i32 = self.pots.iter().map(|p| p.amount).sum::<i32>();
        let total_pot = (raw_pot as f64 * self.modifiers.vars.pot_multiplier).round() as i32;
        if let Ok(index) = self.get_player_index(winner_id) {
            self.players[index].chips += total_pot;
            self.turn_index = index;
        }
        self.pots.clear();
        self.round_pool = 0;

        self.round = Round::Showdown;
    }

    fn end_hand(&mut self) {
        self.games_played += 1;
        self.players.retain(|p| !p.remove);

        let expired = self.modifiers.hand_ended();
        self.clear_modifier_effects(expired);

        if self.games_played >= self.ctx.round_limit {
            self.end_game();
            return;
        }

        if self.players.is_empty() {
            self.round = Round::Room;
            return;
        }

        self.dealer_index = (self.dealer_index + 1) % self.players.len();
        self.round = Round::Preround;
    }

    /**
     * Handles the full process of dealing n community cards to the board, consuming a use of active
       draw rules and populating a given event Vector
     */
    fn deal_community_cards(&mut self, count: usize, events: &mut Vec<MoveEvent>) {
        for _ in 0..count {
            let card = self.pop_next_deck_card();
            self.community.push(card);
        }

        events.push(MoveEvent { 
            actor_id: None, 
            action: MoveAction::DealCommunity { count }, 
            private: false 
        });
    }

    /**
     * Draws a single deck card from the pile, adhering to draw rules and producing a draw event
     * Importantly, this does not populate the event array 
     */
    fn pop_next_deck_card(&mut self) -> ExpandedCard {
        let draw_rules = self.modifiers.draw_rule(self);
        let card = if let Some(index) = self.deck.iter().rposition(|card| draw_rules(card)) {
            self.deck.remove(index)
        } else {
            ExpandedCard::Unmarked
        };

        let expired = self.modifiers.community_draw();
        self.clear_modifier_effects(expired);

        card
    }

    fn remove_community_card(&mut self, index: usize, events: &mut Vec<MoveEvent>) -> Result<(), GameError> {
        if index >= self.community.len() {
            return Err(GameError::CardIndexInvalid);
        }

        self.community.remove(index);
        events.push(MoveEvent { 
            actor_id: None, 
            action: MoveAction::RemoveCommunity { index }, 
            private: false 
        });
        
        Ok(())
    }

    fn deal_special_cards(&mut self, count: usize, events: &mut Vec<MoveEvent>) {
        for p_index in 0..self.players.len() {
            let special_card_count = self.players[p_index].special_cards.len();
            if self.players[p_index].folded || special_card_count >= self.ctx.special_card_limit as usize {
                continue;
            }

            let special_capacity = self.ctx.special_card_limit.saturating_sub(special_card_count);
            if special_capacity <= 0 {
                continue;
            }

            let p_id = self.players[p_index].id;
            let drawn_count = count.min(special_capacity);
            let drawn_cards = SpecialCard::draw_special_cards(drawn_count, p_id, self);
            self.players[p_index].special_cards.extend(drawn_cards);

            events.push(MoveEvent { 
                actor_id: Some(p_id), 
                action: MoveAction::DealSpecial { count: drawn_count }, 
                private: false 
            });
        }
    }

    fn end_game(&mut self) {
        let max_chips = self.players.iter().map(|p| p.chips).max().unwrap_or(0);
        let winners: Vec<usize> = self.players.iter().filter_map(|p| if p.chips == max_chips { Some(p.id) } else { None }).collect();
        self.winner_ids = winners;

        let expired = self.modifiers.active.clone();
        expired.iter().for_each(|m| m.remove_effect(self));
        self.modifiers.game_ended();

        self.round = Round::GameEnd;
    }
}