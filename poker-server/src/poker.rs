/**
 * Contains all poker hand comparison logic and poker-related structs
 */

use std::{cmp::{Ordering::{self, Equal, Greater, Less}}, fmt::{self, Debug}};

use serde::Serialize;

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Debug)]
#[repr(u8)]
pub(crate) enum Rank {
    None = 0,
    Two = 1,
    Three = 2,
    Four = 3,
    Five = 4, 
    Six = 5,
    Seven = 6,
    Eight = 7,
    Nine = 8,
    Ten = 9,
    Jack = 10,
    Queen = 11,
    King = 12,
    Ace = 13
}
impl std::fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Rank::None => "0",
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5", 
            Rank::Six => "6", 
            Rank::Seven => "7", 
            Rank::Eight => "8", 
            Rank::Nine => "9", 
            Rank::Ten => "10", 
            Rank::Jack => "j", 
            Rank::Queen => "q", 
            Rank::King => "k", 
            Rank::Ace => "a", 
        };

        f.write_str(message)
    }
}
impl Rank {
    pub(crate) fn from(num: impl TryInto<u8>) -> Rank {
        let n: u8 = num.try_into().unwrap_or(u8::MAX);
        match n {
            1 => Rank::Two,
            2 => Rank::Three,
            3 => Rank::Four,
            4 => Rank::Five,
            5 => Rank::Six,
            6 => Rank::Seven,
            7 => Rank::Eight,
            8 => Rank::Nine,
            9 => Rank::Ten,
            10 => Rank::Jack,
            11 => Rank::Queen,
            12 => Rank::King,
            13 => Rank::Ace,
            _ => Rank::None,
        }
    }

    pub(crate) fn from_str(str: &str) -> Rank {
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
            _ => Rank::None,
        }
    }
}
pub(crate) static RANKS: [Rank; 13] = [
    Rank::Two,
    Rank::Three, 
    Rank::Four, 
    Rank::Five, 
    Rank::Six, 
    Rank::Seven, 
    Rank::Eight, 
    Rank::Nine, 
    Rank::Ten, 
    Rank::Jack, 
    Rank::Queen, 
    Rank::King, 
    Rank::Ace
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Suit {
    Spades,
    Diamonds,
    Clubs,
    Hearts,
    Suitless,
}
impl std::fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Suit::Spades => "s",
            Suit::Clubs => "c",
            Suit::Hearts => "h",
            Suit::Diamonds => "d",
            Suit::Suitless => "n",
        };

        f.write_str(message)
    }
}
impl std::fmt::Debug for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Suit::Spades => "Spades",
            Suit::Clubs => "Clubs",
            Suit::Hearts => "Hearts",
            Suit::Diamonds => "Diamonds",
            Suit::Suitless => "Suitless",
        };

        f.write_str(message)
    }    
}
impl Suit {
    pub(crate) fn from(suit: &str) -> Suit {
        match suit {
            "s" => Suit::Spades,
            "d" => Suit::Diamonds,
            "c" => Suit::Clubs,
            "h" => Suit::Hearts,
            "n" | _ => Suit::Suitless,
        }
    }
}
pub(crate) static SUITS: [Suit; 4] = [Suit::Spades, Suit::Diamonds, Suit::Clubs, Suit::Hearts];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Card {
    pub(crate) rank: Rank,
    pub(crate) suit: Suit,
}

#[derive(Clone, Copy, Eq)]
pub(crate) enum ExpandedCard {
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
        self.card().rank == other.card().rank && self.card().suit == other.card().suit
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
    pub(crate) fn card(&self) -> &Card {
        match self {
            Self::PlayingCard(c) | Self::Joker(c) => c,
            Self::Unmarked => &Card { rank: Rank::None, suit: Suit::Suitless }
        }
    }
    
    pub(crate) fn get_card(rank: &str, suit: &str) -> ExpandedCard {
        ExpandedCard::PlayingCard( Card { rank: Rank::from_str(rank), suit: Suit::from(suit) } )
    }

