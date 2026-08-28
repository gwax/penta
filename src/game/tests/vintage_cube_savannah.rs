//! Savannah: a Forest Plains that is not a basic land. The types are what a
//! fetchland and a mana ability read; the missing supertype is what a
//! Wasteland and a basic-land search read.

use super::*;

/// Player One with a Savannah out and a Forest beside it for contrast.
fn staged() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let savannah = game
        .put_onto_battlefield(PlayerId::One, cards::SAVANNAH)
        .expect("cataloged");
    let forest = game
        .put_onto_battlefield(PlayerId::One, cards::FOREST)
        .expect("cataloged");
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.empty_mana_pools();
    (game, savannah, forest)
}

fn colors_of(game: &Game, id: GameObjectId) -> Vec<ManaColor> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility { source, color, .. } if source == id => Some(color),
            _ => None,
        })
        .collect()
}

/// "This has the mana abilities associated with both of its basic land
/// types."
#[test]
fn it_taps_for_both_of_its_types() {
    let (game, savannah, _) = staged();

    assert_eq!(
        colors_of(&game, savannah),
        vec![ManaColor::Green, ManaColor::White],
        "green and white, in printed subtype order",
    );
}

/// "Things that affect basic lands don't affect it." A Wasteland answers
/// nonbasic lands, and having Forest and Plains printed on it does not make
/// the Savannah basic.
#[test]
fn a_wasteland_answers_it_and_not_the_forest() {
    let (mut game, savannah, forest) = staged();
    let wasteland = game
        .put_onto_battlefield(PlayerId::Two, cards::WASTELAND)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::Two;

    let named = |game: &Game, land: GameObjectId| {
        game.legal_actions(PlayerId::Two).into_iter().any(|action| {
            matches!(
                &action,
                Action::ActivateAbility { source, targets, .. }
                    if *source == wasteland
                        && targets
                            .iter()
                            .any(|selection| selection.targets() == [Target::Permanent(land)])
            )
        })
    };
    assert!(named(&game, savannah), "a nonbasic land, types and all");
    assert!(!named(&game, forest), "and a Forest is not one");

    let destroy = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                Action::ActivateAbility { source, targets, .. }
                    if *source == wasteland
                        && targets
                            .iter()
                            .any(|selection| selection.targets() == [Target::Permanent(savannah)])
            )
        })
        .expect("it may be named");
    game.apply(PlayerId::Two, destroy).expect("it activates");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == savannah),
        "and destroyed",
    );
}

/// The other half of the same ruling: a search that names basic land cards
/// passes it over, however many basic land types it has.
#[test]
fn a_basic_land_search_will_not_find_it() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (index, definition) in [cards::SAVANNAH, cards::FOREST].into_iter().enumerate() {
        game.players[0].library.push(card(
            98_000 + u32::try_from(index).expect("two cards"),
            definition,
            PlayerId::One,
        ));
    }
    let growth = game
        .build_zone(PlayerId::One, &[cards::RAMPANT_GROWTH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let growth_id = growth.id;
    game.players[0].hand.push(growth);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == growth_id))
        .expect("two mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game
        .pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the search asks");
    let offered = decision
        .options
        .iter()
        .filter_map(|option| match option.card {
            Some((_, ObjectCharacteristics::Card { definition, .. })) => Some(definition),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        offered,
        vec![cards::FOREST],
        "the Forest is a basic land card and the Savannah is not",
    );
}
