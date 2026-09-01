//! Pentad Prism: a two-mana ritual that waits, and counts the colours that
//! paid for it.

use super::*;

/// Player One holding a Pentad Prism, with `mana` available in the listed
/// colours.
fn staged(mana: &[(ManaColor, u16)]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let prism = game
        .build_zone(PlayerId::One, &[cards::PENTAD_PRISM])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let prism_id = prism.id;
    game.players[0].hand.push(prism);
    game.turns_started = [3, 3];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    for (color, amount) in mana {
        game.add_unrestricted_mana(PlayerId::One, *color, *amount);
    }
    (game, prism_id)
}

/// Casts the Prism and resolves it.
fn cast(game: &mut Game, prism: GameObjectId) -> &Permanent {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == prism))
        .expect("two mana casts it");
    game.apply(PlayerId::One, action).expect("it is cast");
    drain_pending(game);
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::PENTAD_PRISM)
        .expect("the Prism resolved")
}

fn charges(game: &Game) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::PENTAD_PRISM)
        .expect("the Prism is there")
        .counters(CounterKind::named("charge"))
}

/// Two colours in, two counters on.
#[test]
fn two_colours_are_two_counters() {
    let (mut game, prism) = staged(&[(ManaColor::White, 1), (ManaColor::Blue, 1)]);

    cast(&mut game, prism);

    assert_eq!(charges(&game), 2, "a counter for each colour spent");
}

/// One colour paying the whole cost is one counter, however much of it there
/// was: the count is colours rather than mana.
#[test]
fn one_colour_is_one_counter() {
    let (mut game, prism) = staged(&[(ManaColor::White, 2)]);

    cast(&mut game, prism);

    assert_eq!(charges(&game), 1);
}

/// Colourless mana is not a colour, so a Prism paid for with it enters with
/// nothing on it and does nothing at all.
#[test]
fn colourless_mana_is_no_colour() {
    let (mut game, prism) = staged(&[(ManaColor::Colorless, 2)]);

    cast(&mut game, prism);

    assert_eq!(charges(&game), 0, "sunburst counts colours");
    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::ActivateManaAbility { source, .. }
                if game.battlefield.iter().any(|permanent| permanent.card.id == *source
                    && permanent.card.definition == cards::PENTAD_PRISM))
        }),
        "and with no counters there is nothing to remove",
    );
}

/// A counter comes off for a mana of any colour, and the counters run out.
#[test]
fn each_counter_buys_a_mana_of_any_colour() {
    let (mut game, prism) = staged(&[(ManaColor::White, 1), (ManaColor::Blue, 1)]);
    let id = cast(&mut game, prism).card.id;
    game.players[0].mana_pool = ManaPool::default();

    let colors = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility { source, color, .. } if source == id => Some(color),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(colors.len(), 5, "one mana of any colour");

    let green = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateManaAbility { source, color, .. }
                if *source == id && *color == ManaColor::Green)
        })
        .expect("green is one of them");
    game.apply(PlayerId::One, green).expect("it activates");

    assert_eq!(game.players[0].mana_pool.green, 1);
    assert_eq!(charges(&game), 1, "one counter spent");

    let second = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateManaAbility { source, .. } if *source == id),
        )
        .expect("the second counter is still there");
    game.apply(PlayerId::One, second).expect("it activates");

    assert_eq!(charges(&game), 0);
    assert!(
        !game.legal_actions(PlayerId::One).iter().any(
            |action| matches!(action, Action::ActivateManaAbility { source, .. } if *source == id)
        ),
        "and an empty Prism makes no more mana",
    );
}

/// No tap symbol anywhere on it: the Prism makes its mana while tapped, and
/// on the turn it arrives, which is what makes it a ritual that waits rather
/// than a rock that has to survive a turn cycle.
#[test]
fn it_needs_neither_untapping_nor_a_turn_to_settle() {
    let (mut game, prism) = staged(&[(ManaColor::White, 1), (ManaColor::Blue, 1)]);
    let id = cast(&mut game, prism).card.id;
    game.players[PlayerId::One.index()].mana_pool = ManaPool::default();
    let arrived = game.turns_started[PlayerId::One.index()];
    let permanent = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == id)
        .expect("it is there");
    permanent.tapped = true;
    permanent.entered_controller_turn = arrived;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateManaAbility { source, color, .. }
                if *source == id && *color == ManaColor::Red)
        })
        .expect("a tapped Prism that arrived this turn still makes mana");
    game.apply(PlayerId::One, action).expect("it activates");

    assert_eq!(game.players[PlayerId::One.index()].mana_pool.red, 1);
    assert_eq!(charges(&game), 1, "one counter is what it cost");
    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == id)
            .expect("it is still there")
            .tapped,
        "and it is as tapped as it was: the cost is the counter, not the tap",
    );
}

/// "Once Pentad Prism has run out of charge counters, it remains on the
/// battlefield." Spent is not sacrificed.
#[test]
fn a_spent_prism_stays_where_it_is() {
    let (mut game, prism) = staged(&[(ManaColor::White, 1), (ManaColor::Blue, 1)]);
    let id = cast(&mut game, prism).card.id;
    for _ in 0..2 {
        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| {
                matches!(action, Action::ActivateManaAbility { source, .. } if *source == id)
            })
            .expect("a counter is still on it");
        game.apply(PlayerId::One, action).expect("it activates");
    }
    game.check_state_based_actions();

    assert_eq!(charges(&game), 0, "both counters are spent");
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == id),
        "and the artifact is still an artifact on the battlefield",
    );
    assert!(
        game.players[PlayerId::One.index()].graveyard.is_empty(),
        "nothing was sacrificed to pay for anything",
    );
}
