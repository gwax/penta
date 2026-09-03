//! Leyline of Singularity: a live layer-4 supertype change over every
//! nonland permanent, regardless of controller.

use super::*;

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("the permanent remains on the battlefield")
}

fn count(game: &Game, definition: CardDefinitionId, controller: PlayerId) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| {
            permanent.card.definition == ObjectKind::Card(definition)
                && permanent.controller == controller
        })
        .count()
}

#[test]
fn every_nonland_permanent_is_legendary_only_while_the_leyline_remains() {
    let mut game = ready_game();
    let leyline = game
        .put_onto_battlefield(PlayerId::One, cards::LEYLINE_OF_SINGULARITY)
        .expect("Leyline of Singularity is cataloged");
    let first_bear = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("Grizzly Bears is cataloged");
    let first_mountain = game
        .put_onto_battlefield(PlayerId::Two, cards::MOUNTAIN)
        .expect("Mountain is cataloged");
    drain_pending(&mut game);

    assert!(
        game.permanent_supertypes(permanent(&game, leyline))
            .is_some_and(|types| types.contains(CardSupertype::Legendary)),
        "the Leyline is itself a nonland permanent",
    );
    assert!(
        game.permanent_supertypes(permanent(&game, first_bear))
            .is_some_and(|types| types.contains(CardSupertype::Legendary)),
        "the effect reaches an opponent's nonland permanent",
    );
    assert!(
        !game
            .permanent_supertypes(permanent(&game, first_mountain))
            .is_some_and(|types| types.contains(CardSupertype::Legendary)),
        "lands are outside the effect",
    );

    game.put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("a second Grizzly Bears is cataloged");
    game.put_onto_battlefield(PlayerId::Two, cards::MOUNTAIN)
        .expect("a second Mountain is cataloged");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        count(&game, cards::GRIZZLY_BEARS, PlayerId::Two),
        1,
        "the legend rule sees the granted supertype",
    );
    assert_eq!(
        count(&game, cards::MOUNTAIN, PlayerId::Two),
        2,
        "same-named lands remain ordinary",
    );

    game.sacrifice_permanent(leyline);
    game.check_state_based_actions();
    game.put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("another Grizzly Bears is cataloged");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        count(&game, cards::GRIZZLY_BEARS, PlayerId::Two),
        2,
        "the continuous effect ends with its source",
    );
}

#[test]
fn the_legend_rule_compares_names_not_underlying_characteristic_records() {
    let mut game = ready_game();
    game.put_onto_battlefield(PlayerId::One, cards::LEYLINE_OF_SINGULARITY)
        .expect("Leyline of Singularity is cataloged");
    drain_pending(&mut game);

    game.create_token(
        PlayerId::Two,
        tokens::creature(&["Spirit"], &[ManaColor::White], 1, 1),
    );
    game.create_token(
        PlayerId::Two,
        tokens::creature(&["Spirit"], &[ManaColor::Green], 2, 2),
    );
    game.check_state_based_actions();
    drain_pending(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| {
                permanent.controller == PlayerId::Two
                    && game.effective_permanent_name(permanent).as_deref() == Some("Spirit")
            })
            .count(),
        1,
        "different token definitions with the same effective name still conflict",
    );
}
