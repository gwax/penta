use super::*;

pub(crate) fn new_game(sideboards: [Vec<CardDefinitionId>; 2]) -> Game {
    let decks = sideboards.map(|sideboard| crate::Deck {
        main: vec![cards::MOUNTAIN; 60],
        sideboard,
        commanders: Vec::new(),
    });
    Game::new_with_format(
        crate::Format::VintageCube,
        poc::catalog().unwrap(),
        decks,
        17,
    )
    .unwrap()
}

pub(crate) fn choose(game: &mut Game, seat: PlayerId, choice: Option<CardDefinitionId>) {
    let decision = game.observe(seat).decision.unwrap();
    let options = choice
        .map(|id| {
            decision
                .options
                .iter()
                .find(|option| {
                    option
                        .card
                        .is_some_and(|(_, card)| card.card_definition() == Some(id))
                })
                .unwrap()
                .id
        })
        .into_iter()
        .collect();
    game.apply(
        seat,
        Action::ChooseDecision {
            decision: decision.id,
            options,
        },
    )
    .unwrap();
}

#[test]
fn selection_precedes_both_opening_hands_and_reveals_only_the_chosen_card() {
    let mut game = new_game([
        vec![cards::LURRUS_OF_THE_DREAM_DEN, cards::ZIRDA_THE_DAWNWAKER],
        vec![cards::LUTRI_THE_SPELLCHASER],
    ]);
    for seat in [PlayerId::One, PlayerId::Two] {
        let observation = game.observe(seat);
        assert!(observation.hand.is_empty());
        assert_eq!(observation.library_sizes, [60, 60]);
        assert!(observation.chosen_companions.iter().all(Option::is_none));
        assert!(!observation.legal_actions.contains(&Action::KeepHand));
    }
    assert!(game.observe(PlayerId::Two).decision.is_none());
    let decision = game.observe(PlayerId::One).decision.unwrap();
    assert!(
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: decision.id,
                options: decision.options.iter().map(|option| option.id).collect(),
            }
        )
        .is_err(),
        "two companions cannot be revealed"
    );
    choose(&mut game, PlayerId::One, Some(cards::ZIRDA_THE_DAWNWAKER));
    let observation = game.observe(PlayerId::Two);
    assert!(observation.hand.is_empty());
    assert_eq!(
        observation.chosen_companions[0].unwrap().definition,
        cards::ZIRDA_THE_DAWNWAKER
    );
    assert_eq!(observation.public_reveals.len(), 1);
    choose(&mut game, PlayerId::Two, None);
    assert_eq!(game.players[0].hand.len(), 7);
    assert_eq!(game.players[1].hand.len(), 7);
    assert!(game.players[1].companion.is_none());
    game.apply(PlayerId::One, Action::KeepHand).unwrap();
    game.apply(PlayerId::Two, Action::KeepHand).unwrap();
    game.step = Step::PrecombatMain;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 6);
    let offers = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::TakeCompanion { .. }))
        .collect::<Vec<_>>();
    assert_eq!(offers.len(), 1);
    let chosen = game.players[0].companion.unwrap();
    assert_eq!(offers[0], Action::TakeCompanion { card: chosen.card });
    game.apply(PlayerId::One, offers[0].clone()).unwrap();
    assert_eq!(game.players[0].mana_pool.total(), 3);
    assert!(game.players[0].companion.unwrap().used);
    assert!(game.apply(PlayerId::One, offers[0].clone()).is_err());
    assert!(
        game.observe(PlayerId::Two).chosen_companions[0]
            .unwrap()
            .used
    );
}

#[test]
fn declining_all_companions_does_not_grant_later_access() {
    let mut game = new_game([vec![cards::LURRUS_OF_THE_DREAM_DEN], vec![]]);
    choose(&mut game, PlayerId::One, None);
    game.pregame = None;
    game.step = Step::PrecombatMain;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::TakeCompanion { .. }))
    );
    assert!(game.observe(PlayerId::Two).public_reveals.is_empty());
}

#[test]
fn match_play_draw_happens_before_companion_selection() {
    let mut game = new_game([vec![cards::LURRUS_OF_THE_DREAM_DEN], vec![]]);
    game.set_match_mode(crate::match_play::MatchMode::FirstToTwoWins)
        .unwrap();
    let decision = game.observe(PlayerId::One).decision.unwrap();
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![1],
        },
    )
    .unwrap();
    assert_eq!(game.starting_player, PlayerId::Two);
    assert!(game.players.iter().all(|p| p.hand.is_empty()));
    assert_eq!(game.decision_player(), Some(PlayerId::One));
    choose(
        &mut game,
        PlayerId::One,
        Some(cards::LURRUS_OF_THE_DREAM_DEN),
    );
    assert_eq!(game.decision_player(), Some(PlayerId::Two));
    assert_eq!(game.players[1].hand.len(), 7);
}

