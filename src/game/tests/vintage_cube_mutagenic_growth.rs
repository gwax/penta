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

/// The reason it costs nothing: on their turn, with their Bolt on the stack
/// and no mana of yours open, two life turns the 2/2 into a 4/4 that lives
/// through three damage.
#[test]
fn two_life_saves_a_creature_from_their_bolt() {
    let (mut game, growth, bears) = staged();
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.players[PlayerId::One.index()].mana_pool = ManaPool::default();
    let bolt = card(113_000, cards::LIGHTNING_BOLT, PlayerId::Two);
    let bolt_id = bolt.id;
    game.players[PlayerId::Two.index()].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(bolt_id, vec![Target::Permanent(bears)], Vec::new(), 0),
    )
    .expect("their Bolt names your bear");

    game.priority = PlayerId::One;
    let save = cast_at(&game, growth, bears, true)
        .expect("no mana at all, and their turn, and it is still castable");
    game.apply(PlayerId::One, save).expect("it is cast");
    for _ in 0..8 {
        if game.stack.iter().count() == 0 {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();

    let survivor = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears)
        .expect("a 4/4 lives through three damage");
    assert_eq!(survivor.damage, 3, "the Bolt still dealt its three");
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        18,
        "and the two life was what paid for the save",
    );
}

/// The life is paid rather than dealt, so a shield that answers damage
/// answers none of it: the Growth still costs its two.
#[test]
fn the_life_is_paid_and_not_dealt() {
    let (mut game, growth, bears) = staged();
    let angel = card(113_100, cards::GUARDIAN_ANGEL, PlayerId::One);
    let angel_id = angel.id;
    game.players[PlayerId::One.index()].hand.push(angel);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2);
    game.apply(
        PlayerId::One,
        cast_action(angel_id, vec![Target::Player(PlayerId::One)], Vec::new(), 2),
    )
    .expect("a shield of two is castable");
    drain_pending(&mut game);
    game.empty_mana_pools();
    let life = game.players[PlayerId::One.index()].life;

    let cast = cast_at(&game, growth, bears, true).expect("two life pays for it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life - 2,
        "the shield answers damage, and this is a payment",
    );
    assert_eq!(power(&game, bears), Some(4), "and the bear grew for it");
}

/// The life is part of the cost and is paid on announcement, so answering
/// the Growth by killing what it named leaves its caster two life down with
/// nothing to show for it: the spell fizzles rather than half-resolving.
#[test]
fn the_life_is_gone_even_when_the_growth_fizzles() {
    let (mut game, growth, bears) = staged();

    let cast = cast_at(&game, growth, bears, true).expect("two life casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    assert_eq!(
        game.players[0].life, 18,
        "the two life went on announcement",
    );

    game.move_permanents_to_graveyard(&[bears]);
    game.check_state_based_actions();
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        if game.apply(game.priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();

    assert_eq!(
        game.players[0].life, 18,
        "and nothing gave it back when the spell found nothing",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MUTAGENIC_GROWTH),
        "the Growth went to the graveyard having done nothing",
    );
}

/// "Phyrexian mana is not a new colour": {G/P} takes green mana or two life
/// and nothing else. A pool of black at one life is a pool that cannot cast
/// it, however much of it there is, and one green beside the black is what
/// buys the spell.
#[test]
fn only_green_mana_pays_the_pip() {
    let (mut game, growth, bears) = staged();
    game.players[0].life = 1;
    for color in [ManaColor::Black, ManaColor::Red, ManaColor::White] {
        game.add_unrestricted_mana(PlayerId::One, color, 3);
    }

    assert!(
        cast_at(&game, growth, bears, false).is_none(),
        "nine mana of the wrong three colours pays no green pip",
    );
    assert!(
        cast_at(&game, growth, bears, true).is_none(),
        "and one life is still not two",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    assert!(
        cast_at(&game, growth, bears, false).is_some(),
        "the one green among them is the whole of what it wanted",
    );
}
