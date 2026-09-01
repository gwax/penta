//! Unexpectedly Absent: two mana that answers any nonland permanent by
//! putting it back on top of the library it came from.

use super::*;

/// Player One holding the spell with `x` worth of mana up, and `battlefield`
/// already down.
fn staged(x: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let absent = card(95_000, cards::UNEXPECTEDLY_ABSENT, PlayerId::One);
    let absent_id = absent.id;
    game.players[0].hand.push(absent);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 2);
    if x > 0 {
        game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, x);
    }
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, absent_id)
}

/// The cast that names `target` for `x`, if it is on offer at all.
fn cast_at(game: &Game, absent: GameObjectId, x: u16, target: GameObjectId) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == absent
                    && choices.x() == x
                    && choices
                        .iter_targets()
                        .any(|slot| *slot == Target::Permanent(target))
            }
            _ => false,
        })
}

fn resolve(game: &mut Game) {
    pass_until_decision(game);
    drain_pending(game);
}

/// "Target nonland permanent": a land is no target, however much you would
/// like it to be.
#[test]
fn a_land_is_not_a_legal_target() {
    let (mut game, absent) = staged(0);
    let island = game
        .put_onto_battlefield(PlayerId::Two, cards::ISLAND)
        .expect("cataloged");
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    assert!(
        cast_at(&game, absent, 0, island).is_none(),
        "the clause says nonland",
    );
    assert!(
        cast_at(&game, absent, 0, bears).is_some(),
        "and the creature beside it is a target from the same seat",
    );
}

/// Anything else is: an enchantment and a planeswalker are both nonland
/// permanents, which is why the card answers what nothing else can.
#[test]
fn an_enchantment_or_a_planeswalker_may_be_named() {
    for definition in [cards::MOAT, cards::JACE_THE_MIND_SCULPTOR] {
        let (mut game, absent) = staged(0);
        let permanent = game
            .put_onto_battlefield(PlayerId::Two, definition)
            .expect("cataloged");
        drain_pending(&mut game);
        game.priority = PlayerId::One;

        let cast = cast_at(&game, absent, 0, permanent)
            .unwrap_or_else(|| panic!("{definition:?} is a nonland permanent"));
        game.apply(PlayerId::One, cast).expect("it is cast");
        resolve(&mut game);

        assert!(
            !game
                .battlefield
                .iter()
                .any(|existing| existing.card.id == permanent),
            "{definition:?} left the battlefield",
        );
        assert_eq!(
            game.players[1].library.last().map(|card| card.definition),
            Some(definition),
            "{definition:?} landed on top of its owner's library",
        );
    }
}

/// "Its owner's library." A creature you stole is still theirs to own, so
/// the Absent hands it back to the library it came from rather than putting
/// it into the library of whoever happens to control it.
#[test]
fn it_goes_to_the_owners_library_and_not_the_controllers() {
    let (mut game, absent) = staged(0);
    let stolen = Permanent::entering(
        card(95_100, cards::GRIZZLY_BEARS, PlayerId::Two),
        CardPartId::PRIMARY,
        PlayerId::One,
        0,
        0,
    );
    let stolen_id = stolen.card.id;
    game.battlefield.push(stolen);
    game.players[0].library.clear();
    game.players[1].library.clear();
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let cast = cast_at(&game, absent, 0, stolen_id).expect("your own creature is a target");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve(&mut game);

    assert!(
        game.players[0].library.is_empty(),
        "the seat that controlled it gets nothing",
    );
    assert_eq!(
        game.players[1].library.last().map(|card| card.definition),
        Some(cards::GRIZZLY_BEARS),
        "and its owner draws it back",
    );
}

/// A token is a nonland permanent and a legal target, and then it is nothing
/// at all: it leaves the battlefield and no card joins any library.
#[test]
fn a_token_is_named_and_then_ceases_to_exist() {
    let (mut game, absent) = staged(0);
    game.create_token(
        PlayerId::Two,
        tokens::creature(&["Bear"], &[ManaColor::Green], 2, 2),
    );
    drain_pending(&mut game);
    let token = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Token)
        .expect("the token is out")
        .card
        .id;
    game.players[1].library.clear();
    game.priority = PlayerId::One;

    let cast = cast_at(&game, absent, 0, token).expect("a token is a nonland permanent");
    game.apply(PlayerId::One, cast).expect("it is cast");
    resolve(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == token),
        "it left the battlefield",
    );
    assert!(
        game.players[1].library.is_empty(),
        "and a token that leaves is gone rather than filed",
    );
}
