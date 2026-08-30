//! Troll of Khazad-dûm: a body nobody blocks, or the Swamp the deck was
//! missing.

use super::*;

/// The Troll attacking, with `blockers` under player Two.
fn attacking(blockers: usize) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let troll = game
        .put_onto_battlefield(PlayerId::One, cards::TROLL_OF_KHAZAD_DUM)
        .expect("cataloged");
    let mut ids = Vec::new();
    for index in 0..blockers {
        let creature = creature(
            98_000 + u32::try_from(index).expect("few creatures"),
            cards::GRIZZLY_BEARS,
            PlayerId::Two,
        );
        ids.push(creature.card.id);
        game.battlefield.push(creature);
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(troll, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;
    (game, troll, ids)
}

/// Whether the blocking player may finish the declaration as it stands.
fn may_finish(game: &Game) -> bool {
    game.legal_actions(PlayerId::Two)
        .into_iter()
        .any(|action| matches!(action, Action::FinishDeclaringBlockers))
}

/// Blocking with nobody is legal: "except by three or more" is not a
/// requirement to block.
#[test]
fn nobody_blocking_is_fine() {
    let (game, _troll, _) = attacking(3);

    assert!(may_finish(&game));
}

/// One blocker is not three, so the declaration cannot be finished.
#[test]
fn one_blocker_is_not_enough() {
    let (mut game, troll, blockers) = attacking(3);
    game.declare_blocker(blockers[0], troll);

    assert!(!may_finish(&game));
}

/// Nor is two, which is what separates it from menace.
#[test]
fn two_blockers_are_not_enough() {
    let (mut game, troll, blockers) = attacking(3);
    game.declare_blocker(blockers[0], troll);
    game.declare_blocker(blockers[1], troll);

    assert!(!may_finish(&game), "two would do for menace, not for this");
}

/// Three is what it asks for.
#[test]
fn three_blockers_may_finish() {
    let (mut game, troll, blockers) = attacking(3);
    for blocker in &blockers {
        game.declare_blocker(*blocker, troll);
    }

    assert!(may_finish(&game));
}

/// A menacing attacker still takes two, which is the rule the Troll's clause
/// generalized rather than replaced.
#[test]
fn menace_still_takes_only_two() {
    let mut game = ready_game();
    game.battlefield.clear();
    let menacing = game
        .put_onto_battlefield(PlayerId::One, cards::RIPSCALE_PREDATOR)
        .expect("cataloged");
    let mut blockers = Vec::new();
    for index in 0..2 {
        let creature = creature(
            98_500 + u32::try_from(index).expect("few creatures"),
            cards::GRIZZLY_BEARS,
            PlayerId::Two,
        );
        blockers.push(creature.card.id);
        game.battlefield.push(creature);
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(menacing, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    drain_pending(&mut game);
    game.step = Step::DeclareBlockers;
    game.blockers_declared = false;

    game.declare_blocker(blockers[0], menacing);
    assert!(!may_finish(&game), "one is not two");
    game.declare_blocker(blockers[1], menacing);
    assert!(may_finish(&game), "and two is");
}

/// Player One with the Troll in hand, `library` beneath it, and the one
/// black mana swampcycling asks for.
fn holding_the_troll(library: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    let troll = game
        .build_zone(PlayerId::One, &[cards::TROLL_OF_KHAZAD_DUM])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let troll_id = troll.id;
    game.players[0].hand.push(troll);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    (game, troll_id)
}

/// The swampcycling activation, if it is on offer.
fn cycling_action(game: &Game, troll: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == troll))
}

/// Runs until the search asks, and reports what it offers.
fn offered_by_the_search(game: &mut Game) -> Vec<CardDefinitionId> {
    for _ in 0..8 {
        if !game.pending_decisions.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the search asks")
        .options
        .iter()
        .filter_map(|option| match option.card {
            Some((_, ObjectCharacteristics::Card { definition, .. })) => Some(definition),
            _ => None,
        })
        .collect()
}

/// Runs the activation out, taking `wanted` when the search asks for it.
fn settle_taking(game: &mut Game, wanted: CardDefinitionId) {
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let chosen = decision
                .options
                .iter()
                .find(|option| {
                    matches!(
                        option.card,
                        Some((_, ObjectCharacteristics::Card { definition, .. }))
                            if definition == wanted
                    )
                })
                .map(|option| option.id);
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: chosen.map(|id| vec![id]).unwrap_or_default(),
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

/// Swampcycling: one mana and the card itself for a Swamp out of the
/// library.
#[test]
fn swampcycling_finds_a_swamp() {
    let (mut game, troll) = holding_the_troll(&[cards::FOREST, cards::SWAMP, cards::MOUNTAIN]);
    let action = cycling_action(&game, troll).expect("swampcycling is activatable from hand");
    game.apply(PlayerId::One, action).expect("it activates");
    settle_taking(&mut game, cards::SWAMP);

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::SWAMP),
        "the Swamp is in hand",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::TROLL_OF_KHAZAD_DUM),
        "and the Troll paid for it",
    );
}

