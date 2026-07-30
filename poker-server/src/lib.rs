use std::{cmp::Ordering::{self, Greater, Less}, fmt::{self, Debug}};
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_json::{self};
use rand::{RngExt, SeedableRng, seq::{SliceRandom}};
use rand_chacha::ChaCha12Rng;
use console_error_panic_hook;

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
            Rank::Two => write!(f, "2"),
            Rank::Three => write!(f, "3"),
            Rank::Four => write!(f, "4"),
            Rank::Five => write!(f, "5"), 
            Rank::Six => write!(f, "6"), 
            Rank::Seven => write!(f, "7"), 
            Rank::Eight => write!(f, "8"), 
            Rank::Nine => write!(f, "9"), 
            Rank::Ten => write!(f, "10"), 
            Rank::Jack => write!(f, "j"), 
            Rank::Queen => write!(f, "q"), 
            Rank::King => write!(f, "k"), 
            Rank::Ace => write!(f, "a"), 
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
            "q" => Rank::Queen,
            "k" => Rank::King,
            "a" => Rank::Ace,
            _ => unreachable!()
        }
    }
}
static RANKS: [Rank; 13] = [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Suit {
    Spades,
    Diamonds,
    Clubs,
    Hearts,
}
impl std::fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Suit::Spades => write!(f, "s"),
            Suit::Clubs => write!(f, "c"),
            Suit::Hearts => write!(f, "h"),
            Suit::Diamonds => write!(f, "d")
        }
    }
}
impl std::fmt::Debug for Suit {
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
            ExpandedCard::Joker(c) => write!(f, "*{}:{}", c.rank, c.suit),
            ExpandedCard::PlayingCard(c) => write!(f, "{}:{}", c.rank, c.suit),
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
        self.card().rank == other.card().rank
    }
    fn ne(&self, other: &Self) -> bool {
        !(self == other)
    }
}
impl PartialOrd for ExpandedCard {
    fn ge(&self, other: &Self) -> bool {
        self.card().rank >= other.card().rank      
    }

    fn gt(&self, other: &Self) -> bool {
        self.card().rank > other.card().rank      
    }

    fn le(&self, other: &Self) -> bool {
        self.card().rank <= other.card().rank       
    }

    fn lt(&self, other: &Self) -> bool {
        self.card().rank < other.card().rank       
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
    fn card(&self) -> &Card {
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
    None = 0,
    HighCard = 1,
    Pair = 2,
    TwoPair = 3,
    ThreeOfAKind = 4,
    FullHouse = 5,
    Flush = 6,
    Straight = 7,
    FourOfAKind = 8,
    StraightFlush = 9,
    RoyalFlush = 10,
}
impl fmt::Display for HandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HandType::None => write!(f, "Nothing"),
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
                    if let Some((cards, spare)) = (get_straight(cards, 5, Some(SUITS[i]), hand_size)) && cards[0][0].card().rank == Rank::Ace {
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
            HandType::HighCard => |cards, hand_size| build_hand(get_representation(cards, vec![1], None, hand_size), HandType::HighCard),
            HandType::None => |cards, hand_size|
                Some(Hand { hand_type: HandType::None, cards: vec![], spare: cards[..hand_size.min(cards.len())].to_vec()
            }),
        } 
    }
}
static HAND_TYPES: [HandType; 11] = [
    HandType::RoyalFlush, HandType::StraightFlush, HandType::FourOfAKind, HandType::Straight, 
    HandType::Flush, HandType::FullHouse, HandType::ThreeOfAKind, HandType::TwoPair, 
    HandType::Pair, HandType::HighCard, HandType::None
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
        extra_cards.sort_by(|a ,b| b.cmp(a));
        
        Some((hand_cards, extra_cards))
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
            return Hand { hand_type: HandType::None, cards: vec![], spare: vec![] };
        }
        for hand_fn in &ctx.hand_fns {
            if let Some(hand) = hand_fn(cards, ctx.hand_size) {
                return hand;
            }
        }
        Hand { hand_type: HandType::None, cards: vec![], spare: cards.clone() }
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

#[derive(Clone, Debug, Copy)]
struct AI {
    ai_type: AIType,
    total_bet: i32,
    budget: i32, // calculated reasonable bet for current turn (only considers folding if above, considers raising if below)
    fear: f64, // measure of reaction to game stakes (bet/relative bet compared to own chips)
    greed: f64, // measure of reaction to the strength of its cards
    randomness: f64, // 0-1, probability of performing an unexpected action
}
impl AI {
    fn from_str(str: &str) -> Self {
        match str.to_lowercase().as_str() {
            "risky" => Self::new(AIType::Risky),
            "smart" => Self::new(AIType::Smart),
            "random" => Self::new(AIType::Random),
            "safe" | _ => Self::new(AIType::Safe),
        } 
    }

    fn new(ai_type: AIType) -> Self {
        match ai_type {
            AIType::Risky => AI { ai_type, total_bet: 0, budget: 0, fear: 0.8, greed: 1.4, randomness: 0.2 },
            AIType::Smart => AI { ai_type, total_bet: 0, budget: 0, fear: 1.0, greed: 1.0, randomness: 0.07 },
            AIType::Random => AI { ai_type, total_bet: 0, budget: 0, fear: 1.0, greed: 1.0, randomness: 0.4 },
            AIType::Safe => AI { ai_type, total_bet: 0, budget: 0, fear: 1.7, greed: 0.7, randomness: 0.1 },
        }
    }

