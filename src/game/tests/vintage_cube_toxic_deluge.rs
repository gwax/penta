//! Toxic Deluge: three mana and as much life as you are willing to spend,
//! and what it takes off every creature it takes off permanently enough to
//! answer things that cannot be destroyed.
//!
//! The cost and the shrink are checked where the spells file keeps them;
//! what this file adds is who the -X/-X reaches and who it does not.

use super::*;

/// Player One holding a Deluge with the mana for it and the life to spend.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let deluge = card(98_000, cards::TOXIC_DELUGE, PlayerId::One);
    let deluge_id = deluge.id;
    game.players[PlayerId::One.index()].hand.push(deluge);
    game.players[PlayerId::One.index()].mana_pool.black = 1;
    game.players[PlayerId::One.index()].mana_pool.colorless = 2;
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, deluge_id)
}

/// Casts it for `x` life and lets it resolve.
fn cast_for(game: &mut Game, deluge: GameObjectId, x: u16) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == deluge && choices.x() == x)
        })
        .unwrap_or_else(|| panic!("{x} life is affordable"));
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(game);
    game.check_state_based_actions();
}

fn on_battlefield(game: &Game, id: GameObjectId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.id == id)
}

/// Indestructible answers destruction and answers nothing here: the Myr is
/// a 0/1, one life takes its toughness to zero, and a creature with zero
/// toughness is put into its owner's graveyard whatever it can shrug off.
#[test]
fn indestructible_does_not_survive_a_toughness_of_zero() {
    let (mut game, deluge) = staged();
    let myr = game
        .put_onto_battlefield(PlayerId::Two, cards::DARKSTEEL_MYR)
        .expect("cataloged");
    drain_pending(&mut game);

    cast_for(&mut game, deluge, 1);

    assert!(
        !on_battlefield(&game, myr),
        "zero toughness is not destruction, so indestructible has no answer",
    );
    assert!(
        game.players[PlayerId::Two.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::DARKSTEEL_MYR),
    );
}

/// "All creatures on the battlefield when Toxic Deluge resolves are
/// affected. Ones that enter the battlefield later in the turn are not."
#[test]
fn a_creature_that_arrives_afterwards_is_untouched() {
    let (mut game, deluge) = staged();
    let doomed = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    cast_for(&mut game, deluge, 2);
    assert!(
        !on_battlefield(&game, doomed),
        "a 2/2 does not survive -2/-2"
    );

    let late = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.check_state_based_actions();

    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == late)
        .expect("the second Bears is still standing");
    assert_eq!(
        (game.power(bears), game.toughness(bears)),
        (Some(2), Some(2)),
        "the effect was over creatures, not over the battlefield",
    );
}

/// It is not their board wipe or yours: X comes off everything, and paying
/// nothing at all is a legal cast that changes nothing.
#[test]
fn it_reaches_both_sides_and_may_be_cast_for_nothing() {
    let (mut game, deluge) = staged();
    let mine = game
        .put_onto_battlefield(PlayerId::One, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);
    let life = game.players[PlayerId::One.index()].life;

    cast_for(&mut game, deluge, 0);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life,
        "nothing was paid",
    );
    for id in [mine, theirs] {
        let permanent = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == id)
            .expect("both are standing");
        assert_eq!(
            game.toughness(permanent),
            Some(if id == mine { 2 } else { 4 }),
            "and nothing shrank",
        );
    }
}

/// "If you cast Toxic Deluge without paying its mana cost, you'll still
/// choose a value for X and pay X life. This is because it doesn't have {X}
/// in its mana cost." Off an Omniscience with an empty pool the mana is
/// waived and the life is not: the choice is still there, the life is still
/// spent, and the board still shrinks by what was paid.
#[test]
fn a_free_cast_still_chooses_x_and_still_pays_the_life() {
    let (mut game, deluge) = staged();
    game.players[PlayerId::One.index()].mana_pool = ManaPool::default();
    game.battlefield
        .push(creature(98_400, cards::OMNISCIENCE, PlayerId::One));
    let bears = creature(98_401, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    let angel = creature(98_402, cards::SERRA_ANGEL, PlayerId::Two);
    let angel_id = angel.card.id;
    game.battlefield.push(angel);
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    let life = game.players[PlayerId::One.index()].life;

    // With no mana at all, every cast on offer is the free one -- and there
    // is still one per value of X.
    let mut offered: Vec<_> = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == deluge => Some(choices.x()),
            _ => None,
        })
        .collect();
    offered.sort_unstable();
    assert!(
        offered.len() > 1 && offered.contains(&3),
        "X is still a choice on a spell that is otherwise free: {offered:?}",
    );

    cast_for(&mut game, deluge, 3);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        life - 3,
        "the three life was paid though no mana was",
    );
    assert!(
        !on_battlefield(&game, bears_id),
        "and a 2/2 is three smaller than it needs to be",
    );
    assert!(
        on_battlefield(&game, angel_id),
        "while the 4/4 lives on one toughness",
    );
}
