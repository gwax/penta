//! Bone Shards: one black mana and a second card, which is the whole cost.
//!
//! The two ways of paying and the discard that pays are pinned in
//! `vintage_cube_more_spells`. What is here is the ruling that the price is
//! not optional, and what the spell may be pointed at once it is paid.

use super::*;

/// Player One holding the Shards with one black up, and nothing else unless
/// the test puts it there.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let shards = card(97_000, cards::BONE_SHARDS, PlayerId::One);
    let shards_id = shards.id;
    game.players[0].hand.push(shards);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    (game, shards_id)
}

/// Every way of casting the Shards this seat is offered.
fn casts(game: &Game, shards: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card, .. } if *card == shards))
        .collect()
}

/// "You must sacrifice exactly one creature or discard exactly one card to
/// cast this spell; you can't cast it without." With nothing else in hand
/// and no creature to give up, one black mana is not enough -- and the
/// Shards itself is no answer, being on the stack by the time its costs are
/// paid rather than still in the hand it would be discarded from.
#[test]
fn with_nothing_to_pay_it_cannot_be_cast_at_all() {
    let (mut game, shards) = staged();
    game.put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    assert!(
        casts(&game, shards).is_empty(),
        "the additional cost has nothing to pay it with",
    );

    game.players[0]
        .hand
        .push(card(97_001, cards::LIGHTNING_BOLT, PlayerId::One));

    assert_eq!(
        casts(&game, shards).len(),
        1,
        "one spare card is one way to pay, and the only one",
    );
}

/// "Sacrifice a creature": one you control. Their board is not a price you
/// are allowed to pay.
#[test]
fn their_creature_is_not_a_price_you_may_pay() {
    let (mut game, shards) = staged();
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    assert!(
        casts(&game, shards).is_empty(),
        "a creature you do not control pays nothing",
    );

    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let paid: Vec<_> = casts(&game, shards)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { sacrifices, .. } => Some(sacrifices),
            _ => None,
        })
        .collect();
    assert!(!paid.is_empty(), "your own creature makes it castable");
    assert!(
        paid.iter().all(|sacrifice| sacrifice == &[mine]),
        "your own is the only one offered",
    );
    assert!(
        paid.iter().all(|sacrifice| sacrifice != &[theirs]),
        "and theirs is never on the list",
    );
}

/// "Destroy target creature or planeswalker." A planeswalker is the half the
/// existing coverage never names, and it dies to the same one black mana.
#[test]
fn a_planeswalker_is_a_target_and_is_destroyed() {
    let (mut game, shards) = staged();
    game.players[0]
        .hand
        .push(card(97_010, cards::LIGHTNING_BOLT, PlayerId::One));
    let walker = game
        .put_onto_battlefield(PlayerId::Two, cards::JACE_THE_MIND_SCULPTOR)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let cast = casts(&game, shards)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { choices, .. } => choices
                .iter_targets()
                .any(|target| *target == Target::Permanent(walker)),
            _ => false,
        })
        .expect("a planeswalker is a legal target");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_until_decision(&mut game);
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == walker),
        "he was destroyed with his loyalty untouched",
    );
}

/// And nothing else is: an artifact and an enchantment are permanents the
/// Shards has no answer for, whatever else is on the table to point it at.
#[test]
fn it_names_neither_an_artifact_nor_an_enchantment() {
    let (mut game, shards) = staged();
    game.players[0]
        .hand
        .push(card(97_020, cards::LIGHTNING_BOLT, PlayerId::One));
    let lotus = game
        .put_onto_battlefield(PlayerId::Two, cards::BLACK_LOTUS)
        .expect("cataloged");
    let moat = game
        .put_onto_battlefield(PlayerId::Two, cards::MOAT)
        .expect("cataloged");
    let angel = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let named: Vec<_> = casts(&game, shards)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { choices, .. } => {
                choices.iter_targets().find_map(|target| match target {
                    Target::Permanent(id) => Some(*id),
                    _ => None,
                })
            }
            _ => None,
        })
        .collect();

    assert!(named.contains(&angel), "the creature is a target");
    assert!(!named.contains(&lotus), "the artifact is not");
    assert!(!named.contains(&moat), "and neither is the enchantment");
}