    pub(crate) fn get_joker() -> ExpandedCard {
        ExpandedCard::Joker( Card { rank: Rank::Ace, suit: Suit::Spades } )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Serialize)]
pub(crate) enum HandType {
    None,
    HighCard,
    Pair,
    TwoPair,
    ThreeOfAKind,
    Straight,
    Flush,
    FullHouse,
    FourOfAKind,
    StraightFlush,
    RoyalFlush,
}
impl fmt::Display for HandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            HandType::None => "Nothing",
            HandType::HighCard => "High Card",
            HandType::Pair => "Pair",
            HandType::TwoPair => "Two Pair",
            HandType::ThreeOfAKind => "Three of a Kind",
            HandType::FullHouse => "Full House",
            HandType::Flush => "Flush",
            HandType::Straight => "Straight",
            HandType::FourOfAKind => "Four of a Kind",
            HandType::StraightFlush => "Straight Flush",
            HandType::RoyalFlush => "Royal Flush",
        };

        f.write_str(message)
    }
}
impl HandType {
    /**
     Returns relevant function for checking the requested hand type, allowing a function array to be build/iterated over.
     * This is used for 2 main reasons:
     * Improving modularity (allows new hands to be added without much extra effort)
     * Improving code performance (does not attempt to find unused hand types)
     */
    pub(crate) fn get_check_fn(hand_type: HandType) -> fn(&Vec<ExpandedCard>, usize) -> Option<Hand> {
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
                    .max_by(|h_1, h_2| h_1.cmp(h_2))
            },
            HandType::FourOfAKind => |cards, hand_size| build_hand(get_representation(cards, vec![4], None, hand_size), HandType::FourOfAKind),
            HandType::FullHouse => |cards, hand_size| build_hand(get_representation(cards, vec![3, 2], None, hand_size), HandType::FullHouse),
            HandType::Straight => |cards, hand_size| build_hand(get_straight(cards, 5, None, hand_size), HandType::Straight),
            HandType::Flush => |cards, hand_size| {
                return SUITS.iter()
                    .filter_map(|s| get_representation(cards, vec![1, 1, 1, 1, 1], Some(*s), hand_size))
                    .map(|(s_c, s_s)| Hand { hand_type: Self::Flush, cards: s_c, spare: s_s } )
                    .max_by(|h_1, h_2| h_1.cmp(h_2))
            },
            HandType::ThreeOfAKind => |cards, hand_size| build_hand(get_representation(cards, vec![3], None, hand_size), HandType::ThreeOfAKind),
            HandType::TwoPair => |cards, hand_size| build_hand(get_representation(cards, vec![2, 2], None, hand_size), HandType::TwoPair),
            HandType::Pair => |cards, hand_size| build_hand(get_representation(cards, vec![2], None, hand_size), HandType::Pair),
            HandType::HighCard => |cards, hand_size| build_hand(get_representation(cards, vec![1], None, hand_size), HandType::HighCard),
            _ => |cards, hand_size|
                Some(Hand { hand_type: HandType::None, cards: vec![], spare: cards[..hand_size.min(cards.len())].to_vec()
            }),
        } 
    }
}
pub(crate) static HAND_TYPES: [HandType; 11] = [
    HandType::RoyalFlush, HandType::StraightFlush, HandType::FourOfAKind, HandType::Straight, 
    HandType::FullHouse, HandType::Flush, HandType::ThreeOfAKind, HandType::TwoPair, 
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
                ExpandedCard::PlayingCard(card) if (suit_needed && card.suit == required_suit) || !suit_needed => { organised_cards[card.rank as usize - 1].push(i); },
                _ => {},
            });
        } else {
            cards.iter().enumerate().for_each(|(i, c)| match c {
                ExpandedCard::Joker(_) => { jokers += 1; joker_indices.push(i); },
                ExpandedCard::PlayingCard(card) => { organised_cards[card.rank as usize - 1].push(i); },
                _ => {}
            });            
        }

        HandContext { cards, organised_cards, joker_indices, jokers, used_counts, suit }
    }

    fn extract_hand(
        &self, 
        mut representation_cards: Vec<(Rank, Vec<usize>)>, 
        hand_size: usize) -> Option<(Vec<Vec<ExpandedCard>>, Vec<ExpandedCard>)> 
    {
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
        extra_cards.sort_by(|a, b| b.cmp(a));
        
        Some((hand_cards, extra_cards))
    }
}

