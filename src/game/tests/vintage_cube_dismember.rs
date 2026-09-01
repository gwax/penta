//! Dismember: five toughness off anything, for one mana and as much life as
//! you are short of black.

use super::*;

/// Player One holding a Dismember, with `theirs` across the table.
fn staged(theirs: CardDefinitionId) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let victim = game
        .put_onto_battlefield(PlayerId::Two, theirs)
        .expect("cataloged");
    drain_pending(&mut game);
    let dismember = card(89_000, cards::DISMEMBER, PlayerId::One);
    let dismember_id = dismember.id;
    game.players[PlayerId::One.index()].hand.push(dismember);
    game.players[PlayerId::One.index()].life = 20;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, dismember_id, victim)
}

/// Every way Player One could cast it at `victim` right now.
fn casts(game: &Game, dismember: GameObjectId, victim: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == dismember
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(victim))
            }
            _ => false,
        })
        .collect()
}

fn cast_it(game: &mut Game, dismember: GameObjectId, victim: GameObjectId) {
    let action = casts(game, dismember, victim)
        .into_iter()
        .next()
        .expect("it is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(game);
    game.check_state_based_actions();
}

fn size(game: &Game, victim: GameObjectId) -> Option<(Option<i16>, Option<i16>)> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == victim)
        .map(|permanent| (game.power(permanent), game.toughness(permanent)))
}

/// Five toughness off is lethal to most things.
#[test]
fn it_kills_what_it_names() {
    let (mut game, dismember, victim) = staged(cards::SERRA_ANGEL);
    game.players[PlayerId::One.index()].mana_pool.black = 2;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;

    cast_it(&mut game, dismember, victim);

    assert!(size(&game, victim).is_none(), "a 4/4 is well short of five");
    assert!(
        game.players[PlayerId::Two.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
    );
}

/// It is -5/-5 and not destruction: something big enough lives, shrunken,
/// and is itself again once the turn ends.
#[test]
fn something_bigger_than_five_survives_until_the_cleanup() {
    let (mut game, dismember, victim) = staged(cards::MYR_BATTLESPHERE);
    game.players[PlayerId::One.index()].mana_pool.black = 2;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    let before = size(&game, victim).expect("it is on the battlefield");

    cast_it(&mut game, dismember, victim);

    let after = size(&game, victim).expect("a 4/7 survives -5/-5");
    assert_eq!(
        (after.0, after.1),
        (before.0.map(|p| p - 5), before.1.map(|t| t - 5)),
        "five off each",
    );

    let turn = game.turn;
    for _ in 0..60 {
        if game.turn > turn {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }
    assert_eq!(
        size(&game, victim),
        Some(before),
        "and until end of turn is over",
    );
}

/// "{B/P} can be paid with either {B} or 2 life": two pips, so with two
/// black and a generic in the pool every mixture is on offer, and what each
/// one costs is nothing, two life, or four.
#[test]
fn each_pip_takes_black_or_two_life() {
    let (game, dismember, victim) = staged(cards::SERRA_ANGEL);
    let mut game = game;
    game.players[PlayerId::One.index()].mana_pool.black = 2;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    let offers = casts(&game, dismember, victim).len();
    assert_eq!(offers, 3, "both pips either way is three ways to pay");

    // Each of the three, taken in its own game: the life it costs is the
    // count of pips it did not pay with black.
    let mut paid = (0..offers)
        .map(|index| {
            let (mut game, dismember, victim) = staged(cards::SERRA_ANGEL);
            game.players[PlayerId::One.index()].mana_pool.black = 2;
            game.players[PlayerId::One.index()].mana_pool.colorless = 1;
            let action = casts(&game, dismember, victim)
                .into_iter()
                .nth(index)
                .expect("the offer is there");
            game.apply(PlayerId::One, action).expect("it is cast");
            drain_pending(&mut game);
            game.check_state_based_actions();
            assert!(size(&game, victim).is_none(), "it resolved either way");
            20 - game.players[PlayerId::One.index()].life
        })
        .collect::<Vec<_>>();
    paid.sort_unstable();

    assert_eq!(paid, vec![0, 2, 4], "no life, one pip's worth, or both");
}

/// Life you do not have is not a way to pay: at three life the pips cannot
/// both be bought, and one black mana beside it is enough again.
#[test]
fn you_cannot_pay_life_you_do_not_have() {
    let (mut game, dismember, victim) = staged(cards::SERRA_ANGEL);
    game.players[PlayerId::One.index()].life = 3;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;

    assert!(
        casts(&game, dismember, victim).is_empty(),
        "four life is more than three",
    );

    game.players[PlayerId::One.index()].mana_pool.black = 1;
    assert!(
        !casts(&game, dismember, victim).is_empty(),
        "one black and two life is within reach",
    );
}

/// "A Phyrexian mana symbol contributes 1 toward the mana value of a card,
/// even if life is paid for it. Specifically, Dismember's mana value is
/// always 3."
#[test]
fn its_mana_value_is_three_however_it_was_paid() {
    for black in [2, 0] {
        let (mut game, dismember, victim) = staged(cards::SERRA_ANGEL);
        game.players[PlayerId::One.index()].mana_pool.black = black;
        game.players[PlayerId::One.index()].mana_pool.colorless = 1;

        let action = casts(&game, dismember, victim)
            .into_iter()
            .next()
            .expect("it is castable");
        game.apply(PlayerId::One, action).expect("it is cast");
        let spell = game
            .stack
            .iter()
            .next()
            .map(|object| object.id)
            .expect("it is on the stack");

        assert_eq!(
            game.current_or_last_known_mana_value(spell),
            Some(3),
            "three, with {black} of the pips paid in mana",
        );
    }
}

/// A toughness of zero is not destruction: an indestructible Myr has no
/// answer to -5/-5, and neither does a regeneration shield.
#[test]
fn indestructible_and_regeneration_are_no_answer() {
    for definition in [cards::DARKSTEEL_MYR, cards::SEDGE_TROLL] {
        let (mut game, dismember, victim) = staged(definition);
        game.players[PlayerId::One.index()].mana_pool.black = 2;
        game.players[PlayerId::One.index()].mana_pool.colorless = 1;
        if let Some(permanent) = game
            .battlefield
            .iter_mut()
            .find(|permanent| permanent.card.id == victim)
        {
            permanent.regeneration_shields = 1;
        }

        cast_it(&mut game, dismember, victim);

        assert!(
            size(&game, victim).is_none(),
            "{definition:?} was put into the graveyard for zero toughness",
        );
    }
}

/// "Target creature" says nothing about whose: your own is on the list,
/// which is how a Dismember answers something the other player has taken.
#[test]
fn it_may_name_your_own_creature() {
    let (mut game, dismember, theirs) = staged(cards::GRIZZLY_BEARS);
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    game.players[PlayerId::One.index()].mana_pool.black = 2;
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    game.priority = PlayerId::One;

    assert!(
        !casts(&game, dismember, mine).is_empty(),
        "your own Angel is a creature like any other",
    );
    assert!(
        !casts(&game, dismember, theirs).is_empty(),
        "and so is their bear",
    );

    cast_it(&mut game, dismember, mine);

    assert!(
        size(&game, mine).is_none(),
        "a 4/4 does not survive -5/-5, whoever controls it",
    );
}
