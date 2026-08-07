/**
 * Handles special card related logic
 */

use std::fmt;
use rand::RngExt;
use serde::Serialize;

use crate::{ExpandedCard, Game, GameError, HandType, MoveAction, MoveEvent, Rank, Suit, poker::Hand};

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

    pub(crate) fn community_draw(&mut self) -> Vec<Modifier> {
        let mut expired = Vec::new();

        self.active.retain_mut(|m| {
            let retain = match m.expiry {
                ModifierExpiry::OnCommunityDraw { count } => {
                    match count {
                        2.. => { m.expiry = ModifierExpiry::OnCommunityDraw { count: count - 1 }; true }
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

    pub(crate) fn draw_rule(&self, game: &Game) -> impl Fn(&ExpandedCard) -> bool + 'static {
        let rules: Vec<Box<dyn Fn(&ExpandedCard) -> bool>> = self.active.iter().filter_map(|m| match m.effect {
            ModifierEffect::ForceCommunityDraw(rule) => Some(rule.match_fn(game)),
            _ => None,
        }).collect();

        move |c| rules.iter().all(|r| r(c))        
    }

    fn combine_or_add_modifiers(&mut self, modifier: Modifier) {
        for i in 0..self.active.len() {
            if self.active[i].effect != modifier.effect || !self.active[i].expiry.type_equal(&modifier.expiry) {
                continue;
            }

            self.active[i].expiry = match (self.active[i].expiry, modifier.expiry) {
                (ModifierExpiry::OnCommunityDraw { count: first_count }, 
                 ModifierExpiry::OnCommunityDraw { count: second_count }) 
                  => ModifierExpiry::OnCommunityDraw { count: first_count + second_count },

                (ModifierExpiry::OnPlayerTurn { count: first_count, player_id: _ }, 
                 ModifierExpiry::OnPlayerTurn { count: second_count, player_id: p_id }) 
                  => ModifierExpiry::OnPlayerTurn { count: first_count + second_count, player_id: p_id },

                (ModifierExpiry::OnRoundEnd { count: first_count }, 
                ModifierExpiry::OnRoundEnd { count: second_count }) 
                  => ModifierExpiry::OnRoundEnd { count: first_count + second_count },

                (ModifierExpiry::OnHandEnd { count: first_count }, 
                 ModifierExpiry::OnHandEnd { count: second_count }) 
                  => ModifierExpiry::OnHandEnd { count: first_count + second_count },

                _ => ModifierExpiry::OnGameEnd,
            };

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
    pub(crate) fn match_fn(&self, game: &Game) -> Box<dyn Fn(&ExpandedCard) -> bool + 'static> {
        match self {
            DrawRule::Heart => Box::new(|c| c.card().suit == Suit::Hearts || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::Spade => Box::new(|c| c.card().suit == Suit::Spades || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::Diamond => Box::new(|c| c.card().suit == Suit::Diamonds || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::Club => Box::new(|c| c.card().suit == Suit::Clubs || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::Face => Box::new(|c| matches!(c.card().rank, Rank::Jack | Rank::Queen | Rank::King) || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::High => Box::new(|c| c.card().rank >= Rank::Ten || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::Low => Box::new(|c| c.card().rank <= Rank::Five || matches!(c, ExpandedCard::Joker(_))),
            DrawRule::HigherThanLast => {
                let last_rank = game.community.last().map_or(Rank::None, |community| community.card().rank);
                Box::new(move |c| c.card().rank >= last_rank || matches!(c, ExpandedCard::Joker(_)))
            },
            DrawRule::LowerThanLast => {
                let last_rank = game.community.last().map_or(Rank::None, |community| community.card().rank);
                Box::new(move |c| c.card().rank <= last_rank || matches!(c, ExpandedCard::Joker(_)))
            },
            DrawRule::SameSuitAsLast => {
                let last_suit = game.community.last().map_or(Suit::Suitless, |community| community.card().suit);
                Box::new(move |c| c.card().suit == last_suit || matches!(c, ExpandedCard::Joker(_)))
            },
            DrawRule::Nothing => Box::new(|_| false),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub(crate) enum ModifierEffect {
    PotMultiplier { multiplier: f64 },
    AnteMultiplier { multiplier: f64 },
    RevealCard { target_id: usize, card_index: usize, viewer_id: usize },
    RaisesBlocked,
    SpecialsBlocked,
    BlackjackScoring,
    ForceCommunityDraw(DrawRule),
    InvalidateHand(HandType),
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
enum ModifierExpiry {
    OnCommunityDraw { count: usize },
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
                        if !vis.contains(&viewer_id) {
                            vis.push(viewer_id);
                        }
                    }
                }
            },
            _ => {},
        }
    }    
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
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
    // forces the next community card drawn to have one of the following qualities
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
}
impl fmt::Display for SpecialCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl SpecialCard {
    pub(crate) fn use_card(&self, player_id: usize, target_id: Option<usize>, card_index: Option<usize>, game: &mut Game, events: &mut Vec<MoveEvent>) -> Result<(), GameError> {
        let t_id = target_id.unwrap_or(usize::MAX);
        let c_index = card_index.unwrap_or(usize::MAX);

        let t_idx_res = game.get_player_index(t_id);
        // reject if invalid player id, invalid target id, or target is folded
        if player_id != usize::MAX && game.get_player_index(player_id).is_err()
            || t_id != usize::MAX && t_idx_res.is_err()
            || t_idx_res.is_ok_and(|idx| game.players[idx].folded)
        {
            return Err(GameError::InvalidTargetPlayer);
        }

        fn check_player_owns_card(player_id: usize, c_index: usize, game: &Game) -> Result<(), GameError> {
            if game.players[game.get_player_index(player_id).unwrap()].cards.get(c_index).is_none() {
                Err(GameError::CardIndexInvalid)
            } else {
                Ok(())
            }
        }

        let player_idx = game.get_player_index(player_id).unwrap_or(0);
        let self_idx = game.players[player_idx]
            .special_cards
            .iter()
            .position(|s| s == self)
            .ok_or(GameError::CardIndexInvalid)?;

        match self {
            SpecialCard::ReplaceCardSelf => {
                check_player_owns_card(player_id, c_index, game)?;
                let card = game.deck.pop().unwrap_or(ExpandedCard::Unmarked);
                // this should never fail, as the player has already been verified as owning the card
                game.replace_hole_card(player_id, c_index, card, events).map_err(|_| unreachable!());
            },
            SpecialCard::DrawCardSelf => {
                game.deal_hole_card(player_id, events);
                let drawn_index = game.players[player_idx].cards.len() - 1;
                let all_player_ids = game.players.iter().map(|p| p.id).collect::<Vec<usize>>();
                game.players[player_idx].card_visibility[drawn_index] = all_player_ids;
            },
            SpecialCard::ReplaceCardCommunity => {
                if c_index >= game.community.len() { return Err(GameError::CardIndexInvalid) }
                let card = game.pop_next_deck_card();
                // this operation should never fail, as the validity of the index has already been checked
                game.replace_community_card(c_index, card, events)?;
            },
            SpecialCard::RemoveCardCommunity => {
                game.remove_community_card(c_index, events)?;
            },
            SpecialCard::DrawCardCommunity => {
                game.deal_community_cards(1, events);
            },
            SpecialCard::RevealOpponentCard => {
                check_player_owns_card(t_id, c_index, game)?;
                let t_idx = game.get_player_index(t_id).unwrap_or(0);
                let t_p = &mut game.players[t_idx];
                if !t_p.card_visibility[c_index].contains(&player_id) {
                    t_p.card_visibility[c_index].push(player_id);
                }
                game.modifiers.combine_or_add_modifiers(Modifier {
                    effect: ModifierEffect::RevealCard { target_id: t_id, card_index: c_index, viewer_id: player_id },
                    expiry: ModifierExpiry::OnHandEnd { count: 1 },
                    source: vec![player_id],
                });
                events.push(MoveEvent { 
                    actor_id: Some(player_id), 
                    action: MoveAction::RevealCard { target_id: t_id, card_index: c_index },
                    private: true,
                });

            },
            SpecialCard::WithdrawBet => {
                let refund = withdraw_bet(player_id, player_idx, game);
                let player = &mut game.players[player_idx];

                player.chips += refund;
                player.folded = true;
                player.acted = true;
            },
            SpecialCard::AnteUp => {
                let ante_mult = 2.0;
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::AnteMultiplier { multiplier: ante_mult }, 
                    expiry: ModifierExpiry::OnGameEnd, 
                    source: vec![player_id],
                });

                game.modifiers.recompute_vars();
                give_special_card(SpecialCard::ChipBoost, 1, true, player_id, game, events)?;
            },
            SpecialCard::PotMult => {
                let pot_mult = 1.4;
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::PotMultiplier { multiplier: pot_mult }, 
                    expiry: ModifierExpiry::OnHandEnd { count: 1 }, 
                    source: vec![player_id],
                });
                
                game.modifiers.recompute_vars();
            },
            SpecialCard::RaiseBlock => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::RaisesBlocked, 
                    expiry: ModifierExpiry::OnPlayerTurn { count: 1, player_id }, 
                    source: vec![player_id],
                })
            },
            SpecialCard::SpecialBlock => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::SpecialsBlocked, 
                    expiry: ModifierExpiry::OnPlayerTurn { count: 1, player_id }, 
                    source: vec![player_id],
                })
            },
            SpecialCard::ChipBoost => {
                game.players[player_idx].chips += (game.ctx.blind_size as f64 * game.modifiers.vars.ante_multiplier * 2.5) as i32;
            },
            SpecialCard::ChipGamble => {
                if game.ctx.rng.random_bool(game.modifiers.vars.gamble_success_chance) {
                    game.players[player_idx].chips += (game.ctx.blind_size as f64 * game.modifiers.vars.ante_multiplier * 7.5) as i32;
                    game.modifiers.vars.gamble_success_chance *= 0.9;
                } else {
                    withdraw_bet(player_id, player_idx, game);
                    let player = &mut game.players[player_idx];
                    player.chips = (player.chips as f64 * 0.5).round() as i32;
                    player.folded = true;
                    player.acted = true;
                }
            },
            SpecialCard::Blackjack => {
                // toggles hand eval function to blackjack
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::BlackjackScoring, 
                    expiry: ModifierExpiry::OnHandEnd { count: 1 }, 
                    source: vec![player_id],
                });

                // gives all players a draw card and allows them to take another turn
                give_special_card_to_all(SpecialCard::DrawCardSelf, 1, true, game, events)?;
                game.players.iter_mut().for_each(|p| p.turn_ended = false);
            },
            SpecialCard::DrawHeart => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Heart), 
                    expiry: ModifierExpiry::OnCommunityDraw { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::DrawSpade => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Spade), 
                    expiry: ModifierExpiry::OnCommunityDraw { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::DrawDiamond => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Diamond), 
                    expiry: ModifierExpiry::OnCommunityDraw { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::DrawClub => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Club), 
                    expiry: ModifierExpiry::OnCommunityDraw { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::DrawFace => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Face), 
                    expiry: ModifierExpiry::OnCommunityDraw { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::DrawHigh => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::High), 
                    expiry: ModifierExpiry::OnCommunityDraw { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::DrawLow => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Low), 
                    expiry: ModifierExpiry::OnCommunityDraw { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::DrawHigherThanLast => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::HigherThanLast), 
                    expiry: ModifierExpiry::OnCommunityDraw { count: 1 }, 
                    source: vec![player_id],
                });                
            },
            SpecialCard::DrawLowerThanLast => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::LowerThanLast), 
                    expiry: ModifierExpiry::OnCommunityDraw { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::DrawSameSuitAsLast => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::SameSuitAsLast), 
                    expiry: ModifierExpiry::OnCommunityDraw { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::InvalidateFlush => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::InvalidateHand(HandType::Flush), 
                    expiry: ModifierExpiry::OnHandEnd { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::InvalidateStraight => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::InvalidateHand(HandType::Straight), 
                    expiry: ModifierExpiry::OnHandEnd { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::InvalidateThreeOfAKind => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::InvalidateHand(HandType::ThreeOfAKind), 
                    expiry: ModifierExpiry::OnHandEnd { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::InvalidateTwoPair => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::InvalidateHand(HandType::TwoPair), 
                    expiry: ModifierExpiry::OnHandEnd { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::InvalidateFullHouse => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::InvalidateHand(HandType::FullHouse), 
                    expiry: ModifierExpiry::OnHandEnd { count: 1 }, 
                    source: vec![player_id],
                });
            },
            // draws some special cards out of a preset equally weighted pool
            // containing only cards with unique effects
            SpecialCard::Special => {
                let count = 5;
                let permitted_cards: [SpecialCard; 16] = [
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
                    SpecialCard::HandSwap,
                    SpecialCard::Joker,
                ];

                for _ in 0..count {
                    let next_card = permitted_cards[game.ctx.rng.random_range(0..permitted_cards.len())];
                    give_special_card(next_card, 1, true, player_id, game, events)?;
                }
            },
            SpecialCard::Discard => {
                game.modifiers.combine_or_add_modifiers(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Nothing), 
                    expiry: ModifierExpiry::OnHandEnd { count: 1 }, 
                    source: vec![player_id],
                });
            },
            SpecialCard::HandSwap => {
                if t_id == usize::MAX {
                    return Err(GameError::InvalidTargetPlayer);
                }

                // get target cards
                let t_idx = game.get_player_index(t_id).unwrap_or(0);
                let target = &game.players[t_idx];
                let target_cards: Vec<ExpandedCard> = [&game.community[..], &target.cards[..]].concat();
                let target_hand = Hand::new(&target_cards, &game.ctx.hand_fns, &game.modifiers.invalidated_hands(), game.ctx.hand_size);

                // get player cards
                let player = &game.players[player_idx];
                let player_cards: Vec<ExpandedCard> = [&game.community[..], &player.cards[..]].concat();
                let player_hand = Hand::new(&player_cards, &game.ctx.hand_fns, &game.modifiers.invalidated_hands(), game.ctx.hand_size);
                
                // if player is better than target, then swap
                if player_hand > target_hand {
                    let temp_player = player.cards.clone();
                    game.players[player_idx].cards = game.players[t_idx].cards.clone();
                    game.players[t_idx].cards = temp_player;
                // otherwise, remove self and give a failure warning
                } else {
                    game.players[player_idx].special_cards.remove(self_idx);
                    return Err(GameError::HandSwapFailed);
                }
            },
            SpecialCard::Joker => {
                if c_index >= game.community.len() { return Err(GameError::CardIndexInvalid) }
                game.replace_community_card(c_index, ExpandedCard::get_joker(), events)?;
            },
            SpecialCard::SpecialSpread => {
                // give 1 special card to all
                game.deal_special_cards(1, events);
            },
        }
        
        game.players[player_idx].special_cards.remove(self_idx);

        // special special card check
        if game.ctx.rng.random_bool(0.005) {
            give_special_card(SpecialCard::Special, 1, true, player_id, game, events)?;
        }

        Ok(())
    }

    /**
     * Plays card discard effects
     */
    pub(crate) fn discard_card(&self, player_id: usize, game: &mut Game, events: &mut Vec<MoveEvent>) -> Result<(), GameError> {
        match self {
            _ => {},
        }

        if game.ctx.rng.random_bool(0.01) {
            // should never fail so long as player_id and game are valid
            give_special_card(SpecialCard::Discard, 1, true, player_id, game, events)?;
        }

        Ok(())
    }

    pub(crate) fn from_str(card_name: &str) -> Option<Self> {
        Some(match card_name.to_lowercase().as_str() {
            "replacecardself" => SpecialCard::ReplaceCardSelf,
            "drawcardself" => SpecialCard::DrawCardSelf,
            "replacecardcommunity" => SpecialCard::ReplaceCardCommunity,
            "removecardcommunity" => SpecialCard::RemoveCardCommunity,
            "drawcardcommunity" => SpecialCard::DrawCardCommunity,
            "revealopponentcard" => SpecialCard::RevealOpponentCard,
            "withdrawbet" => SpecialCard::WithdrawBet,
            "anteup" => SpecialCard::AnteUp,
            "potmult" => SpecialCard::PotMult,
            "raiseblock" => SpecialCard::RaiseBlock,
            "specialblock" => SpecialCard::SpecialBlock,
            "chipboost" => SpecialCard::ChipBoost,
            "chipgamble" => SpecialCard::ChipGamble,
            "blackjack" => SpecialCard::Blackjack,
            "drawheart" => SpecialCard::DrawHeart,
            "drawspade" => SpecialCard::DrawSpade,
            "drawdiamond" => SpecialCard::DrawDiamond,
            "drawclub" => SpecialCard::DrawClub,
            "drawface" => SpecialCard::DrawFace,
            "drawhigh" => SpecialCard::DrawHigh,
            "drawlow" => SpecialCard::DrawLow,
            "drawhigherthanlast" => SpecialCard::DrawHigherThanLast,
            "drawlowerthanlast" => SpecialCard::DrawLowerThanLast,
            "drawsamesuitaslast" => SpecialCard::DrawSameSuitAsLast,
            "invalidateflush" => SpecialCard::InvalidateFlush,
            "invalidatestraight" => SpecialCard::InvalidateStraight,
            "invalidatethreeofakind" => SpecialCard::InvalidateThreeOfAKind,
            "invalidatetwopair" => SpecialCard::InvalidateTwoPair,
            "invalidatefullhouse" => SpecialCard::InvalidateFullHouse,
            "special" => SpecialCard::Special,
            "discard" => SpecialCard::Discard,
            "handswap" => SpecialCard::HandSwap,
            "joker" => SpecialCard::Joker,
            _ => return None,
        })
    }

    pub(crate) fn draw_special_cards(count: usize, player_id: usize, game: &mut Game) -> Vec<SpecialCard> {
        let mut luck: f64 = 1.0;
        // increase luck as game goes on - better cards over time
        luck *= (((game.games_played as f64).sqrt() - 1.0) / 3.0) + 1.0;
        // increase luck for active player with least chips
        let losing_player = game.players.iter().min_by(|x, y| 
            (if x.folded { i32::MAX } else { x.chips + x.total_bet })
            .cmp(&(if y.folded { i32::MAX } else { y.chips + y.total_bet }))
        );
        if let Some(p) = losing_player && p.id == player_id {
            luck *= 1.4;
        }

        let mut special_card_sums: Vec<(SpecialCard, f64)> = Vec::new();
        let mut sum: f64 = 0.0;
        // build binary searchable sum vector
        for (card, weight_fn) in SPECIAL_CARD_WEIGHT_FNS {
            special_card_sums.push((card, sum));
            sum += weight_fn(luck);
        }

        let mut cards_drawn: Vec<SpecialCard> = Vec::new();
        for _ in 0..count {
            // and then get the result of the roll
            let roll = game.ctx.rng.random_range::<f64, _>(0.0..sum);
            let card_idx = match special_card_sums.binary_search_by(|(_, s)| s.total_cmp(&roll)) {
                Err(i) => i - 1,
                Ok(i) => i
            };

            cards_drawn.push(special_card_sums[card_idx].0);
        }

        cards_drawn
    }
}

