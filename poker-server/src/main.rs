use std::{cmp::Ordering::{self, Greater, Less}, fmt::{self, Debug}};
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_json::{self, to_string};
use rand::{RngExt, SeedableRng, rand_core::Rng, rngs::SysRng};
use rand_chacha::ChaCha12Rng;

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug)]
#[repr(u8)]
enum Rank {
    Two = 0,
    Three = 1,
    Four = 2,
    Five = 3, 
    Six = 4,
    Seven = 5,
    Eight = 6,
    Nine = 7,
    Ten = 8,
    Jack = 9,
    Queen = 10,
    King = 11,
    Ace = 12
}
impl std::fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rank::Two => write!(f, "Two"),
            Rank::Three => write!(f, "Three"),
            Rank::Four => write!(f, "Four"),
            Rank::Five => write!(f, "Five"), 
            Rank::Six => write!(f, "Six"), 
            Rank::Seven => write!(f, "Seven"), 
            Rank::Eight => write!(f, "Eight"), 
            Rank::Nine => write!(f, "Nine"), 
            Rank::Ten => write!(f, "Ten"), 
            Rank::Jack => write!(f, "Jack"), 
            Rank::Queen => write!(f, "Queen"), 
            Rank::King => write!(f, "King"), 
            Rank::Ace => write!(f, "Ace"), 
        }
    }
}
impl Rank {
    pub fn from(num: impl TryInto<u8>) -> Rank {
        let n: u8 = num.try_into().unwrap_or(u8::MAX);
        if n >= 13 {
            panic!()
        }
        match n {
            0 => Rank::Two,
            1 => Rank::Three,
            2 => Rank::Four,
            3 => Rank::Five,
            4 => Rank::Six,
            5 => Rank::Seven,
            6 => Rank::Eight,
            7 => Rank::Nine,
            8 => Rank::Ten,
            9 => Rank::Jack,
            10 => Rank::Queen,
            11 => Rank::King,
            12 => Rank::Ace,
            _ => unreachable!()
        }
    }

    pub fn from_str(str: &str) -> Rank {
        match str {
            "2" => Rank::Two,
            "3" => Rank::Three,
            "4" => Rank::Four,
            "5" => Rank::Five,
            "6" => Rank::Six,
            "7" => Rank::Seven,
            "8" => Rank::Eight,
            "9" => Rank::Nine,
            "10" => Rank::Ten,
            "j" => Rank::Jack,
            "k" => Rank::Queen,
            "q" => Rank::King,
            "a" => Rank::Ace,
            _ => unreachable!()
        }
    }
}
static RANKS: [Rank; 13] = [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Suit {
    Spades,
    Diamonds,
    Clubs,
    Hearts,
}
impl std::fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Suit::Spades => write!(f, "Spades"),
            Suit::Clubs => write!(f, "Clubs"),
            Suit::Hearts => write!(f, "Hearts"),
            Suit::Diamonds => write!(f, "Diamonds")
        }
    }
}
impl Suit {
    pub fn from(suit: &str) -> Suit {
        match suit {
            "s" => Suit::Spades,
            "d" => Suit::Diamonds,
            "c" => Suit::Clubs,
            "h" => Suit::Hearts,
            _ => panic!(),
        }
    }
}
static SUITS: [Suit; 4] = [Suit::Spades, Suit::Diamonds, Suit::Clubs, Suit::Hearts];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Card {
    rank: Rank,
    suit: Suit,
}

#[derive(Clone, Copy, Eq)]
enum ExpandedCard {
    PlayingCard(Card),
    Joker(Card),
    Unmarked,
}
impl std::fmt::Display for ExpandedCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExpandedCard::Joker(c) => write!(f, "Joker ({})", ExpandedCard::PlayingCard(*c).to_string()),
            ExpandedCard::PlayingCard(c) => write!(f, "{} of {}", c.rank, c.suit),
            ExpandedCard::Unmarked => write!(f, "Unmarked")
        }
    }
}
impl Debug for ExpandedCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExpandedCard::Joker(c) => write!(f, "Joker ({})", ExpandedCard::PlayingCard(*c).to_string()),
            ExpandedCard::PlayingCard(c) => write!(f, "{} of {}", c.rank, c.suit),
            ExpandedCard::Unmarked => write!(f, "Unmarked")
        }        
    }
}
impl PartialEq for ExpandedCard {
    fn eq(&self, other: &Self) -> bool {
        self.inner_card().rank == other.inner_card().rank
    }
    fn ne(&self, other: &Self) -> bool {
        !(self == other)
    }
}
impl PartialOrd for ExpandedCard {
    fn ge(&self, other: &Self) -> bool {
        self.inner_card().rank >= other.inner_card().rank      
    }

