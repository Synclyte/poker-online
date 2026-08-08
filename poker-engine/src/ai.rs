/**
 * Handles AI moves
 */

use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::{Game, Action, Round, Player};
use crate::poker::*;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) enum AIType {
    Safe,
    Risky,
    Smart,
    Random,
}

#[derive(Clone, Debug, Copy)]
pub(crate) struct AI {
    ai_type: AIType,
    total_bet: i32,
    budget: i32, // calculated reasonable bet for current turn (only considers folding if above, considers raising if below)
    fear: f64, // measure of reaction to game stakes (bet/relative bet compared to own chips)
    greed: f64, // measure of reaction to the strength of its cards
    randomness: f64, // 0-1, probability of performing an unexpected action
}
impl AI {
    pub(crate) fn from_str(str: &str) -> Self {
        match str.to_lowercase().as_str() {
            "risky" => Self::new(AIType::Risky),
            "smart" => Self::new(AIType::Smart),
            "random" => Self::new(AIType::Random),
            "safe" | _ => Self::new(AIType::Safe),
        } 
    }

    pub(crate) fn new(ai_type: AIType) -> Self {
        match ai_type {
            AIType::Risky => AI { ai_type, total_bet: 0, budget: 0, fear: 0.8, greed: 1.4, randomness: 0.2 },
            AIType::Smart => AI { ai_type, total_bet: 0, budget: 0, fear: 1.0, greed: 1.0, randomness: 0.07 },
            AIType::Random => AI { ai_type, total_bet: 0, budget: 0, fear: 1.0, greed: 1.0, randomness: 0.4 },
            AIType::Safe => AI { ai_type, total_bet: 0, budget: 0, fear: 1.7, greed: 0.7, randomness: 0.1 },
        }
    }

    pub(crate) fn update_round_parameters(&mut self) {
        self.total_bet = 0;
        self.budget = 0;        
    }

    /**
     * Uses a non-deterministic decision tree to select an Action based on the current game state, cards, and AI preset parameters
     */
    pub(crate) fn calculate_next_action(&mut self, player_index: usize, game: &mut Game) -> Vec<Action> {
        // there are two primary situations a player can face:
        //  - the game bet is equal to what they have bet
        //  - the game bet is greater than what they have bet

        let mut actions: Vec<Action> = Vec::new();
        let player = &game.players[player_index].clone();

        let fear_greed_sum = self.fear + self.greed;
        let fear_greed_ratio = if fear_greed_sum > 0.0 { self.fear / fear_greed_sum } else { 0.5 };
        let hand_cards: Vec<ExpandedCard> = [&player.cards[..], &game.community[..]].concat();
        let hand_strength = self.calculate_hand_value(&hand_cards, game).clamp(0.0, 1.0);

        if self.budget <= 0 {
            self.budget = self.calculate_round_budget(player, game, hand_strength);
        }

        if game.bet > player.round_bet {
            let new_total = self.total_bet + game.bet - player.round_bet;
            if new_total > self.budget {
                let calibration = (game.ctx.blind_size.max(1) * 3) as f64;
                let denominator = (self.budget as f64 + calibration).max(1.0);
                let raise_ratio = (new_total as f64 + calibration) / denominator;

                let raw_fold = fear_greed_ratio * raise_ratio - hand_strength * 2.0;
                let min_fold = (self.randomness - hand_strength).max(0.0);
                let max_fold = (1.0 - hand_strength).max(min_fold);

                let p_fold = if raw_fold.is_nan() {
                    0.5
                } else {
                    raw_fold.clamp(min_fold, max_fold)
                };

                let p_call = if (fear_greed_ratio - hand_strength).is_nan() { 
                    0.5
                } else { 
                    (fear_greed_ratio - hand_strength).max(self.randomness).clamp(0.0, 1.0) 
                };

                if game.ctx.rng.random_bool(p_fold) {
                    actions.push(Action::Fold);
                } else if game.ctx.rng.random_bool(p_call) {
                    self.total_bet += (game.bet - player.round_bet).min(player.chips);
                    self.budget = self.calculate_round_budget(player, game, hand_strength);
                    actions.push(Action::Call);
                } else {
                    let effective_min = (game.ctx.min_raise as f64 * game.modifiers.vars.ante_multiplier) as i32;
                    let min_r = effective_min.min(player.chips);
                    let raise_amount = game.ctx.blind_size + ((0.5 + game.ctx.rng.random::<f64>()) * 0.1 * self.greed * player.chips as f64) as i32;
                    let adjusted_raise = raise_amount.max(min_r).min(player.chips);
                    if adjusted_raise >= min_r {
                        self.total_bet += adjusted_raise + (game.bet - player.round_bet).min(player.chips);
                        let upper_budget = player.chips.max(1);
                        self.budget = self.calculate_round_budget(player, game, hand_strength).clamp((self.budget + raise_amount).min(upper_budget), upper_budget);
                        actions.push(Action::Raise { amount: adjusted_raise });
                    } else {
                        self.total_bet += (game.bet - player.round_bet).min(player.chips);
                        self.budget = self.calculate_round_budget(player, game, hand_strength);
                        actions.push(Action::Call);
                    }
                }
            } else {
                actions.push(Action::Call);
            }
        } else {
            let budget_used = if self.budget > 0 { (self.total_bet as f64 / self.budget as f64).clamp(0.0, 1.0) } else { 0.0 };
            let p_raise = (fear_greed_ratio * hand_strength * 2.0 * (1.0 - budget_used)).clamp(0.0, 1.0);

            if !p_raise.is_nan() && game.ctx.rng.random_bool(p_raise) {
                let effective_min = (game.ctx.min_raise as f64 * game.modifiers.vars.ante_multiplier) as i32;
                let min_r = effective_min.min(player.chips);
                let raise_amount = game.ctx.blind_size + ((0.5 + game.ctx.rng.random::<f64>()) * 0.15 * self.greed * player.chips as f64) as i32;
                let adjusted_raise = raise_amount.max(min_r).min(self.budget - self.total_bet).min(player.chips);
                if adjusted_raise >= min_r {
                    actions.push(Action::Raise { amount: adjusted_raise });
                } else {
                    actions.push(Action::Call);
                }
            } else {
                actions.push(Action::Call);
            }
        }

        actions
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
        adjusted_budget.clamp(0,player.chips)
    }

