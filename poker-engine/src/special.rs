/**
 * Handles special card related logic
 */

use std::collections::HashMap;
use std::fmt;
use rand::RngExt;
use serde::Serialize;

use crate::{ExpandedCard, Game, GameError, HandType, MoveAction, MoveEvent, Rank, Suit, poker::{Card, Hand}};

#[derive(Default, Clone)]
pub(crate) struct SpecialState {
    pub(crate) modifiers: RoundModifiers,
    pub(crate) rounds_since_ante_up: u32,
    pub(crate) ante_up_drawn_this_hand: bool,
    pub(crate) hand_bets: HashMap<usize, i32>,
}
impl SpecialState {
    pub(crate) fn active_modifiers(&self) -> &[Modifier] {
        &self.modifiers.active
    }

    pub(crate) fn pot_multiplier(&self) -> f64 {
        self.modifiers.vars.pot_multiplier
    }

    pub(crate) fn ante_multiplier(&self) -> f64 {
        self.modifiers.vars.ante_multiplier
    }

    pub(crate) fn gamble_success_chance(&self) -> f64 {
        self.modifiers.vars.gamble_success_chance
    }

    pub(crate) fn blackjack_active(&self) -> bool {
        self.modifiers.blackjack_active()
    }

    pub(crate) fn specials_blocked(&self) -> bool {
        self.modifiers.specials_blocked()
    }

    pub(crate) fn raises_blocked(&self) -> bool {
        self.modifiers.raises_blocked()
    }

