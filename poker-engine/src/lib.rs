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
    CardAlreadyVisible,
}
impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidConfig(reason) => return write!(f, "Failed to parse given config: {}", reason),
            Self::ConfigLocked => "Failed to apply config: round has already started",
            Self::InvalidRound => "Invalid action",
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
            Self::CardAlreadyVisible => "Target card is already revealed to you",
        };

        f.write_str(message)
    }
}
impl GameError {
    pub fn error_type(&self) -> ErrorType {
        match self {
            Self::RaiseBlocked
            | Self::SpecialCardsBlocked
            | Self::SpecialCardsFull
            | Self::HandSwapFailed
            | Self::CardAlreadyVisible => ErrorType::Warning,
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
    pub(crate) extra_special_capacity: usize,
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
            extra_special_capacity: 0,
            id, 
            acted: false, 
            turn_ended: false, 
            folded: false, 
            remove: false 
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum Action {
    Raise { amount: i32 },
    Fold,
    Call,
    Timeout,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
pub(crate) enum Round {
    Room,
    Preround,
    Preflop,
    Flop,
    Turn,
    River,
    WinnerDraft,
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
            Round::WinnerDraft => "winnerdraft",
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
    max_bet_multiplier: Option<u32>,
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
        max_bet_multiplier: Option<u32>,
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
            max_bet_multiplier,
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
    #[serde(default)]
    pub max_bet_multiplier: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pot {
    pub amount: i32,
    pub players: Vec<usize>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MoveEvent {
    pub actor_id: Option<usize>,
    pub action: MoveAction,
    pub private: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum MoveAction {
    Action {
        action: Action,
        #[serde(skip_serializing_if = "Option::is_none")]
        bet_delta: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        round_bet: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        total_bet: Option<i32>,
    },
    DealHole { count: usize },
    DealCommunity { count: usize },
    DealSpecial { count: usize },
    RemoveHole { index: usize },
    RemoveCommunity { index: usize },
    ReplaceHole { index: usize },
    ReplaceCommunity { index: usize },
    RevealCard { target_id: usize, card_index: usize },
    GainChips {
        amount: i32,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        after_showdown: bool,
    },
    SwapBot,
    SwapHuman,
    DraftSkippedFull,
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
    pub(crate) special: SpecialState,
    pub(crate) winner_ids: Vec<usize>,
    pub(crate) pending_drafts: Vec<WinnerDraft>,
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
            4, 
            DeckType::Standard,  
            4, 
            30,
            Some(25),
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
            special: SpecialState::default(),
            winner_ids: Vec::new(),
            pending_drafts: Vec::new(),
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
            config.max_bet_multiplier,
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

    pub(crate) fn get_current_max_bet(&self) -> Option<i32> {
        let mult = self.ctx.max_bet_multiplier?;
        if mult == 0 {
            return None;
        }
        let effective_blind = ((self.ctx.blind_size as f64 * self.special.ante_multiplier()) as i32).max(1);
        Some(effective_blind * mult as i32)
    }

    pub(crate) fn is_player_at_max_bet(&self, player: &Player) -> bool {
        if let Some(max_bet) = self.get_current_max_bet() {
            player.total_bet >= max_bet
        } else {
            false
        }
    }

    pub(crate) fn is_player_all_in(&self, player: &Player) -> bool {
        player.chips == 0 || self.is_player_at_max_bet(player)
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
        self.players.retain(|p| !p.remove);

        let start_chips = if self.ctx.starting_chips > 0 { self.ctx.starting_chips } else { 1000 };
        self.players.iter_mut().for_each(|p| {
            p.chips = start_chips;
            p.total_bet = 0;
            p.round_bet = 0;
            p.folded = false;
            p.acted = false;
            p.turn_ended = false;
            p.cards = Vec::new();
            p.card_visibility.clear();
            p.special_cards = Vec::new();
            p.extra_special_capacity = 0;
        });

        self.deck = self.ctx.deck_type.get_associated_deck();
        self.community = Vec::new();
        self.bet = 0;
        self.round = Round::Preround;
        self.pots = Vec::new();
        self.round_pool = 0;
        self.winning_hand_type.clear();
        self.dealer_index = 0;
        self.special = SpecialState::default();
        self.pending_drafts.clear();

        Ok(())
    }

    /**
     * Progresses the round from the intermission to the Preflop
     */
    pub(crate) fn start(&mut self) -> Result<Vec<MoveEvent>, GameError> {
        let mut events = Vec::new();
        match self.round {
            Round::Preround => self.start_new_hand(&mut events),
            Round::WinnerDraft => {
                events.extend(self.timeout_winner_draft());
                self.end_hand(&mut events);
                if self.round != Round::GameEnd && self.round != Round::Room {
                    self.start_new_hand(&mut events);
                }
            }
            Round::Showdown => {
                self.end_hand(&mut events);
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

            self.pending_drafts.retain(|draft| draft.player_id != player_id);
            if self.round == Round::WinnerDraft && self.pending_drafts.is_empty() {
                self.round = Round::Showdown;
            }
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

            if !self.pending_drafts.is_empty() {
                if let Some(pos) = self.pending_drafts.iter().position(|d| d.player_id == pid) {
                    let draft = self.pending_drafts.remove(pos);
                    if let Some(&card) = draft.options.first() {
                        self.grant_drafted_card(pid, card, draft.bypass_limit, &mut event_log);
                    }
                    if self.round == Round::WinnerDraft && self.pending_drafts.is_empty() {
                        self.round = Round::Showdown;
                    }
                }
            }
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
            if matches!(
                self.round,
                Round::Room | Round::Preround | Round::WinnerDraft | Round::Showdown | Round::GameEnd
            ) {
                break;
            }

            // if all other players have folded, end the hand
            let unfolded_players: Vec<usize> = self.players.iter().filter(|p| !p.folded).map(|p| p.id).collect();
            if unfolded_players.len() <= 1 {
                if let Some(&winner_id) = unfolded_players.first() {
                    self.award_pot_to_single_winner(winner_id, events);
                }
                break;
            }

            // otherwise, if the current round is now complete, advance the round
            if self.is_round_complete() {
                self.advance_to_next_round(events);
                if matches!(self.round, Round::Preround | Round::WinnerDraft | Round::Showdown) {
                    break;
                }
                continue;
            }

            let turn_complete = self.current_turn_complete();
            let current_player = &self.players[self.turn_index];
            let _can_play_specials = !current_player.special_cards.is_empty() && !self.special.specials_blocked();

            if !turn_complete {
                match current_player.player_type {
                    PlayerType::Computer(mut ai) => {
                        let ai_actions: Vec<Action> = ai.calculate_next_action(self.turn_index, self);

                        for ai_action in ai_actions {
                            let res = self.apply_action(self.turn_index, ai_action, events);
                            if res.is_err() {
                                if self.apply_action(self.turn_index, Action::Call, events).is_err() {
                                    let _ = self.apply_action(self.turn_index, Action::Fold, events);
                                }
                            }
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
            } else {
                self.advance_turn_index();
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
        if player.folded {
            return Err(GameError::InvalidAction);
        }

        if player.acted || player.turn_ended {
            return Err(GameError::PrimaryMoveAlreadyMade);
        }

        let player_id = self.players[player_index].id;
        let mut action_events: Vec<MoveEvent> = Vec::new();
        let mut logged_action = action.clone();
        let mut bet_delta: Option<i32> = None;
        let mut round_bet_after: Option<i32> = None;
        let mut total_bet_after: Option<i32> = None;

        match action {
            Action::Fold => {
                self.force_fold_player(player_index, &mut action_events);
            }
            Action::Call => {
                let maybe_max_bet = self.get_current_max_bet();
                let call_amount = {
                    let p = &mut self.players[player_index];
                    let max_hand_capacity = if let Some(max_bet) = maybe_max_bet {
                        (max_bet - p.total_bet).max(0)
                    } else {
                        i32::MAX
                    };
                    let call_amount = (self.bet - p.round_bet).max(0).min(p.chips).min(max_hand_capacity);
                    p.chips -= call_amount;
                    p.round_bet += call_amount;
                    p.total_bet += call_amount;
                    self.round_pool += call_amount;
                    p.acted = true;
                    p.turn_ended = true;

                    bet_delta = Some(call_amount);
                    round_bet_after = Some(p.round_bet);
                    total_bet_after = Some(p.total_bet);
                    call_amount
                };
                if call_amount > 0 {
                    special::on_event(
                        GameEvent::PlayerBet {
                            player_id,
                            amount: call_amount,
                            kind: BetKind::Call,
                        },
                        self,
                        &mut action_events,
                    );
                }
            }
            Action::Raise { amount } => {
                if self.special.raises_blocked() {
                    return Err(GameError::RaiseBlocked);
                }

                let p = &self.players[player_index];
                if self.is_player_at_max_bet(p) {
                    return Err(GameError::InvalidAction);
                }

                let maybe_max_bet = self.get_current_max_bet();
                let max_hand_capacity = if let Some(max_bet) = maybe_max_bet {
                    (max_bet - p.total_bet).max(0)
                } else {
                    i32::MAX
                };

                let call_portion = (self.bet - p.round_bet).max(0).min(p.chips).min(max_hand_capacity);
                let remaining_raise_cap = (max_hand_capacity - call_portion).max(0);

                if remaining_raise_cap == 0 {
                    return Err(GameError::InvalidAction);
                }

                let remaining_chips_for_raise = (p.chips - (self.bet - p.round_bet).max(0)).max(0);
                let max_raise = remaining_chips_for_raise.min(remaining_raise_cap);
                let true_raise = amount.min(max_raise);
                if max_raise == 0 || true_raise <= 0 {
                    return Err(GameError::InvalidAction);
                }
                let raise_cost = ((self.bet + true_raise) - p.round_bet).min(p.chips).min(max_hand_capacity);

                let effective_min_raise = (self.ctx.min_raise as f64 * self.special.ante_multiplier()) as i32;
                let new_total_bet = p.total_bet + raise_cost;
                let reached_cap = maybe_max_bet.map(|m| new_total_bet >= m).unwrap_or(false);

                if amount < effective_min_raise && p.chips > raise_cost && !reached_cap {
                    return Err(GameError::RaiseBelowMin);
                }

                let p = &mut self.players[player_index];
                p.chips -= raise_cost;
                p.round_bet += raise_cost;
                p.total_bet += raise_cost;
                self.round_pool += raise_cost;
                self.bet = p.round_bet.max(self.bet);

                logged_action = Action::Raise { amount: true_raise };
                bet_delta = Some(raise_cost);
                round_bet_after = Some(p.round_bet);
                total_bet_after = Some(p.total_bet);

                if true_raise > 0 {
                    for (i, other) in self.players.iter_mut().enumerate() {
                        if i != player_index && !other.folded {
                            let can_play_specials = !other.special_cards.is_empty() && !self.special.specials_blocked();
                            if other.chips > 0 || can_play_specials {
                                other.acted = false;
                                other.turn_ended = false;
                            }
                        }
                    }
                }
                self.players[player_index].acted = true;
                self.players[player_index].turn_ended = true;

                if raise_cost > 0 {
                    special::on_event(
                        GameEvent::PlayerBet {
                            player_id,
                            amount: raise_cost,
                            kind: BetKind::Raise,
                        },
                        self,
                        &mut action_events,
                    );
                }
            }
            Action::Timeout => {
                if !self.players[player_index].acted {
                    self.force_fold_player(player_index, &mut action_events);
                } else {
                    let p = &mut self.players[player_index];
                    p.acted = true;
                    p.turn_ended = true;
                }
            }
            Action::PlaySpecial { card, target_id, card_index } => {
                if self.special.specials_blocked() {
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
            action: MoveAction::Action {
                action: logged_action,
                bet_delta,
                round_bet: round_bet_after,
                total_bet: total_bet_after,
            }, 
            private: false 
        });

        // ensures events caused by an action are always queued after the action itself
        events.extend(action_events);

        Ok(())
    }

    pub(crate) fn force_fold_player(&mut self, player_index: usize, events: &mut Vec<MoveEvent>) {
        let (player_id, total_bet) = {
            let p = &mut self.players[player_index];
            p.folded = true;
            p.acted = true;
            p.turn_ended = true;
            (p.id, p.total_bet)
        };
        special::on_event(GameEvent::PlayerFolded { player_id, total_bet }, self, events);
    }

    fn current_turn_complete(&self) -> bool {
        let current_player = &self.players[self.turn_index];
        let can_play_specials = !current_player.special_cards.is_empty() && !self.special.specials_blocked();

        current_player.folded || current_player.acted || current_player.turn_ended || (self.is_player_all_in(current_player) && !can_play_specials)
    }

    fn advance_turn_index(&mut self) {
        self.turn_index = (self.turn_index + 1) % self.players.len();

        let next_player_id = self.players[self.turn_index].id;
        // clear expired blocks
        special::on_event(
            GameEvent::TurnAdvanced { player_id: next_player_id },
            self,
            &mut Vec::new(),
        );
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
        active_players.iter().all(|p| {
            let can_play_specials = !p.special_cards.is_empty() && !self.special.specials_blocked();
            (self.is_player_all_in(p) && (!can_play_specials || p.acted || p.turn_ended))
                || (p.acted && p.round_bet == self.bet)
        })
    }

    fn advance_to_next_round(&mut self, events: &mut Vec<MoveEvent>) {
        self.process_pots();

        special::on_event(GameEvent::StreetEnded, self, events);

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
                self.deal_special_cards(1, events);
                self.round = Round::Flop;
            }
            Round::Flop => {
                self.deal_community_cards(1, events);
                self.round = Round::Turn;
            }
            Round::Turn => {
                self.deal_community_cards(1, events);
                self.round = Round::River;
            }
            Round::River => {
                self.round = Round::Showdown;
                self.resolve_showdown(events);
            }
            _ => {}
        }
    }

    fn get_next_active_player_index(&self, from_idx: usize) -> usize {
        let len = self.players.len();
        if len == 0 { return 0; }
        let mut curr = (from_idx + 1) % len;
        for _ in 0..len {
            if self.players[curr].chips > 0 && !self.players[curr].folded {
                return curr;
            }
            curr = (curr + 1) % len;
        }
        from_idx
    }

    fn get_next_player_with_chips(&self, from_idx: usize) -> usize {
        let len = self.players.len();
        if len == 0 {
            return 0;
        }
        for offset in 1..=len {
            let idx = (from_idx + offset) % len;
            if self.players[idx].chips > 0 {
                return idx;
            }
        }
        from_idx % len
    }

    fn make_player_pay_blind(&mut self, player_index: usize, blind_size: i32, events: &mut Vec<MoveEvent>) -> i32 {
        let actual_blind = blind_size.min(self.players[player_index].chips);
        self.players[player_index].chips -= actual_blind;
        self.players[player_index].total_bet = actual_blind;
        self.players[player_index].round_bet = actual_blind;
        if actual_blind > 0 {
            let player_id = self.players[player_index].id;
            special::on_event(
                GameEvent::PlayerBet {
                    player_id,
                    amount: actual_blind,
                    kind: BetKind::Blind,
                },
                self,
                events,
            );
        }
        actual_blind
    }

    fn pay_blinds(&mut self, sb_index: usize, end_index: usize, events: &mut Vec<MoveEvent>) {
        let bb_index = self.get_next_active_player_index(sb_index);
        let blind_size = (self.ctx.blind_size as f64 * self.special.ante_multiplier()) as i32;

        let sb = self.make_player_pay_blind(sb_index, blind_size / 2, events);
        let bb = self.make_player_pay_blind(bb_index, blind_size, events);

        self.round_pool = sb + bb;
        self.bet = sb.max(bb);
        self.turn_index = end_index;
    }

    fn start_new_hand(&mut self, events: &mut Vec<MoveEvent>) {
        let active_human_count = self.players.iter()
            .filter(|p| matches!(p.player_type, PlayerType::Human) && p.chips > 0)
            .count();

        let active_indices: Vec<usize> = self.players.iter().enumerate()
            .filter(|(_, p)| p.chips > 0)
            .map(|(i, _)| i)
            .collect();

        if active_indices.len() <= 1 || active_human_count == 0 {
            self.round = Round::Room;
            self.pots.clear();
            self.community.clear();
            self.round_pool = 0;
            self.bet = 0;
            return;
        }

        if self.dealer_index >= self.players.len() {
            self.dealer_index = 0;
        }
        if self.players[self.dealer_index].chips == 0 {
            self.dealer_index = self.get_next_player_with_chips(self.dealer_index);
        }

        self.round = Round::Preflop;
        self.bet = 0;
        self.round_pool = 0;
        self.pots.clear();
        self.community.clear();
        self.winning_hand_type.clear();
        self.pending_drafts.clear();

        self.deck = self.ctx.deck_type.get_associated_deck();
        self.deck.shuffle(&mut self.ctx.rng);

        for p in self.players.iter_mut() {
            // reset player params
            p.cards.clear();
            p.card_visibility.clear();
            p.round_bet = 0;
            p.total_bet = 0;
            if p.chips == 0 {
                p.acted = true;
                p.turn_ended = true;
                p.folded = true;
            } else {
                p.acted = false;
                p.turn_ended = false;
                p.folded = false;
            }

            // reset ai params
            if let PlayerType::Computer(ref mut ai) = p.player_type {
                ai.update_round_parameters();
            }
        }

        special::on_event(GameEvent::HandStarted, self, events);

        self.deal_hole_cards(2, events);

        // heads up rules if 2 active players
        if active_indices.len() == 2 {
            self.pay_blinds(self.dealer_index, self.dealer_index, events);
        // otherwise typical rules
        } else if active_indices.len() > 2 {
            let sb_idx = self.get_next_active_player_index(self.dealer_index);
            let bb_idx = self.get_next_active_player_index(sb_idx);
            let first_turn = self.get_next_active_player_index(bb_idx);
            self.pay_blinds(sb_idx, first_turn, events);
        } else {
            self.turn_index = self.get_next_active_player_index(self.dealer_index);
        }
    }

    fn deal_hole_cards(&mut self, count: usize, events: &mut Vec<MoveEvent>) {
        for _ in 0..count {
            for i in 0..self.players.len() {
                self.deal_hole_card(self.players[i].id, events);
            }
        }
    }

    pub(crate) fn deal_hole_card(&mut self, player_id: usize, events: &mut Vec<MoveEvent>) {
        let player_index = self.get_player_index(player_id).unwrap();
        if self.players[player_index].folded {
            return;
        }

        let reference_card = self.players[player_index]
            .cards
            .last()
            .copied()
            .or_else(|| self.community.last().copied());
        let next_card = special::draw_deck_card(
            self,
            DrawTarget::Hole { player_id },
            reference_card,
            events,
        );
        let player_index = self.get_player_index(player_id).unwrap();
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
     * Generic pot distribution function. Returns the winner IDs and total chip payouts of all winning players
     */
    fn distribute_pots<T: Ord>(&mut self, mut entries: Vec<(usize, T)>) -> (Vec<usize>, Vec<(usize, i32)>) {
        // sort in descending order
        entries.sort_by(|(_, a), (_, b)| b.cmp(a));

        // apply pot mult
        let pot_mult = self.special.pot_multiplier();
        for pot in &mut self.pots {
            pot.amount = (pot.amount as f64 * pot_mult).round() as i32;
        }

        let mut main_pot_winners = Vec::new();
        let mut payouts: std::collections::HashMap<usize, i32> = std::collections::HashMap::new();

        // iterates through all pots, finding winners for each and individually distributing chips
        for (pot_idx, pot) in self.pots.iter().enumerate() {
            let mut winners = Vec::new();
            if let Some(winning_entry) = entries.iter().find(|(id, _)| pot.players.contains(id)) {
                let winning_hand = &winning_entry.1;
                for (id, hand) in entries.iter() {
                    if pot.players.contains(id) && hand >= winning_hand {
                        winners.push(*id);
                    }
                }
            }

            if pot_idx == 0 {
                main_pot_winners = winners.clone();
            }

            if !winners.is_empty() {
                let split = pot.amount / winners.len() as i32;
                let mut remainder = pot.amount % winners.len() as i32;
                for winner_id in winners {
                    if let Ok(index) = self.get_player_index(winner_id) {
                        let payout = split + if remainder > 0 { remainder -= 1; 1 } else { 0 };
                        self.players[index].chips += payout;
                        *payouts.entry(winner_id).or_insert(0) += payout;
                    }
                }
            }
        }

        let payout_list: Vec<(usize, i32)> = payouts.into_iter().collect();
        (main_pot_winners, payout_list)
    }

    fn resolve_showdown(&mut self, events: &mut Vec<MoveEvent>) {
        self.process_pots();
        let total_chips: i32 = self.players.iter().map(|p| p.chips).sum::<i32>()
            + self.pots.iter().map(|p| p.amount).sum::<i32>();
        let main_pot_amount = self.pots.first().map(|p| p.amount).unwrap_or(0);

        // this function structure allows for modifiers to overwrite the default scoring
        // so long as they can produce a Vec of Ord elements to sort players by
        let (main_pot_winners, payouts) = if self.special.blackjack_active() {
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

            self.distribute_pots(scores)
        } else {
            let invalid_hands = self.special.invalidated_hands();
            let hands: Vec<(usize, Hand)> = self.players.iter()
                .filter(|p| !p.folded)
                .map(|p| {
                    let cards = [&self.community[..], &p.cards[..]].concat();
                    (p.id, Hand::new(&cards, &self.ctx.hand_fns, &invalid_hands, self.ctx.hand_size))
                }).collect();

            if let Some((_, winning_hand)) = hands.iter().max_by(|(_, a), (_, b)| a.cmp(b)) {
                self.winning_hand_type = winning_hand.hand_type.to_string();
            }

            self.distribute_pots(hands)
        };

        self.pots.clear();
        self.round_pool = 0;

        special::on_event(GameEvent::ShowdownResolved { payouts }, self, events);

        self.prepare_winner_drafts(&main_pot_winners, main_pot_amount, total_chips, events);
    }

    fn award_pot_to_single_winner(&mut self, winner_id: usize, events: &mut Vec<MoveEvent>) {
        self.process_pots();
        let total_chips: i32 = self.players.iter().map(|p| p.chips).sum::<i32>()
            + self.pots.iter().map(|p| p.amount).sum::<i32>();
        let raw_pot: i32 = self.pots.iter().map(|p| p.amount).sum::<i32>();
        let total_pot = (raw_pot as f64 * self.special.pot_multiplier()).round() as i32;
        if let Ok(index) = self.get_player_index(winner_id) {
            self.players[index].chips += total_pot;
            self.turn_index = index;
        }
        self.pots.clear();
        self.round_pool = 0;

        special::on_event(
            GameEvent::ShowdownResolved {
                payouts: vec![(winner_id, total_pot)],
            },
            self,
            events,
        );

        self.prepare_winner_drafts(&[winner_id], raw_pot, total_chips, events);
    }

    pub(crate) fn player_special_card_limit(&self, player_index: usize) -> usize {
        if self.ctx.special_card_limit == 0 {
            0
        } else {
            self.ctx
                .special_card_limit
                .saturating_add(self.players[player_index].extra_special_capacity)
        }
    }

    fn grant_drafted_card(&mut self, player_id: usize, card: SpecialCard, bypass_limit: bool, events: &mut Vec<MoveEvent>) {
        if let Ok(idx) = self.get_player_index(player_id) {
            if bypass_limit || self.players[idx].special_cards.len() < self.player_special_card_limit(idx) {
                self.players[idx].special_cards.push(card);
                special::on_event(GameEvent::SpecialReceived { player_id, card }, self, events);
                events.push(MoveEvent {
                    actor_id: Some(player_id),
                    action: MoveAction::DealSpecial { count: 1 },
                    private: false,
                });
            } else {
                events.push(MoveEvent {
                    actor_id: Some(player_id),
                    action: MoveAction::DraftSkippedFull,
                    private: true,
                });
            }
        }
    }

    fn prepare_winner_drafts(
        &mut self,
        main_pot_winners: &[usize],
        pot_amount: i32,
        total_chips: i32,
        events: &mut Vec<MoveEvent>,
    ) {
        self.pending_drafts.clear();

        if self.ctx.special_card_limit == 0 || main_pot_winners.is_empty() || total_chips <= 0 {
            self.round = Round::Showdown;
            return;
        }

        let effective_blind = ((self.ctx.blind_size as f64 * self.special.ante_multiplier()) as i32).max(1);
        let min_contested_pot = effective_blind + (effective_blind / 2);
        let pot_ratio = (pot_amount as f64 / total_chips as f64).clamp(0.0, 1.0);

        // disables winner drafts if the pot is too small relative to the overall wealth of all players
        if pot_amount <= min_contested_pot || pot_ratio < 0.03 {
            self.round = Round::Showdown;
            return;
        }

        for &winner_id in main_pot_winners {
            let Ok(idx) = self.get_player_index(winner_id) else {
                continue;
            };
            if self.players[idx].remove {
                continue;
            }

            if self.players[idx].special_cards.len() >= self.player_special_card_limit(idx) {
                events.push(MoveEvent {
                    actor_id: Some(winner_id),
                    action: MoveAction::DraftSkippedFull,
                    private: true,
                });
                continue;
            }

            let options = SpecialCard::draw_winner_draft_options(3, pot_ratio, self);
            if options.is_empty() {
                continue;
            }

            match self.players[idx].player_type {
                PlayerType::Computer(_) => {
                    if let Some(&auto_card) = options.first() {
                        self.grant_drafted_card(winner_id, auto_card, false, events);
                    }
                }
                PlayerType::Human => {
                    self.pending_drafts.push(WinnerDraft {
                        player_id: winner_id,
                        options,
                        bypass_limit: false,
                    });
                }
            }
        }

        self.round = if self.pending_drafts.is_empty() {
            Round::Showdown
        } else {
            Round::WinnerDraft
        };
    }

    pub(crate) fn pick_winner_draft(&mut self, player_id: usize, choice_index: usize) -> Result<Vec<MoveEvent>, GameError> {
        if self.round != Round::WinnerDraft && self.pending_drafts.is_empty() {
            return Err(GameError::InvalidRound);
        }

        let pos = self
            .pending_drafts
            .iter()
            .position(|draft| draft.player_id == player_id)
            .ok_or(GameError::InvalidAction)?;

        if choice_index >= self.pending_drafts[pos].options.len() {
            return Err(GameError::CardIndexInvalid);
        }

        let draft = self.pending_drafts.remove(pos);
        let chosen_card = draft.options[choice_index];

        let mut events = Vec::new();
        self.grant_drafted_card(player_id, chosen_card, draft.bypass_limit, &mut events);

        if self.round == Round::WinnerDraft && self.pending_drafts.is_empty() {
            self.round = Round::Showdown;
        }

        Ok(events)
    }

    pub(crate) fn timeout_winner_draft(&mut self) -> Vec<MoveEvent> {
        if self.round != Round::WinnerDraft {
            return Vec::new();
        }

        let drafts = std::mem::take(&mut self.pending_drafts);
        let mut events = Vec::new();

        for draft in drafts {
            if let Some(&auto_card) = draft.options.first() {
                self.grant_drafted_card(draft.player_id, auto_card, draft.bypass_limit, &mut events);
            }
        }

        self.round = Round::Showdown;
        events
    }

    fn end_hand(&mut self, events: &mut Vec<MoveEvent>) {
        self.games_played += 1;
        let removed_before_dealer = self
            .players
            .iter()
            .take(self.dealer_index)
            .filter(|p| p.remove)
            .count();
        let dealer_removed = self
            .players
            .get(self.dealer_index)
            .map_or(false, |p| p.remove);
        self.players.retain(|p| !p.remove);

        special::on_event(GameEvent::HandEnded, self, events);

        if self.games_played >= self.ctx.round_limit {
            self.end_game();
            return;
        }

        if self.players.is_empty() {
            self.dealer_index = 0;
            self.round = Round::Room;
            return;
        }

        let adjusted_dealer = self.dealer_index.saturating_sub(removed_before_dealer) % self.players.len();
        self.dealer_index = if dealer_removed {
            if self.players[adjusted_dealer].chips > 0 {
                adjusted_dealer
            } else {
                self.get_next_player_with_chips(adjusted_dealer)
            }
        } else {
            self.get_next_player_with_chips(adjusted_dealer)
        };
        self.round = Round::Preround;
    }

    /**
     * Handles the full process of dealing n community cards to the board, consuming a use of active
       draw rules and populating a given event Vector
     */
    fn deal_community_cards(&mut self, count: usize, events: &mut Vec<MoveEvent>) {
        for _ in 0..count {
            let reference_card = self.community.last().copied();
            let card = special::draw_deck_card(self, DrawTarget::Community, reference_card, events);
            self.community.push(card);
        }

        events.push(MoveEvent { 
            actor_id: None, 
            action: MoveAction::DealCommunity { count }, 
            private: false 
        });
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
        let len = self.players.len();
        if len == 0 {
            return;
        }

        for offset in 0..len {
            let p_index = (self.dealer_index + 1 + offset) % len;
            let special_card_count = self.players[p_index].special_cards.len();
            let player_limit = self.player_special_card_limit(p_index);
            if self.players[p_index].folded || special_card_count >= player_limit {
                continue;
            }

            let special_capacity = player_limit.saturating_sub(special_card_count);
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

        special::on_event(GameEvent::GameEnded, self, &mut Vec::new());

        self.round = Round::GameEnd;
    }

    pub(crate) fn execute_console_command(
        &mut self,
        command: api::ConsoleCommand,
    ) -> Result<(api::ConsoleCommandOutcome, Vec<MoveEvent>), GameError> {
        if matches!(command, api::ConsoleCommand::Help) {
            return Ok((api::ConsoleCommandOutcome::Help, Vec::new()));
        }

        if matches!(self.round, Round::Room | Round::GameEnd) {
            return Err(GameError::InvalidRound);
        }

        let mut events = Vec::new();

        match command {
            api::ConsoleCommand::Help => unreachable!(),
            api::ConsoleCommand::SetChips { player_id, chips } => {
                if chips < 0 {
                    return Err(GameError::InvalidAction);
                }

                let idx = self.get_player_index(player_id)?;
                self.players[idx].chips = chips;

                if matches!(self.round, Round::Preflop | Round::Flop | Round::Turn | Round::River)
                    && self.current_turn_complete()
                {
                    self.advance_game_loop(&mut events);
                }

                Ok((api::ConsoleCommandOutcome::ChipsSet { player_id, chips }, events))
            }
            api::ConsoleCommand::GiveCard { target, card, count } => {
                if count == 0 {
                    return Err(GameError::InvalidAction);
                }
                if self.round == Round::Preround {
                    return Err(GameError::InvalidRound);
                }

                match target {
                    api::CardTarget::Board => {
                        for _ in 0..count {
                            self.community.push(card);
                        }
                        events.push(MoveEvent {
                            actor_id: None,
                            action: MoveAction::DealCommunity { count },
                            private: false,
                        });
                    }
                    api::CardTarget::Player(player_id) => {
                        let idx = self.get_player_index(player_id)?;
                        for _ in 0..count {
                            self.players[idx].cards.push(card);
                            self.players[idx].card_visibility.push(Vec::new());
                        }
                        events.push(MoveEvent {
                            actor_id: Some(player_id),
                            action: MoveAction::DealHole { count },
                            private: false,
                        });
                    }
                }

                Ok((api::ConsoleCommandOutcome::CardGiven { target, card, count }, events))
            }
            api::ConsoleCommand::GiveSpecial { player_id, card, count } => {
                if count == 0 {
                    return Err(GameError::InvalidAction);
                }
                let idx = self.get_player_index(player_id)?;
                for _ in 0..count {
                    self.players[idx].special_cards.push(card);
                    special::on_event(GameEvent::SpecialReceived { player_id, card }, self, &mut events);
                }
                events.push(MoveEvent {
                    actor_id: Some(player_id),
                    action: MoveAction::DealSpecial { count },
                    private: false,
                });

                Ok((api::ConsoleCommandOutcome::SpecialGiven { player_id, card, count }, events))
            }
            api::ConsoleCommand::DealSpecial { player_id, count } => {
                if count == 0 {
                    return Err(GameError::InvalidAction);
                }
                let idx = self.get_player_index(player_id)?;
                let cards = SpecialCard::draw_special_cards(count, player_id, self);
                if cards.is_empty() {
                    return Err(GameError::InvalidAction);
                }

                let dealt_count = cards.len();
                self.players[idx].special_cards.extend(cards.iter().copied());
                events.push(MoveEvent {
                    actor_id: Some(player_id),
                    action: MoveAction::DealSpecial { count: dealt_count },
                    private: false,
                });

                Ok((api::ConsoleCommandOutcome::SpecialDealt { player_id, cards }, events))
            }
            api::ConsoleCommand::SkipStreet { count } => {
                if count == 0 {
                    return Err(GameError::InvalidAction);
                }
                if !matches!(self.round, Round::Preflop | Round::Flop | Round::Turn | Round::River) {
                    return Err(GameError::InvalidRound);
                }

                let mut skipped = 0usize;
                for _ in 0..count {
                    if !matches!(self.round, Round::Preflop | Round::Flop | Round::Turn | Round::River) {
                        break;
                    }

                    // Sweep active street bets into a single pot without creating 1-player side-pots on unmatched bets
                    let street_bets: i32 = self.players.iter().map(|p| p.round_bet).sum();
                    if street_bets > 0 {
                        let eligible_ids: Vec<usize> = {
                            let unfolded: Vec<usize> = self
                                .players
                                .iter()
                                .filter(|p| !p.folded)
                                .map(|p| p.id)
                                .collect();
                            if unfolded.is_empty() {
                                self.players.iter().filter(|p| p.round_bet > 0).map(|p| p.id).collect()
                            } else {
                                unfolded
                            }
                        };

                        if self.pots.is_empty() {
                            self.pots.push(Pot {
                                amount: 0,
                                players: eligible_ids.clone(),
                            });
                        }
                        if let Some(last_pot) = self.pots.last_mut() {
                            last_pot.amount += street_bets;
                            for pid in &eligible_ids {
                                if !last_pot.players.contains(pid) {
                                    last_pot.players.push(*pid);
                                }
                            }
                        }
                        for p in &mut self.players {
                            p.round_bet = 0;
                        }
                        self.round_pool = 0;
                    }

                    self.advance_to_next_round(&mut events);
                    skipped += 1;
                }

                if matches!(self.round, Round::Preflop | Round::Flop | Round::Turn | Round::River) {
                    self.advance_game_loop(&mut events);
                }

                Ok((
                    api::ConsoleCommandOutcome::StreetsSkipped {
                        skipped,
                        round: self.round,
                    },
                    events,
                ))
            }
            api::ConsoleCommand::SkipHand { count } => {
                if count == 0 {
                    return Err(GameError::InvalidAction);
                }

                if matches!(self.round, Round::Preflop | Round::Flop | Round::Turn | Round::River) {
                    let refunds: Vec<(usize, i32)> = self
                        .players
                        .iter_mut()
                        .filter_map(|p| {
                            let refunded = p.total_bet;
                            if refunded > 0 {
                                p.chips += refunded;
                            }
                            p.total_bet = 0;
                            p.round_bet = 0;
                            if refunded > 0 { Some((p.id, refunded)) } else { None }
                        })
                        .collect();

                    for (player_id, amount) in refunds {
                        special::on_event(GameEvent::BetWithdrawn { player_id, amount }, self, &mut events);
                    }
                    special::on_event(GameEvent::StreetEnded, self, &mut events);
                    self.pots.clear();
                    self.round_pool = 0;
                    self.bet = 0;
                }

                self.pending_drafts.clear();

                let mut skipped = 0usize;
                for _ in 0..count {
                    self.end_hand(&mut events);
                    skipped += 1;
                    if matches!(self.round, Round::GameEnd | Round::Room) {
                        break;
                    }
                }

                if self.round == Round::Preround {
                    self.start_new_hand(&mut events);
                    if matches!(self.round, Round::Preflop | Round::Flop | Round::Turn | Round::River) {
                        self.advance_game_loop(&mut events);
                    }
                }

                Ok((
                    api::ConsoleCommandOutcome::HandsSkipped {
                        skipped,
                        games_played: self.games_played,
                    },
                    events,
                ))
            }
            api::ConsoleCommand::CardDraft {
                player_id,
                simulated_chips,
            } => {
                let current_pot: i32 = self.pots.iter().map(|p| p.amount).sum::<i32>() + self.round_pool;
                let chips = simulated_chips.unwrap_or_else(|| {
                    if current_pot > 0 {
                        current_pot
                    } else {
                        (self.ctx.starting_chips / 2)
                            .max(self.ctx.blind_size.saturating_mul(5))
                            .max(1)
                    }
                });
                if chips < 0 {
                    return Err(GameError::InvalidAction);
                }

                let idx = self.get_player_index(player_id)?;

                let total_chips = {
                    let sum = self.players.iter().map(|p| p.chips).sum::<i32>() + current_pot;
                    if sum > 0 {
                        sum
                    } else {
                        (self.ctx.starting_chips * self.players.len().max(1) as i32).max(1)
                    }
                };

                let pot_ratio = (chips as f64 / total_chips as f64).clamp(0.0, 1.0);
                let options = SpecialCard::draw_winner_draft_options(3, pot_ratio, self);
                if options.is_empty() {
                    return Err(GameError::InvalidAction);
                }

                match self.players[idx].player_type {
                    PlayerType::Computer(_) => {
                        let card = options[0];
                        self.grant_drafted_card(player_id, card, true, &mut events);
                        Ok((
                            api::ConsoleCommandOutcome::BotCardDrafted {
                                player_id,
                                simulated_chips: chips,
                                card,
                            },
                            events,
                        ))
                    }
                    PlayerType::Human => {
                        self.pending_drafts.retain(|d| d.player_id != player_id);
                        self.pending_drafts.push(WinnerDraft {
                            player_id,
                            options,
                            bypass_limit: true,
                        });
                        Ok((
                            api::ConsoleCommandOutcome::HumanCardDraftStarted {
                                player_id,
                                simulated_chips: chips,
                                pot_ratio,
                            },
                            events,
                        ))
                    }
                }
            }
        }
    }
}