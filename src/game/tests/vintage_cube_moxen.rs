//! What the Moxen are besides a coloured mana: free artifacts rather than
//! lands, and artifacts is what a lock reads.

use super::*;

/// The five of them, with the colour each makes.
const MOXEN: [(CardDefinitionId, ManaColor); 5] = [
    (cards::MOX_EMERALD, ManaColor::Green),
    (cards::MOX_JET, ManaColor::Black),
    (cards::MOX_PEARL, ManaColor::White),
    (cards::MOX_RUBY, ManaColor::Red),
    (cards::MOX_SAPPHIRE, ManaColor::Blue),
];

/// A Mox costs nothing and is not a land: the whole handful is playable on
/// the turn they are drawn, and the land drop is still there afterwards.
#[test]
fn a_handful_of_moxen_costs_nothing_and_no_land_drop() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].mana_pool = ManaPool::default();
    let mut held = Vec::new();
    for (index, (definition, _)) in MOXEN.into_iter().enumerate() {
        let card = card(
            99_500 + u32::try_from(index).expect("five of them"),
            definition,
            PlayerId::One,
        );
        held.push((card.id, definition));
        game.players[PlayerId::One.index()].hand.push(card);
    }
    let forest = card(99_600, cards::FOREST, PlayerId::One);
    let forest_id = forest.id;
    game.players[PlayerId::One.index()].hand.push(forest);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[PlayerId::One.index()].lands_played_this_turn = 0;

    for (id, definition) in held {
        let cast = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == id))
            .unwrap_or_else(|| panic!("{definition:?} costs nothing at all"));
        game.apply(PlayerId::One, cast).expect("it is cast");
        drain_pending(&mut game);
    }

    assert_eq!(
        game.battlefield.len(),
        5,
        "five artifacts, none of them a land drop",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].lands_played_this_turn,
        0,
        "and the turn's land is still unplayed",
    );

    let play = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::PlayLand { card, .. } if *card == forest_id))
        .expect("which is what the Forest is for");
    game.apply(PlayerId::One, play).expect("the land is played");
    drain_pending(&mut game);

    for (_, color) in MOXEN {
        assert!(
            game.legal_actions(PlayerId::One)
                .iter()
                .any(|action| matches!(action, Action::ActivateManaAbility { color: made, .. } if *made == color)),
            "every one of them taps for its colour the turn it arrived",
        );
    }
}

/// "Activated abilities of artifacts can't be activated." A Mox is an
/// artifact and its mana ability is an activated ability, so a Collector
/// Ouphe turns the whole handful off.
#[test]
fn an_ouphe_turns_every_mox_off() {
    let mut game = ready_game();
    game.battlefield.clear();
    for (definition, _) in MOXEN {
        game.put_onto_battlefield(PlayerId::One, definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let mana_actions = |game: &Game| {
        game.legal_actions(PlayerId::One)
            .into_iter()
            .filter(|action| matches!(action, Action::ActivateManaAbility { .. }))
            .count()
    };
    assert_eq!(mana_actions(&game), 5, "five Moxen, five colours");

    let ouphe = game
        .put_onto_battlefield(PlayerId::One, cards::COLLECTOR_OUPHE)
        .expect("cataloged");
    drain_pending(&mut game);

    assert_eq!(
        mana_actions(&game),
        0,
        "and none of them while the Ouphe is standing",
    );

    game.move_permanents_to_graveyard(&[ouphe]);
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        mana_actions(&game),
        5,
        "the lock is the Ouphe, so they come back with it gone",
    );
}