    pub(crate) fn invalidated_hands(&self) -> Vec<HandType> {
        self.modifiers.invalidated_hands()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrawTarget {
    Community,
    Hole { player_id: usize },
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BetKind {
    Blind,
    Call,
    Raise,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum GameEvent {
    HandStarted,
    TurnAdvanced { player_id: usize },
    StreetEnded,
    CardDrawn { target: DrawTarget, card: ExpandedCard },
    PlayerBet { player_id: usize, amount: i32, kind: BetKind },
    BetWithdrawn { player_id: usize, amount: i32 },
    PlayerFolded { player_id: usize, total_bet: i32 },
    ShowdownResolved { payouts: Vec<(usize, i32)> },
    HandEnded,
    GameEnded,
    SpecialPlayed { player_id: usize, card: SpecialCard },
    SpecialDiscarded { player_id: usize, card: SpecialCard },
    SpecialReceived { player_id: usize, card: SpecialCard },
}

pub(crate) fn on_event(event: GameEvent, game: &mut Game, move_events: &mut Vec<MoveEvent>) {
    match event {
        GameEvent::HandStarted => {
            game.special.ante_up_drawn_this_hand = false;
            game.special.hand_bets.clear();
        }
        GameEvent::TurnAdvanced { player_id } => {
            let expired = game.special.modifiers.player_turn(player_id);
            clear_modifier_effects(expired, game);
        }
        GameEvent::StreetEnded => {
            let expired = game.special.modifiers.round_ended();
            clear_modifier_effects(expired, game);
        }
        GameEvent::CardDrawn { target, .. } => {
            let expired = game.special.modifiers.card_draw(target);
            clear_modifier_effects(expired, game);
            if let DrawTarget::Hole { player_id } = target {
                if game.ctx.special_card_limit > 0 && game.ctx.rng.random_bool(0.002) {
                    let _ = give_special_card(SpecialCard::Draw, 1, true, player_id, game, move_events);
                }
            }
        }
        GameEvent::PlayerBet { player_id, amount, .. } => {
            if amount > 0 {
                *game.special.hand_bets.entry(player_id).or_insert(0) += amount;
                for m in &mut game.special.modifiers.active {
                    if let ModifierEffect::BetRebate {
                        player_id: p_id,
                        accumulated_bet,
                        ..
                    } = &mut m.effect
                    {
                        if *p_id == player_id {
                            *accumulated_bet += amount;
                        }
                    }
                }
            }
        }
        GameEvent::BetWithdrawn { player_id, amount } => {
            if let Some(total) = game.special.hand_bets.get_mut(&player_id) {
                *total = (*total - amount).max(0);
            }
            for m in &mut game.special.modifiers.active {
                if let ModifierEffect::BetRebate {
                    player_id: p_id,
                    accumulated_bet,
                    ..
                } = &mut m.effect
                {
                    if *p_id == player_id {
                        *accumulated_bet = (*accumulated_bet - amount).max(0);
                    }
                }
            }
        }
        GameEvent::PlayerFolded { player_id, total_bet } => {
            if total_bet > 0 {
                let rebate_rate = game.special.modifiers.active.iter().find_map(|m| match m.effect {
                    ModifierEffect::LossInsurance {
                        player_id: p_id,
                        rebate_rate,
                    } if p_id == player_id => Some(rebate_rate),
                    _ => None,
                });
                if let Some(rate) = rebate_rate {
                    let refund = ((total_bet as f64) * rate).round() as i32;
                    if refund > 0 {
                        if let Ok(idx) = game.get_player_index(player_id) {
                            game.players[idx].chips += refund;
                        }
                    }
                    game.special.modifiers.active.retain(|m| {
                        !matches!(
                            m.effect,
                            ModifierEffect::LossInsurance { player_id: p_id, .. } if p_id == player_id
                        )
                    });
                }
            }
        }
        GameEvent::ShowdownResolved { payouts } => {
            let active_insurances: Vec<(usize, f64)> = game
                .special
                .modifiers
                .active
                .iter()
                .filter_map(|m| match m.effect {
                    ModifierEffect::LossInsurance {
                        player_id,
                        rebate_rate,
                    } => Some((player_id, rebate_rate)),
                    _ => None,
                })
                .collect();

            for (player_id, rate) in active_insurances {
                let Ok(idx) = game.get_player_index(player_id) else {
                    continue;
                };
                if game.players[idx].folded {
                    continue;
                }
                let won: i32 = payouts
                    .iter()
                    .filter(|(id, _)| *id == player_id)
                    .map(|(_, amt)| *amt)
                    .sum();
                let spent = game
                    .special
                    .hand_bets
                    .get(&player_id)
                    .copied()
                    .unwrap_or(game.players[idx].total_bet);
                let net_loss = (spent - won).max(0);
                let refund = ((net_loss as f64) * rate).round() as i32;
                if refund > 0 {
                    game.players[idx].chips += refund;
                }
            }

            game.special
                .modifiers
                .active
                .retain(|m| !matches!(m.effect, ModifierEffect::LossInsurance { .. }));
        }
        GameEvent::HandEnded => {
            let stipend_payout = (game.ctx.blind_size as f64 * game.special.ante_multiplier()).round() as i32;
            if stipend_payout > 0 {
                let stipend_recipients: Vec<usize> = game
                    .special
                    .modifiers
                    .active
                    .iter()
                    .filter_map(|m| match m.effect {
                        ModifierEffect::ChipStipend { player_id } => Some(player_id),
                        _ => None,
                    })
                    .collect();
                for player_id in stipend_recipients {
                    if let Ok(idx) = game.get_player_index(player_id) {
                        game.players[idx].chips += stipend_payout;
                    }
                }
            }

            if game.special.ante_up_drawn_this_hand {
                game.special.rounds_since_ante_up = 0;
            } else {
                game.special.rounds_since_ante_up = game.special.rounds_since_ante_up.saturating_add(1);
            }
            game.special.ante_up_drawn_this_hand = false;
            game.special.hand_bets.clear();

            let expired = game.special.modifiers.hand_ended();
            for m in &expired {
                if let ModifierEffect::BetRebate {
                    player_id,
                    rebate_rate,
                    accumulated_bet,
                } = m.effect
                {
                    let payout = ((accumulated_bet as f64) * rebate_rate).round() as i32;
                    if payout > 0 {
                        if let Ok(idx) = game.get_player_index(player_id) {
                            game.players[idx].chips += payout;
                        }
                    }
                }
            }
            clear_modifier_effects(expired, game);
        }
        GameEvent::GameEnded => {
            let expired = game.special.modifiers.active.clone();
            clear_modifier_effects(expired, game);
            game.special.modifiers.game_ended();
            game.special.rounds_since_ante_up = 0;
            game.special.ante_up_drawn_this_hand = false;
            game.special.hand_bets.clear();
        }
        GameEvent::SpecialPlayed { player_id, .. } => {
            if game.ctx.rng.random_bool(0.005) {
                let _ = give_special_card(SpecialCard::Special, 1, true, player_id, game, move_events);
            }
        }
        GameEvent::SpecialDiscarded { player_id, .. } => {
            if game.ctx.rng.random_bool(0.01) {
                // should never fail so long as player_id and game are valid
                let _ = give_special_card(SpecialCard::Discard, 1, true, player_id, game, move_events);
            }
        }
        GameEvent::SpecialReceived { card, .. } => {
            if card == SpecialCard::AnteUp {
                game.special.rounds_since_ante_up = 0;
                game.special.ante_up_drawn_this_hand = true;
            }
        }
    }
}

fn clear_modifier_effects(expired: Vec<Modifier>, game: &mut Game) {
    for m in expired {
        m.remove_effect(game);
    }
}

/**
 * Draws a single card from the deck for either community or player hole placement,
 * applying eligible active DrawRule modifiers and emitting GameEvent::CardDrawn.
 */
pub(crate) fn draw_deck_card(
    game: &mut Game,
    target: DrawTarget,
    reference_card: Option<ExpandedCard>,
    events: &mut Vec<MoveEvent>,
) -> ExpandedCard {
    let rules: Vec<Box<dyn Fn(&ExpandedCard) -> bool>> = game
        .special
        .modifiers
        .active
        .iter()
        .filter_map(|m| match m.effect {
            ModifierEffect::ForceDraw { rule, scope } => {
                let applies = match (target, scope) {
                    (DrawTarget::Community, _) => true,
                    (DrawTarget::Hole { .. }, DrawScope::AllDraws) => true,
                    _ => false,
                };
                if applies {
                    Some(rule.match_fn(reference_card))
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    let card = if let Some(index) = game.deck.iter().rposition(|c| rules.iter().all(|r| r(c))) {
        game.deck.remove(index)
    } else {
        ExpandedCard::Unmarked
    };

    on_event(GameEvent::CardDrawn { target, card }, game, events);
    card
}

#[derive(Default, Clone)]
pub(crate) struct RoundModifiers {
    pub active: Vec<Modifier>,
    pub vars: ModifierVars,
}
impl RoundModifiers {
    pub(crate) fn recompute_vars(&mut self) {
        let mut pot_mult = 1.0;
        let mut ante_mult = 1.0;

        for m in &self.active {
            match m.effect {
                ModifierEffect::PotMultiplier { multiplier } => {
                    pot_mult *= multiplier;
                }
                ModifierEffect::AnteMultiplier { multiplier } => {
                    ante_mult *= multiplier;
                }
                _ => {}
            }
        }

        self.vars.pot_multiplier = pot_mult;
        self.vars.ante_multiplier = ante_mult;
    }

    pub(crate) fn player_turn(&mut self, player_id: usize) -> Vec<Modifier> {
        let mut expired = Vec::new();

        self.active.retain_mut(|m| {
            let retain = match m.expiry {
                ModifierExpiry::OnPlayerTurn { count, player_id: p_id } => {
                    if player_id != p_id {
                        true
                    } else if count > 1 {
                        m.expiry = ModifierExpiry::OnPlayerTurn { count: count - 1, player_id: p_id };
                        true
                    } else {
                        false
                    }
                },
                _ => true
            };

            if !retain { expired.push(m.clone()); }
            retain
        });
        self.recompute_vars();
        expired
    }

    pub(crate) fn hand_ended(&mut self) -> Vec<Modifier> {
        let mut expired = Vec::new();

        self.active.retain_mut(|m| {
            let retain = match m.expiry {
                ModifierExpiry::OnHandEnd { count } => {
                    match count {
                        2.. => { m.expiry = ModifierExpiry::OnHandEnd { count: count - 1 }; true }
                        _ => false
                    }
                },
                _ => true,
            };

            if !retain { expired.push(m.clone()) }
            retain
        });
        self.recompute_vars();
        expired
    }

    pub(crate) fn card_draw(&mut self, target: DrawTarget) -> Vec<Modifier> {
        let mut expired = Vec::new();

        self.active.retain_mut(|m| {
            let should_tick = match (m.effect, target) {
                (ModifierEffect::ForceDraw { .. }, DrawTarget::Community) => true,
                (
                    ModifierEffect::ForceDraw {
                        scope: DrawScope::AllDraws,
                        ..
                    },
                    DrawTarget::Hole { .. },
                ) => true,
                _ => false,
            };

            let retain = match m.expiry {
                ModifierExpiry::OnCardDraw { count } if should_tick => {
                    match count {
                        2.. => { m.expiry = ModifierExpiry::OnCardDraw { count: count - 1 }; true }
                        _ => false
                    }
                },
                _ => true,
            };

            if !retain { expired.push(m.clone()) }
            retain
        });
        self.recompute_vars();
        expired
    }

    pub(crate) fn round_ended(&mut self) -> Vec<Modifier> {
        let mut expired = Vec::new();

        self.active.retain_mut(|m| {
            let retain = match m.expiry {
                ModifierExpiry::OnRoundEnd { count } => {
                    match count {
                        2.. => { m.expiry = ModifierExpiry::OnRoundEnd { count: count - 1 }; true }
                        _ => false
                    }
                },
                _ => true,
            };

            if !retain { expired.push(m.clone()); }
            retain
        });
        self.recompute_vars();
        expired
    }

    pub(crate) fn game_ended(&mut self) {
        self.active.clear();
        self.recompute_vars();
    } 

    pub(crate) fn blackjack_active(&self) -> bool {
        self.active.iter().any(|m| matches!(m.effect, ModifierEffect::BlackjackScoring))
    }

    pub(crate) fn specials_blocked(&self) -> bool {
        self.active.iter().any(|m| matches!(m.effect, ModifierEffect::SpecialsBlocked))
    }

    pub(crate) fn raises_blocked(&self) -> bool {
        self.active.iter().any(|m| matches!(m.effect, ModifierEffect::RaisesBlocked))
    }

    pub(crate) fn invalidated_hands(&self) -> Vec<HandType> {
        self.active.iter().filter_map(|m| match m.effect {
            ModifierEffect::InvalidateHand(hand) => Some(hand),
            _ => None,
        }).collect()
    }

    fn combine_or_add_modifiers(&mut self, modifier: Modifier, stack_mode: StackMode) {
        if stack_mode == StackMode::Separate {
            self.active.push(modifier);
            self.recompute_vars();
            return;
        }

        for i in 0..self.active.len() {
            let effects_match = match (&self.active[i].effect, &modifier.effect, stack_mode) {
                (
                    ModifierEffect::AnteMultiplier { .. },
                    ModifierEffect::AnteMultiplier { .. },
                    StackMode::Effect,
                )
                | (
                    ModifierEffect::PotMultiplier { .. },
                    ModifierEffect::PotMultiplier { .. },
                    StackMode::Effect,
                ) => true,
                (
                    ModifierEffect::LossInsurance {
                        player_id: first_id, ..
                    },
                    ModifierEffect::LossInsurance {
                        player_id: second_id, ..
                    },
                    StackMode::Effect,
                ) => first_id == second_id,
                (a, b, _) => a == b,
            };

            if !effects_match || !self.active[i].expiry.type_equal(&modifier.expiry) {
                continue;
            }

            match stack_mode {
                StackMode::Effect => match (&mut self.active[i].effect, &modifier.effect) {
                    (
                        ModifierEffect::AnteMultiplier { multiplier: first_mult },
                        ModifierEffect::AnteMultiplier { multiplier: second_mult },
                    )
                    | (
                        ModifierEffect::PotMultiplier { multiplier: first_mult },
                        ModifierEffect::PotMultiplier { multiplier: second_mult },
                    ) => {
                        *first_mult *= *second_mult;
                    }
                    (
                        ModifierEffect::LossInsurance {
                            rebate_rate: first_rate, ..
                        },
                        ModifierEffect::LossInsurance {
                            rebate_rate: second_rate, ..
                        },
                    ) => {
                        *first_rate = 1.0 - (1.0 - *first_rate) * (1.0 - *second_rate);
                    }
                    _ => {}
                },
                StackMode::Duration => {
                    self.active[i].expiry = match (self.active[i].expiry, modifier.expiry) {
                        (
                            ModifierExpiry::OnCardDraw { count: first_count },
                            ModifierExpiry::OnCardDraw { count: second_count },
                        ) => ModifierExpiry::OnCardDraw {
                            count: first_count + second_count,
                        },
                        (
                            ModifierExpiry::OnPlayerTurn { count: first_count, player_id: _ },
                            ModifierExpiry::OnPlayerTurn { count: second_count, player_id: p_id },
                        ) => ModifierExpiry::OnPlayerTurn {
                            count: first_count + second_count,
                            player_id: p_id,
                        },
                        (
                            ModifierExpiry::OnRoundEnd { count: first_count },
                            ModifierExpiry::OnRoundEnd { count: second_count },
                        ) => ModifierExpiry::OnRoundEnd {
                            count: first_count + second_count,
                        },
                        (
                            ModifierExpiry::OnHandEnd { count: first_count },
                            ModifierExpiry::OnHandEnd { count: second_count },
                        ) => ModifierExpiry::OnHandEnd {
                            count: first_count + second_count,
                        },
                        _ => ModifierExpiry::OnGameEnd,
                    };
                }
                StackMode::None | StackMode::Separate => {}
            }

            for source_id in modifier.source {
                if !self.active[i].source.contains(&source_id) {
                    self.active[i].source.push(source_id);
                }
            }
            self.recompute_vars();
            return;
        }

        self.active.push(modifier);
        self.recompute_vars();
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ModifierVars {
    pub pot_multiplier: f64,
    pub ante_multiplier: f64,
    pub gamble_success_chance: f64,
}
impl Default for ModifierVars {
    fn default() -> Self {
        Self { 
            pot_multiplier: 1.0, 
            ante_multiplier: 1.0, 
            gamble_success_chance: 0.9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DrawScope {
    CommunityOnly,
    AllDraws,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DrawRule {
    Heart,
    Spade,
    Diamond,
    Club,
    Face,
    High,
    Low,
    HigherThanLast,
    LowerThanLast,
    SameSuitAsLast,
    Nothing,
}
impl DrawRule {
    pub(crate) fn match_fn(&self, reference_card: Option<ExpandedCard>) -> Box<dyn Fn(&ExpandedCard) -> bool + 'static> {
        match self {
            DrawRule::Heart => Box::new(|c| c.card().suit == Suit::Hearts || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::Spade => Box::new(|c| c.card().suit == Suit::Spades || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::Diamond => Box::new(|c| c.card().suit == Suit::Diamonds || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::Club => Box::new(|c| c.card().suit == Suit::Clubs || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::Face => Box::new(|c| matches!(c.card().rank, Rank::Jack | Rank::Queen | Rank::King) || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::High => Box::new(|c| c.card().rank >= Rank::Ten || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::Low => Box::new(|c| c.card().rank <= Rank::Five || c.card().rank == Rank::Ace || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::HigherThanLast => {
                let last_rank = reference_card.map_or(Rank::None, |c| c.card().rank);
                Box::new(move |c| c.card().rank >= last_rank || matches!(c, ExpandedCard::Joker(_)))
            },
            DrawRule::LowerThanLast => {
                let last_rank = reference_card.map_or(Rank::None, |c| c.card().rank);
                Box::new(move |c| c.card().rank <= last_rank || matches!(c, ExpandedCard::Joker(_)))
            },
            DrawRule::SameSuitAsLast => {
                let last_suit = reference_card.map_or(Suit::Suitless, |c| c.card().suit);
                Box::new(move |c| c.card().suit == last_suit || matches!(c, ExpandedCard::Joker(_)))
            },
            DrawRule::Nothing => Box::new(|_| false),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ModifierEffect {
    PotMultiplier { multiplier: f64 },
    AnteMultiplier { multiplier: f64 },
    RevealCard { target_id: usize, card_index: usize, viewer_id: usize },
    RaisesBlocked,
    SpecialsBlocked,
    BlackjackScoring,
    ForceDraw { rule: DrawRule, scope: DrawScope },
    InvalidateHand(HandType),
    LossInsurance { player_id: usize, rebate_rate: f64 },
    BetRebate { player_id: usize, rebate_rate: f64, accumulated_bet: i32 },
    ChipStipend { player_id: usize },
}
impl ModifierEffect {
    pub(crate) fn is_visible(&self, viewer_id: usize) -> bool {
        match self {
            ModifierEffect::RevealCard { target_id, card_index: _, viewer_id: v_id } => *target_id == viewer_id || *v_id == viewer_id,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ModifierExpiry {
    OnCardDraw { count: usize },
    OnPlayerTurn { count: usize, player_id: usize },
    OnRoundEnd { count: usize },
    OnHandEnd { count: usize },
    OnGameEnd,
}
// modifier expiries considered equal if they are of the same type and (if relevant) target the same player
impl ModifierExpiry {
    fn type_equal(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::OnPlayerTurn { count: _, player_id: l_player_id }, Self::OnPlayerTurn { count: _, player_id: r_player_id }) => l_player_id == r_player_id,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct Modifier {
    pub(crate) effect: ModifierEffect,
    expiry: ModifierExpiry,
    source: Vec<usize>,
}
impl Modifier {
    pub(crate) fn remove_effect(&self, game: &mut Game) {
        match self.effect {
            ModifierEffect::RevealCard { target_id, card_index, viewer_id } => {
                if let Ok(idx) = game.get_player_index(target_id) {
                    let player = &mut game.players[idx];
                    if let Some(vis) = player.card_visibility.get_mut(card_index) {
                        vis.retain(|&id| id != viewer_id);
                    }
                }
            },
            _ => {},
        }
    }    
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardRole {
    Economy,
    Board,
    Hand,
    Chaos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetKind {
    None,
    SelfCard,
    CommunityCard,
    OpponentCard,
    OpponentPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardPool {
    Standard,
    WinnerDraft,
    SpecialBurst,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackMode {
    None,
    Duration,
    Effect,
    Separate,
}

pub(crate) struct UseContext<'a> {
    pub player_id: usize,
    pub player_idx: usize,
    pub self_idx: usize,
    pub target_id: usize,
    pub card_index: usize,
    pub stack_mode: StackMode,
    pub game: &'a mut Game,
    pub events: &'a mut Vec<MoveEvent>,
}

#[derive(Clone)]
pub(crate) struct CardDef {
    pub id: &'static str,
    pub roles: Vec<CardRole>,
    pub target: TargetKind,
    pub stack_mode: StackMode,
    pub pools: Vec<(CardPool, fn(f64) -> f64)>,
    pub on_use: fn(UseContext<'_>) -> Result<(), GameError>,
}

fn add_force_draw(ctx: UseContext<'_>, rule: DrawRule) -> Result<(), GameError> {
    ctx.game.special.modifiers.combine_or_add_modifiers(
        Modifier {
            effect: ModifierEffect::ForceDraw {
                rule,
                scope: DrawScope::AllDraws,
            },
            expiry: ModifierExpiry::OnCardDraw { count: 1 },
            source: vec![ctx.player_id],
        },
        ctx.stack_mode,
    );
    Ok(())
}

fn add_invalidate_hand(ctx: UseContext<'_>, hand_type: HandType) -> Result<(), GameError> {
    ctx.game.special.modifiers.combine_or_add_modifiers(
        Modifier {
            effect: ModifierEffect::InvalidateHand(hand_type),
            expiry: ModifierExpiry::OnHandEnd { count: 1 },
            source: vec![ctx.player_id],
        },
        ctx.stack_mode,
    );
    Ok(())
}

fn clear_hole_card_reveals(game: &mut Game, player_id: usize, card_index: usize) {
    if let Ok(player_idx) = game.get_player_index(player_id) {
        if let Some(vis) = game.players[player_idx].card_visibility.get_mut(card_index) {
            vis.clear();
        }
    }
    game.special.modifiers.active.retain(|m| {
        !matches!(
            m.effect,
            ModifierEffect::RevealCard { target_id, card_index: idx, .. }
            if target_id == player_id && idx == card_index
        )
    });
}

fn pick_weighted_index<T>(items: &[(T, f64)], rng: &mut impl RngExt) -> Option<usize> {
    let total_weight: f64 = items.iter().map(|(_, w)| *w).filter(|w| *w > 0.0).sum();
    if total_weight <= 0.0 {
        return None;
    }

    let mut roll = rng.random_range(0.0..total_weight);
    let mut last_positive = None;
    for (idx, (_, weight)) in items.iter().enumerate() {
        if *weight <= 0.0 {
            continue;
        }
        last_positive = Some(idx);
        if roll < *weight {
            return Some(idx);
        }
        roll -= *weight;
    }
    last_positive
}

#[derive(Debug, Clone)]
pub(crate) struct WinnerDraft {
    pub(crate) player_id: usize,
    pub(crate) options: Vec<SpecialCard>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SpecialCard {
    // unique special cards - have distinct effects
    ReplaceCardSelf, // replaces own card with index with one from deck
    DrawCardSelf, // add new own card, but card is visible to all players
    ReplaceCardCommunity, // replaces last drawn community card with new card
    RemoveCardCommunity, // remove last drawn community card without replacement
    DrawCardCommunity, // add new community card
    RevealOpponentCard, // reveals random card from selected opponent
    WithdrawBet, // fold, taking bet back
    AnteUp, // increases blind size for the remainder of the game
    PotMult, // multiplies the current size of the pot
    RaiseBlock, // prevents raising until the next user turn
    SpecialBlock, // prevents special cards from being played until next user turn
    ChipBoost, // gain some chips
    ChipGamble, // chance to gain a significant number of chips or lose all chips
    Blackjack, // changes game eval to blackjack for 1 round - closest hand cards to 21 wins
    // forces the next drawn community or owner hole card to have one of the following qualities
    DrawHeart,
    DrawSpade,
    DrawDiamond,
    DrawClub,
    DrawFace, // king/queen/jack
    DrawHigh, // 10/j/q/k/a
    DrawLow, // 2/3/4/5
    DrawHigherThanLast,
    DrawLowerThanLast,
    DrawSameSuitAsLast,
    // invalidates any of the following hand types for the duration of the round:
    // temporarily removes them from handfns in gamecontext
    InvalidateFlush,
    InvalidateStraight,
    InvalidateThreeOfAKind,
    InvalidateTwoPair,
    InvalidateFullHouse,
    Special, // grants a few special cards to the user. not obtainable through normal draws
    Discard, // prevents any more community cards from being drawn this round
    HandSwap, // swap hands with a given opponent. fails (but still consumes the card) if their hand is better
    Joker, // convert selected community card into a joker
    SpecialSpread, // give 1 special card to everyone, and 3 to the user (bypassing special limits)
    RankBoost, // increments selected hole card rank by 1. wraps aces
    SuitMatch, // sets selected hole card suit to match another random card in hand
    RedrawHand, // discards and redraws all hole cards
    LossInsurance, // refunds 40% of lost chips this hand if used before round loss
    BetRebate, // tracks player expenditure for 3 hands and returns 20%
    ChipStipend, // pays 1x effective blind at the end of each hand for 4 hands
    Draw, // deals 2 revealed hole cards to user, 1 revealed to opponents, 1 community card
}
impl fmt::Display for SpecialCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.def().id)
    }
}

pub(crate) const ALL_SPECIAL_CARDS: [SpecialCard; 41] = [
    SpecialCard::ReplaceCardSelf,
    SpecialCard::DrawCardSelf,
    SpecialCard::ReplaceCardCommunity,
    SpecialCard::RemoveCardCommunity,
    SpecialCard::DrawCardCommunity,
    SpecialCard::RevealOpponentCard,
    SpecialCard::WithdrawBet,
    SpecialCard::AnteUp,
    SpecialCard::PotMult,
    SpecialCard::RaiseBlock,
    SpecialCard::SpecialBlock,
    SpecialCard::ChipBoost,
    SpecialCard::ChipGamble,
    SpecialCard::Blackjack,
    SpecialCard::DrawHeart,
    SpecialCard::DrawSpade,
    SpecialCard::DrawDiamond,
    SpecialCard::DrawClub,
    SpecialCard::DrawFace,
    SpecialCard::DrawHigh,
    SpecialCard::DrawLow,
    SpecialCard::DrawHigherThanLast,
    SpecialCard::DrawLowerThanLast,
    SpecialCard::DrawSameSuitAsLast,
    SpecialCard::InvalidateFlush,
    SpecialCard::InvalidateStraight,
    SpecialCard::InvalidateThreeOfAKind,
    SpecialCard::InvalidateTwoPair,
    SpecialCard::InvalidateFullHouse,
    SpecialCard::Special,
    SpecialCard::Discard,
    SpecialCard::HandSwap,
    SpecialCard::Joker,
    SpecialCard::SpecialSpread,
    SpecialCard::RankBoost,
    SpecialCard::SuitMatch,
    SpecialCard::RedrawHand,
    SpecialCard::LossInsurance,
    SpecialCard::BetRebate,
    SpecialCard::ChipStipend,
    SpecialCard::Draw,
];

impl SpecialCard {
    pub(crate) fn def(self) -> CardDef {
        match self {
            SpecialCard::ReplaceCardSelf => CardDef {
                id: "replace_card_self",
                roles: vec![CardRole::Hand],
                target: TargetKind::SelfCard,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 4.0 * l),
                    (CardPool::WinnerDraft, |l| 4.0 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    let reference_card = ctx.game.players[ctx.player_idx]
                        .cards
                        .get(ctx.card_index)
                        .copied()
                        .or_else(|| ctx.game.community.last().copied());
                    let card = draw_deck_card(
                        ctx.game,
                        DrawTarget::Hole {
                            player_id: ctx.player_id,
                        },
                        reference_card,
                        ctx.events,
                    );
                    ctx.game.replace_hole_card(ctx.player_id, ctx.card_index, card, ctx.events)?;
                    clear_hole_card_reveals(ctx.game, ctx.player_id, ctx.card_index);
                    Ok(())
                },
            },
            SpecialCard::DrawCardSelf => CardDef {
                id: "draw_card_self",
                roles: vec![CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 1.5 * l),
                    (CardPool::WinnerDraft, |l| 1.5 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    ctx.game.deal_hole_card(ctx.player_id, ctx.events);
                    let drawn_index = ctx.game.players[ctx.player_idx].cards.len() - 1;
                    let all_player_ids = ctx.game.players.iter().map(|p| p.id).collect::<Vec<usize>>();
                    ctx.game.players[ctx.player_idx].card_visibility[drawn_index] = all_player_ids;
                    Ok(())
                },
            },
            SpecialCard::ReplaceCardCommunity => CardDef {
                id: "replace_card_community",
                roles: vec![CardRole::Board],
                target: TargetKind::CommunityCard,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |_| 10.0),
                    (CardPool::WinnerDraft, |_| 10.0),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    let reference_card = ctx.game.community.last().copied();
                    let card = draw_deck_card(ctx.game, DrawTarget::Community, reference_card, ctx.events);
                    ctx.game.replace_community_card(ctx.card_index, card, ctx.events)
                },
            },
            SpecialCard::RemoveCardCommunity => CardDef {
                id: "remove_card_community",
                roles: vec![CardRole::Board],
                target: TargetKind::CommunityCard,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 1.0 * l),
                    (CardPool::WinnerDraft, |l| 1.0 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| ctx.game.remove_community_card(ctx.card_index, ctx.events),
            },
            SpecialCard::DrawCardCommunity => CardDef {
                id: "draw_card_community",
                roles: vec![CardRole::Board],
                target: TargetKind::None,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 1.0 * l),
                    (CardPool::WinnerDraft, |l| 1.0 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    ctx.game.deal_community_cards(1, ctx.events);
                    Ok(())
                },
            },
            SpecialCard::RevealOpponentCard => CardDef {
                id: "reveal_opponent_card",
                roles: vec![CardRole::Hand],
                target: TargetKind::OpponentCard,
                stack_mode: StackMode::Separate,
                pools: vec![
                    (CardPool::Standard, |l| 4.0 * l),
                    (CardPool::WinnerDraft, |l| 4.0 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    let t_idx = ctx.game.get_player_index(ctx.target_id).unwrap_or(0);
                    let t_p = &mut ctx.game.players[t_idx];
                    if t_p.card_visibility[ctx.card_index].contains(&ctx.player_id) {
                        return Err(GameError::CardAlreadyVisible);
                    }
                    t_p.card_visibility[ctx.card_index].push(ctx.player_id);
                    ctx.game.special.modifiers.combine_or_add_modifiers(
                        Modifier {
                            effect: ModifierEffect::RevealCard {
                                target_id: ctx.target_id,
                                card_index: ctx.card_index,
                                viewer_id: ctx.player_id,
                            },
                            expiry: ModifierExpiry::OnHandEnd { count: 1 },
                            source: vec![ctx.player_id],
                        },
                        ctx.stack_mode,
                    );
                    ctx.events.push(MoveEvent {
                        actor_id: Some(ctx.player_id),
                        action: MoveAction::RevealCard {
                            target_id: ctx.target_id,
                            card_index: ctx.card_index,
                        },
                        private: true,
                    });
                    Ok(())
                },
            },
            SpecialCard::WithdrawBet => CardDef {
                id: "withdraw_bet",
                roles: vec![CardRole::Economy],
                target: TargetKind::None,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 0.9 * l.powi(2)),
                    (CardPool::WinnerDraft, |l| 0.9 * l.powi(2)),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    let refund = withdraw_bet(ctx.player_id, ctx.player_idx, ctx.game, ctx.events);
                    ctx.game.players[ctx.player_idx].chips += refund;
                    ctx.game.force_fold_player(ctx.player_idx, ctx.events);
                    Ok(())
                },
            },
            SpecialCard::AnteUp => CardDef {
                id: "ante_up",
                roles: vec![CardRole::Economy],
                target: TargetKind::None,
                stack_mode: StackMode::Effect,
                pools: vec![
                    (CardPool::Standard, |l| 1.0 * l),
                    (CardPool::WinnerDraft, |l| 1.0 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    let ante_mult = 2.0;
                    ctx.game.special.modifiers.combine_or_add_modifiers(
                        Modifier {
                            effect: ModifierEffect::AnteMultiplier { multiplier: ante_mult },
                            expiry: ModifierExpiry::OnGameEnd,
                            source: vec![ctx.player_id],
                        },
                        ctx.stack_mode,
                    );
                    ctx.game.special.modifiers.recompute_vars();
                    give_special_card(SpecialCard::ChipBoost, 1, true, ctx.player_id, ctx.game, ctx.events)
                },
            },
            SpecialCard::PotMult => CardDef {
                id: "pot_mult",
                roles: vec![CardRole::Economy],
                target: TargetKind::None,
                stack_mode: StackMode::Effect,
                pools: vec![
                    (CardPool::Standard, |l| 1.5 * l.powi(2)),
                    (CardPool::WinnerDraft, |l| 1.5 * l.powi(2)),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    let pot_mult = 1.5;
                    ctx.game.special.modifiers.combine_or_add_modifiers(
                        Modifier {
                            effect: ModifierEffect::PotMultiplier { multiplier: pot_mult },
                            expiry: ModifierExpiry::OnHandEnd { count: 1 },
                            source: vec![ctx.player_id],
                        },
                        ctx.stack_mode,
                    );
                    ctx.game.special.modifiers.recompute_vars();
                    Ok(())
                },
            },
            SpecialCard::RaiseBlock => CardDef {
                id: "raise_block",
                roles: vec![CardRole::Chaos],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 10.0),
                    (CardPool::WinnerDraft, |_| 10.0),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    ctx.game.special.modifiers.combine_or_add_modifiers(
                        Modifier {
                            effect: ModifierEffect::RaisesBlocked,
                            expiry: ModifierExpiry::OnPlayerTurn {
                                count: 1,
                                player_id: ctx.player_id,
                            },
                            source: vec![ctx.player_id],
                        },
                        ctx.stack_mode,
                    );
                    Ok(())
                },
            },
            SpecialCard::SpecialBlock => CardDef {
                id: "special_block",
                roles: vec![CardRole::Chaos],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 10.0),
                    (CardPool::WinnerDraft, |_| 10.0),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    ctx.game.special.modifiers.combine_or_add_modifiers(
                        Modifier {
                            effect: ModifierEffect::SpecialsBlocked,
                            expiry: ModifierExpiry::OnPlayerTurn {
                                count: 1,
                                player_id: ctx.player_id,
                            },
                            source: vec![ctx.player_id],
                        },
                        ctx.stack_mode,
                    );
                    Ok(())
                },
            },
            SpecialCard::ChipBoost => CardDef {
                id: "chip_boost",
                roles: vec![CardRole::Economy],
                target: TargetKind::None,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 6.0 * l),
                    (CardPool::WinnerDraft, |l| 6.0 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    ctx.game.players[ctx.player_idx].chips +=
                        (ctx.game.ctx.blind_size as f64 * ctx.game.special.ante_multiplier() * 2.5) as i32;
                    Ok(())
                },
            },
            SpecialCard::ChipGamble => CardDef {
                id: "chip_gamble",
                roles: vec![CardRole::Economy],
                target: TargetKind::None,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 1.1 * l.powi(2)),
                    (CardPool::WinnerDraft, |l| 1.1 * l.powi(2)),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    if ctx.game.ctx.rng.random_bool(ctx.game.special.gamble_success_chance()) {
                        ctx.game.players[ctx.player_idx].chips +=
                            (ctx.game.ctx.blind_size as f64 * ctx.game.special.ante_multiplier() * 7.5) as i32;
                        ctx.game.special.modifiers.vars.gamble_success_chance *= 0.9;
                    } else {
                        withdraw_bet(ctx.player_id, ctx.player_idx, ctx.game, ctx.events);
                        let player = &mut ctx.game.players[ctx.player_idx];
                        player.chips = (player.chips as f64 * 0.5).round() as i32;
                        ctx.game.force_fold_player(ctx.player_idx, ctx.events);
                    }
                    Ok(())
                },
            },
            SpecialCard::Blackjack => CardDef {
                id: "blackjack",
                roles: vec![CardRole::Chaos],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 1.0),
                    (CardPool::WinnerDraft, |l| 0.25 * l.powi(2)),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    // toggles hand eval function to blackjack
                    ctx.game.special.modifiers.combine_or_add_modifiers(
                        Modifier {
                            effect: ModifierEffect::BlackjackScoring,
                            expiry: ModifierExpiry::OnHandEnd { count: 1 },
                            source: vec![ctx.player_id],
                        },
                        ctx.stack_mode,
                    );
                    // gives all players a draw card and allows them to take another turn
                    give_special_card_to_all(SpecialCard::DrawCardSelf, 1, true, ctx.game, ctx.events)?;
                    ctx.game.players.iter_mut().for_each(|p| p.turn_ended = false);
                    Ok(())
                },
            },
            SpecialCard::DrawHeart => CardDef {
                id: "draw_heart",
                roles: vec![CardRole::Board, CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 10.0),
                    (CardPool::WinnerDraft, |_| 10.0),
                ],
                on_use: |ctx| add_force_draw(ctx, DrawRule::Heart),
            },
            SpecialCard::DrawSpade => CardDef {
                id: "draw_spade",
                roles: vec![CardRole::Board, CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 10.0),
                    (CardPool::WinnerDraft, |_| 10.0),
                ],
                on_use: |ctx| add_force_draw(ctx, DrawRule::Spade),
            },
            SpecialCard::DrawDiamond => CardDef {
                id: "draw_diamond",
                roles: vec![CardRole::Board, CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 10.0),
                    (CardPool::WinnerDraft, |_| 10.0),
                ],
                on_use: |ctx| add_force_draw(ctx, DrawRule::Diamond),
            },
            SpecialCard::DrawClub => CardDef {
                id: "draw_club",
                roles: vec![CardRole::Board, CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 10.0),
                    (CardPool::WinnerDraft, |_| 10.0),
                ],
                on_use: |ctx| add_force_draw(ctx, DrawRule::Club),
            },
            SpecialCard::DrawFace => CardDef {
                id: "draw_face",
                roles: vec![CardRole::Board, CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 7.5),
                    (CardPool::WinnerDraft, |_| 7.5),
                ],
                on_use: |ctx| add_force_draw(ctx, DrawRule::Face),
            },
            SpecialCard::DrawHigh => CardDef {
                id: "draw_high",
                roles: vec![CardRole::Board, CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 7.5),
                    (CardPool::WinnerDraft, |_| 7.5),
                ],
                on_use: |ctx| add_force_draw(ctx, DrawRule::High),
            },
            SpecialCard::DrawLow => CardDef {
                id: "draw_low",
                roles: vec![CardRole::Board, CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 7.5),
                    (CardPool::WinnerDraft, |_| 7.5),
                ],
                on_use: |ctx| add_force_draw(ctx, DrawRule::Low),
            },
            SpecialCard::DrawHigherThanLast => CardDef {
                id: "draw_higher_than_last",
                roles: vec![CardRole::Board, CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |l| 2.5 * l),
                    (CardPool::WinnerDraft, |l| 2.5 * l),
                ],
                on_use: |ctx| add_force_draw(ctx, DrawRule::HigherThanLast),
            },
            SpecialCard::DrawLowerThanLast => CardDef {
                id: "draw_lower_than_last",
                roles: vec![CardRole::Board, CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |l| 2.5 * l),
                    (CardPool::WinnerDraft, |l| 2.5 * l),
                ],
                on_use: |ctx| add_force_draw(ctx, DrawRule::LowerThanLast),
            },
            SpecialCard::DrawSameSuitAsLast => CardDef {
                id: "draw_same_suit_as_last",
                roles: vec![CardRole::Board, CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |l| 2.5 * l),
                    (CardPool::WinnerDraft, |l| 2.5 * l),
                ],
                on_use: |ctx| add_force_draw(ctx, DrawRule::SameSuitAsLast),
            },
            SpecialCard::InvalidateFlush => CardDef {
                id: "invalidate_flush",
                roles: vec![CardRole::Chaos],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 5.0),
                    (CardPool::WinnerDraft, |_| 5.0),
                ],
                on_use: |ctx| add_invalidate_hand(ctx, HandType::Flush),
            },
            SpecialCard::InvalidateStraight => CardDef {
                id: "invalidate_straight",
                roles: vec![CardRole::Chaos],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 5.0),
                    (CardPool::WinnerDraft, |_| 5.0),
                ],
                on_use: |ctx| add_invalidate_hand(ctx, HandType::Straight),
            },
            SpecialCard::InvalidateThreeOfAKind => CardDef {
                id: "invalidate_three_of_a_kind",
                roles: vec![CardRole::Chaos],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 5.0),
                    (CardPool::WinnerDraft, |_| 5.0),
                ],
                on_use: |ctx| add_invalidate_hand(ctx, HandType::ThreeOfAKind),
            },
            SpecialCard::InvalidateTwoPair => CardDef {
                id: "invalidate_two_pair",
                roles: vec![CardRole::Chaos],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 5.0),
                    (CardPool::WinnerDraft, |_| 5.0),
                ],
                on_use: |ctx| add_invalidate_hand(ctx, HandType::TwoPair),
            },
            SpecialCard::InvalidateFullHouse => CardDef {
                id: "invalidate_full_house",
                roles: vec![CardRole::Chaos],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |_| 5.0),
                    (CardPool::WinnerDraft, |_| 5.0),
                ],
                on_use: |ctx| add_invalidate_hand(ctx, HandType::FullHouse),
            },
            SpecialCard::Special => CardDef {
                id: "special",
                roles: vec![CardRole::Chaos],
                target: TargetKind::None,
                stack_mode: StackMode::None,
                pools: vec![],
                on_use: |ctx| {
                    let count = 5;
                    let pool: Vec<(SpecialCard, f64)> = ALL_SPECIAL_CARDS
                        .iter()
                        .copied()
                        .filter_map(|card| {
                            card.pool_weight(CardPool::SpecialBurst, 1.0)
                                .filter(|w| *w > 0.0)
                                .map(|w| (card, w))
                        })
                        .collect();

                    if !pool.is_empty() {
                        for _ in 0..count {
                            if let Some(idx) = pick_weighted_index(&pool, &mut ctx.game.ctx.rng) {
                                let next_card = pool[idx].0;
                                give_special_card(next_card, 1, true, ctx.player_id, ctx.game, ctx.events)?;
                            }
                        }
                    }
                    Ok(())
                },
            },
            SpecialCard::Discard => CardDef {
                id: "discard",
                roles: vec![CardRole::Chaos, CardRole::Board],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![],
                on_use: |ctx| {
                    ctx.game.special.modifiers.combine_or_add_modifiers(
                        Modifier {
                            effect: ModifierEffect::ForceDraw {
                                rule: DrawRule::Nothing,
                                scope: DrawScope::CommunityOnly,
                            },
                            expiry: ModifierExpiry::OnHandEnd { count: 1 },
                            source: vec![ctx.player_id],
                        },
                        ctx.stack_mode,
                    );
                    Ok(())
                },
            },
            SpecialCard::HandSwap => CardDef {
                id: "hand_swap",
                roles: vec![CardRole::Hand],
                target: TargetKind::OpponentPlayer,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 1.2 * l),
                    (CardPool::WinnerDraft, |l| 1.2 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    let t_idx = ctx.game.get_player_index(ctx.target_id).unwrap_or(0);
                    let target = &ctx.game.players[t_idx];
                    let invalidated = ctx.game.special.invalidated_hands();
                    let target_cards: Vec<ExpandedCard> = [&ctx.game.community[..], &target.cards[..]].concat();
                    let target_hand = Hand::new(&target_cards, &ctx.game.ctx.hand_fns, &invalidated, ctx.game.ctx.hand_size);

                    let player = &ctx.game.players[ctx.player_idx];
                    let player_cards: Vec<ExpandedCard> = [&ctx.game.community[..], &player.cards[..]].concat();
                    let player_hand = Hand::new(&player_cards, &ctx.game.ctx.hand_fns, &invalidated, ctx.game.ctx.hand_size);

                    if player_hand > target_hand {
                        let temp_cards = player.cards.clone();
                        let temp_vis = player.card_visibility.clone();
                        ctx.game.players[ctx.player_idx].cards = ctx.game.players[t_idx].cards.clone();
                        ctx.game.players[ctx.player_idx].card_visibility = ctx.game.players[t_idx].card_visibility.clone();
                        ctx.game.players[t_idx].cards = temp_cards;
                        ctx.game.players[t_idx].card_visibility = temp_vis;

                        for vis in &mut ctx.game.players[ctx.player_idx].card_visibility {
                            vis.retain(|&id| id != ctx.player_id);
                        }
                        for vis in &mut ctx.game.players[t_idx].card_visibility {
                            vis.retain(|&id| id != ctx.target_id);
                        }

                        for m in &mut ctx.game.special.modifiers.active {
                            if let ModifierEffect::RevealCard { target_id, .. } = &mut m.effect {
                                if *target_id == ctx.player_id {
                                    *target_id = ctx.target_id;
                                } else if *target_id == ctx.target_id {
                                    *target_id = ctx.player_id;
                                }
                            }
                        }
                        ctx.game.special.modifiers.active.retain(|m| {
                            !matches!(
                                m.effect,
                                ModifierEffect::RevealCard { target_id, viewer_id, .. } if target_id == viewer_id
                            )
                        });

                        Ok(())
                    } else {
                        ctx.game.players[ctx.player_idx].special_cards.remove(ctx.self_idx);
                        Err(GameError::HandSwapFailed)
                    }
                },
            },
            SpecialCard::Joker => CardDef {
                id: "joker",
                roles: vec![CardRole::Board],
                target: TargetKind::CommunityCard,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 1.2 * l),
                    (CardPool::WinnerDraft, |l| 1.2 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    ctx.game.replace_community_card(ctx.card_index, ExpandedCard::get_joker(), ctx.events)
                },
            },
            SpecialCard::SpecialSpread => CardDef {
                id: "special_spread",
                roles: vec![CardRole::Chaos],
                target: TargetKind::None,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::WinnerDraft, |l| 0.1 * l.powi(2))
                ],
                on_use: |ctx| {
                    ctx.game.deal_special_cards(1, ctx.events);
                    Ok(())
                },
            },
            SpecialCard::RankBoost => CardDef {
                id: "rank_boost",
                roles: vec![CardRole::Hand],
                target: TargetKind::SelfCard,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 2.5 * l),
                    (CardPool::WinnerDraft, |l| 2.5 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    let target_card = ctx.game.players[ctx.player_idx]
                        .cards
                        .get_mut(ctx.card_index)
                        .ok_or(GameError::CardIndexInvalid)?;
                    match target_card {
                        ExpandedCard::PlayingCard(card) => {
                            card.rank = match card.rank {
                                Rank::None => Rank::Ace,
                                Rank::Ace => Rank::Two,
                                rank => Rank::from((rank as u8) + 1),
                            };
                            Ok(())
                        }
                        ExpandedCard::Unmarked => {
                            *target_card = ExpandedCard::PlayingCard(Card {
                                rank: Rank::Ace,
                                suit: Suit::Suitless,
                            });
                            Ok(())
                        }
                        ExpandedCard::Joker(_) => Err(GameError::InvalidAction),
                    }
                },
            },
            SpecialCard::SuitMatch => CardDef {
                id: "suit_match",
                roles: vec![CardRole::Hand],
                target: TargetKind::SelfCard,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 2.0 * l),
                    (CardPool::WinnerDraft, |l| 2.0 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    let cards = &ctx.game.players[ctx.player_idx].cards;
                    if ctx.card_index >= cards.len() {
                        return Err(GameError::CardIndexInvalid);
                    }
                    if !matches!(cards[ctx.card_index], ExpandedCard::PlayingCard(_)) {
                        return Err(GameError::InvalidAction);
                    }
                    let other_suits: Vec<Suit> = cards
                        .iter()
                        .enumerate()
                        .filter_map(|(i, c)| {
                            if i != ctx.card_index {
                                if let ExpandedCard::PlayingCard(card) = c {
                                    if card.suit != Suit::Suitless {
                                        return Some(card.suit);
                                    }
                                }
                            }
                            None
                        })
                        .collect();
                    if other_suits.is_empty() {
                        return Err(GameError::InvalidAction);
                    }
                    let chosen_suit = other_suits[ctx.game.ctx.rng.random_range(0..other_suits.len())];
                    if let ExpandedCard::PlayingCard(card) = &mut ctx.game.players[ctx.player_idx].cards[ctx.card_index]
                    {
                        card.suit = chosen_suit;
                    }
                    Ok(())
                },
            },
            SpecialCard::RedrawHand => CardDef {
                id: "redraw_hand",
                roles: vec![CardRole::Hand],
                target: TargetKind::None,
                stack_mode: StackMode::None,
                pools: vec![
                    (CardPool::Standard, |l| 1.5 * l),
                    (CardPool::WinnerDraft, |l| 1.5 * l),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    let card_count = ctx.game.players[ctx.player_idx].cards.len();
                    for card_index in 0..card_count {
                        let reference_card = ctx.game.players[ctx.player_idx]
                            .cards
                            .get(card_index)
                            .copied()
                            .or_else(|| ctx.game.community.last().copied());
                        let new_card = draw_deck_card(
                            ctx.game,
                            DrawTarget::Hole {
                                player_id: ctx.player_id,
                            },
                            reference_card,
                            ctx.events,
                        );
                        ctx.game.replace_hole_card(ctx.player_id, card_index, new_card, ctx.events)?;
                        clear_hole_card_reveals(ctx.game, ctx.player_id, card_index);
                    }
                    Ok(())
                },
            },
            SpecialCard::LossInsurance => CardDef {
                id: "loss_insurance",
                roles: vec![CardRole::Economy],
                target: TargetKind::None,
                stack_mode: StackMode::Effect,
                pools: vec![
                    (CardPool::Standard, |l| (1.5 * l - 3.0).max(0f64)),
                    (CardPool::WinnerDraft, |l| (1.5 * l - 3.0).max(0f64)),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    ctx.game.special.modifiers.combine_or_add_modifiers(
                        Modifier {
                            effect: ModifierEffect::LossInsurance {
                                player_id: ctx.player_id,
                                rebate_rate: 0.40,
                            },
                            expiry: ModifierExpiry::OnHandEnd { count: 1 },
                            source: vec![ctx.player_id],
                        },
                        ctx.stack_mode,
                    );
                    Ok(())
                },
            },
            SpecialCard::BetRebate => CardDef {
                id: "bet_rebate",
                roles: vec![CardRole::Economy],
                target: TargetKind::None,
                stack_mode: StackMode::Separate,
                pools: vec![
                    (CardPool::Standard, |l| (1.5 * l - 3.0).max(0f64)),
                    (CardPool::WinnerDraft, |l| (1.5 * l - 3.0).max(0f64)),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    ctx.game.special.modifiers.combine_or_add_modifiers(
                        Modifier {
                            effect: ModifierEffect::BetRebate {
                                player_id: ctx.player_id,
                                rebate_rate: 0.20,
                                accumulated_bet: 0,
                            },
                            expiry: ModifierExpiry::OnHandEnd { count: 3 },
                            source: vec![ctx.player_id],
                        },
                        ctx.stack_mode,
                    );
                    Ok(())
                },
            },
            SpecialCard::ChipStipend => CardDef {
                id: "chip_stipend",
                roles: vec![CardRole::Economy],
                target: TargetKind::None,
                stack_mode: StackMode::Duration,
                pools: vec![
                    (CardPool::Standard, |l| (1.5 * l - 3.0).max(0f64)),
                    (CardPool::WinnerDraft, |l| (1.5 * l - 3.0).max(0f64)),
                    (CardPool::SpecialBurst, |_| 1.0),
                ],
                on_use: |ctx| {
                    ctx.game.special.modifiers.combine_or_add_modifiers(
                        Modifier {
                            effect: ModifierEffect::ChipStipend {
                                player_id: ctx.player_id,
                            },
                            expiry: ModifierExpiry::OnHandEnd { count: 4 },
                            source: vec![ctx.player_id],
                        },
                        ctx.stack_mode,
                    );
                    Ok(())
                },
            },
            SpecialCard::Draw => CardDef {
                id: "draw",
                roles: vec![CardRole::Chaos, CardRole::Hand, CardRole::Board],
                target: TargetKind::None,
                stack_mode: StackMode::None,
                pools: vec![],
                on_use: |ctx| {
                    let all_player_ids: Vec<usize> = ctx.game.players.iter().map(|p| p.id).collect();
                    for _ in 0..2 {
                        ctx.game.deal_hole_card(ctx.player_id, ctx.events);
                        let p_idx = ctx.game.get_player_index(ctx.player_id)?;
                        let drawn_index = ctx.game.players[p_idx].cards.len() - 1;
                        ctx.game.players[p_idx].card_visibility[drawn_index] = all_player_ids.clone();
                    }

                    let other_player_ids: Vec<usize> = ctx
                        .game
                        .players
                        .iter()
                        .filter(|p| p.id != ctx.player_id && !p.folded)
                        .map(|p| p.id)
                        .collect();
                    for target_id in other_player_ids {
                        ctx.game.deal_hole_card(target_id, ctx.events);
                        let t_idx = ctx.game.get_player_index(target_id)?;
                        let drawn_index = ctx.game.players[t_idx].cards.len() - 1;
                        ctx.game.players[t_idx].card_visibility[drawn_index] = all_player_ids.clone();
                    }

                    ctx.game.deal_community_cards(1, ctx.events);
                    Ok(())
                },
            },
        }
    }

    #[allow(dead_code)]
    pub(crate) fn has_role(self, role: CardRole) -> bool {
        self.def().roles.contains(&role)
    }

    #[allow(dead_code)]
    pub(crate) fn in_pool(self, pool: CardPool) -> bool {
        self.def().pools.iter().any(|(p, _)| *p == pool)
    }

    pub(crate) fn pool_weight(self, pool: CardPool, luck: f64) -> Option<f64> {
        self.def()
            .pools
            .into_iter()
            .find(|(p, _)| *p == pool)
            .map(|(_, weight_fn)| weight_fn(luck))
    }

    pub(crate) fn use_card(&self, player_id: usize, target_id: Option<usize>, card_index: Option<usize>, game: &mut Game, events: &mut Vec<MoveEvent>) -> Result<(), GameError> {
        let t_id = target_id.unwrap_or(usize::MAX);
        let c_index = card_index.unwrap_or(usize::MAX);

        let player_idx = game
            .get_player_index(player_id)
            .map_err(|_| GameError::InvalidTargetPlayer)?;

        fn check_player_owns_card(player_idx: usize, c_index: usize, game: &Game) -> Result<(), GameError> {
            if game.players[player_idx].cards.get(c_index).is_none() {
                Err(GameError::CardIndexInvalid)
            } else {
                Ok(())
            }
        }

        let self_idx = game.players[player_idx]
            .special_cards
            .iter()
            .position(|s| s == self)
            .ok_or(GameError::CardIndexInvalid)?;

        let def = self.def();

        match def.target {
            TargetKind::None => {}
            TargetKind::SelfCard => {
                check_player_owns_card(player_idx, c_index, game)?;
            }
            TargetKind::CommunityCard => {
                if c_index >= game.community.len() {
                    return Err(GameError::CardIndexInvalid);
                }
            }
            TargetKind::OpponentPlayer => {
                if t_id == usize::MAX || t_id == player_id {
                    return Err(GameError::InvalidTargetPlayer);
                }
                let t_idx = game.get_player_index(t_id).map_err(|_| GameError::InvalidTargetPlayer)?;
                if game.players[t_idx].folded {
                    return Err(GameError::InvalidTargetPlayer);
                }
            }
            TargetKind::OpponentCard => {
                if t_id == usize::MAX || t_id == player_id {
                    return Err(GameError::InvalidTargetPlayer);
                }
                let t_idx = game.get_player_index(t_id).map_err(|_| GameError::InvalidTargetPlayer)?;
                if game.players[t_idx].folded {
                    return Err(GameError::InvalidTargetPlayer);
                }
                check_player_owns_card(t_idx, c_index, game)?;
            }
        }

        (def.on_use)(UseContext {
            player_id,
            player_idx,
            self_idx,
            target_id: t_id,
            card_index: c_index,
            stack_mode: def.stack_mode,
            game,
            events,
        })?;
        
        game.players[player_idx].special_cards.remove(self_idx);

        on_event(
            GameEvent::SpecialPlayed {
                player_id,
                card: *self,
            },
            game,
            events,
        );

        Ok(())
    }

    /**
     * Plays card discard effects
     */
    pub(crate) fn discard_card(&self, player_id: usize, game: &mut Game, events: &mut Vec<MoveEvent>) -> Result<(), GameError> {
        on_event(
            GameEvent::SpecialDiscarded {
                player_id,
                card: *self,
            },
            game,
            events,
        );

        Ok(())
    }

    pub(crate) fn from_str(card_name: &str) -> Option<Self> {
        ALL_SPECIAL_CARDS
            .iter()
            .copied()
            .find(|card| card.def().id == card_name)
    }

    pub(crate) fn draw_special_cards(count: usize, player_id: usize, game: &mut Game) -> Vec<SpecialCard> {
        let mut luck: f64 = 1.0;
        // increase luck as game goes on - better cards over time
        luck *= ((game.games_played as f64).sqrt() / 3.0) + 1.0;
        // increase luck for active player with least chips (breaking ties randomly)
        let min_wealth = game
            .players
            .iter()
            .filter(|p| !p.folded)
            .map(|p| p.chips + p.total_bet)
            .min();
        if let Some(min_w) = min_wealth {
            let tied_ids: Vec<usize> = game
                .players
                .iter()
                .filter(|p| !p.folded && p.chips + p.total_bet == min_w)
                .map(|p| p.id)
                .collect();
            if !tied_ids.is_empty() {
                let picked_id = tied_ids[game.ctx.rng.random_range(0..tied_ids.len())];
                if picked_id == player_id {
                    luck *= 1.4;
                }
            }
        }

        let active_players = game.players.iter().filter(|p| !p.folded).count().max(1) as f64;
        let mut cards_drawn: Vec<SpecialCard> = Vec::new();
        for _ in 0..count {
            if game.special.rounds_since_ante_up >= ANTE_UP_PITY_MAX_ROUNDS {
                on_event(
                    GameEvent::SpecialReceived {
                        player_id,
                        card: SpecialCard::AnteUp,
                    },
                    game,
                    &mut Vec::new(),
                );
                cards_drawn.push(SpecialCard::AnteUp);
                continue;
            }

            let pity_bonus = if game.special.rounds_since_ante_up > 0 {
                3.0 * (game.special.rounds_since_ante_up as f64).powi(2) / active_players
            } else {
                0.0
            };

            let mut weighted_cards: Vec<(SpecialCard, f64)> = Vec::new();
            for card in ALL_SPECIAL_CARDS {
                let Some(mut weight) = card.pool_weight(CardPool::Standard, luck) else {
                    continue;
                };
                if card == SpecialCard::AnteUp {
                    weight += pity_bonus;
                }
                if weight > 0.0 {
                    weighted_cards.push((card, weight));
                }
            }

            if let Some(card_idx) = pick_weighted_index(&weighted_cards, &mut game.ctx.rng) {
                let drawn_card = weighted_cards[card_idx].0;
                on_event(
                    GameEvent::SpecialReceived {
                        player_id,
                        card: drawn_card,
                    },
                    game,
                    &mut Vec::new(),
                );
                cards_drawn.push(drawn_card);
            }
        }

        cards_drawn
    }

    /**
     * Draws count distinct special card options for a winner draft
     * Card quality is scaled by pot ratio (the relative amount of value in the current pot)
     */
    pub(crate) fn draw_winner_draft_options(count: usize, pot_ratio: f64, game: &mut Game) -> Vec<SpecialCard> {
        let linear_t = ((pot_ratio - 0.02) / 0.45).clamp(0.0, 1.0);
        let t = linear_t * linear_t;
        let draft_luck = 0.1 + 3.4 * t;

        let mut pool: Vec<(SpecialCard, f64)> = ALL_SPECIAL_CARDS
            .iter()
            .copied()
            .filter_map(|card| {
                card.pool_weight(CardPool::WinnerDraft, draft_luck)
                    .filter(|w| *w > 0.0)
                    .map(|w| (card, w))
            })
            .collect();

        let target_count = count.min(pool.len());
        let mut chosen = Vec::with_capacity(target_count);

        for _ in 0..target_count {
            let Some(picked_idx) = pick_weighted_index(&pool, &mut game.ctx.rng) else {
                break;
            };
            let (card, _) = pool.swap_remove(picked_idx);
            chosen.push(card);
        }

        chosen
    }
}

