/**
 * Contains tests for modules
 */

#[cfg(test)]
mod tests {
    #[cfg(test)]
    mod special_tests {
        use std::collections::HashSet;
        use crate::Action::{Call, Fold, PlaySpecial, Raise};
        use crate::poker::{Card, ExpandedCard, Rank, Suit};
        use crate::special::*;
        use crate::{ConfigPlayer, DeckType, Game, GameConfig, GameError, Round};

        fn started_game() -> Game {
            let mut game = Game::new();
            let config = GameConfig {
                players: vec![ConfigPlayer::Human { id: 0 }, ConfigPlayer::Human { id: 1 }],
                rng_seed: 0,
                blind_size: 20,
                min_raise: 10,
                starting_chips: 1000,
                special_card_limit: 3,
                deck_type: DeckType::Standard,
                max_players: 2,
                round_limit: 30,
                max_bet_multiplier: None,
            };
            game.update_config(config).unwrap();
            game.initialise().unwrap();
            game.start().unwrap();
            game
        }

        #[test]
        fn test_card_registry_metadata_and_pools() {
            let mut seen_cards = Vec::new();
            let mut seen_ids = HashSet::new();

            for card in ALL_SPECIAL_CARDS {
                assert!(!seen_cards.contains(&card), "Duplicate variant in ALL_SPECIAL_CARDS: {:?}", card);
                seen_cards.push(card);

                let def = card.def();
                assert!(!def.roles.is_empty(), "{:?} must have at least one CardRole", card);
                assert!(
                    !def.id.is_empty()
                        && def.id.chars().all(|c| c.is_ascii_lowercase() || c == '_')
                        && !def.id.starts_with('_')
                        && !def.id.ends_with('_')
                        && !def.id.contains("__"),
                    "CardDef.id must be valid snake_case for {:?}, got '{}'",
                    card,
                    def.id
                );
                assert_eq!(card.to_string(), def.id, "Display ID mismatch for {:?}", card);
                assert_eq!(
                    serde_json::to_string(&card).unwrap(),
                    format!("\"{}\"", def.id),
                    "Serde ID mismatch for {:?}",
                    card
                );
                assert!(seen_ids.insert(def.id), "Duplicate id in registry: {}", def.id);
                assert_eq!(SpecialCard::from_str(def.id), Some(card), "from_str (snake_case) failed for {}", def.id);
            }

            assert_eq!(SpecialCard::from_str("ReplaceCardSelf"), None);
            assert_eq!(SpecialCard::from_str("replaceCardSelf"), None);
            assert_eq!(serde_json::to_string(&ModifierEffect::RaisesBlocked).unwrap(), "\"raises_blocked\"");
            assert_eq!(serde_json::to_string(&DrawRule::HigherThanLast).unwrap(), "\"higher_than_last\"");
            assert_eq!(serde_json::to_string(&crate::poker::HandType::ThreeOfAKind).unwrap(), "\"three_of_a_kind\"");

            assert!(SpecialCard::Special.def().pools.is_empty());
            assert!(SpecialCard::Discard.def().pools.is_empty());
            assert!(SpecialCard::SpecialSpread.def().pools.is_empty());
            assert!(SpecialCard::Draw.def().pools.is_empty());
            assert_eq!(
                SpecialCard::Draw.def().roles,
                vec![CardRole::Chaos, CardRole::Hand, CardRole::Board]
            );

            for draw_card in [
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
            ] {
                assert!(draw_card.has_role(CardRole::Board), "{:?} should have Board role", draw_card);
                assert!(draw_card.has_role(CardRole::Hand), "{:?} should have Hand role", draw_card);
                assert!(draw_card.in_pool(CardPool::Standard));
                assert!(draw_card.in_pool(CardPool::WinnerDraft));
                assert!(!draw_card.in_pool(CardPool::SpecialBurst));
            }
        }

        #[test]
        fn test_draw_rule_applies_to_any_player_hole_draw_and_stacks() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_idx = 1 - p0_idx;
            let p1_id = game.players[p1_idx].id;

            game.players[p0_idx].special_cards.push(SpecialCard::DrawHeart);
            game.players[p1_idx].special_cards.push(SpecialCard::DrawHeart);
            game.players[p1_idx].special_cards.push(SpecialCard::ReplaceCardSelf);
            game.players[p0_idx].special_cards.push(SpecialCard::ReplaceCardSelf);

            // Push non-hearts on top of the deck so unconstrained pops would draw Spades/Clubs
            game.deck.push(ExpandedCard::get_card("3", "c"));
            game.deck.push(ExpandedCard::get_card("2", "s"));

            // p0 plays DrawHeart, ends turn; p1 also plays DrawHeart -> merges into count: 2
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::DrawHeart,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();
            game.player_move(p0_id, Call).unwrap();

            game.player_move(
                p1_id,
                PlaySpecial {
                    card: SpecialCard::DrawHeart,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();
            assert_eq!(game.special.active_modifiers().len(), 1);

            // p1 replaces their own card -> skips non-Heart top card and consumes 1 draw charge
            game.player_move(
                p1_id,
                PlaySpecial {
                    card: SpecialCard::ReplaceCardSelf,
                    target_id: Some(p1_id),
                    card_index: Some(0),
                },
            )
            .unwrap();
            assert_eq!(game.players[p1_idx].cards[0].card().suit, Suit::Hearts);
            assert_eq!(
                game.special.active_modifiers().len(),
                1,
                "Second ForceDraw charge should remain active after first draw"
            );

            // p1 raises so action returns to p0 in Preflop without dealing Flop community cards
            let _ = game.player_move(p1_id, crate::Action::Raise { amount: 20 }).unwrap();
            assert_eq!(game.get_current_turn_player_id(), p0_id);

            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::ReplaceCardSelf,
                    target_id: Some(p0_id),
                    card_index: Some(0),
                },
            )
            .unwrap();
            assert_eq!(game.players[p0_idx].cards[0].card().suit, Suit::Hearts);
            assert!(
                game.special.active_modifiers().is_empty(),
                "Merged ForceDraw modifier should expire after second draw"
            );
        }