// weight functions for special cards - determines end draw probability
// takes a luck value l, determined through several factors, and produces a card. generally, better cards will scale better with luck
static SPECIAL_CARD_WEIGHT_FNS: [(SpecialCard, fn(f64) -> f64); 31] = [
    (SpecialCard::ReplaceCardSelf, |l| 4.0 * l),
    (SpecialCard::DrawCardSelf, |l| 1.5 * l),
    (SpecialCard::ReplaceCardCommunity, |_| 10.0),
    (SpecialCard::RemoveCardCommunity, |l| 1.0 * l),
    (SpecialCard::DrawCardCommunity, |l| 1.0 * l),
    (SpecialCard::RevealOpponentCard, |l| 4.0 * l),
    (SpecialCard::WithdrawBet, |l| 0.9 * l.powi(2)),
    (SpecialCard::AnteUp, |l| 2.0 * l.powi(2)),
    (SpecialCard::PotMult, |l| 1.5 * l.powi(2)),
    (SpecialCard::RaiseBlock, |_| 10.0),
    (SpecialCard::SpecialBlock, |_| 10.0),
    (SpecialCard::ChipBoost, |l| 6.0 * l),
    (SpecialCard::ChipGamble, |l| 1.1 * l.powi(2)),
    (SpecialCard::Blackjack, |_| 1.0),
    (SpecialCard::HandSwap, |l| 1.2 * l),
    (SpecialCard::Joker, |l| 1.2 * l),
    (SpecialCard::DrawHeart, |_| 10.0),
    (SpecialCard::DrawSpade, |_| 10.0),
    (SpecialCard::DrawDiamond, |_| 10.0),
    (SpecialCard::DrawClub, |_| 10.0),
    (SpecialCard::DrawFace, |_| 7.5),
    (SpecialCard::DrawHigh, |_| 7.5),
    (SpecialCard::DrawLow, |_| 7.5),
    (SpecialCard::DrawHigherThanLast, |l| 2.5 * l),
    (SpecialCard::DrawLowerThanLast, |l| 2.5 * l),
    (SpecialCard::DrawSameSuitAsLast, |l| 2.5 * l),
    (SpecialCard::InvalidateFlush, |_| 5.0),
    (SpecialCard::InvalidateStraight, |_| 5.0),
    (SpecialCard::InvalidateThreeOfAKind, |_| 5.0),
    (SpecialCard::InvalidateTwoPair, |_| 5.0),
    (SpecialCard::InvalidateFullHouse, |_| 5.0),
];

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