pub(crate) const ANTE_UP_PITY_MAX_ROUNDS: u32 = 12;

/**
 * Evaluation function for blackjack modifier
 */
pub(crate) fn evaluate_blackjack_hand(player_cards: &[ExpandedCard]) -> i32 {
    let mut score = 0;
    let mut aces = 0;
    let mut jokers = 0;

    for card in player_cards {
        match card {
            ExpandedCard::PlayingCard(c) => match c.rank {
                Rank::Ace => { score += 11; aces += 1; },
                Rank::King | Rank::Queen | Rank::Jack => score += 10,
                r => score += r as u8 + 1,
            },
            ExpandedCard::Joker(_) => jokers += 1,
            ExpandedCard::Unmarked => {},
        }
    }

    while score > 21 && aces > 0 {
        score -= 10;
        aces -= 1;
    }

    // assuming a joker can become an unmarked card, jokers are worth 0-11
    while score < 21 && jokers > 0 {
        score += (21 - score).clamp(0, 11);
        jokers -= 1;
    }
    
    if score > 21 {
        -1
    } else { 
        score as i32
    }
}

pub(crate) fn withdraw_bet(player_id: usize, player_idx: usize, game: &mut Game, events: &mut Vec<MoveEvent>) -> i32 {
    let tracked_total = game.special.hand_bets.get(&player_id).copied().unwrap_or(0);
    let current_street_refund = game.players[player_idx].round_bet.min(tracked_total);
    game.round_pool -= current_street_refund;

    let mut remaining_from_pots = (tracked_total - current_street_refund).max(0);
    let mut refunded_from_pots = 0;

    for pot in game.pots.iter_mut().rev() {
        if pot.players.contains(&player_id) {
            if remaining_from_pots > 0 {
                let take = remaining_from_pots.min(pot.amount);
                pot.amount -= take;
                remaining_from_pots -= take;
                refunded_from_pots += take;
            }
            pot.players.retain(|&id| id != player_id);
        }
    }
    game.pots.retain(|pot| pot.amount > 0);

    let refund = current_street_refund + refunded_from_pots;

    let player = &mut game.players[player_idx];
    player.total_bet = (player.total_bet - refund).max(0);
    player.round_bet = 0;

    if refund > 0 {
        on_event(GameEvent::BetWithdrawn { player_id, amount: refund }, game, events);
    }

    refund
}


