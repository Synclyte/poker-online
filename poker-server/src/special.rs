/**
 * Handles special card related logic
 */

use std::fmt;
use rand::RngExt;
use serde::Serialize;

use crate::{ExpandedCard, Game, GameError, HandType, MoveEvent, Rank, Round, Suit};

#[derive(Default, Clone, Serialize)]
pub(crate) struct RoundModifiers {
    active: Vec<Modifier>,
}
impl RoundModifiers {
    pub(crate) fn player_turn(&mut self, player_id: usize) {
        self.active.retain(|m| match m.expiry { 
            ModifierExpiry::OnPlayerTurn { player_id: p_id } => p_id != player_id,
             _ => true, 
        });
    }

    pub(crate) fn hand_ended(&mut self) {
        self.active.retain(|m| !matches!(m.expiry, ModifierExpiry::OnHandEnd));
    }

    pub(crate) fn community_draw(&mut self) {
        self.active.retain(|m| !matches!(m.expiry, ModifierExpiry::OnCommunityDraw));
    }

    pub(crate) fn round_ended(&mut self) {
        self.active.retain(|m| !matches!(m.expiry, ModifierExpiry::OnRoundEnd));
    }

    pub(crate) fn game_ended(&mut self) {
        self.active.clear();
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

    pub(crate) fn pot_multiplier(&self) -> f64 {
        self.active.iter().fold(1.0, |product, m| match m.effect {
            ModifierEffect::PotMultiplier { multiplier } => product * multiplier,
            _ => product
        })
    }

    pub(crate) fn blind_multiplier(&self) -> f64 {
        self.active.iter().fold(1.0, |product, m| match m.effect {
            ModifierEffect::AnteMultiplier { multiplier } => product * multiplier,
            _ => product
        })
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum DrawRule {
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
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
enum ModifierEffect {
    PotMultiplier { multiplier: f64 },
    AnteMultiplier { multiplier: f64 },
    RaisesBlocked,
    SpecialsBlocked,
    BlackjackScoring,
    ForceCommunityDraw(DrawRule),
    InvalidateHand(HandType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum ModifierExpiry {
    OnCommunityDraw,
    OnPlayerTurn { player_id: usize },
    OnRoundEnd,
    OnHandEnd,
    OnGameEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub (crate) struct Modifier {
    effect: ModifierEffect,
    expiry: ModifierExpiry,
    source: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    ChipGamble, // 50:50 to gain a significant number of chips or lose all chips
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
    Special, // grants a few special cards. not obtainable through normal draws
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

        if player_id != usize::MAX && game.get_player_index(player_id).is_err() {
            return Err(GameError::InvalidTargetPlayer);
        }

        if t_id != usize::MAX && game.get_player_index(t_id).is_err() {
            return Err(GameError::InvalidTargetPlayer);
        }

        fn check_player_owns_card(player_id: usize, c_index: usize, game: &Game) -> Result<(), GameError> {
            if game.players[game.get_player_index(player_id).unwrap()].cards.get(c_index).is_none() {
                Err(GameError::CardIndexInvalid)
            } else {
                Ok(())
            }
        }

        fn check_community_owns_card(c_index: usize, game: &Game) -> Result<(), GameError> {
            if game.community.get(c_index).is_none() {
                Err(GameError::CardIndexInvalid)
            } else {
                Ok(())
            }
        }

        let player_idx = game.get_player_index(player_id).unwrap_or(0);
        let self_idx = game.players[player_idx].special_cards.iter().position(|s| s == self).unwrap_or(usize::MAX);
        if self_idx == usize::MAX {
            return Err(GameError::SpecialCardNotOwned);
        }

        match self {
            SpecialCard::ReplaceCardSelf => {
                check_player_owns_card(player_id, c_index, game)?;
                game.remove_hole_card(player_id, c_index)?;
                game.deal_hole_card(player_id);
            },
            SpecialCard::DrawCardSelf => {
                game.deal_hole_card(player_id);
                let c_index = game.players[player_idx].cards.len() - 1;
                let all_player_ids = game.players.iter().map(|p| p.id).collect::<Vec<usize>>();
                game.players[player_idx].card_visibility[c_index] = all_player_ids;
            },
            SpecialCard::ReplaceCardCommunity => {
                check_community_owns_card(c_index, game)?;
                game.community.remove(c_index);
                game.deal_community_cards(1);
            },
            SpecialCard::RemoveCardCommunity => {
                check_community_owns_card(c_index, game)?;
                game.community.remove(c_index);
            },
            SpecialCard::DrawCardCommunity => {
                game.deal_community_cards(1);
            },
            SpecialCard::RevealOpponentCard => {
                check_player_owns_card(t_id, c_index, game)?;
                let t_idx = game.get_player_index(t_id).unwrap_or(0);
                let t_p = &mut game.players[t_idx];
                if !t_p.card_visibility[c_index].contains(&player_id) {
                    t_p.card_visibility[c_index].push(player_id);
                }
                let revealed_card = t_p.cards[c_index].to_string();
                events.push(MoveEvent { 
                    player_id: player_id, 
                    action: format!("REVEAL {} {} {}", t_id, c_index, revealed_card) 
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
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::AnteMultiplier { multiplier: 2.0 }, 
                    expiry: ModifierExpiry::OnGameEnd, 
                    source: player_id,
                })
            },
            SpecialCard::PotMult => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::PotMultiplier { multiplier: 1.25 }, 
                    expiry: ModifierExpiry::OnHandEnd, 
                    source: player_id,
                });
            },
            SpecialCard::RaiseBlock => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::RaisesBlocked, 
                    expiry: ModifierExpiry::OnRoundEnd, 
                    source: player_id,
                })
            },
            SpecialCard::SpecialBlock => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::SpecialsBlocked, 
                    expiry: ModifierExpiry::OnRoundEnd, 
                    source: player_id,
                })
            },
            SpecialCard::ChipBoost => {
                game.players[player_idx].chips += game.ctx.blind_size * 2;
            },
            SpecialCard::ChipGamble => {
                if game.ctx.rng.random_bool(0.5) {
                    game.players[player_idx].chips += game.ctx.blind_size * 10;
                } else {
                    withdraw_bet(player_id, player_idx, game);
                    let player = &mut game.players[player_idx];
                    player.chips = 0;
                    player.folded = true;
                    player.acted = true;
                }
            },
            SpecialCard::Blackjack => {
                // toggles hand eval function to blackjack
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::BlackjackScoring, 
                    expiry: ModifierExpiry::OnHandEnd, 
                    source: player_id,
                });

                // gives all players a draw card and allows them to take another turn
                give_special_card_to_all(SpecialCard::DrawCardSelf, 1, true, game)?;
                game.players.iter_mut().for_each(|p| p.turn_ended = false);
            },
            SpecialCard::DrawHeart => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Heart), 
                    expiry: ModifierExpiry::OnCommunityDraw, 
                    source: player_id,
                });
            },
            SpecialCard::DrawSpade => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Spade), 
                    expiry: ModifierExpiry::OnCommunityDraw, 
                    source: player_id,
                });
            },
            SpecialCard::DrawDiamond => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Diamond), 
                    expiry: ModifierExpiry::OnCommunityDraw, 
                    source: player_id,
                });
            },
            SpecialCard::DrawClub => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Club), 
                    expiry: ModifierExpiry::OnCommunityDraw, 
                    source: player_id,
                });
            },
            SpecialCard::DrawFace => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Face), 
                    expiry: ModifierExpiry::OnCommunityDraw, 
                    source: player_id,
                });
            },
            SpecialCard::DrawHigh => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::High), 
                    expiry: ModifierExpiry::OnCommunityDraw, 
                    source: player_id,
                });
            },
            SpecialCard::DrawLow => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::Low), 
                    expiry: ModifierExpiry::OnCommunityDraw, 
                    source: player_id,
                });
            },
            SpecialCard::DrawHigherThanLast => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::HigherThanLast), 
                    expiry: ModifierExpiry::OnCommunityDraw, 
                    source: player_id,
                });                
            },
            SpecialCard::DrawLowerThanLast => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::LowerThanLast), 
                    expiry: ModifierExpiry::OnCommunityDraw, 
                    source: player_id,
                });
            },
            SpecialCard::DrawSameSuitAsLast => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::ForceCommunityDraw(DrawRule::SameSuitAsLast), 
                    expiry: ModifierExpiry::OnCommunityDraw, 
                    source: player_id,
                });
            },
            SpecialCard::InvalidateFlush => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::InvalidateHand(HandType::Flush), 
                    expiry: ModifierExpiry::OnHandEnd, 
                    source: player_id,
                });
            },
            SpecialCard::InvalidateStraight => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::InvalidateHand(HandType::Straight), 
                    expiry: ModifierExpiry::OnHandEnd, 
                    source: player_id,
                });
            },
            SpecialCard::InvalidateThreeOfAKind => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::InvalidateHand(HandType::ThreeOfAKind), 
                    expiry: ModifierExpiry::OnHandEnd, 
                    source: player_id,
                });
            },
            SpecialCard::InvalidateTwoPair => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::InvalidateHand(HandType::TwoPair), 
                    expiry: ModifierExpiry::OnHandEnd, 
                    source: player_id,
                });
            },
            SpecialCard::InvalidateFullHouse => {
                game.modifiers.active.push(Modifier { 
                    effect: ModifierEffect::InvalidateHand(HandType::Flush), 
                    expiry: ModifierExpiry::OnHandEnd, 
                    source: player_id,
                });
            },
            // draws some special cards out of a preset equally weighted pool
            // containing only cards with unique effects
            SpecialCard::Special => {
                let count = 5;
                let permitted_cards: [SpecialCard; 14] = [
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
                ];

                for _ in 0..count {
                    let next_card = permitted_cards[game.ctx.rng.random_range(0..permitted_cards.len())];
                    give_special_card(next_card, 1, true, player_id, game)?;
                }
            },
        }
        
        game.players[player_idx].special_cards.remove(self_idx);

        // special special card check
        if game.ctx.rng.random_bool(0.005) {
            give_special_card(SpecialCard::Special, 1, true, player_id, game)?;
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
        
        // increased luck for winner special card
        if game.round == Round::Showdown {
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
static SPECIAL_CARD_WEIGHT_FNS: [(SpecialCard, fn(f64) -> f64); 29] = [
    (SpecialCard::ReplaceCardSelf, |l| 4.0 * l),
    (SpecialCard::DrawCardSelf, |l| 1.5 * l),
    (SpecialCard::ReplaceCardCommunity, |_| 10.0),
    (SpecialCard::RemoveCardCommunity, |l| 1.0 * l),
    (SpecialCard::DrawCardCommunity, |l| 1.0 * l),
    (SpecialCard::RevealOpponentCard, |l| 4.0 * l),
    (SpecialCard::WithdrawBet, |l| 0.8 * l.powi(2)),
    (SpecialCard::AnteUp, |l| 2.0 * l.powi(2)),
    (SpecialCard::PotMult, |l| 1.2 * l.powi(2)),
    (SpecialCard::RaiseBlock, |_| 10.0),
    (SpecialCard::SpecialBlock, |_| 10.0),
    (SpecialCard::ChipBoost, |l| 6.0 * l),
    (SpecialCard::ChipGamble, |l| 1.0 * l.powi(2)),
    (SpecialCard::Blackjack, |_| 1.0),
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
pub(crate) fn give_special_card_to_all(special_card: SpecialCard, count: usize, ignore_limit: bool, game: &mut Game) -> Result<(), GameError> {
    for i in 0..game.players.len() {
        give_special_card(special_card, count, ignore_limit, game.players[i].id, game)?;
    }
    Ok(())
}

/**
 * Gives a specified special card count times to one player by ID, conditionally ignoring limits
 */
pub(crate) fn give_special_card(special_card: SpecialCard, count: usize, ignore_limit: bool, player_id: usize, game: &mut Game) -> Result<(), GameError> {
    let player = game.get_player_index(player_id).map_or(Err(GameError::InvalidTargetPlayer), |p_idx| Ok(&mut game.players[p_idx]))?;
    // return without error if player is eliminated
    if player.chips <= 0 || player.folded {
        return Ok(())
    }

    for _ in 0..count {
        if player.special_cards.len() >= game.ctx.special_card_limit && !ignore_limit {
            return Err(GameError::SpecialCardsFull);
        } else {
            player.special_cards.push(special_card);
        }
    }
    Ok(())
}