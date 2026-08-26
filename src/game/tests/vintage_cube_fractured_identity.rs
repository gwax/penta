//! Fractured Identity: what leaves their board arrives on yours, which is
//! the whole reason to pay five mana for an exile effect.

use super::*;

/// Player One holding the spell with five mana up, and `theirs` on the
/// battlefield under Player Two.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut ids = Vec::new();
    for definition in theirs {
        ids.push(
            game.put_onto_battlefield(PlayerId::Two, *definition)
                .expect("cataloged"),
        );
    }
    drain_pending(&mut game);
    let card = game
        .build_zone(PlayerId::One, &[cards::FRACTURED_IDENTITY])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let held = card.id;
    game.players[0].hand.push(card);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, held, ids)
}

/// Casts it at `target` and lets it resolve.
fn cast(game: &mut Game, held: GameObjectId, target: GameObjectId) {
    let action =
        game.legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| match action {
                Action::CastSpell { card, choices, .. } => {
                    *card == held
                        && choices.targets().iter().any(|selection| {
                            selection.targets().contains(&Target::Permanent(target))
                        })
                }
                _ => false,
            })
            .expect("it can point at that permanent");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(game);
}

/// Every permanent that player controls whose effective name is `name` --
/// a token copy carries the copied name rather than the copied card.
fn controlled_by<'a>(game: &'a Game, player: PlayerId, name: &str) -> Vec<&'a Permanent> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.controller == player)
        .filter(|permanent| {
            game.effective_permanent_name(permanent)
                .is_some_and(|effective| effective == name)
        })
        .collect()
}

/// Their creature is exiled and you get the copy.
#[test]
fn their_permanent_becomes_your_token() {
    let (mut game, held, theirs) = staged(&[cards::GRAVE_TITAN]);

    cast(&mut game, held, theirs[0]);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == theirs[0]),
        "the original is gone",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::GRAVE_TITAN),
        "exiled rather than destroyed",
    );
    let copies = controlled_by(&game, PlayerId::One, "Grave Titan");
    assert_eq!(copies.len(), 1, "and the copy is yours");
    assert!(
        copies[0].card.definition != cards::GRAVE_TITAN,
        "a token, not the card itself",
    );
    assert_eq!(game.power(copies[0]), Some(6), "a full copy of the 6/6");
    assert!(
        controlled_by(&game, PlayerId::Two, "Grave Titan").is_empty(),
        "its own controller gets nothing",
    );
}

/// "Each player other than its controller" is read off the target: pointed
/// at your own permanent, the copy goes to your opponent.
#[test]
fn your_own_permanent_hands_them_the_copy() {
    let (mut game, held, _) = staged(&[]);
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    cast(&mut game, held, mine);

    assert!(
        controlled_by(&game, PlayerId::One, "Grizzly Bears").is_empty(),
        "yours is exiled and you get nothing back",
    );
    assert_eq!(
        controlled_by(&game, PlayerId::Two, "Grizzly Bears").len(),
        1,
        "the copy went to the other player",
    );
}

/// It answers any nonland permanent, not only creatures.
#[test]
fn it_takes_a_noncreature_permanent_too() {
    let (mut game, held, theirs) = staged(&[cards::HOWLING_MINE]);

    cast(&mut game, held, theirs[0]);

    assert_eq!(
        controlled_by(&game, PlayerId::One, "Howling Mine").len(),
        1,
        "the artifact is copied the same way",
    );
}

/// A land is not a legal target.
#[test]
fn a_land_cannot_be_taken() {
    let (mut game, held, _) = staged(&[]);
    let their_land = game
        .put_onto_battlefield(PlayerId::Two, cards::FOREST)
        .expect("cataloged");
    drain_pending(&mut game);

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
            if *card == held
                && choices.targets().iter().any(|selection| {
                    selection.targets().contains(&Target::Permanent(their_land))
                }))
        }),
        "\"nonland permanent\" leaves their lands alone",
    );
}