    fn gt(&self, other: &Self) -> bool {
        self.inner_card().rank > other.inner_card().rank      
    }

    fn le(&self, other: &Self) -> bool {
        self.inner_card().rank <= other.inner_card().rank       
    }

    fn lt(&self, other: &Self) -> bool {
        self.inner_card().rank < other.inner_card().rank       
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self < other { Some(Ordering::Less) }
        else if self > other { Some(Ordering::Greater) }
        else { Some(Ordering::Equal) }
    }
}
impl Ord for ExpandedCard {
    fn clamp(self, min: Self, max: Self) -> Self where Self: Sized {
        if self < min { min }
        else if self > max { max }
        else { self }
    }
    
    fn max(self, other: Self) -> Self where Self: Sized {
        if other < self { self } else { other }
    }
    
    fn min(self, other: Self) -> Self where Self: Sized {
        if other < self { other } else { self }
    }
    
    fn cmp(&self, other: &Self) -> Ordering {
        if self < other { Ordering::Less }
        else if self > other { Ordering::Greater }
        else { Ordering::Equal }
    }
}
impl ExpandedCard {
    fn inner_card(&self) -> &Card {
        match self {
            Self::PlayingCard(c) | Self::Joker(c) => c,
            Self::Unmarked => panic!()
        }
    }
    
    fn get_card(rank: &str, suit: &str) -> ExpandedCard {
        ExpandedCard::PlayingCard( Card { rank: Rank::from_str(rank), suit: Suit::from(suit) } )
    }

    fn get_joker() -> ExpandedCard {
        ExpandedCard::Joker( Card { rank: Rank::Ace, suit: Suit::Spades } )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum HandType {
    HighCard = 0,
    Pair = 1,
    TwoPair = 2,
    ThreeOfAKind = 3,
    FullHouse = 4,
    Flush = 5,
    Straight = 6,
    FourOfAKind = 7,
    StraightFlush = 8,
    RoyalFlush = 9,
}
impl fmt::Display for HandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandType::HighCard => write!(f, "High Card"),
            HandType::Pair => write!(f, "Pair"),
            HandType::TwoPair => write!(f, "Two Pair"),
            HandType::ThreeOfAKind => write!(f, "Three of a Kind"),
            HandType::FullHouse => write!(f, "Full House"),
            HandType::Flush => write!(f, "Flush"),
            HandType::Straight => write!(f, "Straight"),
            HandType::FourOfAKind => write!(f, "Four of a Kind"),
            HandType::StraightFlush => write!(f, "Straight Flush"),
            HandType::RoyalFlush => write!(f, "Royal Flush"),
        }
    }
}
impl HandType {
    /**
     Returns relevant function for checking the requested hand type, allowing a function array to be build/iterated over.
     * This is used for 2 main reasons:
     * Improving modularity (allows new hands to be added without much extra effort)
     * Improving code performance (does not attempt to find unused hand types)
     */
    fn get_check_fn(hand_type: HandType) -> fn(&Vec<ExpandedCard>, usize) -> Option<Hand> {
        fn build_hand(hand_info: Option<(Vec<Vec<ExpandedCard>>, Vec<ExpandedCard>)>, hand_type: HandType) -> Option<Hand> {
            if let Some(hand) = hand_info {
                return Some( Hand { hand_type: hand_type, cards: hand.0, spare: hand.1 } );
            }
            None
        }

        match hand_type {
            HandType::RoyalFlush => |cards, hand_size| {
                for i in 0..SUITS.len() {
                    if let Some((cards, spare)) = (get_straight(cards, 5, Some(SUITS[i]), hand_size)) && cards[0][0].inner_card().rank == Rank::Ace {
                        return Some( Hand { hand_type: HandType::RoyalFlush, cards, spare } );
                    }
                }
                None
            },
            HandType::StraightFlush => |cards, hand_size| {
                return SUITS.iter()
                    .filter_map(|s| get_straight(cards, 5, Some(*s), hand_size))
                    .map(|(s_c, s_s)| Hand { hand_type: Self::StraightFlush, cards: s_c, spare: s_s } )
                    .max_by(|h_1, h_2| h_1.compare(h_2))
            },
            HandType::FourOfAKind => |cards, hand_size| build_hand(get_representation(cards, vec![4], None, hand_size), HandType::FourOfAKind),
            HandType::FullHouse => |cards, hand_size| build_hand(get_representation(cards, vec![3, 2], None, hand_size), HandType::FullHouse),
            HandType::Straight => |cards, hand_size| build_hand(get_straight(cards, 5, None, hand_size), HandType::Straight),
            HandType::Flush => |cards, hand_size| {
                return SUITS.iter()
                    .filter_map(|s| get_representation(cards, vec![1, 1, 1, 1, 1], Some(*s), hand_size))
                    .map(|(s_c, s_s)| Hand { hand_type: Self::Flush, cards: s_c, spare: s_s } )
                    .max_by(|h_1, h_2| h_1.compare(h_2))
            },
            HandType::ThreeOfAKind => |cards, hand_size| build_hand(get_representation(cards, vec![3], None, hand_size), HandType::ThreeOfAKind),
            HandType::TwoPair => |cards, hand_size| build_hand(get_representation(cards, vec![2, 2], None, hand_size), HandType::TwoPair),
            HandType::Pair => |cards, hand_size| build_hand(get_representation(cards, vec![2], None, hand_size), HandType::Pair),
            HandType::HighCard => |cards, hand_size| build_hand(get_representation(cards, vec![1], None, hand_size), HandType::HighCard)
        } 
    }
}
static HAND_TYPES: [HandType; 10] = [
    HandType::RoyalFlush, HandType::StraightFlush, HandType::FourOfAKind, HandType::Straight, 
    HandType::Flush, HandType::FullHouse, HandType::ThreeOfAKind, HandType::TwoPair, 
    HandType::Pair, HandType::HighCard
];