pub(crate) fn withdraw_bet(player_id: usize, player_idx: usize, game: &mut Game) -> i32 {
    let mut refund = 0;

    for pot in &mut game.pots {
        if let Some(position) = pot.players.iter().position(|id| *id == player_id) {
            let participant_count = pot.players.len() as i32;
            if participant_count > 0 {
                let contribution = pot.amount / participant_count;
                pot.amount -= contribution;
                refund += contribution;
            }

            pot.players.remove(position);
        }
    }

    let current_street_refund = game.players[player_idx].round_bet;
    refund += current_street_refund;
    game.round_pool -= current_street_refund;

    let player = &mut game.players[player_idx];
    player.total_bet = (player.total_bet - refund).max(0);
    player.round_bet = 0;

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

    if game.players[p_id].special_cards.len() == game.ctx.special_card_limit {
        return Err(GameError::SpecialCardsFull);
    }

    let mut given_cards = 0;
    for _ in 0..count {
        if game.players[p_id].special_cards.len() >= game.ctx.special_card_limit && !ignore_limit {
            break;
        } else {
            game.players[p_id].special_cards.push(special_card);
            given_cards += 1;
        }
    }

    events.push(MoveEvent { 
        actor_id: Some(player_id), 
        action: MoveAction::DealSpecial { count: given_cards }, 
        private: false 
    });


    if given_cards < count {
        return Err(GameError::SpecialCardsFull);
    }

    Ok(())
}