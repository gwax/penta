//! Lightning Bolt: one red for three damage to any target.
//!
//! The card is the suite's measuring stick and appears as a tool in most of
//! these files. What is here is the card read on its own: the three things
//! "any target" names, and what three damage does to each of them.

use super::*;

/// Player One holding a Bolt with one red up, a Giant Spider and a Teferi
/// across the table.
fn staged() -> (Game, GameObjectId, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let spider = game
        .put_onto_battlefield(PlayerId::Two, cards::GIANT_SPIDER)
        .expect("cataloged");
    let walker = game
        .put_onto_battlefield(PlayerId::Two, cards::TEFERI_TIME_RAVELER)
        .expect("cataloged");
    drain_pending(&mut game);
    let bolt = game
        .build_zone(PlayerId::One, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let bolt_id = bolt.id;
    game.players[0].hand.push(bolt);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);
    (game, bolt_id, spider, walker)
}

/// "Any target" is three kinds of thing: a creature, a player, and a
/// planeswalker, all on offer from the same card.
#[test]
fn any_target_is_all_three() {
    let (game, bolt, spider, walker) = staged();

    let targets: Vec<Target> = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == bolt => {
                Some(choices.iter_targets().copied().collect::<Vec<_>>())
            }
            _ => None,
        })
        .flatten()
        .collect();

    for wanted in [
        Target::Permanent(spider),
        Target::Permanent(walker),
        Target::Player(PlayerId::Two),
        Target::Player(PlayerId::One),
    ] {
        assert!(
            targets.contains(&wanted),
            "{wanted:?} is a legal target: {targets:?}",
        );
    }
}

/// Three damage is damage rather than destruction: a four-toughness Spider
/// carries it until cleanup and is whole again afterwards.
#[test]
fn a_creature_it_does_not_kill_carries_the_damage_until_cleanup() {
    let (mut game, bolt, spider, _) = staged();

    game.apply(
        PlayerId::One,
        cast_action(bolt, vec![Target::Permanent(spider)], Vec::new(), 0),
    )
    .expect("one red casts it at the Spider");
    drain_pending(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == spider)
            .expect("four toughness survives three")
            .damage,
        3,
        "the damage is marked on it",
    );

    game.cleanup();
    game.finish_cleanup();
    drain_pending(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == spider)
            .expect("it is still there")
            .damage,
        0,
        "and cleanup takes it off again",
    );
}

/// Three to a planeswalker is three loyalty, and three to a player is three
/// life.
#[test]
fn it_takes_three_off_a_walker_or_a_player() {
    let (mut game, bolt, _, walker) = staged();
    game.apply(
        PlayerId::One,
        cast_action(bolt, vec![Target::Permanent(walker)], Vec::new(), 0),
    )
    .expect("a planeswalker is a legal target");
    drain_pending(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == walker)
            .expect("four loyalty survives three")
            .counters(CounterKind::Loyalty),
        1,
        "the damage came off his loyalty",
    );

    let (mut game, bolt, _, _) = staged();
    let life = game.players[1].life;
    game.apply(
        PlayerId::One,
        cast_action(bolt, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("a player is a legal target");
    drain_pending(&mut game);

    assert_eq!(
        game.players[1].life,
        life - 3,
        "and a player takes it as life",
    );
}