struct HandContext<'a> {
    cards: &'a Vec<ExpandedCard>,
    organised_cards: Vec<Vec<usize>>,
    joker_indices: Vec<usize>,
    jokers: usize,
    used_counts: [usize; 13],
    suit: Option<Suit>,
}
impl HandContext<'_> {
    fn new<'a>(cards: &'a Vec<ExpandedCard>, suit: Option<Suit>) -> HandContext<'a> {
        let mut organised_cards: Vec<Vec<usize>> = vec![vec![]; 13];
        let mut joker_indices: Vec<usize> = Vec::new();
        let mut jokers: usize = 0;
        let suit_needed = suit.is_some();
        let required_suit= suit.unwrap_or(Suit::Spades);
        let used_counts: [usize; 13] = [0; 13];

        if suit_needed {
            cards.iter().enumerate().for_each(|(i, c)| match c {
                ExpandedCard::Joker(_) => { jokers += 1; joker_indices.push(i); },
                ExpandedCard::PlayingCard(card) if (suit_needed && card.suit == required_suit) || !suit_needed => { organised_cards[card.rank as usize].push(i); },
                _ => {},
            });
        } else {
            cards.iter().enumerate().for_each(|(i, c)| match c {
                ExpandedCard::Joker(_) => { jokers += 1; joker_indices.push(i); },
                ExpandedCard::PlayingCard(card) => { organised_cards[card.rank as usize].push(i); },
                _ => {}
            });            
        }

        HandContext { cards, organised_cards, joker_indices, jokers, used_counts, suit }
    }

    fn extract_hand(&self, mut representation_cards: Vec<(Rank, Vec<usize>)>, hand_size: usize) -> Option<(Vec<Vec<ExpandedCard>>, Vec<ExpandedCard>)> {
        let mut available_cards: Vec<bool> = vec![true; self.cards.len()];
        let mut joker_iter = self.joker_indices.iter();
        let mut hand_cards: Vec<Vec<ExpandedCard>> = Vec::new();

        // populate empty joker indices
        for i in 0..representation_cards.len() {
            hand_cards.push(vec![]);
            for j in 0..representation_cards[i].1.len() {
                if representation_cards[i].1[j] == usize::MAX {
                    let joker_index = joker_iter.next().unwrap();
                    representation_cards[i].1[j] = *joker_index;
                    hand_cards[i].push(ExpandedCard::Joker( Card { rank: representation_cards[i].0, suit: self.suit.unwrap_or(Suit::Spades)} ))
                } else {
                    hand_cards[i].push(self.cards[representation_cards[i].1[j]]);
                }
                available_cards[representation_cards[i].1[j]] = false;
            }
        }

        // get spare cards to populate remainder of hand
        let representation_card_len = representation_cards.iter().fold(0, |acc, (_, v)| acc + v.len());
        let cards_needed = hand_size - representation_card_len;
        let mut card_iter = available_cards.iter().enumerate();
        let mut extra_cards: Vec<ExpandedCard> = Vec::with_capacity(cards_needed);
        for _ in 0..cards_needed {
            if let Some(next_index) = card_iter.find_map(|(i, available)| if *available { Some(i) } else { None }) {
                extra_cards.push(self.cards[next_index]);
            }
        }
        
        Some((hand_cards, extra_cards))
    }
}

