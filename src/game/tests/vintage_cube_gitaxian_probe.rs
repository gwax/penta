//! Gitaxian Probe: a free card that reads their hand on the way past.

use super::*;

/// Player One holding a Probe, with `theirs` in the other player's hand and
/// a library to draw from.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    game.players[0].library.clear();
    for index in 0..4 {
        game.players[0]
            .library
            .push(card(70_000 + index, cards::ISLAND, PlayerId::One));
    }
    for (index, definition) in theirs.iter().enumerate() {
        let id = 70_100 + u32::try_from(index).expect("a handful of cards");
        game.players[1]
            .hand
            .push(card(id, *definition, PlayerId::Two));
    }
    let probe = game
        .build_zone(PlayerId::One, &[cards::GITAXIAN_PROBE])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let probe_id = probe.id;
    game.players[0].hand.push(probe);
    game.turns_started = [3, 3];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].life = 20;
    (game, probe_id)
}

/// The cast of the Probe at Player Two that pays with `life` rather than
/// mana when asked for.
fn cast_at_them(game: &Game, probe: GameObjectId, with_life: bool) -> Option<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == probe
                    && choices.mana_payment().alternatives().is_empty() != with_life
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::Two)))
        })
}

/// Two life and no mana at all: the card is drawn and their hand is seen.
#[test]
fn two_life_buys_a_card_and_a_look() {
    let (mut game, probe) = staged(&[cards::ANCESTRAL_RECALL, cards::SWAMP]);
    let hand = game.players[0].hand.len();

    let cast = cast_at_them(&game, probe, true).expect("two life casts it with no mana");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(game.players[0].life, 18, "two life for the blue pip");
    assert_eq!(
        game.players[0].hand.len(),
        hand,
        "the Probe left and a card arrived, so the hand is the same size",
    );
    assert_eq!(
        game.observe(PlayerId::One)
            .last_seen_hand
            .map(|(player, cards)| (player, cards.len())),
        Some((PlayerId::Two, 2)),
        "and their hand was seen",
    );
}

/// The blue pip may be paid with mana instead, and then it costs no life.
#[test]
fn a_blue_mana_pays_for_it_too() {
    let (mut game, probe) = staged(&[cards::SWAMP]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    let cast = cast_at_them(&game, probe, false).expect("a blue mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(game.players[0].life, 20, "no life was paid");
    assert_eq!(game.players[0].library.len(), 3, "and a card was drawn");
}

/// "Target player" is any player: reading your own hand is legal and, for a
/// deck counting spells cast, sometimes the point.
#[test]
fn it_may_be_aimed_at_yourself() {
    let (game, probe) = staged(&[cards::SWAMP]);

    assert!(
        game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == probe
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Player(PlayerId::One)))
        }),
        "your own hand is a legal target",
    );
}

/// CR 118.4: nobody may pay more life than they have. Paying down to exactly
/// zero is allowed -- it is a loss on the next state-based check, not an
/// illegal payment -- so the offer stops below two rather than at two.
#[test]
fn the_life_payment_stops_when_the_life_runs_out() {
    let (mut game, probe) = staged(&[cards::SWAMP]);
    game.players[0].life = 2;
    assert!(
        cast_at_them(&game, probe, true).is_some(),
        "two life is exactly two life",
    );

    game.players[0].life = 1;
    assert!(
        cast_at_them(&game, probe, true).is_none(),
        "one life cannot pay two",
    );
    assert!(
        cast_at_them(&game, probe, false).is_none(),
        "and there is no mana to pay it with instead",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    assert!(
        cast_at_them(&game, probe, false).is_some(),
        "a blue mana is the way out",
    );
}

/// "The targeted player may have no cards in their hand. You'll still draw a
/// card."
#[test]
fn an_empty_hand_is_still_a_look_and_a_draw() {
    let (mut game, probe) = staged(&[]);
    let library = game.players[0].library.len();

    let cast = cast_at_them(&game, probe, true).expect("it is castable");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert_eq!(
        game.observe(PlayerId::One)
            .last_seen_hand
            .map(|(player, cards)| (player, cards.len())),
        Some((PlayerId::Two, 0)),
        "an empty hand is what was seen",
    );
    assert_eq!(
        game.players[0].library.len(),
        library - 1,
        "and the card is drawn anyway",
    );
}

/// "Count each Phyrexian mana symbol as 1": the Probe is a one-mana spell
/// whichever way the pip was paid, unlike the two-brid symbols that carry a
/// generic alternative.
#[test]
fn the_phyrexian_pip_counts_as_one_for_mana_value() {
    let catalog = poc::catalog().expect("catalog builds");
    let probe = catalog
        .get(cards::GITAXIAN_PROBE)
        .expect("the Probe is cataloged");

    assert_eq!(
        probe.rules.mana_cost().map(ManaCost::mana_value),
        Some(1),
        "two life is not two mana",
    );
}
