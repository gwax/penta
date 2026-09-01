//! Sacred Foundry: a Mountain Plains that asks for two life on the way in.
//!
//! The shock clause itself -- both answers, every member of the cycle, one
//! life, and a fetched one arriving tapped whatever it paid -- is pinned in
//! `vintage_cube_lands`. What is here is its ruling: "it's not basic, so
//! cards such as District Guide can't find it, but it does have the
//! appropriate land types."

use super::*;

/// Player One with `library` stacked and a Rampant Growth in hand, ready to
/// search for a basic land card.
fn searching_for_a_basic(library: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (index, definition) in library.iter().enumerate() {
        game.players[0].library.push(card(
            114_000 + u32::try_from(index).expect("few cards"),
            *definition,
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
    game
}

/// What the pending search is offering.
fn offered(game: &Game) -> Vec<CardDefinitionId> {
    game.pending_decisions
        .first()
        .map(|pending| pending.observation.clone())
        .expect("the search asks")
        .options
        .iter()
        .filter_map(|option| match option.card {
            Some((_, ObjectCharacteristics::Card { definition, .. })) => Some(definition),
            _ => None,
        })
        .collect()
}

/// "It's not basic, so cards such as District Guide can't find it": a search
/// for a basic land card walks past two basic land types.
#[test]
fn a_basic_land_search_walks_past_it() {
    let game = searching_for_a_basic(&[cards::SACRED_FOUNDRY, cards::MOUNTAIN]);

    assert_eq!(
        offered(&game),
        vec![cards::MOUNTAIN],
        "the Mountain is a basic land card and the Foundry is not",
    );
}

/// "...but it does have the appropriate land types": a fetchland that reads
/// for a Mountain or a Plains finds it, printed supertype or not.
#[test]
fn a_fetchland_reading_its_types_finds_it() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].library.clear();
    game.players[0]
        .library
        .push(card(114_100, cards::SACRED_FOUNDRY, PlayerId::One));
    let heath = game
        .put_onto_battlefield(PlayerId::One, cards::ARID_MESA)
        .expect("cataloged");
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let fetch = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == heath))
        .expect("the fetchland is ready to crack");
    game.apply(PlayerId::One, fetch).expect("it activates");
    pass_priority_pair(&mut game);

    assert_eq!(
        offered(&game),
        vec![cards::SACRED_FOUNDRY],
        "a Mountain Plains answers a search for a Mountain or Plains card",
    );
}

/// The nonbasic half read from the other direction: a Wasteland may name it,
/// where it may not name the basic beside it.
#[test]
fn a_wasteland_answers_it_and_not_the_basic() {
    let mut game = ready_game();
    game.battlefield.clear();
    let foundry = game
        .put_onto_battlefield(PlayerId::One, cards::SACRED_FOUNDRY)
        .expect("cataloged");
    let mountain = game
        .put_onto_battlefield(PlayerId::One, cards::MOUNTAIN)
        .expect("cataloged");
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
    assert!(named(&game, foundry), "a nonbasic land, types and all");
    assert!(!named(&game, mountain), "and a Mountain is not one");
}

/// A fetchland says nothing about tapped, so the shock clause is live when
/// it lands: pay and it arrives ready, decline and it arrives tapped. The
/// mirror of the Wight fetching it tapped, where the payment buys nothing.
#[test]
fn a_fetchland_leaves_the_shock_choice_worth_making() {
    for pay in [true, false] {
        let mut game = ready_game();
        game.battlefield.clear();
        game.players[0].library.clear();
        game.players[0]
            .library
            .push(card(114_200, cards::SACRED_FOUNDRY, PlayerId::One));
        let mesa = game
            .put_onto_battlefield(PlayerId::One, cards::ARID_MESA)
            .expect("cataloged");
        drain_pending(&mut game);
        game.active_player = PlayerId::One;
        game.step = Step::PrecombatMain;
        game.priority = PlayerId::One;
        let life = game.players[0].life;

        let fetch = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(
                |action| matches!(action, Action::ActivateAbility { source, .. } if *source == mesa),
            )
            .expect("the fetchland is ready to crack");
        game.apply(PlayerId::One, fetch).expect("it activates");
        pass_priority_pair(&mut game);

        let search = game
            .observe(PlayerId::One)
            .decision
            .expect("the search offers what it found");
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: search.id,
                options: vec![search.options[0].id],
            },
        )
        .expect("taking it is legal");

        let shock = game
            .observe(PlayerId::One)
            .decision
            .expect("and then it asks about its two life");
        game.apply(
            PlayerId::One,
            Action::ChooseDecision {
                decision: shock.id,
                options: vec![u32::from(pay)],
            },
        )
        .expect("either answer is legal");
        drain_pending(&mut game);

        let foundry = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.definition == cards::SACRED_FOUNDRY)
            .expect("it arrived");
        assert_eq!(
            foundry.tapped, !pay,
            "nothing said tapped, so the payment decides it (paid: {pay})",
        );
        assert_eq!(
            game.players[0].life,
            life - 1 - if pay { 2 } else { 0 },
            "the Mesa's own life plus what the shock was answered with (paid: {pay})",
        );
    }
}
