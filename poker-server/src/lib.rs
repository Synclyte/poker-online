use std::{cmp::Ordering::{self, Greater, Less}, fmt::{self, Debug}};
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use serde_json::{self};
use rand::{RngExt, SeedableRng, seq::{SliceRandom}};
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
            HandType::None => |cards, hand_size| Some(Hand { hand_type: HandType::None, cards: vec![], spare: cards[..hand_size].to_vec() }),
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

        let fear_greed_ratio = self.fear / (self.fear + self.greed);
        let hand_cards: Vec<ExpandedCard> = [&player.cards[..], &game.community[..]].concat();
        let hand_strength = self.calculate_hand_value(&hand_cards, game);

        // if the game bet is greater than their own bet:
        if game.bet > player.bet {
            let new_total = self.total_bet + game.bet - player.bet;
            if new_total > self.budget {
                let calibration = (game.ctx.blind_size * 3) as f64;
                let raise_ratio = (new_total as f64 + calibration) / (self.budget as f64 + calibration);

                let p_fold = (fear_greed_ratio * raise_ratio - hand_strength * 2.0).clamp((self.randomness - hand_strength).max(0.0), 1.0 - hand_strength);
                let p_call = (fear_greed_ratio - hand_strength).max(self.randomness);

                if game.ctx.rng.random_bool(p_fold) {
                    return Action::Fold;
                } else if game.ctx.rng.random_bool(p_call) {
                    self.total_bet += game.bet - player.bet;
                    self.budget = self.calculate_round_budget(player, game, hand_strength);
                    return Action::Call;
                } else {
                    let raise_amount = game.ctx.blind_size + ((0.5 + game.ctx.rng.random::<f64>()) * 0.1 * self.greed * player.chips as f64) as i32;
                    let adjusted_raise = raise_amount.min(player.chips);
                    self.total_bet += adjusted_raise + game.bet - player.bet;
                    self.budget = self.calculate_round_budget(player, game, hand_strength).clamp(self.budget + raise_amount, player.chips);
                    return Action::Raise(adjusted_raise);
                }
            } else {
                return Action::Call;
            }
        // if the game bet is the same as their bet:
        } else {
            let budget_used = self.total_bet / self.budget;
            let p_raise = (fear_greed_ratio * hand_strength * 2.0 * (1 - budget_used) as f64).max(1.0);
            if game.ctx.rng.random_bool(p_raise) {
                let raise_amount = game.ctx.blind_size + ((0.5 + game.ctx.rng.random::<f64>()) * 0.15 * self.greed * player.chips as f64) as i32;
                let adjusted_raise = raise_amount.min(self.budget - self.total_bet).min(player.chips);
                return Action::Raise(adjusted_raise);
            } else if game.ctx.rng.random_bool(1.0 - (self.randomness / 4.0 - hand_strength).clamp(0.0, 1.0)) {
                return Action::Call;
            } else {
                return Action::Fold;
            }
        }
    }

    fn calculate_round_budget(&self, player: &Player, game: &mut Game, hand_strength: f64) -> i32 {
        if game.round <= Round::Preflop {
            // by default, will stake at most 1/10 of chips + blind size on preflop
            let base_bet: f64 = player.chips as f64 / 10.0 * self.greed;
            let mut base_bet_blind: i32 = (base_bet as i32 + game.ctx.blind_size).max(player.chips);

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
        let sunk_cost: f64 = self.total_bet as f64 / (self.total_bet + player.chips) as f64;

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
    bet: i32,
    cards: Vec<ExpandedCard>,
    special_cards: Vec<SpecialCard>,
    id: usize,
    acted: bool,
    folded: bool,
    forced_bet: i32,
    remove: bool,
}
impl Player {
    fn new(player_type: PlayerType, id: usize) -> Self {
        Player { player_type, chips: 0, bet: 0, cards: Vec::new(), special_cards: Vec::new(), id, acted: false, folded: false, forced_bet: 0, remove: false }
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
            Round::Room => write!(f, "Not Started"),
            Round::Preround => write!(f, "Pre-Round"),
            Round::Preflop => write!(f, "Pre-Flop"),
            Round::Flop => write!(f, "Flop"),
            Round::Turn => write!(f, "Turn"),
            Round::River => write!(f, "River"),
            Round::Showdown => write!(f, "Showdown"),
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
    bet: i32,
    round_pool: i32,
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
    pub special_cards: Vec<String>,
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
    pub fn new(config_json: &str) -> Result<Game, String> {
        let config: GameConfig = serde_json::from_str(config_json)
            .map_err(|e| format!("Failed to parse given config: {}", e))?;

        let total_players = config.player_count + config.bots.len();
        let mut ctx = GameContext::new(5, ChaCha12Rng::seed_from_u64(config.rng_seed), HAND_TYPES.to_vec(), config.blind_size, config.min_raise, config.starting_chips, config.deck_type, config.max_players);
        let deck = get_custom_deck(&ctx.deck_type);

        let mut players: Vec<Player> = Vec::with_capacity(total_players);
        for _ in 0..config.player_count {
            players.push(Player::new(PlayerType::Human, ctx.id));
            ctx.id += 1;
        }
        for bot_type in config.bots {
            players.push(Player::new(PlayerType::Computer(AI::from_str(&bot_type)), ctx.id));
            ctx.id += 1;
        }

        Ok(Game { 
            ctx, 
            round: Round::Preround, 
            games_played: 0,
            players, 
            turn_index: 0,
            bet: 0,
            round_pool: 0,
            pots: Vec::new(),
            deck, 
            community: Vec::new() 
        })
    }

    /**
     * Adds a player to the current game, given the round has not started and there is space
     * Returns a json string containing an error or the id of the newly added player
     */
    #[wasm_bindgen]
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
            
            serde_json::json!({"id": &self.ctx.id - 1}).to_string()
        } else {
            serde_json::json!({"error": "Specified player type is invalid"}).to_string()
        }
    }

    #[wasm_bindgen]
    /**
     * Transitions the game from the room to the preround, through resetting relevant game data
     */
    pub fn initialise(&mut self) -> String {
        if self.round != Round::Room {
            return serde_json::json!({"error": "Could not initialise game from current game round"}).to_string();
        }

        self.players.iter_mut().for_each(|p| {
            p.chips = self.ctx.starting_chips;
            p.bet = 0;
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

        self.get_game_state(usize::MAX).to_string()
    }

    #[wasm_bindgen]
    /**
     * Transitions the current game from the round intermission to the game
     */
    pub fn start(&mut self) -> String {
        if self.round == Round::Preround {
            self.update_round();
            return self.get_game_state(usize::MAX);
        }
        serde_json::json!({"error": "Invalid state for starting game"}).to_string()
    }

    #[wasm_bindgen]
    /**
     * Takes a move from a given player, and executes it after confirming move validity
     */
    pub fn player_move(&mut self, player_id: i32, str_action: String, amount: i32) -> String {
        let action = Action::from_str(str_action, Some(amount));
        if action.is_none() {
            return serde_json::json!({"error": "Invalid move"}).to_string();
        }

        let mut events: Vec<MoveEvent> = Vec::new();
        if let Err(e) = self.execute_action(player_id, action.unwrap(), &mut events) {
            return e;
        }

        let game_state_string = self.get_game_state(usize::MAX);
        let game_state: serde_json::Value = serde_json::from_str(&game_state_string).unwrap();

        let result: MoveResult = MoveResult { events, game_state };

        serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
    }

    #[wasm_bindgen]
    /**
     * Removes a player at the end of the current game
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
     * Removes a player immediately
     */
    pub fn force_remove_player(&mut self, player_id: i32) -> String {
        if let Ok(player_index) = self.get_player_index(player_id as usize) {
            self.players.remove(player_index);
            "{}".to_string()
        } else {
            serde_json::json!({"error": "Could not remove player with specified id: id does not exist"}).to_string()
        }
    }

    fn get_player_index(&self, player_id: usize) -> Result<usize, String> {
        if let Some(pos) = self.players.iter().position(|p| p.id == player_id) {
            Ok(pos)
        } else {
            Err(serde_json::json!({"error": "Could not find player with specified id"}).to_string())
        }

    }

    fn execute_action(&mut self, player_id: i32, action: Action, events: &mut Vec<MoveEvent>) -> Result<(), String> {
        let index_result = self.get_player_index(player_id as usize);
        if index_result.is_err() {
            return Err(index_result.unwrap_err());
        }
        let player_index = index_result.unwrap();

        if self.turn_index as i32 != player_id {
            return Err(serde_json::json!({"error": "Invalid turn order"}).to_string());
        } else if self.players[player_index].folded {
            return Err(serde_json::json!({"error": "Already folded"}).to_string());
        }

        let mut turn_ended = false;
        match action {
            Action::Fold => {
                let player = &mut self.players[player_index];
                player.folded = true;
                player.acted = true;
                turn_ended = true;
            },
            Action::Call => {
                let player = &mut self.players[player_index];
                let player_bet = (self.bet + player.forced_bet - player.bet).min(player.chips);
                player.chips -= player_bet;
                player.bet += player_bet;
                self.round_pool += player_bet;

                player.acted = true;
            },
            Action::Raise(raise) => {
                {
                    let player = &mut self.players[player_index];
                    // reject raise if the player cannot afford it
                    if raise < self.ctx.min_raise {
                        return Err(serde_json::json!({"error": "Raise below minimum threshold"}).to_string());
                    }

                    let raise_cost = self.bet + player.forced_bet + raise - player.bet;
                    if player.chips < raise_cost {
                        return Err(serde_json::json!({"error": "Cannot afford raise"}).to_string());
                    }

                    player.chips -= raise_cost;
                    player.bet += raise_cost;
                    self.round_pool += raise_cost;
                    self.bet += raise;
                }

                // all remaining players should need to act again after a raise
                for p in self.players.iter_mut() {
                    if !p.folded || !(p.chips == 0) {
                        p.acted = false;
                    }
                }

                self.players[player_index].acted = true;
            },
            Action::Timeout => {
                let player = &mut self.players[player_index];
                if !player.acted {
                    player.folded = true;
                }
                player.acted = true;
                turn_ended = true;
            },
            Action::EndMove => {
                let player = &mut self.players[player_index];
                if !player.acted {
                    return Err(serde_json::json!({"error": "Must act before ending move"}).to_string());
                }
                turn_ended = true;
            }
        }

        let player = &self.players[player_index];
        events.push(
            MoveEvent { player_id: player_id as usize, action: action.to_string() }
        );
        if player.folded || (player.chips == 0 && player.special_cards.is_empty()) {
            turn_ended = true;
        }

        if turn_ended {
            return self.handle_round_end(player_index, events);
        }

        Ok(())
    }

    fn handle_round_end(&mut self, player_index: usize, events: &mut Vec<MoveEvent>) -> Result<(), String> {
        let mut active_players = 0;
        let mut round_finished = true;

        // check for remaining players in round
        for p in &self.players {
            if !p.folded && p.chips > 0 {
                active_players += 1;
                if !p.acted || p.bet != self.bet {
                    round_finished = false;
                }
            }
        }        

        if round_finished || active_players <= 1 {
            self.update_round();
        } else {
            let player_count = self.players.len();
            self.turn_index = (self.turn_index + 1) % player_count;

            while self.players[self.turn_index].folded || self.players[self.turn_index].chips == 0 {
                self.players[self.turn_index].acted = true;
                self.turn_index = (self.turn_index + 1) % player_count;
            }

            let ai_data = match self.players[self.turn_index].player_type {
                PlayerType::Computer(ai) => Some(ai),
                _ => None,
            };
            
            if let Some(mut ai) = ai_data {
                let action = ai.calculate_next_action(self.turn_index, self);
                self.players[self.turn_index].player_type = PlayerType::Computer(ai);
                if let Err(e) = self.execute_action(self.turn_index as i32, action, events) {
                    return Err(e);
                }
                return self.execute_action(self.turn_index as i32, Action::EndMove, events);
            };
        }

        Ok(())
    }

    fn process_pots(&mut self) {
        loop {
            let mut min_bet = i32::MAX;
            let mut max_bet = i32::MIN;
            let mut participants = 0;

            for p in &self.players {
                if p.bet > 0 {
                    min_bet = min_bet.min(p.bet);
                    max_bet = max_bet.max(p.bet);
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

                    if !player.folded && !current_pot.players.contains(&player.id) {
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

            game.players.iter_mut().for_each(|p| { p.bet = 0; p.acted = false; } );
            game.bet = 0;
            game.round_pool = 0;
            game.turn_index = 0;
            game.round = next_round;              
        }

        match round {
            Round::Room => self.round = Round::Preround,
            Round::Preround => {
                let mut participants = 0;
                for p in &self.players {
                    if p.chips > 0 {
                        participants += 1;
                    }
                }

                if participants <= 1 {
                    self.round = Round::Room;
                    return;
                }

                self.deck.shuffle(&mut self.ctx.rng);
                for player in self.players.iter_mut() {
                    player.cards.push(self.deck.pop().unwrap_or(ExpandedCard::Unmarked));
                    player.cards.push(self.deck.pop().unwrap_or(ExpandedCard::Unmarked));
                }

                for (i, p) in self.players.iter_mut().enumerate() {
                    p.forced_bet = if i == 0 {
                        self.ctx.blind_size / 2
                    } else if i == 1 {
                        self.ctx.blind_size
                    } else {
                        0
                    }
                }

                process_round(self, 0, 0, Round::Preflop);
            },
            Round::Preflop => process_round(self, 3, 0, Round::Flop),
            Round::Flop => process_round(self, 1, 0, Round::Turn),
            Round::Turn => process_round(self, 1, 0, Round::River),
            Round::River => process_round(self, 0, 0, Round::Showdown),
            Round::Showdown => {
                let mut hands: Vec<(usize, Hand)> = self.players.iter().map(|p| {
                    let hand_cards: Vec<ExpandedCard> = [&self.community[..], &p.cards[..]].concat();

                    (p.id, Hand::new(&hand_cards, &self.ctx))
                }).collect();

                hands.sort_by(|(_, a_h), (_, b_h)| a_h.compare(b_h));
                for pot in &self.pots {
                    let mut winners: Vec<usize> = Vec::new();
                    let winning_hand = &hands.iter().rev().find(|(i, _)| { pot.players.contains(i) }).unwrap().1;
                    for (i, hand) in hands.iter().rev() {
                        let won = hand.compare(winning_hand);
                        if pot.players.contains(i) && (won == Ordering::Equal || won == Ordering::Greater) {
                            winners.push(*i);
                            break;
                        }
                    }

                    let pot_split = pot.amount / winners.len() as i32;
                    let mut remaining_chips = pot.amount % winners.len() as i32;
                    for winner in winners {
                        if let Ok(index) = self.get_player_index(winner) {
                            self.players[index].chips += pot_split;
                            if remaining_chips > 0 {
                                self.players[index].chips += 1;
                                remaining_chips -= 1;
                            }
                        }
                    }
                }
                self.end_round();
            }
        }
    }

    fn end_round(&mut self) {
        for index in (0..self.players.len()).rev() {
            if self.players[index].remove {
                self.players.remove(index);
            }
        }

        self.games_played += 1;
        self.players.rotate_left(1);
        self.round = Round::Preround;
    }

    #[wasm_bindgen]
    pub fn get_human_players(&self) -> i32 {
        let mut humans = 0;
        self.players.iter().for_each(|p| match p.player_type {
            PlayerType::Human => humans += 1,
            PlayerType::Computer(_) => {},
        });
        humans
    }

    #[wasm_bindgen]
    pub fn toggle_id_with_bot(&mut self, player_id: i32, bot: bool) -> String {
        let ai_type = AIType::Safe;
        let index_result = self.get_player_index(player_id as usize);
        if let Err(error) = index_result {
            return error;
        }
        let index = index_result.unwrap();
        let player = &mut self.players[index];
        let mut event_log: Vec<MoveEvent> = Vec::new();
        
        if bot {
            event_log.push(MoveEvent { player_id: player.id, action: "SWAP_BOT".to_string() });
            let mut new_ai = AI::new(ai_type);
            player.player_type = PlayerType::Computer(new_ai);

            if self.turn_index == index && !player.folded {
                let action = new_ai.calculate_next_action(index, self);
                if let Err(e) = self.execute_action(player_id, action, &mut event_log) {
                    return e;
                }
            }
        } else {
            event_log.push(MoveEvent { player_id: player.id, action: "SWAP_HUMAN".to_string() });
            player.player_type = PlayerType::Human;
        }

        let state_string: String = self.get_game_state(usize::MAX);
        let state_value: serde_json::Value = serde_json::from_str(&state_string).unwrap();
        let result = MoveResult { events: event_log, game_state: state_value };

        serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string())
    }

    #[wasm_bindgen]
    pub fn get_current_turn_player(&self) -> i32 {
        if let Ok(i) = self.get_player_index(self.turn_index) {
            return i as i32;
        } else {
            panic!()
        }
    }

    /**
     * Returns a JSON string capturing the full game state as seen by the specified player id
     */
    #[wasm_bindgen]
    pub fn get_game_state(&self, player_id: usize) -> String {
        let player_states: Vec<PlayerState> = self.players.iter().enumerate().map(|(i, p)| {
            let hole_cards = if self.round == Round::Showdown || i == player_id {
                p.cards.iter().map(|c| c.to_string()).collect()
            } else {
                p.cards.iter().map(|_| "HIDDEN".to_string()).collect()
            };

            let special_cards = if i == player_id {
                p.special_cards.iter().map(|c| c.to_string()).collect()
            } else {
                vec![]
            };

            PlayerState {
                id: p.id,
                chips: p.chips,
                round_bet: p.bet,
                folded: p.folded,
                acted: p.acted,
                is_turn: self.turn_index == i && !(self.round == Round::Showdown),
                hole_cards,
                special_cards,
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

        serde_json::to_string(&complete_state).unwrap_or_else(|_| "Failed to parse state as string".to_string())
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

fn main() {

}