        #[test]
        fn test_draw_rule_last_card_rules_work_on_preflop_hole_draws() {
            let mut game = started_game();
            assert!(game.community.is_empty());

            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();

            // Set deterministic hole cards: index 0 = 5 of Diamonds, last = 9 of Clubs
            game.players[p0_idx].cards = vec![
                ExpandedCard::get_card("5", "d"),
                ExpandedCard::get_card("9", "c"),
            ];
            game.players[p0_idx].card_visibility = vec![vec![], vec![]];
            game.players[p0_idx].special_cards.push(SpecialCard::DrawSameSuitAsLast);
            game.players[p0_idx].special_cards.push(SpecialCard::DrawCardSelf);
            game.players[p0_idx].special_cards.push(SpecialCard::DrawHigherThanLast);
            game.players[p0_idx].special_cards.push(SpecialCard::ReplaceCardSelf);

            // Top of deck is 2 of Hearts (neither Clubs nor >= Queen)
            game.deck.push(ExpandedCard::get_card("q", "s"));
            game.deck.push(ExpandedCard::get_card("2", "h"));

            // Preflop DrawSameSuitAsLast + DrawCardSelf uses player's last hole card (Clubs)
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::DrawSameSuitAsLast,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::DrawCardSelf,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();
            assert_eq!(game.players[p0_idx].cards.last().unwrap().card().suit, Suit::Clubs);

            // Set index 0 to King of Spades; DrawHigherThanLast + ReplaceCardSelf(0) must draw >= King
            game.players[p0_idx].cards[0] = ExpandedCard::get_card("k", "s");
            game.deck.push(ExpandedCard::get_card("4", "d"));
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::DrawHigherThanLast,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::ReplaceCardSelf,
                    target_id: Some(p0_id),
                    card_index: Some(0),
                },
            )
            .unwrap();
            assert!(game.players[p0_idx].cards[0].card().rank >= Rank::King);
        }

        #[test]
        fn test_discard_blocks_community_draws_but_not_hole_draws() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();

            game.players[p0_idx].special_cards.push(SpecialCard::Discard);
            game.players[p0_idx].special_cards.push(SpecialCard::ReplaceCardSelf);

            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::Discard,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();

            // ReplaceCardSelf must still draw a real card even while Discard is active
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::ReplaceCardSelf,
                    target_id: Some(p0_id),
                    card_index: Some(0),
                },
            )
            .unwrap();
            assert_ne!(game.players[p0_idx].cards[0], ExpandedCard::Unmarked);

            // Community deal must still be blocked by Discard
            let mut events = Vec::new();
            game.deal_community_cards(1, &mut events);
            assert_eq!(game.community.last(), Some(&ExpandedCard::Unmarked));
        }

        #[test]
        fn test_pot_mult_stacks_by_effect_not_duration() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();

            game.players[p0_idx].special_cards.push(SpecialCard::PotMult);
            game.players[p0_idx].special_cards.push(SpecialCard::PotMult);

            for _ in 0..2 {
                game.player_move(
                    p0_id,
                    PlaySpecial {
                        card: SpecialCard::PotMult,
                        target_id: None,
                        card_index: None,
                    },
                )
                .unwrap();
            }

            assert_eq!(game.special.active_modifiers().len(), 1);
            assert!((game.special.pot_multiplier() - 1.96).abs() < 1e-6);

            let mut events = Vec::new();
            game.end_hand(&mut events);
            assert!(game.special.active_modifiers().is_empty());
            assert!((game.special.pot_multiplier() - 1.0).abs() < 1e-6);
        }

        #[test]
        fn test_hand_swap_swaps_card_visibility_and_remaps_reveals() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_idx = 1 - p0_idx;
            let p1_id = game.players[p1_idx].id;

            // Give p0 a 3-card royal/trips hand (with a public 3rd card) that beats p1's 2-card hand
            let p0_cards = vec![
                ExpandedCard::get_card("a", "s"),
                ExpandedCard::get_card("a", "h"),
                ExpandedCard::get_card("a", "d"),
            ];
            let p1_cards = vec![
                ExpandedCard::get_card("2", "c"),
                ExpandedCard::get_card("7", "d"),
            ];
            game.players[p0_idx].cards = p0_cards.clone();
            game.players[p0_idx].card_visibility = vec![vec![], vec![], vec![p0_id, p1_id]];
            game.players[p1_idx].cards = p1_cards.clone();
            game.players[p1_idx].card_visibility = vec![vec![], vec![]];

            // p0 reveals p1's slot 0 first, then plays HandSwap targeting p1
            game.players[p0_idx].special_cards.push(SpecialCard::RevealOpponentCard);
            game.players[p0_idx].special_cards.push(SpecialCard::HandSwap);

            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RevealOpponentCard,
                    target_id: Some(p1_id),
                    card_index: Some(0),
                },
            )
            .unwrap();
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::HandSwap,
                    target_id: Some(p1_id),
                    card_index: None,
                },
            )
            .unwrap();

            assert_eq!(game.players[p0_idx].cards, p1_cards);
            assert_eq!(game.players[p1_idx].cards, p0_cards);
            assert_eq!(game.players[p0_idx].card_visibility.len(), 2);
            assert_eq!(game.players[p1_idx].card_visibility.len(), 3);
            assert!(game.players[p1_idx].card_visibility[2].contains(&p0_id));
            assert!(!game.players[p0_idx].card_visibility[0].contains(&p0_id));
            assert!(
                !game
                    .special
                    .active_modifiers()
                    .iter()
                    .any(|m| matches!(m.effect, ModifierEffect::RevealCard { target_id, viewer_id, .. } if target_id == viewer_id)),
                "Self-reveal modifier should be pruned after HandSwap"
            );

            // Subsequent reveal on p1's new 3rd slot (already public to p0) fails cleanly;
            // revealing unrevealed slot 1 succeeds without panic
            game.players[p0_idx].special_cards.push(SpecialCard::RevealOpponentCard);
            let res_already_visible = game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RevealOpponentCard,
                    target_id: Some(p1_id),
                    card_index: Some(2),
                },
            );
            assert_eq!(res_already_visible.unwrap_err(), GameError::CardAlreadyVisible);
            let res_hidden_slot = game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RevealOpponentCard,
                    target_id: Some(p1_id),
                    card_index: Some(1),
                },
            );
            assert!(res_hidden_slot.is_ok());
        }

        #[test]
        fn test_withdraw_bet_refunds_exact_multi_street_bets_and_clears_pot_players() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_idx = 1 - p0_idx;
            let p1_id = game.players[p1_idx].id;

            let initial_total_chips: i32 = game.players.iter().map(|p| p.chips + p.total_bet).sum();

            // Preflop: p0 raises 100 (total bet 120), p1 calls 100 (total bet 120) -> moves to Flop
            game.player_move(p0_id, crate::Action::Raise { amount: 100 }).unwrap();
            game.player_move(p1_id, Call).unwrap();

            // Flop: first actor raises 50, other calls -> moves to Turn
            let flop_first = game.get_current_turn_player_id();
            let flop_second = if flop_first == p0_id { p1_id } else { p0_id };
            game.player_move(flop_first, crate::Action::Raise { amount: 50 }).unwrap();
            game.player_move(flop_second, Call).unwrap();

            // Turn: current actor bets 30, next actor plays WithdrawBet (after investing 170 total)
            let turn_first = game.get_current_turn_player_id();
            let turn_second = if turn_first == p0_id { p1_id } else { p0_id };
            game.player_move(turn_first, crate::Action::Raise { amount: 30 }).unwrap();

            let withdrawer_idx = game.get_player_index(turn_second).unwrap();
            let chips_before = game.players[withdrawer_idx].chips;
            let tracked_before = game.special.hand_bets.get(&turn_second).copied().unwrap_or(0);
            assert_eq!(tracked_before, 170);

            game.players[withdrawer_idx].special_cards.push(SpecialCard::WithdrawBet);
            game.player_move(
                turn_second,
                PlaySpecial {
                    card: SpecialCard::WithdrawBet,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();

            assert_eq!(game.players[withdrawer_idx].chips, chips_before + 170);
            assert!(game.players[withdrawer_idx].folded);
            assert!(game.players[withdrawer_idx].acted);
            assert!(game.players[withdrawer_idx].turn_ended);
            assert!(
                game.pots.iter().all(|pot| !pot.players.contains(&turn_second)),
                "Withdrawn player must have zero entries remaining in any pot"
            );

            // Advance/complete hand check: total chips across players + pots + round_pool remain conserved
            let remaining_pool = game.pots.iter().map(|p| p.amount).sum::<i32>() + game.round_pool;
            let current_player_chips: i32 = game.players.iter().map(|p| p.chips).sum();
            assert_eq!(current_player_chips + remaining_pool, initial_total_chips);
        }

        #[test]
        fn test_replace_card_self_clears_existing_reveal() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_idx = 1 - p0_idx;
            let p1_id = game.players[p1_idx].id;

            game.players[p0_idx].special_cards.push(SpecialCard::RevealOpponentCard);
            game.players[p1_idx].special_cards.push(SpecialCard::ReplaceCardSelf);

            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RevealOpponentCard,
                    target_id: Some(p1_id),
                    card_index: Some(0),
                },
            )
            .unwrap();
            assert!(game.players[p1_idx].card_visibility[0].contains(&p0_id));

            game.player_move(p0_id, Call).unwrap();
            game.player_move(
                p1_id,
                PlaySpecial {
                    card: SpecialCard::ReplaceCardSelf,
                    target_id: Some(p1_id),
                    card_index: Some(0),
                },
            )
            .unwrap();

            assert!(game.players[p1_idx].card_visibility[0].is_empty());
            assert!(game.special.active_modifiers().is_empty());
        }

        #[test]
        fn test_give_special_card_zero_count_emits_no_event() {
            let mut game = started_game();
            let p0_id = game.players[0].id;
            let mut events = Vec::new();

            let res = give_special_card(SpecialCard::ChipBoost, 0, true, p0_id, &mut game, &mut events);
            assert!(res.is_ok());
            assert!(events.is_empty());
        }

        #[test]
        fn test_reveal_opponent_card_rejects_already_visible_card() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_idx = 1 - p0_idx;
            let p1_id = game.players[p1_idx].id;

            game.players[p0_idx].special_cards.push(SpecialCard::RevealOpponentCard);
            game.players[p0_idx].special_cards.push(SpecialCard::RevealOpponentCard);

            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RevealOpponentCard,
                    target_id: Some(p1_id),
                    card_index: Some(0),
                },
            )
            .unwrap();
            assert_eq!(game.players[p1_idx].card_visibility[0], vec![p0_id]);

            let dup = game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RevealOpponentCard,
                    target_id: Some(p1_id),
                    card_index: Some(0),
                },
            );
            assert_eq!(dup.unwrap_err(), GameError::CardAlreadyVisible);
            assert_eq!(game.players[p0_idx].special_cards, vec![SpecialCard::RevealOpponentCard]);

            // Different target slot on the same opponent stacks separately
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RevealOpponentCard,
                    target_id: Some(p1_id),
                    card_index: Some(1),
                },
            )
            .unwrap();
            assert_eq!(game.special.active_modifiers().len(), 2);
            assert_eq!(game.players[p1_idx].card_visibility[1], vec![p0_id]);
        }

        #[test]
        fn test_rank_boost_increments_and_wraps_ace_to_two() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();

            game.players[p0_idx].cards = vec![
                ExpandedCard::get_card("a", "s"),
                ExpandedCard::get_card("9", "h"),
                ExpandedCard::Unmarked,
                ExpandedCard::PlayingCard(Card {
                    rank: Rank::None,
                    suit: Suit::Suitless,
                }),
                ExpandedCard::get_joker(),
            ];
            game.players[p0_idx].card_visibility = vec![vec![], vec![], vec![], vec![], vec![]];
            for _ in 0..5 {
                game.players[p0_idx].special_cards.push(SpecialCard::RankBoost);
            }

            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RankBoost,
                    target_id: Some(p0_id),
                    card_index: Some(1),
                },
            )
            .unwrap();
            assert_eq!(game.players[p0_idx].cards[1].card().rank, Rank::Ten);
            assert_eq!(game.players[p0_idx].cards[1].card().suit, Suit::Hearts);

            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RankBoost,
                    target_id: Some(p0_id),
                    card_index: Some(0),
                },
            )
            .unwrap();
            assert_eq!(game.players[p0_idx].cards[0].card().rank, Rank::Two);
            assert_eq!(game.players[p0_idx].cards[0].card().suit, Suit::Spades);

            // Unmarked rankless card converts to a suitless Ace
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RankBoost,
                    target_id: Some(p0_id),
                    card_index: Some(2),
                },
            )
            .unwrap();
            assert!(matches!(game.players[p0_idx].cards[2], ExpandedCard::PlayingCard(_)));
            assert_eq!(game.players[p0_idx].cards[2].card().rank, Rank::Ace);
            assert_eq!(game.players[p0_idx].cards[2].card().suit, Suit::Suitless);

            // PlayingCard with Rank::None converts to an Ace while preserving suitless
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RankBoost,
                    target_id: Some(p0_id),
                    card_index: Some(3),
                },
            )
            .unwrap();
            assert!(matches!(game.players[p0_idx].cards[3], ExpandedCard::PlayingCard(_)));
            assert_eq!(game.players[p0_idx].cards[3].card().rank, Rank::Ace);
            assert_eq!(game.players[p0_idx].cards[3].card().suit, Suit::Suitless);

            let err = game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RankBoost,
                    target_id: Some(p0_id),
                    card_index: Some(4),
                },
            );
            assert_eq!(err.unwrap_err(), GameError::InvalidAction);
            assert_eq!(game.players[p0_idx].special_cards, vec![SpecialCard::RankBoost]);
        }

        #[test]
        fn test_suit_match_matches_other_hole_card_suit() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();

            game.players[p0_idx].cards = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_card("k", "h"),
                ExpandedCard::get_card("a", "h"),
            ];
            game.players[p0_idx].card_visibility = vec![vec![], vec![], vec![]];
            game.players[p0_idx].special_cards.push(SpecialCard::SuitMatch);

            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::SuitMatch,
                    target_id: Some(p0_id),
                    card_index: Some(0),
                },
            )
            .unwrap();
            assert_eq!(game.players[p0_idx].cards[0].card().rank, Rank::Two);
            assert_eq!(game.players[p0_idx].cards[0].card().suit, Suit::Hearts);

            // Rejects when no other suited PlayingCard exists in hand
            game.players[p0_idx].cards = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_joker(),
            ];
            game.players[p0_idx].special_cards.push(SpecialCard::SuitMatch);
            let err = game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::SuitMatch,
                    target_id: Some(p0_id),
                    card_index: Some(0),
                },
            );
            assert_eq!(err.unwrap_err(), GameError::InvalidAction);
            assert_eq!(game.players[p0_idx].special_cards, vec![SpecialCard::SuitMatch]);
        }

        #[test]
        fn test_redraw_hand_replaces_all_cards_clears_reveals_and_uses_draw_rules() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_idx = 1 - p0_idx;
            let p1_id = game.players[p1_idx].id;

            game.players[p1_idx].cards = vec![
                ExpandedCard::get_card("2", "c"),
                ExpandedCard::get_card("3", "d"),
            ];
            game.players[p1_idx].card_visibility = vec![vec![], vec![]];
            let initial_p1_cards = game.players[p1_idx].cards.clone();

            game.players[p0_idx].special_cards.push(SpecialCard::RevealOpponentCard);
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::RevealOpponentCard,
                    target_id: Some(p1_id),
                    card_index: Some(0),
                },
            )
            .unwrap();
            assert!(game.players[p1_idx].card_visibility[0].contains(&p0_id));

            game.player_move(p0_id, Call).unwrap();

            // Place non-Hearts on top of deck so DrawHeart must skip them on index 0
            game.deck.push(ExpandedCard::get_card("4", "c"));
            game.deck.push(ExpandedCard::get_card("5", "s"));
            game.players[p1_idx].special_cards.push(SpecialCard::DrawHeart);
            game.players[p1_idx].special_cards.push(SpecialCard::RedrawHand);

            game.player_move(
                p1_id,
                PlaySpecial {
                    card: SpecialCard::DrawHeart,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();
            game.player_move(
                p1_id,
                PlaySpecial {
                    card: SpecialCard::RedrawHand,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();

            assert_eq!(game.players[p1_idx].cards.len(), 2);
            assert_eq!(game.players[p1_idx].cards[0].card().suit, Suit::Hearts);
            assert_ne!(game.players[p1_idx].cards, initial_p1_cards);
            assert!(game.players[p1_idx].card_visibility.iter().all(|v| v.is_empty()));
            assert!(game.special.active_modifiers().is_empty());
        }

        #[test]
        fn test_loss_insurance_stacks_multiplicatively_and_pays_on_fold_and_showdown() {
            // 1. Fold refund with multiplicative stacking: 40% + 40% -> 64%
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_idx = 1 - p0_idx;
            let p1_id = game.players[p1_idx].id;

            game.players[p0_idx].special_cards.push(SpecialCard::LossInsurance);
            game.players[p0_idx].special_cards.push(SpecialCard::LossInsurance);
            for _ in 0..2 {
                game.player_move(
                    p0_id,
                    PlaySpecial {
                        card: SpecialCard::LossInsurance,
                        target_id: None,
                        card_index: None,
                    },
                )
                .unwrap();
            }
            assert_eq!(game.special.active_modifiers().len(), 1);
            match game.special.active_modifiers()[0].effect {
                ModifierEffect::LossInsurance { rebate_rate, .. } => {
                    assert!((rebate_rate - 0.64).abs() < 1e-6);
                }
                other => panic!("Expected LossInsurance modifier, got {:?}", other),
            }

            game.player_move(p0_id, Raise { amount: 100 }).unwrap();
            game.player_move(p1_id, Raise { amount: 50 }).unwrap();

            let risked = game.players[p0_idx].total_bet;
            let chips_before_fold = game.players[p0_idx].chips;
            game.player_move(p0_id, Fold).unwrap();
            let expected_fold_refund = ((risked as f64) * 0.64).round() as i32;
            assert_eq!(game.players[p0_idx].chips, chips_before_fold + expected_fold_refund);

            // 2. Showdown loss refund (40%)
            let mut showdown_game = started_game();
            let s0_id = showdown_game.get_current_turn_player_id();
            let s0_idx = showdown_game.get_player_index(s0_id).unwrap();
            let s1_idx = 1 - s0_idx;
            let s1_id = showdown_game.players[s1_idx].id;

            showdown_game.players[s0_idx].special_cards.push(SpecialCard::LossInsurance);
            showdown_game.player_move(
                s0_id,
                PlaySpecial {
                    card: SpecialCard::LossInsurance,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();
            showdown_game.player_move(s0_id, Raise { amount: 100 }).unwrap();
            showdown_game.player_move(s1_id, Call).unwrap();

            while matches!(showdown_game.round, Round::Flop | Round::Turn) {
                let turn_id = showdown_game.get_current_turn_player_id();
                showdown_game.player_move(turn_id, Call).unwrap();
            }
            assert_eq!(showdown_game.round, Round::River);

            showdown_game.players[s0_idx].cards = vec![
                ExpandedCard::get_card("2", "c"),
                ExpandedCard::get_card("7", "d"),
            ];
            showdown_game.players[s1_idx].cards = vec![
                ExpandedCard::get_card("a", "s"),
                ExpandedCard::get_card("a", "h"),
            ];
            showdown_game.community = vec![
                ExpandedCard::get_card("a", "d"),
                ExpandedCard::get_card("a", "c"),
                ExpandedCard::get_card("k", "s"),
                ExpandedCard::get_card("q", "h"),
                ExpandedCard::get_card("9", "s"),
            ];

            let s0_risked = showdown_game.special.hand_bets[&s0_id];
            let s0_chips_before = showdown_game.players[s0_idx].chips;
            while showdown_game.round == Round::River {
                let turn_id = showdown_game.get_current_turn_player_id();
                showdown_game.player_move(turn_id, Call).unwrap();
            }
            if showdown_game.round == Round::WinnerDraft {
                let _ = showdown_game.timeout_winner_draft();
            }
            assert_eq!(showdown_game.round, Round::Showdown);
            assert_eq!(
                showdown_game.players[s0_idx].chips,
                s0_chips_before + ((s0_risked as f64) * 0.40).round() as i32
            );

            // 3. WithdrawBet must not double-dip with LossInsurance
            let mut wb_game = started_game();
            let w0_id = wb_game.get_current_turn_player_id();
            let w0_idx = wb_game.get_player_index(w0_id).unwrap();
            let w1_idx = 1 - w0_idx;
            let w1_id = wb_game.players[w1_idx].id;

            wb_game.players[w0_idx].special_cards.push(SpecialCard::LossInsurance);
            wb_game.players[w0_idx].special_cards.push(SpecialCard::WithdrawBet);
            wb_game.player_move(
                w0_id,
                PlaySpecial {
                    card: SpecialCard::LossInsurance,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();
            wb_game.player_move(w0_id, Raise { amount: 100 }).unwrap();
            wb_game.player_move(w1_id, Raise { amount: 50 }).unwrap();

            let risked_before_wb = wb_game.special.hand_bets[&w0_id];
            let chips_before_wb = wb_game.players[w0_idx].chips;
            wb_game.player_move(
                w0_id,
                PlaySpecial {
                    card: SpecialCard::WithdrawBet,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();
            assert_eq!(wb_game.players[w0_idx].chips, chips_before_wb + risked_before_wb);
        }

        #[test]
        fn test_bet_rebate_tracks_blinds_and_bets_separately_and_pays_after_three_hands() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_idx = 1 - p0_idx;
            let p1_id = game.players[p1_idx].id;

            // Hand 1: activate first BetRebate, raise 100, then p1 folds
            let h1_before = game.special.hand_bets.get(&p0_id).copied().unwrap_or(0);
            game.players[p0_idx].special_cards.push(SpecialCard::BetRebate);
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::BetRebate,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();
            game.player_move(p0_id, Raise { amount: 100 }).unwrap();
            let h1_tracked = game.special.hand_bets[&p0_id] - h1_before;
            game.player_move(p1_id, Fold).unwrap();
            let mut events = Vec::new();
            game.end_hand(&mut events);

            // Hand 2: new hand blind is tracked in BetRebate #1; activate independent BetRebate #2
            game.start_new_hand(&mut events);
            let h2_blind = game.special.hand_bets.get(&p0_id).copied().unwrap_or(0);
            assert!(h2_blind > 0);

            if game.get_current_turn_player_id() != p0_id {
                game.player_move(p1_id, Call).unwrap();
            }
            game.players[p0_idx].special_cards.push(SpecialCard::BetRebate);
            game.player_move(
                p0_id,
                PlaySpecial {
                    card: SpecialCard::BetRebate,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();
            assert_eq!(
                game.special.active_modifiers().len(),
                2,
                "BetRebate uses StackMode::Separate so both trackers run independently"
            );

            let h2_before_raise = game.special.hand_bets[&p0_id];
            game.player_move(p0_id, Raise { amount: 50 }).unwrap();
            let h2_raise = game.special.hand_bets[&p0_id] - h2_before_raise;
            game.player_move(p1_id, Fold).unwrap();
            game.end_hand(&mut events);

            // Hand 3: ending hand 3 expires BetRebate #1 (20% of Hand 1 raise + Hand 2 blind & raise + Hand 3 blind)
            game.start_new_hand(&mut events);
            let h3_blind = game.special.hand_bets.get(&p0_id).copied().unwrap_or(0);
            let chips_before_h3_end = game.players[p0_idx].chips;
            game.end_hand(&mut events);

            let expected_r1 = (((h1_tracked + h2_blind + h2_raise + h3_blind) as f64) * 0.20).round() as i32;
            assert_eq!(game.players[p0_idx].chips, chips_before_h3_end + expected_r1);
            assert_eq!(game.special.active_modifiers().len(), 1);

            // Hand 4: ending hand 4 expires BetRebate #2 (20% of Hand 2 raise + Hand 3 blind + Hand 4 blind)
            game.start_new_hand(&mut events);
            let h4_blind = game.special.hand_bets.get(&p0_id).copied().unwrap_or(0);
            let chips_before_h4_end = game.players[p0_idx].chips;
            game.end_hand(&mut events);

            let expected_r2 = (((h2_raise + h3_blind + h4_blind) as f64) * 0.20).round() as i32;
            assert_eq!(game.players[p0_idx].chips, chips_before_h4_end + expected_r2);
            assert!(game.special.active_modifiers().is_empty());
        }

        #[test]
        fn test_chip_stipend_scales_with_ante_and_stacks_by_duration() {
            let mut game = started_game();
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();

            game.players[p0_idx].special_cards.push(SpecialCard::ChipStipend);
            game.players[p0_idx].special_cards.push(SpecialCard::ChipStipend);

            for _ in 0..2 {
                game.player_move(
                    p0_id,
                    PlaySpecial {
                        card: SpecialCard::ChipStipend,
                        target_id: None,
                        card_index: None,
                    },
                )
                .unwrap();
            }
            assert_eq!(
                game.special.active_modifiers().len(),
                1,
                "ChipStipend should merge duration cleanly into a single modifier"
            );

            let start_chips = game.players[p0_idx].chips;
            let mut events = Vec::new();
            game.end_hand(&mut events);
            assert_eq!(game.players[p0_idx].chips, start_chips + 20);

            game.start_new_hand(&mut events);
            let turn_id = game.get_current_turn_player_id();
            let turn_idx = game.get_player_index(turn_id).unwrap();
            game.players[turn_idx].special_cards.push(SpecialCard::AnteUp);
            game.player_move(
                turn_id,
                PlaySpecial {
                    card: SpecialCard::AnteUp,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();

            for hand in 2..=8 {
                let before = game.players[p0_idx].chips;
                game.end_hand(&mut events);
                assert_eq!(
                    game.players[p0_idx].chips,
                    before + 40,
                    "Hand {hand} should pay 1x ante-scaled blind (40)"
                );
            }

            // Stipend expired after hand 8; hand 9 pays nothing
            let before_h9 = game.players[p0_idx].chips;
            game.end_hand(&mut events);
            assert_eq!(game.players[p0_idx].chips, before_h9);
        }

        #[test]
        fn test_draw_secret_card_deals_revealed_hole_cards_and_community_card() {
            let mut game = Game::new();
            let config = GameConfig {
                players: vec![
                    ConfigPlayer::Human { id: 0 },
                    ConfigPlayer::Human { id: 1 },
                    ConfigPlayer::Human { id: 2 },
                ],
                rng_seed: 0,
                blind_size: 20,
                min_raise: 10,
                starting_chips: 1000,
                special_card_limit: 3,
                deck_type: DeckType::Standard,
                max_players: 3,
                round_limit: 30,
                max_bet_multiplier: None,
            };
            game.update_config(config).unwrap();
            game.initialise().unwrap();
            game.start().unwrap();

            let caster_id = game.get_current_turn_player_id();
            let caster_idx = game.get_player_index(caster_id).unwrap();
            let active_opp_idx = (caster_idx + 1) % 3;
            let folded_opp_idx = (caster_idx + 2) % 3;
            game.players[folded_opp_idx].folded = true;

            game.players[caster_idx].special_cards.push(SpecialCard::Draw);
            assert!(game.community.is_empty());

            game.player_move(
                caster_id,
                PlaySpecial {
                    card: SpecialCard::Draw,
                    target_id: None,
                    card_index: None,
                },
            )
            .unwrap();

            assert_eq!(game.players[caster_idx].cards.len(), 4);
            assert_eq!(game.players[caster_idx].card_visibility[2], vec![0, 1, 2]);
            assert_eq!(game.players[caster_idx].card_visibility[3], vec![0, 1, 2]);
            assert_eq!(game.players[active_opp_idx].cards.len(), 3);
            assert_eq!(game.players[active_opp_idx].card_visibility[2], vec![0, 1, 2]);
            assert_eq!(game.players[folded_opp_idx].cards.len(), 2);
            assert_eq!(game.community.len(), 1);
        }
    }

    #[cfg(test)]
    mod game_tests {
        use crate::Action::{Raise, Call, Fold, PlaySpecial};
        use crate::api::view_game_state;
        use crate::ai::{AI, AIType};
        use crate::{Action, ConfigPlayer, DeckType, Game, GameConfig, GameError, MoveAction, MoveEvent, PlayerType, Round, SpecialCard};
        use crate::poker::*;
        use crate::special::*;

        fn get_configured_started_game(player_count: usize, bot_count: usize) -> Game {
            let mut game = Game::new();
            let _ = game.update_config(create_valid_config(player_count, bot_count));
            let _ = game.initialise();
            let _ = game.start();

            game
        }

        fn create_valid_config(player_count: usize, bot_count: usize) -> GameConfig {
            let mut players = Vec::with_capacity(player_count + bot_count);

            for id in 0..player_count {
                players.push(ConfigPlayer::Human { id });
            }

            for offset in 0..bot_count {
                players.push(ConfigPlayer::Bot {
                    id: player_count + offset,
                    ai_type: AIType::Smart,
                });
            }

            GameConfig { 
                players, 
                rng_seed: 0, 
                blind_size: 20, 
                min_raise: 10, 
                starting_chips: 1000, 
                special_card_limit: 3, 
                deck_type: DeckType::Standard, 
                max_players: player_count + bot_count, 
                round_limit: 30,
                max_bet_multiplier: None,
            }
        }

        fn first_hand_beats_second_hand(first_hand: Vec<ExpandedCard>, second_hand: Vec<ExpandedCard>) -> bool {
            let mut game = get_configured_started_game(2, 0);

            let (p1_id, p2_id) = (game.players[0].id as i32, game.players[1].id as i32);

            // preflop
            make_move_and_end_turn(Call, game.players[game.turn_index].id, &mut game);
            make_move_and_end_turn(Call, game.players[game.turn_index].id, &mut game);
            // flop
            make_move_and_end_turn(Call, game.players[game.turn_index].id, &mut game);
            make_move_and_end_turn(Call, game.players[game.turn_index].id, &mut game);
            // river
            make_move_and_end_turn(Call, game.players[game.turn_index].id, &mut game);
            make_move_and_end_turn(Call, game.players[game.turn_index].id, &mut game);
            // turn
            game.community.clear();

            let p1_idx = game.get_player_index(p1_id as usize).unwrap();
            game.players[p1_idx].cards = first_hand;

            let p2_idx = game.get_player_index(p2_id as usize).unwrap();
            game.players[p2_idx].cards = second_hand;

            make_move_and_end_turn(Raise { amount: 10000 }, game.players[game.turn_index].id, &mut game);
            make_move_and_end_turn(Call, game.players[game.turn_index].id, &mut game);

            game.players[p1_idx].chips == game.ctx.starting_chips * 2 && game.players[p2_idx].chips == 0
        }

        fn make_move_and_end_turn(player_move: Action, player_id: usize, game: &mut Game) {
            let _ = game.player_move(player_id, player_move);
        }

        #[test]
        fn test_game_creation_and_config_update() {
            let mut game = Game::new();
            assert_eq!(game.round, Round::Room);

            let config = create_valid_config(2, 0);
            let err = game.update_config(config);
            assert!(err.is_ok());
        }

        #[test]
        fn test_initialise_transition() {
            let mut game = Game::new();
            let _ = game.update_config(create_valid_config(2, 2));

            let _ =game.initialise();
            assert_eq!(game.round, Round::Preround);

            let state = view_game_state(&game, 0);
            let players = state.players;

            assert_eq!(players.len(), 4);
            for player in players {
                assert_eq!(player.chips, 1000);
                assert_eq!(player.folded, false);
            }
        }

        #[test]
        fn test_round_transition_from_preround_to_preflop() {
            let game = get_configured_started_game(2, 0);

            let state = view_game_state(&game, 0);
            let players = state.players;

            // players should have received their hole cards
            assert_eq!(players[0].hole_cards.len(), 2);
            assert_eq!(players[1].hole_cards.len(), 2);

            // and should have paid blinds
            assert_eq!(players[0].total_bet, 10, "Expected total bet to be 10 (small blind), was {}", players[0].total_bet);
            assert_eq!(players[1].total_bet, 20, "Expected total bet to be 20 (big blind), was {}", players[1].total_bet);
        }

        #[test]
        fn test_player_moves_and_turn_rotation() {
            let mut game = get_configured_started_game(2, 0);

            let active_player = game.get_current_turn_player_id();
            
            let _ = game.player_move(active_player, Call);
            
            let state = view_game_state(&game, active_player);
            assert_eq!(state.highest_bet, 20);
        }

        #[test]
        fn test_bot_execution_without_panic() {
            // 1 human, 1 bot
            let mut game = get_configured_started_game(1, 1);

            let human_id = game.players.iter().find(|p| match p.player_type {
                PlayerType::Human => true,
                _ => false
            }).map(|p| p.id).unwrap_or(0);

            // make a move if it is currently the player's turn
            if game.get_current_turn_player_id() == human_id {
                let _ = game.player_move(human_id, Call);
            }

            assert!(matches!(game.round, Round::Preflop | Round::Flop));
        }

        #[test]
        fn test_midgame_add_and_remove_player() {
            let mut game = get_configured_started_game(2, 0);
            
            let add_res_str = game.try_add_player(PlayerType::Human);
            assert!(add_res_str.is_err(), "Adding player mid-hand should fail");
        }

        #[test]
        fn test_game_ends_after_no_humans_remain() {
            let mut game = get_configured_started_game(1, 1);

            let human_id = game.players.iter().find(|p| match p.player_type {
                PlayerType::Human => true,
                _ => false
            }).map(|p| p.id).unwrap_or(0);

            let _ = game.player_move(human_id, Fold);

            assert_eq!(game.round, Round::Showdown, "Expected round to end after last human player folded, instead round is {}", game.round);
        }

        #[test]
        fn test_game_round_progresses_after_moves() {
            let mut game = get_configured_started_game(2, 0);

            assert_eq!(game.round, Round::Preflop, "Game round expected to be Preflop, actual: {}", game.round);

            make_move_and_end_turn(Call, game.players[game.turn_index].id, &mut game);
            make_move_and_end_turn(Call, game.players[game.turn_index].id, &mut game);

            assert_eq!(game.round, Round::Flop, "Game round expected to be Flop, actual: {}", game.round);
        }

        #[test]
        fn test_complex_pot_paid_correctly() {
            let mut game = Game::new();
            let mut config = create_valid_config(3, 0);
            config.special_card_limit = 0;
            _ = game.update_config(config);
            _ = game.initialise();

            let (p1_id, p2_id, p3_id) = (game.players[0].id, game.players[1].id, game.players[2].id);
            game.players[0].chips = 200;
            game.players[1].chips = 500;
            game.players[2].chips = 1000;

            _ = game.start();

            // p1 has 4 aces (wins overall round)
            game.players[0].cards = vec![
                ExpandedCard::get_card("a", "s"), 
                ExpandedCard::get_card("a", "s"), 
                ExpandedCard::get_card("a", "s"), 
                ExpandedCard::get_card("a", "s")
            ];
            // p2 has 4 kings (second)
            game.players[1].cards = vec![
                ExpandedCard::get_card("k", "s"), 
                ExpandedCard::get_card("k", "s"), 
                ExpandedCard::get_card("k", "s"), 
                ExpandedCard::get_card("k", "s")
            ];
            // p3 has 4 queens (loses)
            game.players[2].cards = vec![
                ExpandedCard::get_card("q", "s"), 
                ExpandedCard::get_card("q", "s"), 
                ExpandedCard::get_card("q", "s"), 
                ExpandedCard::get_card("q", "s")
            ];

            // all players go all in
            make_move_and_end_turn(Raise { amount: 10000 }, p1_id, &mut game);
            make_move_and_end_turn(Raise { amount: 10000 }, p2_id, &mut game);
            make_move_and_end_turn(Raise { amount: 10000 }, p3_id, &mut game);

            // p1 has 600 chips at the end (200 from p1, p2, and p3)
            assert_eq!(game.players[0].chips, 600, "Expected P1 to have 600 chips. Instead P1: {}, P2: {}, P3: {}", game.players[0].chips, game.players[1].chips, game.players[2].chips);

            // p2 has 600 chips at the end (300 from p2, and p3)
            assert_eq!(game.players[1].chips, 600, "Expected P2 to have 600 chips. Instead P1: {}, P2: {}, P3: {}", game.players[0].chips, game.players[1].chips, game.players[2].chips);

            // p3 has 500 chips at the end (lost 200 to p1, 300 to p2)
            assert_eq!(game.players[2].chips, 500);
        }

        #[test]
        fn test_full_house_beats_flush() {
            // full house of king/10
            let first_hand = vec![
                ExpandedCard::get_card("k", "d"), 
                ExpandedCard::get_card("k", "s"), 
                ExpandedCard::get_card("k", "c"), 
                ExpandedCard::get_card("10", "s"), 
                ExpandedCard::get_card("10", "h")
            ];

            // flush, ace high
            let second_hand = vec![
                ExpandedCard::get_card("a", "h"), 
                ExpandedCard::get_card("q", "h"), 
                ExpandedCard::get_card("j", "h"), 
                ExpandedCard::get_card("9", "h"), 
                ExpandedCard::get_card("7", "h")
            ];

            assert!(first_hand_beats_second_hand(first_hand, second_hand))
        }

        #[test]
        fn test_high_flush_beats_low_flush() {
            // flush, ace high
            let first_hand = vec![
                ExpandedCard::get_card("a", "h"), 
                ExpandedCard::get_card("q", "h"), 
                ExpandedCard::get_card("j", "h"), 
                ExpandedCard::get_card("9", "h"), 
                ExpandedCard::get_card("7", "h")
            ];

            // flush, king high
            let second_hand = vec![
                ExpandedCard::get_card("k", "d"), 
                ExpandedCard::get_card("j", "d"), 
                ExpandedCard::get_card("10", "d"), 
                ExpandedCard::get_card("8", "d"), 
                ExpandedCard::get_card("7", "d")
            ];

            assert!(first_hand_beats_second_hand(first_hand, second_hand))
        }

        #[test]
        fn test_better_2pair_kickers_win() {
            // two pair, king+9 with jack kicker
            let first_hand = vec![
                ExpandedCard::get_card("k", "d"), 
                ExpandedCard::get_card("k", "s"), 
                ExpandedCard::get_card("9", "h"), 
                ExpandedCard::get_card("9", "c"), 
                ExpandedCard::get_card("j", "c")
            ];

            // two pair, king+9 with 8 kicker
            let second_hand = vec![
                ExpandedCard::get_card("k", "d"), 
                ExpandedCard::get_card("k", "s"), 
                ExpandedCard::get_card("9", "h"), 
                ExpandedCard::get_card("9", "c"), 
                ExpandedCard::get_card("8", "d")
            ];

            assert!(first_hand_beats_second_hand(first_hand, second_hand))
        }

        #[test]
        fn test_ace_low_straight_detected() {
            let mut game = Game::new();
            let _ = game.update_config(create_valid_config(2, 0));

            let cards = vec![
                ExpandedCard::get_card("a", "d"), 
                ExpandedCard::get_card("2", "d"), 
                ExpandedCard::get_card("3", "d"), 
                ExpandedCard::get_card("4", "d"), 
                ExpandedCard::get_card("5", "d")
            ];
            let hand = Hand::new(&cards, &game.ctx.hand_fns, &game.special.invalidated_hands(), game.ctx.hand_size);

            assert_eq!(hand.hand_type, HandType::StraightFlush);
        }

        #[test]
        fn test_ace_high_straight_detected() {
            let mut game = Game::new();
            let _ = game.update_config(create_valid_config(2, 0));
        
            let cards = vec![
                ExpandedCard::get_card("a", "d"), 
                ExpandedCard::get_card("k", "d"), 
                ExpandedCard::get_card("q", "d"), 
                ExpandedCard::get_card("j", "d"), 
                ExpandedCard::get_card("10", "d")
            ];
            let hand = Hand::new(&cards, &game.ctx.hand_fns, &game.special.invalidated_hands(), game.ctx.hand_size);

            assert_eq!(hand.hand_type, HandType::RoyalFlush);
        }    

        #[test]
        fn test_4oak_detected() {
            let mut game = Game::new();
            let _ = game.update_config(create_valid_config(2, 0));

            let cards = vec![
                ExpandedCard::get_card("a", "d"), 
                ExpandedCard::get_card("a", "s"), 
                ExpandedCard::get_card("a", "c"), 
                ExpandedCard::get_card("a", "h")
                ];
            let hand = Hand::new(&cards, &game.ctx.hand_fns, &game.special.invalidated_hands(), game.ctx.hand_size);

            assert_eq!(hand.hand_type, HandType::FourOfAKind);
        }

        #[test]
        fn test_flush_detected() {
            let mut game = Game::new();
            let _ = game.update_config(create_valid_config(2, 0));
        
            let cards = vec![
                ExpandedCard::get_card("a", "d"), 
                ExpandedCard::get_card("3", "d"), 
                ExpandedCard::get_card("5", "d"), 
                ExpandedCard::get_card("7", "d"), 
                ExpandedCard::get_card("9", "d")
            ];
            let hand = Hand::new(&cards, &game.ctx.hand_fns, &game.special.invalidated_hands(), game.ctx.hand_size);

            assert_eq!(hand.hand_type, HandType::Flush);
        }

        #[test]
        fn test_turn_advances_immediately_when_acting_even_with_special_cards() {
            let mut game = get_configured_started_game(2, 0);

            let active_player_id = game.get_current_turn_player_id();
            let active_idx = game.get_player_index(active_player_id as usize).unwrap();

            // gives current player a special card
            game.players[active_idx].special_cards.push(SpecialCard::ChipBoost);

            // player calls while holding special card - turn ends immediately
            let _ = game.player_move(active_player_id, Call);

            assert_ne!(game.get_current_turn_player_id(), active_player_id, "Turn should advance immediately after primary move");
            assert_eq!(game.players[active_idx].acted, true);
        }

        #[test]
        fn test_turn_advances_automatically_when_player_has_no_special_cards() {
            let mut game = get_configured_started_game(2, 0);

            let p1_id = game.get_current_turn_player_id();
            let p1_idx = game.get_player_index(p1_id as usize).unwrap();

            // player has no special cards
            game.players[p1_idx].special_cards.clear();
            let _ = game.player_move(p1_id, Call);

            assert_ne!(game.get_current_turn_player_id(), p1_id, "Turn should automatically advance to next player when no special cards are held");
        }

        #[test]
        fn test_turn_advances_immediately_on_fold_even_with_special_cards() {
            let mut game = get_configured_started_game(2, 0);

            let active_player_id = game.get_current_turn_player_id();
            let active_idx = game.get_player_index(active_player_id as usize).unwrap();

            // gives the current player a special card
            game.players[active_idx].special_cards.push(SpecialCard::ChipBoost);
            let _ = game.player_move(active_player_id, Fold);

            assert_eq!(game.players[active_idx].folded, true);
            assert_ne!(game.get_current_turn_player_id(), active_player_id, "Folding must immediately yield turn even if special cards remain");
        }

        #[test]
        fn test_zero_chip_player_skipped_in_turn_order() {
            let mut game = get_configured_started_game(3, 0);

            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_idx = (p0_idx + 1) % 3;
            let p1_id = game.players[p1_idx].id;

            // set p1 chips to 0 (all-in)
            game.players[p1_idx].chips = 0;
            game.players[p1_idx].round_bet = 10;

            // p0 raises
            let _ = game.player_move(p0_id, Action::Raise { amount: 50 });

            // turn should skip p1 (0 chips)
            assert_ne!(game.get_current_turn_player_id(), p1_id, "Player with 0 chips must be skipped in turn order");
        }

        #[test]
        fn test_all_in_player_with_special_cards_retains_turn_opportunity() {
            let mut game = get_configured_started_game(2, 0);

            let p0_idx = game.get_player_index(game.get_current_turn_player_id()).unwrap();
            game.players[p0_idx].chips = 0;
            game.players[p0_idx].special_cards.push(SpecialCard::ChipBoost);

            assert!(!game.is_round_complete(), "Round must not complete if all-in players hold unplayed special cards");
        }

        #[test]
        fn test_playing_special_card_consumes_card_and_keeps_turn() {
            let mut game = get_configured_started_game(2, 0);

            let p1_id = game.get_current_turn_player_id();
            let p1_idx = game.get_player_index(p1_id as usize).unwrap();

            game.players[p1_idx].special_cards.push(SpecialCard::ChipBoost);

            // play owned special card
            let _ = game.player_move(p1_id, PlaySpecial { card: SpecialCard::ChipBoost, target_id: None, card_index: None });

            assert!(game.players[p1_idx].special_cards.is_empty());
            assert_eq!(game.get_current_turn_player_id(), p1_id, "turn should not change after playing special card");
        }

        #[test]
        fn test_special_card_block_prevents_play_and_allows_turn_auto_completion() {
            let mut game = get_configured_started_game(2, 0);

            let p1_id = game.get_current_turn_player_id();
            let p1_idx = game.get_player_index(p1_id as usize).unwrap();

            // player plays special block card
            game.players[p1_idx].special_cards.push(SpecialCard::SpecialBlock);
            let _ = game.player_move(p1_id, PlaySpecial { card: SpecialCard::SpecialBlock, target_id: None, card_index: None });
            make_move_and_end_turn(Call, p1_id, &mut game);

            let p2_id = game.get_current_turn_player_id();
            let p2_idx = game.get_player_index(p2_id as usize).unwrap();

            // second player attempts to play special card
            game.players[p2_idx].special_cards.push(SpecialCard::ChipBoost);
            let result = game.player_move(p2_id, PlaySpecial { card: SpecialCard::ChipBoost, target_id: None, card_index: None });
            assert!(result.is_err());

            let _ = game.player_move(p2_id, Call);
            assert_ne!(game.get_current_turn_player_id(), p2_id, "Turn should auto-complete when special cards are blocked");
        }

        #[test]
        fn invalid_special_card_index_returns_error() {
            let mut game = get_configured_started_game(2, 0);
            let player_id = game.get_current_turn_player_id();
            let player_index = game.get_player_index(player_id as usize).unwrap();

            game.players[player_index].special_cards.push(SpecialCard::ReplaceCardSelf);

            let before = game.players[player_index].cards.clone();
            let result = game.player_move(
                player_id,
                PlaySpecial { card: SpecialCard::ReplaceCardSelf, target_id: None, card_index: Some(999) },
            );

            assert!(result.is_err());
            assert_eq!(game.players[player_index].cards, before);
            assert!(game.players[player_index].special_cards.contains(&SpecialCard::ReplaceCardSelf));
        }

        #[test]
        fn start_emits_one_hole_deal_per_card_and_player() {
            let mut game = Game::new();
            game.update_config(create_valid_config(2, 0)).unwrap();
            game.initialise().unwrap();

            let events = game.start().unwrap();

            let hole_deals: Vec<_> = events
                .iter()
                .filter(|event| {
                    matches!(
                        event.action,
                        MoveAction::DealHole { count: 1 }
                    )
                })
                .collect();

            assert_eq!(hole_deals.len(), 4);
        }

        #[test]
        fn community_deal_is_public_and_has_no_player_owner() {
            let mut game = get_configured_started_game(2, 0);

            let mut events = Vec::new();
            game.deal_community_cards(1, &mut events);

            assert!(matches!(
                events.as_slice(),
                [MoveEvent {
                    actor_id: None,
                    action: MoveAction::DealCommunity { count: 1 },
                    private: false,
                }]
            ));
        }

        #[test]
        fn capped_special_draw_emits_no_false_deal_event() {
            let mut game = get_configured_started_game(2, 0);
            let player_index = 0;

            game.players[player_index].special_cards =
                vec![SpecialCard::ChipBoost; game.ctx.special_card_limit];

            let mut events = Vec::new();
            game.deal_special_cards(1, &mut events);

            assert!(!events.iter().any(|event| {
                matches!(event.action, MoveAction::DealSpecial { .. })
                    && event.actor_id == Some(game.players[player_index].id)
            }));
        }

        #[test]
        fn rejects_second_normal_action_in_a_turn() {
            let mut game = get_configured_started_game(2, 0);
            let index = game.turn_index;
            let mut events = Vec::new();

            game.apply_action(index, Action::Call, &mut events).unwrap();

            assert_eq!(
                game.apply_action(index, Action::Raise { amount: 10 }, &mut events),
                Err(GameError::PrimaryMoveAlreadyMade),
            );
        }        

        #[test]
        fn allows_special_at_start_of_turn() {
            let mut game = get_configured_started_game(2, 0);
            let index = game.turn_index;
            game.players[index]
                .special_cards
                .push(SpecialCard::ChipBoost);

            let mut events = Vec::new();

            // play special card at start of turn (before acting)
            game.apply_action(
                index,
                Action::PlaySpecial {
                    card: SpecialCard::ChipBoost,
                    target_id: None,
                    card_index: None,
                },
                &mut events,
            ).unwrap();

            game.apply_action(index, Action::Call, &mut events).unwrap();

            assert!(game.players[index].acted);
        }

        #[test]
        fn rejects_special_after_primary_action() {
            let mut game = get_configured_started_game(2, 0);
            let index = game.turn_index;
            game.players[index]
                .special_cards
                .push(SpecialCard::ChipBoost);

            let mut events = Vec::new();

            game.apply_action(index, Action::Call, &mut events).unwrap();

            assert_eq!(
                game.apply_action(
                    index,
                    Action::PlaySpecial {
                        card: SpecialCard::ChipBoost,
                        target_id: None,
                        card_index: None,
                    },
                    &mut events,
                ),
                Err(GameError::PrimaryMoveAlreadyMade),
            );
        }

        #[test]
        fn test_zero_chip_player_eliminated_in_next_hand() {
            let mut game = Game::new();
            let config = create_valid_config(3, 0);
            _ = game.update_config(config);
            _ = game.initialise();
            _ = game.start();

            // player 0 loses all chips
            game.players[0].chips = 0;
            game.players[1].chips = 1000;
            game.players[2].chips = 1000;

            // transition to next hand via start_new_hand
            let mut events = Vec::new();
            game.start_new_hand(&mut events);

            // player 0 should be marked folded, as they are eliminated, and not receive cards
            assert!(game.players[0].folded, "0-chip player should be folded automatically");
            assert!(game.players[0].acted, "0-chip player should be marked acted");
            assert!(game.players[0].cards.is_empty(), "0-chip player should not be dealt hole cards");

            // active turn must be assigned to an active player with chips
            assert_ne!(game.turn_index, 0, "0-chip player should not receive turn");
            assert!(game.players[game.turn_index].chips > 0, "Turn player must have chips");
        }

        #[test]
        fn test_game_ends_when_only_one_player_has_chips() {
            let mut game = Game::new();
            let config = create_valid_config(3, 0);
            _ = game.update_config(config);
            _ = game.initialise();
            _ = game.start();

            // players 0 and 1 lose all chips
            game.players[0].chips = 0;
            game.players[1].chips = 0;
            game.players[2].chips = 2000;

            let mut events = Vec::new();
            game.start_new_hand(&mut events);

            // game should end and transition to Room round
            assert_eq!(game.round, Round::Room, "Game should transition to Room round when only 1 player has chips");
        }

        #[test]
        fn test_fold_eliminates_player_from_round_and_awards_pot() {
            let mut game = get_configured_started_game(3, 0);

            let t1 = game.get_current_turn_player_id();
            make_move_and_end_turn(Fold, t1, &mut game);

            let t2 = game.get_current_turn_player_id();
            make_move_and_end_turn(Fold, t2, &mut game);

            // active non-folded players should be 1
            let active_count = game.players.iter().filter(|p| !p.folded).count();
            assert_eq!(active_count, 1, "Only 1 active player should remain after others fold");
        }

        #[test]
        fn test_game_ends_when_all_humans_eliminated() {
            let mut game = Game::new();
            let config = create_valid_config(1, 2); // 1 human, 2 bots
            _ = game.update_config(config);
            _ = game.initialise();
            _ = game.start();

            // human player (player 0) loses all chips
            game.players[0].chips = 0;
            game.players[1].chips = 1200;
            game.players[2].chips = 800;

            let mut events = Vec::new();
            game.start_new_hand(&mut events);

            // game ends, and bots retain their current chip balances
            assert_eq!(game.round, Round::Room, "Game should end when all humans are eliminated");
            assert_eq!(game.players[1].chips, 1200, "Bot 1 should retain its chip balance");
            assert_eq!(game.players[2].chips, 800, "Bot 2 should retain its chip balance");
        }

        #[test]
        fn test_all_in_player_with_specials_advances_turn_on_call() {
            let mut game = get_configured_started_game(2, 0);

            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();

            // set player 0 to 0 chips, forcing an all in, and give them a special card
            game.players[p0_idx].chips = 0;
            game.players[p0_idx].special_cards.push(SpecialCard::ChipBoost);

            // player 0 calls
            let result = game.player_move(p0_id, Action::Call);
            assert!(result.is_ok(), "Game should not stall on all in player move");

            // turn should advance correctly
            let next_turn_id = game.get_current_turn_player_id();
            assert_ne!(next_turn_id, p0_id, "Turn must advance past all in player after they make a move");
        }

        #[test]
        fn test_reveal_opponent_card_cleans_up_on_new_hand() {
            let mut game = get_configured_started_game(2, 0);

            // forces player 1 to see player 0s first hole card through modifying visibility
            game.players[0].card_visibility[0].push(1);

            // start new hand
            let mut events = Vec::new();
            game.start_new_hand(&mut events);

            // check that player 1 can no longer see player 0s card
            assert!(game.players[0].card_visibility[0].is_empty(), "Card visibility must be cleared between hands");
        }

        #[test]
        fn test_special_card_ignores_limit_when_specified() {
            let mut game = get_configured_started_game(2, 0);
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();

            // fill player 0s special cards
            let limit = game.ctx.special_card_limit;
            game.players[p0_idx].special_cards = vec![SpecialCard::ChipBoost; limit];

            let mut events = Vec::new();
            // ensure that player 0 cannot receive any more special cards through typical draw
            let mut limit_ignored = false;
            let err_res = give_special_card(SpecialCard::DrawCardSelf, 1, limit_ignored, p0_id, &mut game, &mut events);
            assert!(err_res.is_err(), "Should respect limit when ignore_limit is false");

            // but is able to when limit is ignored
            limit_ignored = true;
            let ok_res = give_special_card(SpecialCard::DrawCardSelf, 2, limit_ignored, p0_id, &mut game, &mut events);
            assert!(ok_res.is_ok(), "Should exceed limit when ignore_limit is true");
            assert_eq!(game.players[p0_idx].special_cards.len(), limit + 2);
        }

        #[test]
        fn test_bet_capped_at_maximum_threshold() {
            let mut game = get_configured_started_game(2, 0);
            game.ctx.max_bet_multiplier = Some(25);
            // blind cost is 20 and multiplier is 25x, so max bet should be 500
            assert_eq!(game.get_current_max_bet(), Some(500));

            let p_turn = game.get_current_turn_player_id();
            let p_idx = game.get_player_index(p_turn).unwrap();
            game.players[p_idx].chips = 5000;

            // attempt to raise by 2000
            let res = game.player_move(p_turn, Action::Raise { amount: 2000 });
            assert!(res.is_ok());

            // player total bet should still be 500
            assert_eq!(game.players[p_idx].total_bet, 500);
            assert_eq!(game.players[p_idx].round_bet, 500);
            assert_eq!(game.bet, 500);
            assert!(game.is_player_at_max_bet(&game.players[p_idx]));
        }

        #[test]
        fn test_player_at_max_bet_cannot_raise() {
            let mut game = get_configured_started_game(2, 0);
            game.ctx.max_bet_multiplier = Some(25);
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            game.players[p0_idx].chips = 5000;

            // player 0 raises to cap (500)
            let _ = game.player_move(p0_id, Action::Raise { amount: 1000 });
            assert_eq!(game.players[p0_idx].total_bet, 500);

            // error on additional raise
            let mut events = Vec::new();
            let raise_err = game.apply_action(p0_idx, Action::Raise { amount: 100 }, &mut events);
            assert!(raise_err.is_err(), "Player at max bet and should not be able to raise further");
        }

        #[test]
        fn test_player_at_max_bet_all_in_treatment_and_call() {
            let mut game = get_configured_started_game(2, 0);
            game.ctx.max_bet_multiplier = Some(25);
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_idx = (p0_idx + 1) % 2;
            let p1_id = game.players[p1_idx].id;

            game.players[p0_idx].chips = 5000;
            game.players[p1_idx].chips = 5000;

            // player 0 raises to cap (500)
            let _ = game.player_move(p0_id, Action::Raise { amount: 1000 });
            assert_eq!(game.players[p0_idx].total_bet, 500);

            // player 1 can call to match the max bet
            let res = game.player_move(p1_id, Action::Call);
            assert!(res.is_ok());
            assert_eq!(game.players[p1_idx].total_bet, 500);
            assert!(game.is_player_at_max_bet(&game.players[p1_idx]));

            // as both are now at max bet and acted, round must advance
            assert_ne!(game.round, Round::Preflop, "Round should advance when all active players match max bet");
        }

        #[test]
        fn test_max_bet_persists_across_rounds_within_hand() {
            let mut game = get_configured_started_game(2, 0);
            game.ctx.max_bet_multiplier = Some(25);
            // cap is 500 per hand
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_idx = (p0_idx + 1) % 2;
            let p1_id = game.players[p1_idx].id;

            game.players[p0_idx].chips = 5000;
            game.players[p1_idx].chips = 5000;

            // player 0 calls for 20 on preflop
            let _ = game.player_move(p0_id, Action::Call);
            // player 1 calls
            let _ = game.player_move(p1_id, Action::Call);

            // both players should have bet 20 by flop
            assert_eq!(game.round, Round::Flop);
            assert_eq!(game.players[p0_idx].total_bet, 20);
            assert_eq!(game.players[p1_idx].total_bet, 20);

            let flop_turn_id = game.get_current_turn_player_id();
            let flop_turn_idx = game.get_player_index(flop_turn_id).unwrap();

            // attempt to raise by 2000 on the flop
            let res = game.player_move(flop_turn_id, Action::Raise { amount: 2000 });
            assert!(res.is_ok());

            // flop raise must be capped at 480 (reaching 500 total_bet for the hand)
            assert_eq!(game.players[flop_turn_idx].round_bet, 480);
            assert_eq!(game.players[flop_turn_idx].total_bet, 500);
            assert!(game.is_player_at_max_bet(&game.players[flop_turn_idx]));
        }

        #[test]
        fn test_max_bet_scales_dynamically_with_ante_up() {
            let mut game = get_configured_started_game(2, 0);
            game.ctx.max_bet_multiplier = Some(25);
            // cap is 500 chips
            assert_eq!(game.get_current_max_bet(), Some(500));

            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            game.players[p0_idx].chips = 5000;
            game.players[p0_idx].special_cards.push(SpecialCard::AnteUp);

            // play Ante Up special card
            let res = game.player_move(p0_id, Action::PlaySpecial { 
                card: SpecialCard::AnteUp, 
                target_id: None, 
                card_index: None 
            });
            assert!(res.is_ok());

            // ante multiplier is now 2.0, so blind cost is 40 - max bet should now be 1000
            assert_eq!(game.get_current_max_bet(), Some(1000));

            // player can now raise up to 1000
            let raise_res = game.player_move(p0_id, Action::Raise { amount: 2000 });
            assert!(raise_res.is_ok());
            assert_eq!(game.players[p0_idx].round_bet, 1000);
            assert_eq!(game.bet, 1000);
        }

        #[test]
        fn test_uncapped_mode() {
            let mut game = get_configured_started_game(2, 0);
            // set max_bet_multiplier to none, uncapping it
            game.ctx.max_bet_multiplier = None;
            assert_eq!(game.get_current_max_bet(), None);

            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            game.players[p0_idx].chips = 5000;

            // player raises by 2000
            let res = game.player_move(p0_id, Action::Raise { amount: 2000 });
            assert!(res.is_ok());
            // round bet goes through, and total bet is now blind cost (20) + bet (2000)
            assert_eq!(game.players[p0_idx].round_bet, 2020);
        }

        #[test]
        fn test_non_targeting_special_card_succeeds_after_player_zero_folds() {
            let mut game = get_configured_started_game(3, 0);

            // Fold player 0
            let p0_idx = game.get_player_index(0).unwrap();
            game.players[p0_idx].folded = true;
            if game.get_current_turn_player_id() == 0 {
                game.advance_turn_index();
            }

            let active_id = game.get_current_turn_player_id();
            let active_idx = game.get_player_index(active_id).unwrap();
            game.players[active_idx].special_cards.push(SpecialCard::ChipBoost);
            game.players[active_idx].special_cards.push(SpecialCard::DrawCardCommunity);

            // Even if target_id is Some(0) (folded player), non-targeting cards should succeed
            let res_with_zero = game.player_move(
                active_id,
                PlaySpecial {
                    card: SpecialCard::ChipBoost,
                    target_id: Some(0),
                    card_index: Some(0),
                },
            );
            assert!(res_with_zero.is_ok(), "Non-targeting special card should succeed even if target_id is Some(0) and player 0 folded");

            // And with target_id: None
            let res_with_none = game.player_move(
                active_id,
                PlaySpecial {
                    card: SpecialCard::DrawCardCommunity,
                    target_id: None,
                    card_index: None,
                },
            );
            assert!(res_with_none.is_ok(), "Non-targeting special card should succeed with target_id: None");

            // Targeting special card against folded player 0 should still fail with InvalidTargetPlayer
            game.players[active_idx].special_cards.push(SpecialCard::HandSwap);
            let res_targeting_folded = game.player_move(
                active_id,
                PlaySpecial {
                    card: SpecialCard::HandSwap,
                    target_id: Some(0),
                    card_index: None,
                },
            );
            assert!(matches!(res_targeting_folded, Err(GameError::InvalidTargetPlayer)));
        }

        #[test]
        fn test_ante_up_stacks_multiplicatively_and_persists_across_hands() {
            let mut game = get_configured_started_game(2, 0);
            game.ctx.max_bet_multiplier = Some(25);
            assert_eq!(game.special.ante_multiplier(), 1.0);
            assert_eq!(game.get_current_max_bet(), Some(500));

            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            game.players[p0_idx].chips = 10000;
            game.players[p0_idx].special_cards.push(SpecialCard::AnteUp);
            game.players[p0_idx].special_cards.push(SpecialCard::AnteUp);
            game.players[p0_idx].special_cards.push(SpecialCard::AnteUp);

            for expected_mult in [2.0, 4.0, 8.0] {
                let res = game.player_move(
                    p0_id,
                    PlaySpecial {
                        card: SpecialCard::AnteUp,
                        target_id: None,
                        card_index: None,
                    },
                );
                assert!(res.is_ok());
                assert_eq!(game.special.ante_multiplier(), expected_mult);
            }

            assert_eq!(game.get_current_max_bet(), Some(4000));
            assert_eq!(
                game.special
                    .active_modifiers()
                    .iter()
                    .filter(|m| matches!(m.effect, ModifierEffect::AnteMultiplier { .. }))
                    .count(),
                1
            );

            // End current hand and start a new hand: stacked ante multiplier should persist and scale blinds
            let mut events = Vec::new();
            game.end_hand(&mut events);
            game.start_new_hand(&mut events);

            assert_eq!(game.special.ante_multiplier(), 8.0);
            // Blind size is 20 * 8.0 = 160 (SB = 80, BB = 160)
            assert_eq!(game.bet, 160);
            assert_eq!(game.round_pool, 240);
        }

        #[test]
        fn test_winner_draft_disabled_on_uncontested_small_pot() {
            let mut game = get_configured_started_game(2, 0);
            let p0_id = game.get_current_turn_player_id();
            let _ = game.player_move(p0_id, Fold).unwrap();

            assert_eq!(game.round, Round::Showdown);
            assert!(game.pending_drafts.is_empty());
        }

        #[test]
        fn test_winner_draft_offered_privately_and_pick_transitions_to_showdown() {
            let mut game = get_configured_started_game(2, 0);
            let p0_id = game.get_current_turn_player_id();
            let p1_id = game.players.iter().find(|p| p.id != p0_id).unwrap().id;

            // p0 raises meaningfully (> 2% of 2000 chips and > 1.5x blind), p1 folds
            let _ = game.player_move(p0_id, Raise { amount: 200 }).unwrap();
            let _ = game.player_move(p1_id, Fold).unwrap();

            assert_eq!(game.round, Round::WinnerDraft);
            assert_eq!(game.pending_drafts.len(), 1);
            assert_eq!(game.pending_drafts[0].options.len(), 3);

            // Winner sees 3 options; loser does not
            let winner_view = view_game_state(&game, p0_id);
            let loser_view = view_game_state(&game, p1_id);
            assert_eq!(winner_view.winner_draft_options.as_ref().map(|v| v.len()), Some(3));
            assert!(loser_view.winner_draft_options.is_none());
            assert_eq!(winner_view.winner_draft_pending_players, vec![p0_id]);

            let expected_card = game.pending_drafts[0].options[1];
            let pick_events = game.pick_winner_draft(p0_id, 1).unwrap();
            assert_eq!(game.round, Round::Showdown);
            assert!(game.pending_drafts.is_empty());
            assert!(pick_events.iter().any(|e| e.actor_id == Some(p0_id) && matches!(e.action, MoveAction::DealSpecial { count: 1 })));

            let winner_idx = game.get_player_index(p0_id).unwrap();
            assert_eq!(game.players[winner_idx].special_cards, vec![expected_card]);
        }

        #[test]
        fn test_winner_draft_parallel_tie_and_timeout() {
            let mut game = get_configured_started_game(2, 0);
            let p0_id = game.players[0].id;
            let p1_id = game.players[1].id;

            // Give both players identical royal flush hands and go all-in to tie the main pot
            let royal = vec![
                ExpandedCard::get_card("10", "s"),
                ExpandedCard::get_card("j", "s"),
                ExpandedCard::get_card("q", "s"),
                ExpandedCard::get_card("k", "s"),
                ExpandedCard::get_card("a", "s"),
            ];
            game.players[0].cards = royal.clone();
            game.players[1].cards = royal;

            let first_id = game.get_current_turn_player_id();
            let second_id = if first_id == p0_id { p1_id } else { p0_id };
            let _ = game.player_move(first_id, Raise { amount: 10000 }).unwrap();
            let _ = game.player_move(second_id, Call).unwrap();

            // All-in players receive a special card on Turn and get a chance to act on Turn/River
            while matches!(game.round, Round::Preflop | Round::Flop | Round::Turn | Round::River) {
                let turn_id = game.get_current_turn_player_id();
                let _ = game.player_move(turn_id, Call).unwrap();
            }

            assert_eq!(game.round, Round::WinnerDraft);
            assert_eq!(game.pending_drafts.len(), 2);

            // Second player picks first (parallel order independence)
            let _ = game.pick_winner_draft(second_id, 0).unwrap();
            assert_eq!(game.round, Round::WinnerDraft);
            assert_eq!(game.pending_drafts.len(), 1);

            // Timeout resolves remaining player and moves to Showdown
            let timeout_events = game.timeout_winner_draft();
            assert_eq!(game.round, Round::Showdown);
            assert!(game.pending_drafts.is_empty());
            assert_eq!(timeout_events.len(), 1);
            assert_eq!(timeout_events[0].actor_id, Some(first_id));
        }

        #[test]
        fn test_winner_draft_scales_card_quality_by_pot_ratio() {
            let mut game = get_configured_started_game(2, 0);

            let high_tier = [
                SpecialCard::PotMult,
                SpecialCard::ChipBoost,
                SpecialCard::ChipGamble,
                SpecialCard::WithdrawBet,
                SpecialCard::LossInsurance,
                SpecialCard::Joker,
                SpecialCard::HandSwap,
                SpecialCard::AnteUp,
                SpecialCard::Blackjack,
            ];

            let mut low_pot_high_hits = 0usize;
            let mut high_pot_high_hits = 0usize;
            let samples = 200;

            for _ in 0..samples {
                let low_opts = SpecialCard::draw_winner_draft_options(3, 0.03, &mut game);
                low_pot_high_hits += low_opts.iter().filter(|c| high_tier.contains(c)).count();

                let high_opts = SpecialCard::draw_winner_draft_options(3, 0.60, &mut game);
                high_pot_high_hits += high_opts.iter().filter(|c| high_tier.contains(c)).count();
            }

            assert!(
                low_pot_high_hits < samples / 4,
                "Low pots should rarely offer high-tier cards (got {low_pot_high_hits} out of {})",
                samples * 3
            );
            assert!(
                high_pot_high_hits > low_pot_high_hits * 4,
                "High pots should offer substantially more high-tier cards ({high_pot_high_hits} vs {low_pot_high_hits})"
            );
        }

        #[test]
        fn test_bot_forced_all_in_previous_street_handles_next_street_without_panic_or_fold() {
            let mut game = get_configured_started_game(1, 1);
            let human_idx = game
                .players
                .iter()
                .position(|p| matches!(p.player_type, PlayerType::Human))
                .unwrap();
            let bot_idx = 1 - human_idx;
            let human_id = game.players[human_idx].id;

            // Give human plenty of chips and short-stack the bot so bot is forced all-in on Preflop
            game.players[human_idx].chips = 2000;
            game.players[bot_idx].chips = 40;
            game.players[bot_idx].round_bet = 0;
            game.players[bot_idx].total_bet = 0;

            // Human raises 500; if it's currently bot's turn first, let bot act first
            if game.get_current_turn_player_id() != human_id {
                let _ = game.player_move(game.players[bot_idx].id, Call);
            }
            let _ = game.player_move(human_id, Raise { amount: 500 });

            // If bot wasn't already forced all-in on preflop, advance until Flop/Turn
            if game.round == Round::Preflop && game.get_current_turn_player_id() == human_id {
                let _ = game.player_move(human_id, Call);
            }

            // Force bot to all-in (0 chips) with a special card on Turn while human bets
            game.round = Round::Turn;
            game.players[bot_idx].chips = 0;
            game.players[bot_idx].folded = false;
            game.players[bot_idx].acted = false;
            game.players[bot_idx].turn_ended = false;
            game.players[bot_idx].round_bet = 0;
            game.players[bot_idx].special_cards.push(SpecialCard::ChipBoost);
            game.players[human_idx].acted = false;
            game.players[human_idx].turn_ended = false;
            game.players[human_idx].round_bet = 0;
            game.bet = 0;
            game.turn_index = human_idx;

            let res = game.player_move(human_id, Raise { amount: 100 });
            assert!(res.is_ok());
            assert!(!game.players[bot_idx].folded, "All-in bot must not fold on Turn");
        }

        #[test]
        fn test_winner_draft_skipped_when_special_cards_full() {
            let mut game = get_configured_started_game(2, 0);
            let p0_id = game.get_current_turn_player_id();
            let p0_idx = game.get_player_index(p0_id).unwrap();
            let p1_id = game.players.iter().find(|p| p.id != p0_id).unwrap().id;

            game.players[p0_idx].special_cards = vec![SpecialCard::ChipBoost; game.ctx.special_card_limit];

            let _ = game.player_move(p0_id, Raise { amount: 200 }).unwrap();
            let events = game.player_move(p1_id, Fold).unwrap();

            assert_eq!(game.round, Round::Showdown);
            assert!(game.pending_drafts.is_empty());
            assert!(events.iter().any(|e| {
                e.actor_id == Some(p0_id)
                    && e.private
                    && matches!(e.action, MoveAction::DraftSkippedFull)
            }));
        }

        #[test]
        fn test_short_stack_raise_rejected_and_bot_falls_back_to_call() {
            let mut game = get_configured_started_game(1, 1);
            let human_idx = game
                .players
                .iter()
                .position(|p| matches!(p.player_type, PlayerType::Human))
                .unwrap();
            let bot_idx = 1 - human_idx;

            game.round = Round::Flop;
            game.bet = 100;
            game.players[human_idx].round_bet = 100;
            game.players[human_idx].acted = true;
            game.players[human_idx].turn_ended = true;

            // Bot only has 50 chips while facing a 100-chip bet (cannot cover call, let alone raise)
            game.players[bot_idx].chips = 50;
            game.players[bot_idx].round_bet = 0;
            game.players[bot_idx].acted = false;
            game.players[bot_idx].turn_ended = false;
            game.players[bot_idx].cards = vec![
                ExpandedCard::get_card("10", "s"),
                ExpandedCard::get_card("j", "s"),
                ExpandedCard::get_card("q", "s"),
                ExpandedCard::get_card("k", "s"),
                ExpandedCard::get_card("a", "s"),
            ];
            game.turn_index = bot_idx;

            let mut events = Vec::new();
            let direct_raise = game.apply_action(bot_idx, Raise { amount: 20 }, &mut events);
            assert!(matches!(direct_raise, Err(GameError::InvalidAction)));

            let mut ai = AI::new(AIType::Risky);
            let actions = ai.calculate_next_action(bot_idx, &mut game);
            assert!(
                !actions.iter().any(|a| matches!(a, Action::Raise { .. })),
                "Short-stacked bot unable to cover call must not emit Action::Raise"
            );
        }

        #[test]
        fn test_bot_raise_waits_for_human_before_advancing_street() {
            let mut game = get_configured_started_game(1, 1);
            let human_idx = game
                .players
                .iter()
                .position(|p| matches!(p.player_type, PlayerType::Human))
                .unwrap();
            let bot_idx = 1 - human_idx;
            let human_id = game.players[human_idx].id;

            game.round = Round::Flop;
            game.bet = 0;
            game.round_pool = 0;
            game.players[human_idx].chips = 2000;
            game.players[human_idx].round_bet = 0;
            game.players[human_idx].acted = false;
            game.players[human_idx].turn_ended = false;

            // Give Risky bot a Royal Flush on Flop so it raises when human checks
            game.players[bot_idx].player_type = PlayerType::Computer(AI::new(AIType::Risky));
            game.players[bot_idx].chips = 2000;
            game.players[bot_idx].round_bet = 0;
            game.players[bot_idx].acted = false;
            game.players[bot_idx].turn_ended = false;
            game.players[bot_idx].cards = vec![
                ExpandedCard::get_card("10", "s"),
                ExpandedCard::get_card("j", "s"),
                ExpandedCard::get_card("q", "s"),
                ExpandedCard::get_card("k", "s"),
                ExpandedCard::get_card("a", "s"),
            ];
            game.turn_index = human_idx;

            let events = game.player_move(human_id, Call).unwrap();

            assert!(
                events.iter().any(|e| e.actor_id == Some(game.players[bot_idx].id)
                    && matches!(e.action, MoveAction::Action { action: Action::Raise { .. }, .. })),
                "Expected bot to raise on Flop"
            );
            assert_eq!(game.round, Round::Flop, "Street must not advance until human matches bot raise");
            assert!(game.bet > 0);
            assert!(!game.players[human_idx].acted, "Human must be reset to acted = false after bot raise");
            assert_eq!(game.get_current_turn_player_id(), human_id);
        }

        #[test]
        fn test_multi_street_events_record_per_action_bet_deltas() {
            let mut game = get_configured_started_game(1, 2);
            let human_idx = game
                .players
                .iter()
                .position(|p| matches!(p.player_type, PlayerType::Human))
                .unwrap();
            let human_id = game.players[human_idx].id;

            // Position on Preflop where human acts last, bot acts right after on Flop with a Royal Flush
            let bot1_idx = (human_idx + 1) % 3;
            let bot2_idx = (human_idx + 2) % 3;

            game.players[human_idx].chips = 2000;
            game.players[human_idx].round_bet = 0;
            game.players[human_idx].total_bet = 0;
            game.players[human_idx].acted = false;
            game.players[human_idx].turn_ended = false;

            game.players[bot1_idx].player_type = PlayerType::Computer(AI::new(AIType::Risky));
            game.players[bot1_idx].chips = 2000;
            game.players[bot1_idx].round_bet = 20;
            game.players[bot1_idx].total_bet = 20;
            game.players[bot1_idx].acted = true;
            game.players[bot1_idx].turn_ended = true;
            game.players[bot1_idx].cards = vec![
                ExpandedCard::get_card("10", "s"),
                ExpandedCard::get_card("j", "s"),
                ExpandedCard::get_card("q", "s"),
                ExpandedCard::get_card("k", "s"),
                ExpandedCard::get_card("a", "s"),
            ];

            game.players[bot2_idx].player_type = PlayerType::Computer(AI::new(AIType::Safe));
            game.players[bot2_idx].chips = 2000;
            game.players[bot2_idx].round_bet = 20;
            game.players[bot2_idx].total_bet = 20;
            game.players[bot2_idx].acted = true;
            game.players[bot2_idx].turn_ended = true;

            game.round = Round::Preflop;
            game.bet = 20;
            game.round_pool = 40;
            game.dealer_index = human_idx;
            game.turn_index = human_idx;

            let events = game.player_move(human_id, Call).unwrap();

            // First event is human's Preflop Call (bet_delta = 20, round_bet = 20, total_bet = 20)
            assert!(matches!(
                events.first().map(|e| &e.action),
                Some(MoveAction::Action {
                    action: Action::Call,
                    bet_delta: Some(20),
                    round_bet: Some(20),
                    total_bet: Some(20),
                })
            ));

            // Subsequent DealCommunity should precede any Flop Raise by bot1
            let deal_pos = events.iter().position(|e| matches!(e.action, MoveAction::DealCommunity { count: 3 })).unwrap();
            let flop_raise = events[deal_pos + 1..].iter().find(|e| {
                e.actor_id == Some(game.players[bot1_idx].id)
                    && matches!(e.action, MoveAction::Action { action: Action::Raise { .. }, .. })
            });
            assert!(flop_raise.is_some(), "Expected bot1 to raise on Flop after DealCommunity");
            if let MoveAction::Action {
                action: Action::Raise { amount },
                bet_delta: Some(delta),
                round_bet: Some(rb),
                total_bet: Some(tb),
            } = &flop_raise.unwrap().action
            {
                assert_eq!(delta, amount);
                assert_eq!(rb, amount);
                assert_eq!(*tb, 20 + *amount);
            } else {
                panic!("Expected bet metadata on Flop Raise event");
            }
        }

        #[test]
        fn test_ante_up_pity_increments_across_rounds_and_resets_on_draw() {
            let mut game = get_configured_started_game(2, 0);
            assert_eq!(game.special.rounds_since_ante_up, 0);
            assert!(!game.special.ante_up_drawn_this_hand);

            let mut events = Vec::new();
            // Simulate consecutive hands with no AnteUp drawn
            for expected_round in 1..=ANTE_UP_PITY_MAX_ROUNDS {
                game.end_hand(&mut events);
                assert_eq!(game.special.rounds_since_ante_up, expected_round);
                assert!(!game.special.ante_up_drawn_this_hand);
                game.start_new_hand(&mut events);
            }

            // At max pity rounds, next draw is guaranteed to be AnteUp and immediately resets pity
            let drawn = SpecialCard::draw_special_cards(2, game.players[0].id, &mut game);
            assert_eq!(drawn[0], SpecialCard::AnteUp);
            assert_eq!(game.special.rounds_since_ante_up, 0);
            assert!(game.special.ante_up_drawn_this_hand);

            // Ending the hand where AnteUp was drawn keeps rounds_since_ante_up at 0
            game.end_hand(&mut events);
            assert_eq!(game.special.rounds_since_ante_up, 0);
            assert!(!game.special.ante_up_drawn_this_hand);

            // Ending the subsequent hand with no AnteUp steps pity to 1
            game.end_hand(&mut events);
            assert_eq!(game.special.rounds_since_ante_up, 1);
        }

        #[test]
        fn test_ante_up_pity_increases_draw_rate_over_rounds() {
            let mut game = get_configured_started_game(2, 0);
            let player_id = game.players[0].id;
            let trials = 200;

            let mut hits_round_0 = 0usize;
            for _ in 0..trials {
                game.special.rounds_since_ante_up = 0;
                game.special.ante_up_drawn_this_hand = false;
                if SpecialCard::draw_special_cards(1, player_id, &mut game)[0] == SpecialCard::AnteUp {
                    hits_round_0 += 1;
                }
            }

            let mut hits_round_5 = 0usize;
            for _ in 0..trials {
                game.special.rounds_since_ante_up = 5;
                game.special.ante_up_drawn_this_hand = false;
                if SpecialCard::draw_special_cards(1, player_id, &mut game)[0] == SpecialCard::AnteUp {
                    hits_round_5 += 1;
                }
            }

            assert!(
                hits_round_5 > hits_round_0 * 3,
                "Expected AnteUp draw rate to step up substantially after 5 rounds ({hits_round_5} vs {hits_round_0})"
            );
        }
    }

    #[cfg(test)]
    mod ai_tests {
        use crate::Action::PlaySpecial;
use crate::ai::{AI, AIType};
        use crate::poker::*;
        use crate::special::SpecialCard;
use crate::{Action, ConfigPlayer, DeckType, Game, GameConfig, Round};

        fn get_test_game(player_count: usize, bot_count: usize) -> Game {
            let mut game = Game::new();
            let mut players = Vec::with_capacity(player_count + bot_count);

            for id in 0..player_count {
                players.push(ConfigPlayer::Human { id });
            }

            for offset in 0..bot_count {
                players.push(ConfigPlayer::Bot {
                    id: player_count + offset,
                    ai_type: AIType::Smart,
                });
            }

            let config = GameConfig { 
                players, 
                rng_seed: 0, 
                blind_size: 20, 
                min_raise: 10, 
                starting_chips: 1000, 
                special_card_limit: 3, 
                deck_type: DeckType::Standard, 
                max_players: player_count + bot_count, 
                round_limit: 30,
                max_bet_multiplier: None,
            };
            let _ = game.update_config(config);
            let _ = game.initialise();
            let _ = game.start();

            game
        }

        #[test]
        fn test_all_ai_types_can_be_configured_and_play() {
            // Verify that all AI presets (Safe, Risky, Smart, Random) can be initialized,
            // integrated into a game configuration, and produce valid poker actions.
            let ai_types = [AIType::Safe, AIType::Risky, AIType::Smart, AIType::Random];

            for (idx, ai_type) in ai_types.into_iter().enumerate() {
                let mut game = Game::new();
                let config = GameConfig {
                    players: vec![
                        ConfigPlayer::Human { id: 0 },
                        ConfigPlayer::Bot { id: 1, ai_type },
                    ],
                    rng_seed: (idx + 1) as u64,
                    blind_size: 20,
                    min_raise: 10,
                    starting_chips: 1000,
                    special_card_limit: 3,
                    deck_type: DeckType::Standard,
                    max_players: 2,
                    round_limit: 30,
                    max_bet_multiplier: None,
                };
                assert!(game.update_config(config).is_ok());
                assert!(game.initialise().is_ok());
                assert!(game.start().is_ok());

                let mut ai = AI::new(ai_type);
                let actions = ai.calculate_next_action(1, &mut game);
                assert!(!actions.is_empty(), "AI of type {:?} should produce at least one action", ai_type);
                for action in &actions {
                    assert!(matches!(action, Action::Call | Action::Raise { .. } | Action::Fold));
                }
            }
        }

        #[test]
        fn test_ai_resets_state_between_rounds() {
            let mut game = get_test_game(2, 0);
            let mut ai = AI::new(AIType::Smart);

            // Act in preflop
            let preflop_actions = ai.calculate_next_action(0, &mut game);
            assert!(!preflop_actions.is_empty());

            // Advance street and reset round-specific state
            ai.update_round_parameters();

            // AI calculates fresh actions on Flop
            game.round = Round::Flop;
            game.community = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_card("7", "h"),
                ExpandedCard::get_card("k", "d"),
            ];
            let flop_actions = ai.calculate_next_action(0, &mut game);
            assert!(!flop_actions.is_empty());
        }

        #[test]
        fn test_ai_facing_bet_behavior() {
            let mut game = get_test_game(2, 0);

            // 1. Calling an affordable bet: facing a small raise within budget
            let mut ai_caller = AI::new(AIType::Safe);
            game.bet = 30;
            game.players[0].round_bet = 20;
            let call_actions = ai_caller.calculate_next_action(0, &mut game);
            assert_eq!(call_actions, vec![Action::Call]);

            // 2. Folding when facing an overwhelming bet with weak holdings
            let mut ai_folder = AI::new(AIType::Safe);
            game.bet = 800;
            game.players[0].round_bet = 20;
            game.players[0].cards = vec![
                ExpandedCard::get_card("2", "c"),
                ExpandedCard::get_card("7", "d"),
            ];
            let fold_actions = ai_folder.calculate_next_action(0, &mut game);
            assert_eq!(fold_actions, vec![Action::Fold]);

            // 3. Re-raising with a dominant hand
            let mut ai_raiser = AI::new(AIType::Risky);
            game.bet = 100;
            game.players[0].round_bet = 20;
            game.players[0].cards = vec![
                ExpandedCard::get_card("10", "s"),
                ExpandedCard::get_card("j", "s"),
                ExpandedCard::get_card("q", "s"),
                ExpandedCard::get_card("k", "s"),
                ExpandedCard::get_card("a", "s"),
            ];
            let raise_actions = ai_raiser.calculate_next_action(0, &mut game);
            assert!(matches!(raise_actions.first(), Some(Action::Raise { .. })));

            // 4. Calling when player is already at the table's max bet limit
            let mut ai_max_bet = AI::new(AIType::Safe);
            game.ctx.max_bet_multiplier = Some(5);
            game.players[0].total_bet = 100;
            game.bet = 200;
            game.players[0].round_bet = 20;
            let max_actions = ai_max_bet.calculate_next_action(0, &mut game);
            assert_eq!(max_actions, vec![Action::Call]);

            // 5. Calling when player has no chips left to raise
            let mut ai_all_in = AI::new(AIType::Risky);
            game.ctx.max_bet_multiplier = None;
            game.players[0].total_bet = 0;
            game.players[0].chips = 0;
            game.bet = 100;
            game.players[0].round_bet = 20;
            let all_in_actions = ai_all_in.calculate_next_action(0, &mut game);
            assert_eq!(all_in_actions, vec![Action::Call]);
        }

        #[test]
        fn test_ai_uncontested_betting_behavior() {
            let mut game = get_test_game(2, 0);

            // 1. Checking when holding weak cards and no bet is needed
            let mut ai_checker = AI::new(AIType::Safe);
            game.bet = 20;
            game.players[0].round_bet = 20;
            game.players[0].cards = vec![
                ExpandedCard::get_card("2", "c"),
                ExpandedCard::get_card("7", "d"),
            ];
            let check_actions = ai_checker.calculate_next_action(0, &mut game);
            assert_eq!(check_actions, vec![Action::Call]);

            // 2. Value-betting / Raising when holding a premium hand
            let mut ai_value_better = AI::new(AIType::Risky);
            game.bet = 20;
            game.players[0].round_bet = 20;
            game.players[0].chips = 1000;
            game.players[0].cards = vec![
                ExpandedCard::get_card("10", "s"),
                ExpandedCard::get_card("j", "s"),
                ExpandedCard::get_card("q", "s"),
                ExpandedCard::get_card("k", "s"),
                ExpandedCard::get_card("a", "s"),
            ];
            let value_actions = ai_value_better.calculate_next_action(0, &mut game);
            assert!(matches!(value_actions.first(), Some(Action::Raise { .. })));

            // 3. Checking when already at table max-bet limit
            let mut ai_at_max = AI::new(AIType::Risky);
            game.ctx.max_bet_multiplier = Some(5);
            game.players[0].total_bet = 100;
            game.bet = 20;
            game.players[0].round_bet = 20;
            let at_max_actions = ai_at_max.calculate_next_action(0, &mut game);
            assert_eq!(at_max_actions, vec![Action::Call]);
        }

        #[test]
        fn test_ai_budget_behavior_across_streets() {
            let mut game = get_test_game(2, 0);
            let ai = AI::new(AIType::Smart);
            let mut player = game.players[0].clone();

            // Preflop: budget scales with stack size and respects chip cap
            game.round = Round::Preflop;
            player.chips = 1000;
            let budget_deep = ai.calculate_round_budget(&player, &mut game, 0.5);
            player.chips = 200;
            let budget_short = ai.calculate_round_budget(&player, &mut game, 0.5);
            assert!(budget_deep > budget_short, "Deeper stack should have higher preflop budget");
            assert!(budget_short <= 200, "Budget cannot exceed chip stack");

            // Post-preflop (Flop/Turn/River): budget increases with hand strength
            game.round = Round::Flop;
            player.chips = 800;
            let budget_weak_hand = ai.calculate_round_budget(&player, &mut game, 0.1);
            let budget_strong_hand = ai.calculate_round_budget(&player, &mut game, 0.9);
            assert!(
                budget_strong_hand > budget_weak_hand,
                "Stronger hand must produce a higher post-preflop budget"
            );
            assert!(budget_strong_hand <= player.chips);

            // Sunk cost effect: bot with committed chips allocates more budget to defend its investment
            let mut committed_ai = AI::new(AIType::Smart);
            game.round = Round::Turn;
            let budget_no_commit = committed_ai.calculate_round_budget(&player, &mut game, 0.5);
            // Simulate committed bets through a previous raise in this hand
            game.bet = 200;
            game.players[0].round_bet = 20;
            let _ = committed_ai.calculate_next_action(0, &mut game);
            let budget_with_commit = committed_ai.calculate_round_budget(&player, &mut game, 0.5);
            assert!(
                budget_with_commit >= budget_no_commit,
                "Sunk cost commitment should not decrease round budget"
            );
        }

        #[test]
        fn test_hand_value_hierarchy_and_street_dynamics() {
            let mut game = get_test_game(2, 0);
            let ai = AI::new(AIType::Smart);

            // 1. Empty cards have zero value
            assert_eq!(ai.calculate_hand_value(&vec![], &game), 0.0);

            // 2. Relative poker hand rankings: every tier must evaluate stronger than the tier below it
            let high_card = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_card("4", "h"),
                ExpandedCard::get_card("6", "d"),
                ExpandedCard::get_card("8", "c"),
                ExpandedCard::get_card("10", "s"),
            ];
            let pair = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_card("2", "h"),
                ExpandedCard::get_card("4", "d"),
                ExpandedCard::get_card("6", "c"),
                ExpandedCard::get_card("8", "s"),
            ];
            let two_pair = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_card("2", "h"),
                ExpandedCard::get_card("4", "d"),
                ExpandedCard::get_card("4", "c"),
                ExpandedCard::get_card("8", "s"),
            ];
            let trips = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_card("2", "h"),
                ExpandedCard::get_card("2", "d"),
                ExpandedCard::get_card("4", "c"),
                ExpandedCard::get_card("8", "s"),
            ];
            let straight = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_card("3", "h"),
                ExpandedCard::get_card("4", "d"),
                ExpandedCard::get_card("5", "c"),
                ExpandedCard::get_card("6", "s"),
            ];
            let flush = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_card("4", "s"),
                ExpandedCard::get_card("6", "s"),
                ExpandedCard::get_card("8", "s"),
                ExpandedCard::get_card("10", "s"),
            ];
            let full_house = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_card("2", "h"),
                ExpandedCard::get_card("2", "d"),
                ExpandedCard::get_card("3", "c"),
                ExpandedCard::get_card("3", "s"),
            ];
            let quads = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_card("2", "h"),
                ExpandedCard::get_card("2", "d"),
                ExpandedCard::get_card("2", "c"),
                ExpandedCard::get_card("8", "s"),
            ];
            let straight_flush = vec![
                ExpandedCard::get_card("2", "s"),
                ExpandedCard::get_card("3", "s"),
                ExpandedCard::get_card("4", "s"),
                ExpandedCard::get_card("5", "s"),
                ExpandedCard::get_card("6", "s"),
            ];
            let royal_flush = vec![
                ExpandedCard::get_card("10", "s"),
                ExpandedCard::get_card("j", "s"),
                ExpandedCard::get_card("q", "s"),
                ExpandedCard::get_card("k", "s"),
                ExpandedCard::get_card("a", "s"),
            ];

            let v_high = ai.calculate_hand_value(&high_card, &game);
            let v_pair = ai.calculate_hand_value(&pair, &game);
            let v_two_pair = ai.calculate_hand_value(&two_pair, &game);
            let v_trips = ai.calculate_hand_value(&trips, &game);
            let v_straight = ai.calculate_hand_value(&straight, &game);
            let v_flush = ai.calculate_hand_value(&flush, &game);
            let v_full_house = ai.calculate_hand_value(&full_house, &game);
            let v_quads = ai.calculate_hand_value(&quads, &game);
            let v_straight_flush = ai.calculate_hand_value(&straight_flush, &game);
            let v_royal_flush = ai.calculate_hand_value(&royal_flush, &game);

            assert!(v_pair > v_high, "Pair must rank above High Card");
            assert!(v_two_pair > v_pair, "Two Pair must rank above Pair");
            assert!(v_trips > v_two_pair, "Three of a Kind must rank above Two Pair");
            assert!(v_straight > v_trips, "Straight must rank above Three of a Kind");
            assert!(v_flush > v_straight, "Flush must rank above Straight");
            assert!(v_full_house > v_flush, "Full House must rank above Flush");
            assert!(v_quads > v_full_house, "Four of a Kind must rank above Full House");
            assert!(v_straight_flush > v_quads, "Straight Flush must rank above Four of a Kind");
            assert!(v_royal_flush > v_straight_flush, "Royal Flush must rank above Straight Flush");

            // 3. Street dynamics for drawing hands:
            // A 4-to-a-royal draw has highest potential on the Flop (2 cards to come),
            // reduced potential on the Turn (1 card to come), and no draw bonus on the River.
            let drawing_hand = vec![
                ExpandedCard::get_card("10", "s"),
                ExpandedCard::get_card("j", "s"),
                ExpandedCard::get_card("q", "s"),
                ExpandedCard::get_card("k", "s"),
                ExpandedCard::get_card("2", "c"),
            ];

            game.round = Round::Flop;
            let val_flop = ai.calculate_hand_value(&drawing_hand, &game);

            game.round = Round::Turn;
            let val_turn = ai.calculate_hand_value(&drawing_hand, &game);

            game.round = Round::River;
            let val_river = ai.calculate_hand_value(&drawing_hand, &game);

            assert!(val_flop > val_turn, "Drawing potential on Flop must exceed Turn");
            assert!(val_turn > val_river, "Drawing potential on Turn must exceed River");

            // 4. Board penalty: when winning combination is on community board, value is discounted
            game.community = pair.clone();
            let val_on_board = ai.calculate_hand_value(&pair, &game);
            game.community.clear();
            let val_in_hand = ai.calculate_hand_value(&pair, &game);
            assert!(val_on_board < val_in_hand, "Playing the board must discount hand value");
        }
    }

    #[cfg(test)]
    mod poker_tests {
        use crate::poker::*;

        #[test]
        fn extract_hand_does_not_underflow_when_representation_exceeds_hand_size() {
            let cards = vec![
                ExpandedCard::get_card("a", "s"),
                ExpandedCard::get_card("a", "h"),
                ExpandedCard::get_card("a", "d"),
                ExpandedCard::get_card("a", "c"),
                ExpandedCard::get_card("k", "s"),
                ExpandedCard::get_card("k", "h"),
            ];

            let result = crate::poker::get_representation(&cards, vec![3, 2], None, 4);

            assert!(result.is_none());
        }
    }
}