fn settle(game: &mut Game) {
    for _ in 0..30 {
        drain_pending(game);
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            return;
        }
        game.apply(game.priority, Action::PassPriority).unwrap();
    }
    panic!("unresolved triggers");
}

#[test]
fn kaheera_buffs_only_other_matching_creatures_you_control() {
    let mut game = ready_game();
    game.battlefield.clear();
    let cat = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH_LIONS)
        .unwrap();
    let bear = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .unwrap();
    let enemy = game
        .put_onto_battlefield(PlayerId::Two, cards::SAVANNAH_LIONS)
        .unwrap();
    let kaheera = game
        .put_onto_battlefield(PlayerId::One, cards::KAHEERA_THE_ORPHANGUARD)
        .unwrap();
    for (id, power, vigilant) in [
        (cat, 3, true),
        (bear, 2, false),
        (enemy, 2, false),
        (kaheera, 3, true),
    ] {
        let permanent = game.battlefield.iter().find(|p| p.card.id == id).unwrap();
        assert_eq!(game.power(permanent), Some(power));
        assert_eq!(
            game.permanent_has_executable_keyword(permanent, KeywordAbility::Vigilance),
            vigilant
        );
    }
    game.destroy_permanent(kaheera);
    let permanent = game.battlefield.iter().find(|p| p.card.id == cat).unwrap();
    assert_eq!(game.power(permanent), Some(2));
    assert!(!game.permanent_has_executable_keyword(permanent, KeywordAbility::Vigilance));
}

#[test]
fn keruga_counts_other_expensive_permanents_when_its_trigger_resolves() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.put_onto_battlefield(PlayerId::One, cards::SNEAK_ATTACK)
        .unwrap();
    game.put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .unwrap();
    game.put_onto_battlefield(PlayerId::Two, cards::SNEAK_ATTACK)
        .unwrap();
    let keruga = game
        .put_onto_battlefield(PlayerId::One, cards::KERUGA_THE_MACROSAGE)
        .unwrap();
    game.destroy_permanent(keruga);
    let before = game.players[0].hand.len();
    settle(&mut game);
    assert_eq!(game.players[0].hand.len(), before + 1);
}

#[test]
fn yorion_returns_its_exact_exiled_cards_even_after_leaving() {
    let mut game = ready_game();
    game.battlefield.clear();
    let own = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .unwrap();
    let other = game
        .put_onto_battlefield(PlayerId::One, cards::SOL_RING)
        .unwrap();
    let land = game
        .put_onto_battlefield(PlayerId::One, cards::MOUNTAIN)
        .unwrap();
    let mut stolen = creature(970_001, cards::SAVANNAH_LIONS, PlayerId::Two);
    stolen.controller = PlayerId::One;
    game.battlefield.push(stolen);
    let yorion = game
        .put_onto_battlefield(PlayerId::One, cards::YORION_SKY_NOMAD)
        .unwrap();
    while game.pending_decisions.is_empty() {
        game.apply(game.priority, Action::PassPriority).unwrap();
    }
    let decision = game.observe(PlayerId::One).decision.unwrap();
    assert_eq!(
        decision.options.len(),
        2,
        "only owned and controlled nonlands are eligible"
    );
    assert_eq!(
        decision
            .options
            .iter()
            .map(|option| option.card.unwrap().0)
            .collect::<Vec<_>>(),
        vec![own, other]
    );
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: decision.options.iter().map(|option| option.id).collect(),
        },
    )
    .unwrap();
    let exiled = game.players[0]
        .exile
        .iter()
        .find(|c| c.definition == cards::GRIZZLY_BEARS)
        .unwrap()
        .id;
    assert_ne!(exiled, own);
    assert!(game.battlefield.iter().any(|p| p.card.id == land));
    // An independently moved card is no longer one of the exact exile objects.
    let index = game.players[0]
        .exile
        .iter()
        .position(|card| card.definition == cards::SOL_RING)
        .unwrap();
    let moved = game.players[0].exile.remove(index);
    let (moved, _) = game.zone_change_card(moved);
    game.players[0].hand.push(moved);
    game.destroy_permanent(yorion);
    game.step = Step::PostcombatMain;
    game.advance_step();
    settle(&mut game);
    let returned = game
        .battlefield
        .iter()
        .find(|p| p.card.definition == cards::GRIZZLY_BEARS)
        .unwrap();
    assert_ne!(returned.card.id, exiled);
    assert_eq!(returned.controller, PlayerId::One);
    assert!(game.players[0].exile.is_empty());
}

#[test]
fn match_setup_accepts_only_the_nonstarting_player_having_a_companion() {
    let mut game = new_game([vec![], vec![cards::LURRUS_OF_THE_DREAM_DEN]]);
    assert_eq!(game.decision_player(), Some(PlayerId::Two));
    game.set_match_mode(crate::match_play::MatchMode::FirstToTwoWins)
        .unwrap();
    assert_eq!(game.decision_player(), Some(PlayerId::One));
    assert!(game.pending_decisions.is_empty());
}
