//! Mutagenic Growth: two power and two toughness for two life, or for a
//! green mana when there is one to spare.

use super::*;

/// The Growth in hand and a Grizzly Bears to point it at.
fn staged() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let bears = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    let growth = game
        .build_zone(PlayerId::One, &[cards::MUTAGENIC_GROWTH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let growth_id = growth.id;
    game.players[0].hand.push(growth);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].life = 20;
    (game, growth_id, bears)
}

/// The cast at the Bears that pays with `life` rather than mana.
fn cast_at(
    game: &Game,
    growth: GameObjectId,
    bears: GameObjectId,
    with_life: bool,
) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == growth
                    && choices.mana_payment().alternatives().is_empty() != with_life
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(bears)))
        })
}

fn power(game: &Game, id: GameObjectId) -> Option<i16> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .and_then(|permanent| game.power(permanent))
}

/// Two life and no mana at all buys the Bears two power and two toughness.
#[test]
fn two_life_makes_a_four_four() {
    let (mut game, growth, bears) = staged();

    let cast = cast_at(&game, growth, bears, true).expect("two life casts it with no mana");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(power(&game, bears), Some(4), "a 2/2 became a 4/4");
    assert_eq!(game.players[0].life, 18, "two life for the pip");
}

/// A green mana pays it instead, and then it costs no life.
#[test]
fn a_green_mana_pays_for_it_too() {
    let (mut game, growth, bears) = staged();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);

    let cast = cast_at(&game, growth, bears, false).expect("a green mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(power(&game, bears), Some(4));
    assert_eq!(game.players[0].life, 20, "no life was paid");
}

/// "Until end of turn": the Bears are a 2/2 again next turn.
#[test]
fn the_bonus_wears_off() {
    let (mut game, growth, bears) = staged();

    let cast = cast_at(&game, growth, bears, true).expect("it is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);
    assert_eq!(power(&game, bears), Some(4));

    game.step = Step::Cleanup;
    game.finish_cleanup();
    game.start_next_turn();

    assert_eq!(power(&game, bears), Some(2), "back to what it prints");
}

/// CR 118.4: a player may pay life down to exactly zero and no further, so
/// the life half is gone at one life -- and with no green mana that is the
/// whole of the card.
#[test]
fn one_life_leaves_only_the_green_mana() {
    let (mut game, growth, bears) = staged();
    game.players[0].life = 2;
    assert!(
        cast_at(&game, growth, bears, true).is_some(),
        "two life is exactly two life",
    );

    game.players[0].life = 1;
    assert!(
        cast_at(&game, growth, bears, true).is_none(),
        "one life cannot pay two",
    );
    assert!(
        cast_at(&game, growth, bears, false).is_none(),
        "and there is no mana"
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    assert!(
        cast_at(&game, growth, bears, false).is_some(),
        "the green mana is the way out",
    );
}

/// "A card with Phyrexian mana symbols in its mana cost is each colour that
/// appears in that cost", and the symbol counts as one for mana value --
/// however it was paid for.
#[test]
fn it_is_a_green_card_worth_one_mana() {
    let catalog = poc::catalog().expect("catalog builds");
    let growth = catalog
        .get(cards::MUTAGENIC_GROWTH)
        .expect("the Growth is cataloged");

    assert_eq!(
        growth.rules.mana_cost().map(ManaCost::mana_value),
        Some(1),
        "the pip counts as one",
    );
    let green = ManaColor::Green.color_index().expect("green is a colour");
    assert!(
        growth.rules.colors()[green],
        "and the card is green whether or not a green mana ever paid for it",
    );
}

/// "You choose how to pay for each Phyrexian mana symbol at the same time you
/// would choose modes or a value for X." Both ways are on offer as the spell
/// is cast, and they are two different casts rather than one the engine
/// settles for you.
#[test]
fn both_ways_to_pay_are_offered_as_the_spell_is_cast() {
    let (mut game, growth, bears) = staged();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);

    let with_life = cast_at(&game, growth, bears, true).expect("twenty life pays it");
    let with_mana = cast_at(&game, growth, bears, false).expect("so does the green mana");
    assert_ne!(with_life, with_mana, "and they are not the same cast");

    game.apply(PlayerId::One, with_mana).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(power(&game, bears), Some(4), "the Bears grew");
    assert_eq!(
        game.players[0].life, 20,
        "and paying the mana costs no life at all",
    );
}

/// "Target creature" does not say whose. Pumping their blocker is a strange
/// play, but the spell allows it.
#[test]
fn it_can_point_at_a_creature_they_control() {
    let (mut game, growth, _bears) = staged();
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;

    let action = cast_at(&game, growth, theirs, true).expect("their Bears are a legal target");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(power(&game, theirs), Some(4), "their Bears grew");
    assert_eq!(game.players[0].life, 18, "on your two life");
}

/// Paying at exactly two life is legal and leaves you at zero, which the
/// game notices immediately.
#[test]
fn paying_the_last_two_life_loses_the_game() {
    let (mut game, growth, bears) = staged();
    game.players[0].life = 2;

    let action = cast_at(&game, growth, bears, true).expect("two life is exactly enough");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(game.players[0].life, 0);
    assert!(
        game.result.is_some(),
        "nobody survives paying their last two life",
    );
}