    fn calculate_hand_value(self, hand_cards: &Vec<ExpandedCard>, game: &Game) -> f64 {
        fn get_raw_value(hand: &Hand) -> f64 {
            fn base_hand_value(hand_type: HandType) -> f64 {
                match hand_type {
                    HandType::HighCard => 1.0,
                    HandType::Pair => 2.0,
                    HandType::TwoPair => 3.0,
                    HandType::ThreeOfAKind => 5.0,
                    HandType::Straight => 6.0,
                    HandType::Flush => 7.0,
                    HandType::FullHouse => 8.0,
                    HandType::FourOfAKind => 15.0,
                    HandType::StraightFlush => 20.0,
                    HandType::RoyalFlush => 100.0,
                    HandType::None => 0.0,
                }
            }

            let base_value: f64 = base_hand_value(hand.hand_type);

            let normalised_rank_value: f64 = (hand.cards[0][0].card().rank as u8 - 1) as f64 / 13.0;
            // high card (2) = 0, straight flush (K) = ~9.03
            let centered_adjusted_value: f64 = base_value * (normalised_rank_value * 0.25 + 1.0) - 1.0;
            // range 0-1, between worst and best hand
            let normalised_adjusted_value: f64 = centered_adjusted_value / 9.03;

            normalised_adjusted_value
        }

        let hand = Hand::new(&hand_cards, &game.ctx.hand_fns, &game.modifiers.invalidated_hands(), game.ctx.hand_size);

        if hand.hand_type == HandType::None {
            return 0.0;
        } else if hand.hand_type == HandType::RoyalFlush {
            return 1000.0;
        }

        let mut raw_value = get_raw_value(&hand);
        
        // additional adjustments
        // if this hand is the same as the hand on the board, penalise
        let board_hand = Hand::new(&game.community, &game.ctx.hand_fns, &game.modifiers.invalidated_hands(), game.ctx.hand_size);
        let is_board = hand == board_hand;
        let playing_board = hand.hand_type == hand.hand_type;
        if is_board {
            raw_value /= 3.0;
        } else if playing_board {
            raw_value /= 2.0;
        }

        // grants boost if hand could be much better within the next few rounds
        if game.round == Round::Flop || game.round == Round::Turn {
            let potential_hand = Hand::new(
                &[hand_cards.clone(), vec![ExpandedCard::get_joker()]].concat(), 
                &game.ctx.hand_fns, 
                &game.modifiers.invalidated_hands(), 
                game.ctx.hand_size
            );
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