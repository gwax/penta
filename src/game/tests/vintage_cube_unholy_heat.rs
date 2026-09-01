//! Unholy Heat: one red for two damage, or six once the graveyard is deep
//! enough in kinds.
//!
//! The threshold itself, and that it counts your graveyard rather than
//! theirs, is pinned in `vintage_cube_graveyard`. What is here is its second
//! ruling -- "at that time, Unholy Heat isn't in the graveyard yet" -- and
//! the other half of its target line.

use super::*;

/// Player One holding `heats` copies of the Heat with the mana for them,
/// `graveyard` behind them, and a Serra Angel across the table.
fn staged(graveyard: &[CardDefinitionId], heats: usize) -> (Game, Vec<GameObjectId>, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    game.players[1].graveyard.clear();
    for (index, definition) in graveyard.iter().enumerate() {
        game.players[0].graveyard.push(card(
            115_000 + u32::try_from(index).expect("few cards"),
            *definition,
            PlayerId::One,
        ));
    }
    let mut ids = Vec::new();
    for index in 0..heats {
        let heat = card(
            115_100 + u32::try_from(index).expect("few cards"),
            cards::UNHOLY_HEAT,
            PlayerId::One,
        );
        ids.push(heat.id);
        game.players[0].hand.push(heat);
    }
    let angel = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(
        PlayerId::One,
        ManaColor::Red,
        u16::try_from(heats).unwrap_or(1),
    );
    (game, ids, angel)
}

fn damage_on(game: &Game, permanent: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|candidate| candidate.card.id == permanent)
        .map_or(u16::MAX, |candidate| candidate.damage)
}

/// Its ruling: "Unholy Heat checks your graveyard as it resolves... At that
/// time, Unholy Heat isn't in the graveyard yet." Three types behind it and
/// no instant among them: the first copy deals two, and the second deals six
/// because the first is now the instant that was missing.
#[test]
fn it_is_not_yet_the_fourth_card_type_it_needs() {
    let (mut game, heats, angel) = staged(
        &[cards::GRIZZLY_BEARS, cards::FOREST, cards::BLACK_LOTUS],
        2,
    );

    game.apply(
        PlayerId::One,
        cast_action(heats[0], vec![Target::Permanent(angel)], Vec::new(), 0),
    )
    .expect("one red casts it");
    drain_pending(&mut game);

    assert_eq!(
        damage_on(&game, angel),
        2,
        "creature, land and artifact are three: the spell on the stack is no instant in the yard",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::UNHOLY_HEAT),
        "and only now is it there",
    );

    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(heats[1], vec![Target::Permanent(angel)], Vec::new(), 0),
    )
    .expect("the second is castable too");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != angel),
        "the second deals six off the instant the first became, which kills a 4/4",
    );
    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "into their graveyard",
    );
}

/// "Target creature or planeswalker": the other half of the target line,
/// and delirium counts for it just the same.
#[test]
fn it_burns_a_planeswalker_for_the_same_amount() {
    let (mut game, heats, _angel) = staged(
        &[
            cards::GRIZZLY_BEARS,
            cards::FOREST,
            cards::BLACK_LOTUS,
            cards::LIGHTNING_BOLT,
        ],
        1,
    );
    let walker = game
        .put_onto_battlefield(PlayerId::Two, cards::TEFERI_TIME_RAVELER)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    game.apply(
        PlayerId::One,
        cast_action(heats[0], vec![Target::Permanent(walker)], Vec::new(), 0),
    )
    .expect("a planeswalker is a legal target");
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != walker),
        "six damage is more loyalty than he had",
    );
}

/// Casts one Heat from a graveyard of `graveyard` and reports the damage it
/// dealt. A Wall of Stone rather than the Angel takes it, so six is a number
/// to read rather than a creature to bury.
fn heat_damage(graveyard: &[CardDefinitionId]) -> u16 {
    let (mut game, heats, _angel) = staged(graveyard, 1);
    let wall = game
        .put_onto_battlefield(PlayerId::Two, cards::WALL_OF_STONE)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(heats[0], vec![Target::Permanent(wall)], Vec::new(), 0),
    )
    .expect("one red casts it");
    drain_pending(&mut game);
    damage_on(&game, wall)
}

/// "Four or more card *types*", not four cards: a graveyard of four
/// creatures is one type over and over.
#[test]
fn four_cards_of_one_type_are_still_one_type() {
    assert_eq!(
        heat_damage(&[
            cards::GRIZZLY_BEARS,
            cards::SERRA_ANGEL,
            cards::SAVANNAH_LIONS,
            cards::ORNITHOPTER,
        ]),
        2,
        "creature and artifact between them, and that is two",
    );
}

/// And one card can be two of them: an Ornithopter is an artifact creature,
/// so three cards carry the four types the spell is counting.
#[test]
fn one_card_may_supply_two_of_the_types() {
    assert_eq!(
        heat_damage(&[cards::ORNITHOPTER, cards::LIGHTNING_BOLT, cards::FOREST,]),
        6,
        "artifact, creature, instant and land out of three cards",
    );
}

/// "Legendary, basic, and snow are supertypes, not card types; Kavu and
/// Equipment are subtypes." A legendary creature, a basic land and an
/// Equipment are three cards carrying three types between them, and three is
/// not four.
#[test]
fn supertypes_and_subtypes_are_not_card_types() {
    assert_eq!(
        heat_damage(&[
            cards::ADUN_OAKENSHIELD,
            cards::FOREST,
            cards::UMEZAWAS_JITTE,
            cards::GRIZZLY_BEARS,
        ]),
        2,
        "legendary, basic and Equipment add nothing to creature, land and artifact",
    );
    assert_eq!(
        heat_damage(&[
            cards::ADUN_OAKENSHIELD,
            cards::FOREST,
            cards::UMEZAWAS_JITTE,
            cards::LIGHTNING_BOLT,
        ]),
        6,
        "while a fourth card type is a fourth card type",
    );
}