    /**
     * Uses a non-deterministic decision tree to select an Action based on the current game state, cards, and AI preset parameters
     */
    fn calculate_next_action(&mut self, player_index: usize, game: &mut Game) -> Action {
        // there are two primary situations a player can face:
        //  - the game bet is equal to what they have bet
        //  - the game bet is greater than what they have bet

        let player = &game.players[player_index].clone();

        let fear_greed_sum = self.fear + self.greed;
        let fear_greed_ratio = if fear_greed_sum > 0.0 { self.fear / fear_greed_sum } else { 0.5 };
        let hand_cards: Vec<ExpandedCard> = [&player.cards[..], &game.community[..]].concat();
        let hand_strength = self.calculate_hand_value(&hand_cards, game).clamp(0.0, 1.0);

        if self.budget <= 0 {
            self.budget = self.calculate_round_budget(player, game, hand_strength);
        }

        if game.bet > player.total_bet {
            let new_total = self.total_bet + game.bet - player.total_bet;
            if new_total > self.budget {
                let calibration = (game.ctx.blind_size.max(1) * 3) as f64;
                let denominator = (self.budget as f64 + calibration).max(1.0);
                let raise_ratio = (new_total as f64 + calibration) / denominator;

                let raw_fold = fear_greed_ratio * raise_ratio - hand_strength * 2.0;
                let min_fold = (self.randomness - hand_strength).max(0.0);
                let max_fold = (1.0 - hand_strength).max(min_fold);
                let p_fold = if raw_fold.is_nan() { 0.5 } else { raw_fold.clamp(min_fold, max_fold) };

                let p_call = if (fear_greed_ratio - hand_strength).is_nan() { 0.5 } else { (fear_greed_ratio - hand_strength).max(self.randomness).clamp(0.0, 1.0) };

                if game.ctx.rng.random_bool(p_fold) {
                    return Action::Fold;
                } else if game.ctx.rng.random_bool(p_call) {
                    self.total_bet += (game.bet - player.total_bet).min(player.chips);
                    self.budget = self.calculate_round_budget(player, game, hand_strength);
                    return Action::Call;
                } else {
                    let raise_amount = game.ctx.blind_size + ((0.5 + game.ctx.rng.random::<f64>()) * 0.1 * self.greed * player.chips as f64) as i32;
                    let adjusted_raise = raise_amount.min(player.chips);
                    self.total_bet += adjusted_raise + (game.bet - player.total_bet).min(player.chips);
                    let upper_budget = player.chips.max(1);
                    self.budget = self.calculate_round_budget(player, game, hand_strength).clamp((self.budget + raise_amount).min(upper_budget), upper_budget);
                    return Action::Raise(adjusted_raise);
                }
            } else {
                return Action::Call;
            }
        } else {
            let budget_used = if self.budget > 0 { (self.total_bet as f64 / self.budget as f64).clamp(0.0, 1.0) } else { 0.0 };
            let p_raise = (fear_greed_ratio * hand_strength * 2.0 * (1.0 - budget_used)).clamp(0.0, 1.0);

            if !p_raise.is_nan() && game.ctx.rng.random_bool(p_raise) {
                let raise_amount = game.ctx.blind_size + ((0.5 + game.ctx.rng.random::<f64>()) * 0.15 * self.greed * player.chips as f64) as i32;
                let adjusted_raise = raise_amount.min(self.budget - self.total_bet).min(player.chips);
                return Action::Raise(adjusted_raise);
            } 

            Action::Call
        }
    }

    fn calculate_round_budget(&self, player: &Player, game: &mut Game, hand_strength: f64) -> i32 {
        if game.round <= Round::Preflop {
            // by default, will stake at most 1/10 of chips + blind size on preflop
            let base_bet: f64 = player.chips as f64 / 10.0 * self.greed;
            let mut base_bet_blind: i32 = (base_bet as i32 + game.ctx.blind_size).min(player.chips);

            // but can randomly commit everything or far less than usual
            if game.ctx.rng.random_bool(self.randomness) {
                let (amp_greed, amp_fear) = (self.greed.powi(3), self.fear.powi(3));
                let greed_ratio = amp_greed / (amp_greed + amp_fear);
                if game.ctx.rng.random_bool(greed_ratio) {
                    base_bet_blind = player.chips;
                } else {
                    base_bet_blind = game.ctx.blind_size.min(player.chips);
                }
            }

            return base_bet_blind;
        }

        // % of chips already committed - used to ensure AI does not fold after committing 90% of its chips
        let total_committed = self.total_bet + player.chips;
        let sunk_cost: f64 = if total_committed > 0 { self.total_bet as f64 / (total_committed) as f64 } else { 0.0 };

        // budget calculation - considers three main factors:
        //  - base allowance (calibrated from blind size)
        //  - how good is my current hand
        //  - how much have i already committed to this round
        let budget = game.ctx.blind_size as f64 + game.ctx.starting_chips as f64 * hand_strength + player.chips as f64 * sunk_cost.powi(2) * 1.5;

        let adjusted_budget = (budget * self.greed) as i32;
        adjusted_budget.max(player.chips)
    }

