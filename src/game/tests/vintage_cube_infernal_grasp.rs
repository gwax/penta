//! Infernal Grasp: two mana for anything, and two life whether or not it
//! works.
//!
//! The life is the second half of what resolves rather than a cost, which
//! cuts both ways: a creature that survives being destroyed is still paid
//! for, and a Grasp that never resolves is free.

use super::*;

/// Player One holding a Grasp with the mana for it, and `theirs` opposite.
fn staged(theirs: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let victim = game
        .put_onto_battlefield(PlayerId::Two, theirs)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    let grasp = card(94_500, cards::INFERNAL_GRASP, PlayerId::One);
    let grasp_id = grasp.id;
    game.players[PlayerId::One.index()].hand.push(grasp);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.players[PlayerId::One.index()].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, grasp_id, victim)
}

fn cast_at(game: &mut Game, grasp: GameObjectId, victim: GameObjectId) {
    game.apply(
        PlayerId::One,
        cast_action(grasp, vec![Target::Permanent(victim)], Vec::new(), 0),
    )
    .expect("a creature is what it names");
}

fn alive(game: &Game, id: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.id == id)
}

/// The ordinary case: the creature dies and you pay for it.
#[test]
fn it_kills_anything_for_two_life() {
    let (mut game, grasp, victim) = staged(cards::GRAVE_TITAN);

    cast_at(&mut game, grasp, victim);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(!alive(&game, victim), "a 6/6 is no harder than a 2/2");
    assert_eq!(game.players[PlayerId::One.index()].life, 18);
}

/// "A target that survives being destroyed still costs it": indestructible
/// answers the destruction and nothing answers the life.
#[test]
fn an_indestructible_creature_survives_and_you_pay_anyway() {
    let (mut game, grasp, victim) = staged(cards::DARKSTEEL_MYR);

    cast_at(&mut game, grasp, victim);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(alive(&game, victim), "the Myr shrugs the destruction off");
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        18,
        "and the second half of the sentence happens regardless",
    );
}

/// Regeneration is the other way to survive it, and it is allowed: the
/// shield is spent, the Troll stays, and the life is gone all the same.
#[test]
fn a_regeneration_shield_saves_the_troll_and_not_your_life() {
    let (mut game, grasp, victim) = staged(cards::SEDGE_TROLL);
    game.players[PlayerId::Two.index()].mana_pool.black = 1;
    game.priority = PlayerId::Two;
    let regenerate = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == victim),
        )
        .expect("the Troll regenerates for a black mana");
    game.apply(PlayerId::Two, regenerate).expect("it activates");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    cast_at(&mut game, grasp, victim);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(alive(&game, victim), "the shield answered the destruction");
    assert_eq!(game.players[PlayerId::One.index()].life, 18, "you paid it");
}

/// "A Grasp that never resolves at all costs nothing": the target leaves
/// underneath it, the spell is countered on resolution, and no life moves.
#[test]
fn a_grasp_that_fizzles_costs_no_life() {
    let (mut game, grasp, victim) = staged(cards::GRIZZLY_BEARS);

    cast_at(&mut game, grasp, victim);
    game.move_permanents_to_graveyard(&[victim]);
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        20,
        "nothing resolved, so nothing was lost",
    );
    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::INFERNAL_GRASP),
        "and the Grasp is spent either way",
    );
}

/// "Target creature" names no controller: your own is as legal a target as
/// theirs, which is a line a deck with a Sheoldred sometimes wants.
#[test]
fn it_reaches_your_own_creatures() {
    let (mut game, grasp, theirs) = staged(cards::GRIZZLY_BEARS);
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);

    let named = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == grasp => Some(
                choices
                    .iter_targets()
                    .filter_map(|target| match target {
                        Target::Permanent(id) => Some(*id),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    assert!(named.contains(&theirs), "theirs is a creature");
    assert!(named.contains(&mine), "and so is yours");

    cast_at(&mut game, grasp, mine);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(!alive(&game, mine), "your own Angel is what you destroyed");
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        18,
        "and the two life is yours to pay either way",
    );
}

/// Hexproof is the one thing it cannot answer: a Caryatid is not a legal
/// target, so the Grasp stays in hand.
#[test]
fn a_hexproof_creature_is_no_target_at_all() {
    let (game, grasp, hexproof) = staged(cards::SYLVAN_CARYATID);

    let named = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == grasp => Some(
                choices
                    .iter_targets()
                    .filter_map(|target| match target {
                        Target::Permanent(id) => Some(*id),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();

    assert!(
        !named.contains(&hexproof),
        "hexproof takes it off the list: {named:?}",
    );
    assert!(
        named.is_empty(),
        "and there is nothing else to point it at, so it is uncastable",
    );
}

/// The life is not optional and not a cost: at two life the Grasp kills the
/// creature and then kills you.
#[test]
fn it_can_kill_its_own_caster() {
    let (mut game, grasp, victim) = staged(cards::GRIZZLY_BEARS);
    game.players[PlayerId::One.index()].life = 2;

    cast_at(&mut game, grasp, victim);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(!alive(&game, victim), "the Bears died first");
    assert_eq!(game.players[PlayerId::One.index()].life, 0);
    assert!(
        matches!(
            game.result,
            Some(GameResult::Winner {
                winner: PlayerId::Two,
                ..
            })
        ),
        "and then you did: {:?}",
        game.result,
    );
}