#[derive(Eq)]
pub(crate) struct Hand {
    pub(crate) hand_type: HandType,
    pub(crate) cards: Vec<Vec<ExpandedCard>>,
    pub(crate) spare: Vec<ExpandedCard>,
}
impl fmt::Display for Hand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {:?}: {:?}", self.hand_type, self.cards, self.spare)
    }
}
impl Hand {
    pub(crate) fn new(cards: &Vec<ExpandedCard>, hand_fns: &Vec<(HandType, fn(&Vec<ExpandedCard>, usize) -> Option<Hand>)>, invalidated_hands: &Vec<HandType>, hand_size: usize) -> Self {
        if cards.len() <= 0 {
            return Hand { hand_type: HandType::None, cards: vec![], spare: vec![] };
        }
        for (hand_type, hand_fn) in hand_fns {
            if invalidated_hands.iter().all(
                |invalid_hand| invalid_hand != hand_type) && let Some(hand) = hand_fn(cards, hand_size) 
            {
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

        Equal
    }
}
impl PartialEq for Hand {
    fn eq(&self, other: &Self) -> bool {
        Hand::compare(&self, other) == Equal
    }

    fn ne(&self, other: &Self) -> bool {
        !(self == other)
    }
}
impl PartialOrd for Hand {
    fn lt(&self, other: &Self) -> bool {
        Hand::compare(&self, other) == Less
    }

    fn gt(&self, other: &Self) -> bool {
        Hand::compare(&self, other) == Greater
    }

    fn ge(&self, other: &Self) -> bool {
        let result = Hand::compare(&self, other);
        result == Greater || result == Equal 
    }

    fn le(&self, other: &Self) -> bool {
        let result = Hand::compare(&self, other);
        result == Less || result == Equal
    }

    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Hand::compare(&self, other))
    }
}
impl Ord for Hand {
    fn cmp(&self, other: &Self) -> Ordering {
        Hand::compare(&self, other)
    }

    fn clamp(self, min: Self, max: Self) -> Self
    where
        Self: Sized,
    {
        if max > self { max } else if min < self { min } else { self }
    }
    
    fn max(self, other: Self) -> Self
    where
        Self: Sized,
    {
        if other < self { self } else { other }
    }
    
    fn min(self, other: Self) -> Self
    where
        Self: Sized,
    {
        if other < self { other } else { self }
    }
}

/**
 * Takes an unordered list of cards, a Vec<usize> containing a representation of a hand (i.e. (3, 2) is a full house), and a suit
 * Returns the cards which match the given representation and suit and the best remaining cards in hand, otherwise returns None
 * Works greedily - will always select the highest ranked cards matching a representation first
 */
fn get_representation(
    cards: &Vec<ExpandedCard>, 
    representation: Vec<usize>, 
    suit: Option<Suit>, 
    hand_size: usize) 
    -> Option<(Vec<Vec<ExpandedCard>>, Vec<ExpandedCard>)> 
{
    let cards_required = representation.iter().sum::<usize>();
    if hand_size < cards_required || cards.len() < cards_required {
        return None;
    }

    let mut ctx = HandContext::new(&cards, suit);
    if ctx.cards.len() < cards_required {
        return None;
    }

    // recursive backtracking matcher - guaranteed to find best representation with jokers if one exists
    fn match_group(
        group: usize, 
        rep_index: usize, 
        representation: &Vec<usize>, 
        matched_group: &mut Vec<(Rank, Vec<usize>)>, 
        ctx: &mut HandContext<'_>) 
        -> bool 
    {
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