struct GameContext {
    hand_size: usize,
    rng: ChaCha12Rng,
    hand_fns: Vec<fn(&Vec<ExpandedCard>, usize) -> Option<Hand>>,
    round: Round,
}
impl GameContext {
    fn new(hand_size: usize, rng: ChaCha12Rng, mut permitted_hands: Vec<HandType>) -> Self {
        permitted_hands.sort_by(|a, b| b.cmp(a));
        let hand_fns: Vec<fn(&Vec<ExpandedCard>, usize) -> Option<Hand>> = permitted_hands.iter().map(|h| HandType::get_check_fn(*h)).collect();

        GameContext { hand_size, rng, hand_fns, round: Round::Preround }
    }
}

struct Hand {
    hand_type: HandType,
    cards: Vec<Vec<ExpandedCard>>,
    spare: Vec<ExpandedCard>,
}
impl fmt::Display for Hand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {:?}: {:?}", self.hand_type, self.cards, self.spare)
    }
}
impl Hand {
    fn new(cards: &Vec<ExpandedCard>, ctx: &GameContext) -> Self {
        if cards.len() <= 0 {
            panic!();
        }
        for hand_fn in &ctx.hand_fns {
            if let Some(hand) = hand_fn(cards, ctx.hand_size) {
                return hand;
            }
        }
        unreachable!()
    }

    fn compare(&self, other: &Self) -> Ordering {
        if self.hand_type > other.hand_type {
            return Greater;
        } else if self.hand_type < other.hand_type {
            return Less;
        }

        for i in 0..self.cards.len() {
            if self.cards[i][0] > other.cards[i][0] {
                return Greater;
            } else if self.cards[i][0] < other.cards[i][0] {
                return Less;
            }
        }

        for i in 0..self.spare.len().min(other.spare.len()) {
            if self.spare[i] > other.spare[i] {
                return Greater;
            } else if self.spare[i] < other.spare[i] {
                return Less;
            }
        }

        Ordering::Equal
    }
}

#[derive(Clone, Copy)]
enum PlayerType {
    Human,
    AI(AIType),
}

#[derive(Clone, Copy)]
enum AIType {
    Safe,
    Risky,
    Smart,
    Random,
}
impl AIType {
    fn from_str(str: &String) -> Self {
        match str.to_lowercase().as_str() {
            "safe" => AIType::Safe,
            "risky" => AIType::Risky,
            "smart" => AIType::Smart,
            "random" => AIType::Random,
            _ => AIType::Smart,
        } 
    }

    /**
     * Takes the current game state, and returns 
     */
    fn calculate_next_action(&self, player_index: usize, game: &Game) -> Action {
        match self {
            AIType::Safe => Action::Fold,
            AIType::Random => Action::Fold,
            AIType::Risky => Action::Fold,
            AIType::Smart => Action::Fold,
        }
    }
}

// not implemented
enum SpecialCard {
    A,
}

struct Player {
    player_type: PlayerType,
    chips: i32,
    bet: i32,
    cards: Vec<ExpandedCard>,
    special_cards: Vec<SpecialCard>,
    id: usize,
}
impl Player {
    fn new(player_type: PlayerType, id: usize) -> Self {
        Player { player_type, chips: 0, bet: 0, cards: Vec::new(), special_cards: Vec::new(), id }
    }
}