/// "Unlike the normal cycling ability, typecycling doesn't allow you to draw
/// a card." The Swamp is the whole of what the mana bought: the two cards it
/// was shuffled back among are still in the library.
#[test]
fn swampcycling_draws_nothing() {
    let (mut game, troll) = holding_the_troll(&[cards::FOREST, cards::SWAMP, cards::MOUNTAIN]);
    let action = cycling_action(&game, troll).expect("swampcycling is activatable");
    game.apply(PlayerId::One, action).expect("it activates");
    settle_taking(&mut game, cards::SWAMP);

    assert_eq!(
        game.players[0]
            .hand
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::SWAMP],
        "the search is the whole of it, with no draw on top",
    );
    assert_eq!(game.players[0].library.len(), 2, "and the rest stayed put");
}

/// "Typecycling is a form of cycling", and cycling is an activated ability
/// with no timing restriction of its own: their turn, their spell on the
/// stack, and the Troll still turns into a Swamp.
#[test]
fn swampcycling_is_not_sorcery_speed() {
    let (mut game, troll) = holding_the_troll(&[cards::SWAMP]);
    game.active_player = PlayerId::Two;
    game.priority = PlayerId::One;
    game.stack
        .push(spell(20_000, cards::LIGHTNING_BOLT, PlayerId::Two, 0));

    assert!(
        cycling_action(&game, troll).is_some(),
        "an instant-speed answer to being short a land",
    );
}

/// The ability is a hand ability: the discard is its cost, so a Troll that
/// is already in the graveyard has nothing to pay with.
#[test]
fn swampcycling_is_only_offered_from_hand() {
    let (mut game, troll) = holding_the_troll(&[cards::SWAMP]);
    let card = game.players[0].hand.pop().expect("the Troll was in hand");
    game.players[0].graveyard.push(card);

    assert!(
        cycling_action(&game, troll).is_none(),
        "the graveyard is not a zone it cycles from",
    );
}

/// It cycles for a Swamp and nothing else.
#[test]
fn swampcycling_offers_only_swamps() {
    let (mut game, troll) = holding_the_troll(&[cards::FOREST, cards::SWAMP, cards::MOUNTAIN]);
    let action = cycling_action(&game, troll).expect("swampcycling is activatable");
    game.apply(PlayerId::One, action).expect("it activates");

    assert_eq!(offered_by_the_search(&mut game), vec![cards::SWAMP]);
}

/// "A Swamp card" is a card with the Swamp land type, basic or not: a Bayou
/// is a Swamp Forest and answers the search, where a Mountain does not.
#[test]
fn swampcycling_finds_a_nonbasic_swamp() {
    let (mut game, troll) = holding_the_troll(&[cards::MOUNTAIN, cards::BAYOU]);
    let action = cycling_action(&game, troll).expect("swampcycling is activatable");
    game.apply(PlayerId::One, action).expect("it activates");

    assert_eq!(
        offered_by_the_search(&mut game),
        vec![cards::BAYOU],
        "the type is what is searched for, not the printing",
    );
}