    fn calculate_hand_value(self, hand_cards: &Vec<ExpandedCard>, game: &Game) -> f64 {
        fn get_raw_value(hand: &Hand) -> f64 {
            // high card = 1, straight flush = ~8.2, scaling exponentially
            let base_value: f64 = 1.3_f64.powi(hand.hand_type as i32 - 1);
            let normalised_rank_value: f64 = hand.cards[0][0].card().rank as u8 as f64 / 12.0;
            // high card (2) = 0, straight flush (K) = ~9.03
            let centered_adjusted_value: f64 = base_value * (normalised_rank_value * 0.25 + 1.0) - 1.0;
            // range 0-1, between worst and best hand
            let normalised_adjusted_value: f64 = centered_adjusted_value / 9.03;

            normalised_adjusted_value
        }

        let hand = Hand::new(&hand_cards, &game.ctx);

        if hand.hand_type == HandType::None {
            return 0.0;
        } else if hand.hand_type == HandType::RoyalFlush {
            return 1000.0;
        }

        let mut raw_value = get_raw_value(&hand);
        
        // additional adjustments
        // if this hand is the same as the hand on the board, penalise
        let board_hand = Hand::new(&game.community, &game.ctx);
        let is_board = hand.compare(&board_hand) == Ordering::Equal;
        let playing_board = hand.hand_type == hand.hand_type;
        if is_board {
            raw_value /= 3.0;
        } else if playing_board {
            raw_value /= 2.0;
        }

        // grants boost if hand could be much better within the next few rounds
        if game.round == Round::Flop || game.round == Round::Turn {
            let potential_hand = Hand::new(&[hand_cards.clone(), vec![ExpandedCard::get_joker()]].concat(), &game.ctx);
            let potential_value = get_raw_value(&potential_hand);
            let potential_increase = potential_value - raw_value;
            let mut value_added = potential_increase * 0.25;
            if game.round == Round::Turn {
                value_added *= 0.5;
            }
            raw_value += value_added;
        }

        raw_value
    }
}

#[derive(Clone, Copy, Debug)]
enum AIType {
    Safe,
    Risky,
    Smart,
    Random,
}
impl AIType {

}

#[derive(Debug, Clone, Copy)]
enum SpecialCard {
    A,
}
impl fmt::Display for SpecialCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecialCard::A => write!(f, "Test")
        }
    }
}

#[derive(Clone)]
struct Player {
    player_type: PlayerType,
    chips: i32,
    total_bet: i32,
    round_bet: i32,
    cards: Vec<ExpandedCard>,
    special_cards: Vec<SpecialCard>,
    id: usize,
    acted: bool,
    folded: bool,
    remove: bool,
}
impl Player {
    fn new(player_type: PlayerType, id: usize) -> Self {
        Player { player_type, chips: 0, total_bet: 0, round_bet: 0, cards: Vec::new(), special_cards: Vec::new(), id, acted: false, folded: false, remove: false }
    }
}

enum Action {
    Raise(i32),
    Fold,
    Call,
    Timeout,
    EndMove,
}
impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Raise(n) => write!(f, "RAISE {n}"),
            Action::Fold => write!(f, "FOLD"),
            Action::Call => write!(f, "CALL"),
            Action::Timeout => write!(f, "TIMEOUT"),
            Action::EndMove => write!(f, "ENDMOVE"),
        }
    }
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
            "timeout" => Some(Action::Timeout),
            "endmove" => Some(Action::EndMove),
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
}
impl fmt::Display for Round {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Round::Room => write!(f, "room"),
            Round::Preround => write!(f, "preround"),
            Round::Preflop => write!(f, "preflop"),
            Round::Flop => write!(f, "flop"),
            Round::Turn => write!(f, "turn"),
            Round::River => write!(f, "river"),
            Round::Showdown => write!(f, "showdown"),
        }
    }
}

struct GameContext {
    hand_size: usize,
    rng: ChaCha12Rng,
    hand_fns: Vec<fn(&Vec<ExpandedCard>, usize) -> Option<Hand>>,
    blind_size: i32,
    min_raise: i32,
    starting_chips: i32,
    deck_type: String,
    max_players: usize,
    id: usize,
}
impl GameContext {
    fn new(
        hand_size: usize, 
        rng: ChaCha12Rng, 
        mut permitted_hands: Vec<HandType>, 
        blind_size: i32, 
        min_raise:i32, 
        starting_chips: i32, 
        deck_type: String, 
        max_players: usize
    ) -> Self {
        permitted_hands.sort_by(|a, b| b.cmp(a));
        let hand_fns: Vec<fn(&Vec<ExpandedCard>, usize) -> Option<Hand>> = permitted_hands.iter().map(|h| HandType::get_check_fn(*h)).collect();

        GameContext { hand_size, rng, hand_fns, blind_size, min_raise, starting_chips, deck_type, max_players, id: 0 }
    }
}

#[derive(Deserialize)]
pub struct GameConfig {
    pub player_count: usize,
    pub bots: Vec<String>,
    pub rng_seed: u64,
    pub blind_size: i32,
    pub min_raise: i32,
    pub starting_chips: i32,
    pub deck_type: String,
    pub max_players: usize,
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
    games_played: i32,
    players: Vec<Player>,
    turn_index: usize,
    dealer_index: usize,
    bet: i32,
    round_pool: i32,
    pots: Vec<Pot>,
    deck: Vec<ExpandedCard>,
    community: Vec<ExpandedCard>,
    winning_hand_type: String,
}