enum Action {
    Raise(i32),
    Fold,
    Call,
}
impl Action {
    fn from_str(action: String, amount: Option<i32>) -> Option<Self> {
        match action.to_lowercase().as_str() {
            "raise" => {
                if let Some(a) = amount && a >= 0 {
                    Some(Action::Raise(a))
                } else {
                    None
                }
            }
            "fold" => Some(Action::Fold),
            "call" => Some(Action::Call),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Round {
    Preround,
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
}
impl fmt::Display for Round {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Round::Preround => write!(f, "Pre-Round"),
            Round::Preflop => write!(f, "Pre-Flop"),
            Round::Flop => write!(f, "Flop"),
            Round::Turn => write!(f, "Turn"),
            Round::River => write!(f, "River"),
            Round::Showdown => write!(f, "Showdown"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pot {
    pub amount: i32,
    pub players: Vec<usize>,
}

#[wasm_bindgen]
pub struct Game {
    ctx: GameContext,
    round: Round,
    players: Vec<Player>,
    player_turn: usize,
    bet: i32,
    round_pool: i32,
    acted: Vec<bool>,
    folded: Vec<bool>,
    pots: Vec<Pot>,
    deck: Vec<ExpandedCard>,
    community: Vec<ExpandedCard>,
}

#[derive(Serialize)]
pub struct PlayerState {
    pub id: usize,
    pub chips: i32,
    pub round_bet: i32,
    pub folded: bool,
    pub acted: bool,
    pub is_turn: bool,
    pub hole_cards: Vec<String>,
}

#[derive(Serialize)]
pub struct GameState {
    pub round_name: String,
    pub community_cards: Vec<String>,
    pub pots: Vec<i32>,
    pub round_bet_sum: i32,
    pub overall_sum: i32,
    pub highest_bet: i32,
    pub deck_cards: usize,
    pub players: Vec<PlayerState>,
}

#[derive(Deserialize)]
pub struct GameConfig {
    pub player_count: usize,
    pub bots: Vec<String>,
    pub rng_seed: u64,
    pub deck_type: String,
    pub min_bet: i32,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str) -> Result<Game, String> {
        let config: GameConfig = serde_json::from_str(config_json)
            .map_err(|e| format!("Failed to parse given config: {}", e))?;

        let total_players = config.player_count + config.bots.len();
        let ctx = GameContext::new(5, ChaCha12Rng::seed_from_u64(config.rng_seed), HAND_TYPES.to_vec());
        let deck = get_custom_deck(config.deck_type);

        let mut id = 0;
        let mut players: Vec<Player> = Vec::with_capacity(total_players);
        for _ in 0..config.player_count {
            players.push(Player::new(PlayerType::Human, id));
            id += 1;
        }
        for bot_type in config.bots {
            players.push(Player::new(PlayerType::AI(AIType::from_str(&bot_type)), id));
            id += 1;
        }

        Ok(Game { 
            ctx, 
            round: Round::Preround, 
            players, 
            player_turn: 0,
            bet: 0,
            round_pool: 0,
            acted: vec![false; total_players],
            folded: vec![false; total_players],
            pots: Vec::new(),
            deck, 
            community: Vec::new() 
        })
    }

    #[wasm_bindgen]
    pub fn start(&mut self) -> Vec<String> {
        if self.round == Round::Preround {
            self.update_round();
            return (0..self.players.len()).map(|i| self.get_game_state(i)).collect();
        }
        let err_str = serde_json::json!({"error": "Game already started"}).to_string();
        vec![err_str]
    }

    #[wasm_bindgen]
    pub fn player_move(&mut self, player_id: i32, str_action: String, amount: i32) -> String {
        let action = Action::from_str(str_action, Some(amount));
        if action.is_none() {
            return serde_json::json!({"error": "Invalid move"}).to_string();
        }

        self.execute_action(player_id, action.unwrap())
    }

    fn execute_action(&mut self, player_id: i32, action: Action) -> String {
        if self.player_turn as i32 != player_id {
            return serde_json::json!({"error": "Invalid turn order"}).to_string();
        } else if self.folded[player_id as usize] {
            return serde_json::json!({"error": "Already folded"}).to_string();
        }

        let id = player_id as usize;
        {
            let player = &mut self.players[player_id as usize];

            match action {
                Action::Fold => self.folded[id] = true,
                Action::Call => {
                    let player_bet = (self.bet - player.bet).min(player.chips);
                    player.chips -= player_bet;
                    player.bet += player_bet;
                    self.round_pool += player_bet;
                },
                Action::Raise(raise) => {
                    if player.chips < raise {
                        return serde_json::json!({"error": "Cannot afford raise"}).to_string();
                    }

                    let player_bet = self.bet - player.bet + raise;
                    player.chips -= player_bet;
                    player.bet += player_bet;
                    self.round_pool += player_bet;
                    self.bet = player_bet;

                    for i in 0..self.acted.len() {
                        self.acted[i] = false;
                    }
                }
            }
        }

        self.acted[id] = true;
        let mut active_players = 0;
        let mut round_finished = true;

        for (i, p) in self.players.iter().enumerate() {
            if !self.folded[i] && p.chips > 0 {
                active_players += 1;
                if !self.acted[i] || p.bet != self.bet {
                    round_finished = false;
                }
            }
        }

        if round_finished || active_players <= 1 {
            self.update_round();
        } else {
            self.player_turn = (self.player_turn + 1) % self.players.len();
            while self.folded[self.player_turn] || self.players[self.player_turn].chips == 0 {
                self.acted[self.player_turn] = true;
                self.player_turn = (self.player_turn + 1) % self.players.len();
            }

            let next_player = &self.players[self.player_turn];
            if let PlayerType::AI(t) = next_player.player_type {
                self.execute_action(self.player_turn as i32, t.calculate_next_action(self.player_turn, &self));
            }
        }

        self.get_game_state(player_id as usize)
    }

    fn process_pots(&mut self) {
        loop {
            let mut min_bet = i32::MAX;
            let mut max_bet = i32::MIN;
            let mut participants = 0;

            for i in 0..self.players.len() {
                let player = &self.players[i];
                if player.bet > 0 {
                    min_bet = min_bet.min(player.bet);
                    max_bet = max_bet.max(player.bet);
                }
                participants += 1;
            }

            if participants == 0 {
                break;
            }

            if self.pots.is_empty() {
                self.pots.push(Pot { amount: 0, players: Vec::new() });
            }
            
            let pot_i = self.pots.len() - 1;
            for i in 0..self.players.len() {
                let player = &mut self.players[i];
                if player.bet > 0 {
                    player.bet -= player.bet.min(min_bet);
                    let current_pot = &mut self.pots[pot_i];
                    current_pot.amount += min_bet;

                    if !self.folded[i] && !current_pot.players.contains(&player.id) {
                        current_pot.players.push(player.id);
                    }
                }
            }

            if min_bet != max_bet {
                self.pots.push(Pot { amount: 0, players: Vec::new() });
            }
        }
    }

    fn update_round(&mut self) {
        self.process_pots();

        let round = self.round.clone();
        fn process_round(game: &mut Game, cards_dealt: usize, special_cards_dealt: usize, next_round: Round) {
            (0..cards_dealt).for_each(|_| {
                let next_card = game.deck.pop().unwrap_or(ExpandedCard::Unmarked);
                game.community.push(next_card);
            });

            game.players.iter_mut().for_each(|p| p.bet = 0);
            game.acted.iter_mut().for_each(|a| *a = false);
            game.bet = 0;
            game.round_pool = 0;
            game.round = next_round;              
        }

        match round {
            Round::Preround => {
                shuffle(&mut self.deck, &mut self.ctx.rng);
                for player in self.players.iter_mut() {
                    player.cards.push(self.deck.pop().unwrap_or(ExpandedCard::Unmarked));
                    player.cards.push(self.deck.pop().unwrap_or(ExpandedCard::Unmarked));
                }
                process_round(self, 0, 0, Round::Preflop);
            },
            Round::Preflop => process_round(self, 3, 0, Round::Flop),
            Round::Flop => process_round(self, 1, 0, Round::Turn),
            Round::Turn => process_round(self, 1, 0, Round::River),
            Round::River => process_round(self, 0, 0, Round::Showdown),
            Round::Showdown => {
                let mut hands: Vec<(usize, Hand)> = self.players.iter().map(|p| {
                    let cards = &p.cards;
                    let mut hand_cards = self.community.clone();
                    hand_cards.extend(cards);

                    (p.id, Hand::new(&hand_cards, &self.ctx))
                }).collect();

                hands.sort_by(|(_, a_h), (_, b_h)| a_h.compare(b_h));
                for pot in &self.pots {
                    for (p_id, _) in hands.iter().rev() {
                        if pot.players.contains(p_id) {
                            self.players[*p_id].chips += pot.amount;
                            break;
                        }
                    }
                }
            }
        }
    }

    #[wasm_bindgen]
    pub fn get_game_state(&self, player_id: usize) -> String {
        let player_states: Vec<PlayerState> = self.players.iter().enumerate().map(|(i, p)| {
            let hole_cards = if self.round == Round::Showdown || i == player_id {
                p.cards.iter().map(|c| c.to_string()).collect()
            } else {
                p.cards.iter().map(|_| "HIDDEN".to_string()).collect()
            };

            PlayerState {
                id: p.id,
                chips: p.chips,
                round_bet: p.bet,
                folded: self.folded[i],
                acted: self.acted[i],
                is_turn: self.player_turn == i && !(self.round == Round::Showdown),
                hole_cards,
            }
        }).collect();

        let pots: Vec<i32> = self.pots.iter().map(|p| p.amount).collect();
        let pot_total: i32 = pots.iter().sum();

        let complete_state = GameState {
            round_name: self.round.to_string(),
            community_cards: self.community.iter().map(|c| c.to_string()).collect(),
            pots,
            round_bet_sum: self.round_pool,
            overall_sum: pot_total + self.round_pool,
            highest_bet: self.bet,
            deck_cards: self.deck.len(),
            players: player_states,
        };

        serde_json::to_string(&complete_state).unwrap_or_else(|_| "{}".to_string())
    }
}

/**
 * Takes an unordered list of cards, a Vec<usize> containing a representation of a hand (i.e. (3, 2) is a full house), and a suit
 * Returns the cards which match the given representation and suit and the best remaining cards in hand, otherwise returns None
 * Works greedily - will always select the highest ranked cards matching a representation first
 */
fn get_representation(cards: &Vec<ExpandedCard>, representation: Vec<usize>, suit: Option<Suit>, hand_size: usize) -> Option<(Vec<Vec<ExpandedCard>>, Vec<ExpandedCard>)> {
    let cards_required = representation.iter().sum::<usize>();
    if hand_size < cards_required || cards.len() < cards_required {
        return None;
    }

    let mut ctx = HandContext::new(&cards, suit);
    if ctx.cards.len() < cards_required {
        return None;
    }

    // recursive backtracking matcher - guaranteed to find best representation with jokers if one exists
    fn match_group(group: usize, rep_index: usize, representation: &Vec<usize>, matched_group: &mut Vec<(Rank, Vec<usize>)>, ctx: &mut HandContext<'_>) -> bool {
        // termination cases - exit when representation is populated or all groups have been explored
        if rep_index == representation.len() {
            return true;
        }

        let cards_required = representation[rep_index];
        let matching_cards = ctx.organised_cards[group].len();
        let rank_uses = ctx.used_counts[group];
        let available = matching_cards - rank_uses;

        // checks whether the card requirement can be met for the given rank
        if available + ctx.jokers >= cards_required {
            let cards_taken = available.min(cards_required);
            let jokers_taken = cards_required - cards_taken;

            let mut matched_cards = ctx.organised_cards[group][rank_uses..rank_uses + cards_taken].to_vec();
            matched_cards.extend(vec![usize::MAX; jokers_taken]);

            ctx.used_counts[group] += cards_taken;
            ctx.jokers -= jokers_taken;
            matched_group.push((Rank::from(group), matched_cards));

            if match_group(12, rep_index + 1, representation, matched_group, ctx) {
                return true;
            }

            matched_group.pop();
            ctx.jokers += jokers_taken;
            ctx.used_counts[group] -= cards_taken;            
        }

        // no match failure - if this was the final group, no match can be found
        if group == 0 {
            return false;
        }

        // no match recursion - if there are not enough cards in the current group to reach the representation, try the next group 
        match_group(group - 1, rep_index, representation, matched_group, ctx)
    }

    let mut representation_cards: Vec<(Rank, Vec<usize>)> = Vec::new();
    if match_group(12, 0, &representation, &mut representation_cards, &mut ctx) {
        ctx.extract_hand(representation_cards, hand_size)
    } else {
        None
    }
}

/**
 * Takes an unordered list of cards, a length, and a suit
 * Returns a straight of the specified length and suit
 * Greedily chooses the straight made of the highest ranked cards, considering straights possible with jokers
 */
fn get_straight(cards: &Vec<ExpandedCard>, size: usize, suit: Option<Suit>, hand_size: usize) -> Option<(Vec<Vec<ExpandedCard>>, Vec<ExpandedCard>)> {
    if hand_size < size || cards.len() < size {
        return None;
    }

    let mut ctx = HandContext::new(&cards, suit);

    fn find_straight(size: usize, straight: &mut Vec<(Rank, Vec<usize>)>, ctx: &mut HandContext<'_>) -> bool {
        let size_i32 = size as i32;
        for i in (size_i32 - 2..=12).rev() {
            let mut matches = 0;

            // check for matches
            for n in (i - size_i32 + 1..=i).rev() {
                // loops - ensures ace low straights are checked
                let idx = n.rem_euclid(13) as usize;
                if ctx.organised_cards[idx].len() >= 1 {
                    matches += 1;
                }
            }

            // if there are sufficient matches, build the straight
            if matches + ctx.jokers >= size {
                for n in (i - size_i32 + 1..=i).rev() {
                    let idx = n.rem_euclid(13) as usize;
                    straight.push((Rank::from(idx), if ctx.organised_cards[idx].len() >= 1 {
                        vec![ctx.organised_cards[idx][ctx.used_counts[idx]]]
                    } else {
                        vec![usize::MAX]
                    }));
                }
                return true;
            }
        }
        false
    }

    let mut straight = Vec::with_capacity(size);
    if find_straight(size, &mut straight, &mut ctx) {
        ctx.extract_hand(straight, hand_size)
    } else {
        None
    }
}

fn get_custom_deck(deck_type: String) -> Vec<ExpandedCard> {
    match deck_type.to_lowercase().as_str() {
        "restricted" => get_deck(0, Some(
            |c| c.inner_card().rank as usize >= 5
        )),
        "double" => { 
            let mut deck = get_deck(0, None);
            deck.extend(get_deck(0, None));
            deck
        },
        "half" => get_deck(0,Some(
            |c| c.inner_card().suit == Suit::Spades || c.inner_card().suit == Suit::Hearts
        )),
        "1joker" => get_deck(1, None),
        "2jokers" => get_deck(2, None),
        _ => get_deck(0, None),
    }
}

/**
 Produces a Vec<ExpandedCard> deck, given a number of jokers to be added to the standard deck and rules to filter cards out
 */
fn get_deck(jokers: usize, rule: Option<fn(&ExpandedCard) -> bool>) -> Vec<ExpandedCard> {
    let default_rule: fn(&ExpandedCard) -> bool = |_: &ExpandedCard| true;
    let card_rule = rule.unwrap_or(default_rule);
    let mut deck: Vec<ExpandedCard> = Vec::new();
    for suit in SUITS {
        for rank in RANKS {
            let card = ExpandedCard::PlayingCard(Card {suit, rank});
            if card_rule(&card) {
                deck.push(card);
            }
        }
    }
    deck.extend((0..jokers).into_iter().map(|_| ExpandedCard::Joker(Card {suit: Suit::Spades, rank: Rank::Ace})));
    deck
}

/**
 In situ seeded shuffle
 */
fn shuffle<T: Copy, R: Rng>(list: &mut Vec<T>, rng: &mut R) {
    for i in (0..list.len()).rev() {
        let temp = list[i];
        let swap_index = rng.random_range(0..=i);
        list[i] = list[swap_index];
        list[swap_index] = temp;
    }
}

/**
  Descending order counting sort
 */
fn sort_cards(list: Vec<ExpandedCard>) -> Vec<ExpandedCard> {
    let mut counts: [usize; 13] = [0; 13];
    let mut output_array: Vec<ExpandedCard> = vec![ExpandedCard::PlayingCard( Card { rank: Rank::Two, suit: Suit::Spades } ); list.len()];

    for i in 0..list.len() {
        let next_rank = list[i].inner_card().rank;
        counts[next_rank as usize] += 1;
    }

    for i in (1..counts.len()).rev() {
        counts[i - 1] += counts[i];
    }

    for i in (0..list.len()).rev() {
        let next_rank = list[i].inner_card().rank;
        output_array[counts[next_rank as usize] - 1] = list[i];
        counts[next_rank as usize] -= 1;
    }

    output_array
}

fn main() {
    let mut ctx = GameContext::new(5, ChaCha12Rng::seed_from_u64(0), HAND_TYPES.to_vec());
    let mut deck = get_deck(1, None);

    let cards = vec![
        ExpandedCard::get_card("2", "s"), 
        ExpandedCard::get_card("3", "h"), 
        ExpandedCard::get_card("2", "d"), 
        ExpandedCard::get_card("3", "c"), 
        ExpandedCard::get_card("a", "c"), 
        ExpandedCard::get_joker(), 
        ExpandedCard::get_joker(), 
        ExpandedCard::get_joker()
    ];
    let hand = Hand::new(&cards, &ctx);
    println!("{}", hand);
}
