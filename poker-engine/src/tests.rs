/**
 * Contains tests for modules
 */

#[cfg(test)]
mod tests {
    #[cfg(test)]
    mod special_tests {}

    #[cfg(test)]
    mod game_tests {
        use crate::Action::{Raise, Call, Fold, PlaySpecial};
        use crate::api::view_game_state;
        use crate::ai::AIType;
        use crate::{Action, ConfigPlayer, DeckType, Game, GameConfig, GameError, MoveAction, MoveEvent, PlayerType, Round, SpecialCard};
        use crate::poker::*;

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
                round_limit: 30  
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
            make_move_and_end_turn(Raise { amount: 10000 }, game.players[game.turn_index].id, &mut game);

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
            let hand = Hand::new(&cards, &game.ctx.hand_fns, &game.modifiers.invalidated_hands(), game.ctx.hand_size);

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
            let hand = Hand::new(&cards, &game.ctx.hand_fns, &game.modifiers.invalidated_hands(), game.ctx.hand_size);

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
            let hand = Hand::new(&cards, &game.ctx.hand_fns, &game.modifiers.invalidated_hands(), game.ctx.hand_size);

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
            let hand = Hand::new(&cards, &game.ctx.hand_fns, &game.modifiers.invalidated_hands(), game.ctx.hand_size);

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