/**
 * Gives a specified special card count times to all remaining players, conditionally ignoring the card limit
 */
pub(crate) fn give_special_card_to_all(special_card: SpecialCard, count: usize, ignore_limit: bool, game: &mut Game, events: &mut Vec<MoveEvent>) -> Result<(), GameError> {
    for i in 0..game.players.len() {
        give_special_card(special_card, count, ignore_limit, game.players[i].id, game, events)?;
    }
    Ok(())
}

/**
 * Gives a specified special card count times to one player by ID, conditionally ignoring limits
 */
pub(crate) fn give_special_card(special_card: SpecialCard, count: usize, ignore_limit: bool, player_id: usize, game: &mut Game, events: &mut Vec<MoveEvent>) -> Result<(), GameError> {
    let p_id = game
        .get_player_index(player_id)
        .map_err(|_| GameError::InvalidTargetPlayer)?;

    // return without error if player is folded or special cards are disabled
    if game.players[p_id].folded || game.ctx.special_card_limit == 0 {
        return Ok(())
    }

    if !ignore_limit && game.players[p_id].special_cards.len() >= game.ctx.special_card_limit {
        return Err(GameError::SpecialCardsFull);
    }

    let mut given_cards = 0;
    for _ in 0..count {
        if !ignore_limit && game.players[p_id].special_cards.len() >= game.ctx.special_card_limit {
            break;
        } else {
            game.players[p_id].special_cards.push(special_card);
            given_cards += 1;
        }
    }

    if given_cards > 0 {
        on_event(
            GameEvent::SpecialReceived {
                player_id,
                card: special_card,
            },
            game,
            events,
        );

        events.push(MoveEvent { 
            actor_id: Some(player_id), 
            action: MoveAction::DealSpecial { count: given_cards }, 
            private: false 
        });
    }


    if !ignore_limit && given_cards < count {
        return Err(GameError::SpecialCardsFull);
    }

    Ok(())
}