#[derive(Serialize)]
pub struct PlayerState {
    pub id: usize,
    pub chips: i32,
    pub total_bet: i32,
    pub round_bet: i32,
    pub folded: bool,
    pub acted: bool,
    pub is_turn: bool,
    pub is_dealer: bool,
    pub hole_cards: Vec<String>,
    pub special_cards: Vec<String>,
    pub hand_type: String,
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
    pub winning_hand_type: String,
    pub players: Vec<PlayerState>,
}

#[derive(Serialize)]
pub struct MoveEvent {
    pub player_id: usize,
    pub action: String
}

#[derive(Serialize)]
pub struct MoveResult {
    pub events: Vec<MoveEvent>,
    pub game_state: serde_json::Value,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    /**
     * Creates a game, given a valid JSON config string. Config is expected to be formed as a valid GameConfig struct
     */
    pub fn new() -> Game {
        let ctx = GameContext::new(5, ChaCha12Rng::seed_from_u64(0), HAND_TYPES.to_vec(), 0, 0, 0, "standard".to_string(), 0);
        let deck = get_custom_deck("standard");

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
        }
    }

    #[wasm_bindgen]
    pub fn init_panic_hook(&self) {
        console_error_panic_hook::set_once();
    }

    #[wasm_bindgen]
    /**
     * Updates the current game config with a newly provided one
     * Returns a JSON string containing an error if not possible
     */
    pub fn update_config(&mut self, config_json: &str) -> String {
        if self.round > Round::Preround { 
            return serde_json::json!({"error": "Failed to apply given config - round has already started"}).to_string(); 
        }

        let str_config: Result<GameConfig, serde_json::Error> = serde_json::from_str(config_json);
        if str_config.is_err() {
            return serde_json::json!({"error": "Failed to parse given config"}).to_string();
        }
        let config: GameConfig = str_config.unwrap();
        let mut ctx = GameContext::new(
            5, 
            ChaCha12Rng::seed_from_u64(config.rng_seed), 
            HAND_TYPES.to_vec(), 
            config.blind_size, 
            config.min_raise, 
            config.starting_chips, 
            config.deck_type, 
            config.max_players
        );
        let deck = get_custom_deck(&ctx.deck_type);

        let mut players: Vec<Player> = Vec::with_capacity(config.player_count + config.bots.len());
        for _ in 0..config.player_count {
            players.push(Player::new(PlayerType::Human, ctx.id));
            ctx.id += 1;
        }
        for bot_type in config.bots {
            players.push(Player::new(PlayerType::Computer(AI::from_str(&bot_type)), ctx.id));
            ctx.id += 1;
        }
    
        self.ctx = ctx;
        self.deck = deck;
        self.players = players;

        "".to_string()
    }

    #[wasm_bindgen]
    /**
     * Attempts to add a player to the room
     * Returns a JSON string containing either the ID of the new player or an error
     */
    pub fn try_add_player(&mut self, player_type: String) -> String {
        if self.players.len() >= self.ctx.max_players {
            return serde_json::json!({"error": "Room already at max capacity"}).to_string();
        } else if self.round >= Round::Preflop {
            return serde_json::json!({"error": "Game already started"}).to_string();
        }

        if let Some(p_type) = PlayerType::from_str(&player_type) {
            let mut new_player = Player::new(p_type, self.ctx.id);
            new_player.chips = self.ctx.starting_chips;
            self.players.push(new_player);
            self.ctx.id += 1;
            
            serde_json::json!({"id": self.ctx.id - 1}).to_string()
        } else {
            serde_json::json!({"error": "Specified player type is invalid"}).to_string()
        }
    }

    #[wasm_bindgen]
    /**
     * Resets all player and game related data, progressing the round from Room to Preround
     */
    pub fn initialise(&mut self) -> String {
        if self.round != Round::Room {
            return serde_json::json!({"error": "Could not initialise game from current round"}).to_string();
        }

        let start_chips = if self.ctx.starting_chips > 0 { self.ctx.starting_chips } else { 1000 };
        self.players.iter_mut().for_each(|p| {
            p.chips = start_chips;
            p.folded = false;
            p.acted = false;
            p.cards = Vec::new();
            p.special_cards = Vec::new();
        });

        self.deck = get_custom_deck(&self.ctx.deck_type);
        self.community = Vec::new();
        self.bet = 0;
        self.round = Round::Preround;
        self.pots = Vec::new();
        self.round_pool = 0;
        self.winning_hand_type.clear();
        self.dealer_index = 0;

        self.get_game_state(usize::MAX)
    }

    #[wasm_bindgen]
    /**
     * Progresses the round from the intermission to the Preflop
     */
    pub fn start(&mut self) -> String {
        if self.round == Round::Preround {
            self.start_new_hand();
        } else if self.round == Round::Showdown {
            self.end_hand();
            self.start_new_hand();
        } else {
            return serde_json::json!({"error": "Invalid state for starting game"}).to_string();
        }

        if self.round == Round::Room {
            return self.get_game_state(usize::MAX);
        }

        let mut events = Vec::new();
        self.advance_game_loop(&mut events);

        self.get_game_state(usize::MAX)
    }

    #[wasm_bindgen]
    /**
     * Public API allowing players to make moves to modify the internal game state
     * Given a player id and action, handles the full process of validating, executing, and advancing the game loop
     * Returns a JSON string containing either an error or a response indicating the events which occurred and the complete new game state
     */
    pub fn player_move(&mut self, player_id: i32, str_action: String, amount: i32) -> String {
        let action = match Action::from_str(str_action, Some(amount)) {
            Some(a) => a,
            None => return serde_json::json!({"error": "Invalid move"}).to_string(),
        };

        let current_turn_id = match self.players.get(self.turn_index) {
            Some(p) => p.id as i32,
            None => return serde_json::json!({"error": "No active turn player"}).to_string(),
        };

        if current_turn_id != player_id {
            return serde_json::json!({"error": "Invalid turn order"}).to_string();
        }

        let mut events = Vec::new();
        if let Err(err) = self.apply_action(self.turn_index, action, &mut events) {
            return err;
        }

        self.advance_game_loop(&mut events);

        let game_state_val: serde_json::Value = serde_json::from_str(&self.get_game_state(usize::MAX)).unwrap();
        serde_json::to_string(&MoveResult { events, game_state: game_state_val }).unwrap_or_else(|_| "{}".to_string())
    }

    #[wasm_bindgen]
    /**
     * Queues a player for removal at the end of the current round
     * Returns a JSON string error if not possible
     */
    pub fn queue_remove_player(&mut self, player_id: i32) -> String {
        if let Ok(player_index) = self.get_player_index(player_id as usize) {
            self.players[player_index].remove = true;
            "{}".to_string()
        } else {
            serde_json::json!({"error": "Could not remove player with specified id: id does not exist"}).to_string()
        }
    }

    #[wasm_bindgen]
    /**
     * Immediately removes a player from the game
     * Returns a JSON string error if not possible
     */
    pub fn force_remove_player(&mut self, player_id: i32) -> String {
        if let Ok(player_index) = self.get_player_index(player_id as usize) {
            self.players.remove(player_index);
            "{}".to_string()
        } else {
            serde_json::json!({"error": "Could not remove player with specified id: id does not exist"}).to_string()
        }
    }

    #[wasm_bindgen]
    /**
     * Returns a JSON string containing information about the current number of humans and bots in the game
     */
    pub fn get_player_info(&self) -> String {
        let mut humans = 0;
        let mut bots = 0;
        self.players.iter().for_each(|p| match p.player_type {
            PlayerType::Human => humans += 1,
            PlayerType::Computer(_) => bots += 1,
        });

        serde_json::json!({"humans": humans, "bots": bots}).to_string()
    }

    #[wasm_bindgen]
    /**
     * Gets the current round as a string
     */
    pub fn get_round(&self) -> String {
        self.round.to_string()
    }

    #[wasm_bindgen]
    /**
     * Gets the ID of the player who needs to act next
     */
    pub fn get_current_turn_player(&self) -> i32 {
        if self.turn_index < self.players.len() {
            return self.players[self.turn_index].id as i32;
        }
        0
    }

    #[wasm_bindgen]
    /**
     * Swaps a player with a given ID's type with a bot or human
     * Returns a JSON string including events and the new game state after swapping
     */
    pub fn toggle_id_with_bot(&mut self, player_id: i32, bot: bool) -> String {
        let index = match self.get_player_index(player_id as usize) {
            Ok(index) => index,
            Err(err) => return err,
        };

        let mut event_log = Vec::new();
        
        if bot {
            let pid = self.players[index].id;
            event_log.push(MoveEvent { player_id: pid, action: "SWAP_BOT".to_string() });
            let new_ai = AI::new(AIType::Safe);
            self.players[index].player_type = PlayerType::Computer(new_ai);

            if self.turn_index == index && !self.players[index].folded && self.round >= Round::Preflop && self.round < Round::Showdown {
                self.advance_game_loop(&mut event_log);
            }
        } else {
            let pid = self.players[index].id;
            event_log.push(MoveEvent { player_id: pid, action: "SWAP_HUMAN".to_string() });
            self.players[index].player_type = PlayerType::Human;
        }

        let state_value: serde_json::Value = serde_json::from_str(&self.get_game_state(usize::MAX)).unwrap();
        serde_json::to_string(&MoveResult { events: event_log, game_state: state_value }).unwrap_or_else(|_| "{}".to_string())
    }

    #[wasm_bindgen]
    /**
     * Gets the current game state, as seen by the specified player as a JSON string
     */
    pub fn get_game_state(&self, player_id: usize) -> String {
        let is_showdown = self.round == Round::Showdown;
        let player_states: Vec<PlayerState> = self.players.iter().enumerate().map(|(i, p)| {
            let hole_cards = if is_showdown || p.id == player_id {
                p.cards.iter().map(|c| c.to_string()).collect()
            } else {
                p.cards.iter().map(|_| "HIDDEN".to_string()).collect()
            };

            let special_cards = if p.id == player_id {
                p.special_cards.iter().map(|c| c.to_string()).collect()
            } else {
                vec![]
            };

            let full_cards = [&self.community[..], &p.cards[..]].concat();
            let hand_eval = Hand::new(&full_cards, &self.ctx);
            let hand_type_str = if !p.folded && !full_cards.is_empty() && (p.id == player_id || is_showdown) {
                hand_eval.hand_type.to_string()
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
                is_turn: self.turn_index == i && !is_showdown && self.round != Round::Preround,
                is_dealer: self.dealer_index == i,
                hole_cards,
                special_cards,
                hand_type: hand_type_str,
            }
        }).collect();

        let pots: Vec<i32> = self.pots.iter().map(|p| p.amount).collect();
        let total_pot: i32 = pots.iter().sum::<i32>() + self.round_pool;

        let complete_state = GameState {
            round_name: self.round.to_string(),
            community_cards: self.community.iter().map(|c| c.to_string()).collect(),
            pots,
            round_bet_sum: self.round_pool,
            overall_sum: total_pot,
            highest_bet: self.bet,
            deck_cards: self.deck.len(),
            winning_hand_type: self.winning_hand_type.clone(),
            players: player_states,
        };

        serde_json::to_string(&complete_state).unwrap_or_else(|_| "Failed to parse state as string".to_string())
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
                self.advance_to_next_round();
                if self.round == Round::Preround || self.round == Round::Showdown {
                    break;
                }
                continue;
            }

            // if the current round is not over, advance the turn index to the next player
            let current_player = &self.players[self.turn_index];
            if !current_player.acted {
                match current_player.player_type {
                    PlayerType::Computer(mut ai) => {
                        let ai_action = ai.calculate_next_action(self.turn_index, self);
                        let _ = self.apply_action(self.turn_index, ai_action, events);

                        self.turn_index = (self.turn_index + 1) % self.players.len();
                        continue;
                    }
                    PlayerType::Human => {
                        break;
                    }
                }
            }

            if current_player.acted && current_player.round_bet == self.bet {
                self.turn_index = (self.turn_index + 1) % self.players.len();
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
    fn apply_action(&mut self, player_index: usize, action: Action, events: &mut Vec<MoveEvent>) -> Result<(), String> {
        let player_id = self.players[player_index].id;

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
            Action::Raise(raise) => {
                if raise < self.ctx.min_raise {
                    return Err(serde_json::json!({"error": "Raise below minimum threshold"}).to_string());
                }

                let p = &self.players[player_index];
                let max_raise = p.chips - (self.bet - p.round_bet).max(0);
                let true_raise = raise.min(max_raise);
                let raise_cost = ((self.bet + true_raise) - p.round_bet).min(p.chips);

                let p = &mut self.players[player_index];
                p.chips -= raise_cost;
                p.round_bet += raise_cost;
                p.total_bet += raise_cost;
                self.round_pool += raise_cost;
                self.bet = p.round_bet.max(self.bet);

                for (i, other) in self.players.iter_mut().enumerate() {
                    if i != player_index && !other.folded && other.chips > 0 {
                        other.acted = false;
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
            }
            Action::EndMove => {
                if !self.players[player_index].acted {
                    return Err(serde_json::json!({"error": "Must act before ending move"}).to_string());
                }
            }
        }

        events.push(MoveEvent { player_id, action: action.to_string() });
        Ok(())
    }

    /**
     * Gets .players() array index from player id
     */
    fn get_player_index(&self, player_id: usize) -> Result<usize, String> {
        if let Some(pos) = self.players.iter().position(|p| p.id == player_id) {
            Ok(pos)
        } else {
            Err(serde_json::json!({"error": "Could not find player with specified id"}).to_string())
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

                    if !player.folded && !self.pots[pot_i].players.contains(&player.id) {
                        self.pots[pot_i].players.push(player.id);
                    }
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

    fn advance_to_next_round(&mut self) {
        self.process_pots();

        for p in self.players.iter_mut() {
            p.round_bet = 0;
            p.acted = false;
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
                self.deal_community_cards(3);
                self.round = Round::Flop;
            }
            Round::Flop => {
                self.deal_community_cards(1);
                self.round = Round::Turn;
            }
            Round::Turn => {
                self.deal_community_cards(1);
                self.round = Round::River;
            }
            Round::River => {
                self.round = Round::Showdown;
                self.resolve_showdown();
            }
            _ => {}
        }
    }

    fn start_new_hand(&mut self) {
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

        self.deck = get_custom_deck(&self.ctx.deck_type);
        self.deck.shuffle(&mut self.ctx.rng);

        for p in self.players.iter_mut() {
            // reset player params
            p.cards.clear();
            p.special_cards.clear();
            p.round_bet = 0;
            p.total_bet = 0;
            p.acted = false;
            p.folded = p.chips <= 0;

            // reset ai params
            if let PlayerType::Computer(ref mut ai) = p.player_type {
                ai.total_bet = 0;
                ai.budget = 0;
            }

            // give cards
            if !p.folded {
                p.cards.push(self.deck.pop().unwrap_or(ExpandedCard::Unmarked));
                p.cards.push(self.deck.pop().unwrap_or(ExpandedCard::Unmarked));
            }
        }

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

            let sb = make_player_pay_blind(sb_index, self.ctx.blind_size / 2);
            let bb = make_player_pay_blind(bb_index, self.ctx.blind_size);

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

    fn resolve_showdown(&mut self) {
        self.process_pots();

        let mut hands: Vec<(usize, Hand)> = self.players.iter()
            .filter(|p| !p.folded)
            .map(|p| {
                let cards = [&self.community[..], &p.cards[..]].concat();
                (p.id, Hand::new(&cards, &self.ctx))
            }).collect();

        hands.sort_by(|(_, a), (_, b)| a.compare(b));

        if let Some(winning_entry) = hands.last() {
            self.winning_hand_type = winning_entry.1.hand_type.to_string();
        }

        // iterates through all pots, finding winners for each and individually distributing
        // pot chips
        for pot in &self.pots {
            let mut winners = Vec::new();
            if let Some(winning_entry) = hands.iter().rev().find(|(id, _)| pot.players.contains(id)) {
                let winning_hand = &winning_entry.1;
                for (id, hand) in hands.iter().rev() {
                    if pot.players.contains(id) && hand.compare(winning_hand) != Ordering::Less {
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
                        // this should never execute, as pot size always has to be a multiple of the number of participants
                        if remainder > 0 {
                            self.players[index].chips += 1;
                            remainder -= 1;
                        }
                    }
                }
            }
        }

        self.pots.clear();
        self.round_pool = 0;
    }

    fn award_pot_to_single_winner(&mut self, winner_id: usize) {
        self.process_pots();
        let total_pot: i32 = self.pots.iter().map(|p| p.amount).sum::<i32>();
        if let Ok(index) = self.get_player_index(winner_id) {
            self.players[index].chips += total_pot;
        }
        self.pots.clear();
        self.round_pool = 0;

        self.round = Round::Showdown;
    }

    fn end_hand(&mut self) {
        self.games_played += 1;
        self.players.retain(|p| !p.remove);
        if !self.players.is_empty() {
            self.dealer_index = (self.dealer_index + 1) % self.players.len();
        }
        self.round = Round::Preround;
    }

    fn deal_community_cards(&mut self, count: usize) {
        for _ in 0..count { 
            let card = self.deck.pop().unwrap_or(ExpandedCard::Unmarked);
            self.community.push(card);
        }
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
                let index = n.rem_euclid(13) as usize;
                if ctx.organised_cards[index].len() >= 1 {
                    matches += 1;
                }
            }

            // if there are sufficient matches, build the straight
            if matches + ctx.jokers >= size {
                for n in (i - size_i32 + 1..=i).rev() {
                    let index = n.rem_euclid(13) as usize;
                    straight.push((Rank::from(index), if ctx.organised_cards[index].len() >= 1 {
                        vec![ctx.organised_cards[index][ctx.used_counts[index]]]
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

fn get_custom_deck(deck_type: &str) -> Vec<ExpandedCard> {
    match deck_type.to_lowercase().as_str() {
        "restricted" => get_deck(0, Some(
            |c| c.card().rank as usize >= 5
        )),
        "double" => { 
            let mut deck = get_deck(0, None);
            deck.extend(get_deck(0, None));
            deck
        },
        "half" => get_deck(0,Some(
            |c| c.card().suit == Suit::Spades || c.card().suit == Suit::Hearts
        )),
        "1joker" => get_deck(1, None),
        "2jokers" => get_deck(2, None),
        "standard" | _ => get_deck(0, None),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn create_valid_config_json(player_count: usize, bot_count: usize) -> String {
        let bots: Vec<String> = (0..bot_count).map(|_| "smart".to_string()).collect();
        serde_json::json!({
            "player_count": player_count,
            "bots": bots,
            "rng_seed": 0,
            "blind_size": 20,
            "min_raise": 10,
            "starting_chips": 1000,
            "deck_type": "standard",
            "max_players": 4
        }).to_string()
    }

    #[test]
    fn test_player_count_cannot_exceed_max() {
        let mut game = Game::new();
        let config_str = serde_json::json!({
            "player_count": 5,
            "bots": 0,
            "rng_seed": 0,
            "blind_size": 20,
            "min_raise": 10,
            "starting_chips": 1000,
            "deck_type": "standard",
            "max_players": 4
        }).to_string();

        let err = game.update_config(&config_str);
        assert_ne!(err, "");
    }

    #[test]
    fn test_game_creation_and_config_update() {
        let mut game = Game::new();
        assert_eq!(game.get_round(), "room");

        let config_str = create_valid_config_json(2, 0);
        let err = game.update_config(&config_str);
        assert_eq!(err, "");

        let state_str = game.get_game_state(0);
        let state: Value = serde_json::from_str(&state_str).expect("Failed to parse GameState JSON");
        
        assert_eq!(state["round_name"], "room");
        assert_eq!(state["players"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_initialise_transition() {
        let mut game = Game::new();
        game.update_config(&create_valid_config_json(2, 2));

        game.initialise();
        assert_eq!(game.get_round(), "preround");

        let state_str = game.get_game_state(0);
        let state: Value = serde_json::from_str(&state_str).unwrap();
        let players = state["players"].as_array().unwrap();

        assert_eq!(players.len(), 4);
        for player in players {
            assert_eq!(player["chips"], 1000);
            assert_eq!(player["folded"], false);
        }
    }

    #[test]
    fn test_round_transition_from_preround_to_preflop() {
        let mut game = Game::new();
        game.update_config(&create_valid_config_json(2, 0));
        game.initialise();
        game.start();

        let state_str = game.get_game_state(0);
        let state: Value = serde_json::from_str(&state_str).unwrap();
        let players = state["players"].as_array().unwrap();

        // players should have received their hole cards
        assert_eq!(players[0]["hole_cards"].as_array().unwrap().len(), 2);
        assert_eq!(players[1]["hole_cards"].as_array().unwrap().len(), 2);

        // and should have paid blinds
        assert_eq!(players[0]["total_bet"], 10, "Expected total bet to be 10 (small blind), was {}", players[0]["bet"]);
        assert_eq!(players[1]["total_bet"], 20, "Expected total bet to be 20 (big blind), was {}", players[1]["bet"]);
    }

    #[test]
    fn test_player_moves_and_turn_rotation() {
        let mut game = Game::new();
        game.update_config(&create_valid_config_json(2, 0));
        game.initialise();
        game.start(); // Preflop

        let active_player = game.get_current_turn_player();
        
        game.player_move(active_player, "call".to_string(), 0);
        
        let state_str = game.get_game_state(active_player as usize);
        let state: Value = serde_json::from_str(&state_str).unwrap();
        assert_eq!(state["highest_bet"], 20);
    }

    #[test]
    fn test_bot_execution_without_panic() {
        let mut game = Game::new();
        // 1 human, 1 bot
        game.update_config(&create_valid_config_json(1, 1)); 
        game.initialise();
        game.start();

        let human_id = game.players.iter().find(|p| match p.player_type {
            PlayerType::Human => true,
            _ => false
        }).map(|p| p.id as i32).unwrap_or(0);

        // If it's human's turn, make a valid move
        if game.get_current_turn_player() == human_id {
            game.player_move(human_id, "call".to_string(), 0);
        }

        // Validate that state is active without panicking or locking up
        assert!(game.get_round() != "room");
    }

    #[test]
    fn test_midgame_add_and_remove_player() {
        let mut game = Game::new();
        game.update_config(&create_valid_config_json(2, 0));
        
        game.initialise();
        game.start();
        
        let add_res_str = game.try_add_player("human".to_string());
        let add_res: Value = serde_json::from_str(&add_res_str).unwrap();
        assert!(add_res.as_object().unwrap().contains_key("error"), "Adding player mid-hand should fail");
    }

    #[test]
    fn test_game_ends_after_no_humans_remain() {
        let mut game = Game::new();
        game.update_config(&create_valid_config_json(1, 1));

        game.initialise();
        game.start();

        let human_id = game.players.iter().find(|p| match p.player_type {
            PlayerType::Human => true,
            _ => false
        }).map(|p| p.id as i32).unwrap_or(0);

        game.player_move(human_id, "FOLD".to_string(), 0);

        assert_eq!(game.round, Round::Showdown, "Expected round to end after last human player folded, instead round is {}", game.round);
    }

    #[test]
    fn test_game_round_progresses_after_moves() {
        let mut game = Game::new();
        game.update_config(&create_valid_config_json(2, 0));

        game.initialise();
        game.start();

        let (p1_id, p2_id) = (game.players[0].id as i32, game.players[1].id as i32);    

        assert_eq!(game.round, Round::Preflop, "Game round expected to be Preflop, actual: {}", game.round);

        game.player_move(p1_id, "CALL".to_string(), 0);
        game.player_move(p1_id, "ENDMOVE".to_string(), 0);

        game.player_move(p2_id, "CALL".to_string(), 0);
        game.player_move(p2_id, "ENDMOVE".to_string(), 0);

        assert_eq!(game.round, Round::Flop, "Game round expected to be Flop, actual: {}", game.round);
    }

    #[test]
    fn test_complex_pot_paid_correctly() {
        let mut game = Game::new();
        game.update_config(&create_valid_config_json(3, 0));

        game.initialise();

        let (p1_id, p2_id, p3_id) = (game.players[0].id as i32, game.players[1].id as i32, game.players[2].id as i32);
        game.players[0].chips = 200;
        game.players[1].chips = 500;
        game.players[2].chips = 1000;

        game.start();

        // p1 has 4 aces (wins overall round)
        game.players[0].cards = vec![ExpandedCard::get_card("a", "s"), ExpandedCard::get_card("a", "s"), ExpandedCard::get_card("a", "s"), ExpandedCard::get_card("a", "s")];
        // p2 has 4 kings (second)
        game.players[1].cards = vec![ExpandedCard::get_card("k", "s"), ExpandedCard::get_card("k", "s"), ExpandedCard::get_card("k", "s"), ExpandedCard::get_card("k", "s")];
        // p3 has 4 queens (loses)
        game.players[2].cards = vec![ExpandedCard::get_card("q", "s"), ExpandedCard::get_card("q", "s"), ExpandedCard::get_card("q", "s"), ExpandedCard::get_card("q", "s")];

        // all players go all in
        game.player_move(p1_id, "RAISE".to_string(), 10000);
        game.player_move(p1_id, "ENDMOVE".to_string(), 0);

        game.player_move(p2_id, "RAISE".to_string(), 10000);
        game.player_move(p2_id, "ENDMOVE".to_string(), 0);

        game.player_move(p3_id, "RAISE".to_string(), 10000);
        game.player_move(p3_id, "ENDMOVE".to_string(), 0);

        // p1 has 600 chips at the end (200 from p1, p2, and p3)
        assert_eq!(game.players[0].chips, 600, "Expected P1 to have 600 chips. Instead P1: {}, P2: {}, P3: {}", game.players[0].chips, game.players[1].chips, game.players[2].chips);

        // p2 has 600 chips at the end (300 from p2, and p3)
        assert_eq!(game.players[1].chips, 600, "Expected P2 to have 600 chips. Instead P1: {}, P2: {}, P3: {}", game.players[0].chips, game.players[1].chips, game.players[2].chips);

        // p3 has 500 chips at the end (lost 200 to p1, 300 to p2)
        assert_eq!(game.players[2].chips, 500);
    }
}

fn main() {}
