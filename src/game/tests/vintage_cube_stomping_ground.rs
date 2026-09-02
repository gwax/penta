//! Stomping Ground: the Gruul shockland, and the Mountain Forest it shares a
//! type line with.
//!
//! The shock clause itself -- both answers, every member of the cycle, the
//! low-life edges, and one fetched tapped keeping its two life for nothing
//! -- is pinned in `vintage_cube_lands` and `vintage_cube_sacred_foundry`.
//! What is here is the pair a Wooded Foothills sees: two lands that are both
//! Mountain Forests and arrive on entirely different terms.

use super::*;

/// Player One with a Wooded Foothills out and both Mountain Forests in the
/// library, cracked, with the search waiting.
fn cracked() -> Game {
    cracked_over(&[cards::STOMPING_GROUND, cards::COMMERCIAL_DISTRICT])
}

/// The same, over whichever library `lands` names.
fn cracked_over(lands: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    let foothills = game
        .put_onto_battlefield(PlayerId::One, cards::WOODED_FOOTHILLS)
        .expect("cataloged");
    game.players[0].library.clear();
    for (index, definition) in lands.iter().enumerate() {
        game.players[0].library.push(card(
            122_000 + u32::try_from(index).expect("a short library"),
            *definition,
            PlayerId::One,
        ));
    }
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let crack = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { source, .. } if *source == foothills)
        })
        .expect("the fetch is offered");
    game.apply(PlayerId::One, crack).expect("it activates");
    pass_priority_pair(&mut game);
    game
}

/// Takes `wanted` out of the pending search.
fn take(game: &mut Game, wanted: CardDefinitionId) {
    let search = game
        .observe(PlayerId::One)
        .decision
        .expect("the search offers what it found");
    let option = search
        .options
        .iter()
        .find(|option| {
            matches!(
                option.card,
                Some((_, ObjectCharacteristics::Card { definition, .. })) if definition == wanted
            )
        })
        .unwrap_or_else(|| panic!("{wanted:?} is on the menu"))
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: vec![option],
        },
    )
    .expect("taking it is legal");
}

/// Both are Mountain Forests, so a Foothills reading those two types offers
/// both of them.
#[test]
fn a_foothills_sees_both_mountain_forests() {
    let game = cracked();

    let offered: Vec<_> = game
        .observe(PlayerId::One)
        .decision
        .expect("the search asks")
        .options
        .iter()
        .filter_map(|option| match option.card {
            Some((_, ObjectCharacteristics::Card { definition, .. })) => Some(definition),
            _ => None,
        })
        .collect();
    assert!(offered.contains(&cards::STOMPING_GROUND));
    assert!(offered.contains(&cards::COMMERCIAL_DISTRICT));
}

/// The same fetch, the same types, and two different arrivals: the shockland
/// asks for two life and comes in ready when it is paid, while the surveil
/// land says tapped and asks nothing.
#[test]
fn the_two_of_them_arrive_on_different_terms() {
    let mut game = cracked();
    take(&mut game, cards::STOMPING_GROUND);
    let shock = game
        .observe(PlayerId::One)
        .decision
        .expect("the shockland asks about its two life");
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: shock.id,
            options: vec![1],
        },
    )
    .expect("paying is offered");
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.definition == cards::STOMPING_GROUND)
            .expect("it arrived")
            .tapped,
        "two life buys it untapped",
    );
    assert_eq!(
        game.players[0].life, 17,
        "the Foothills' one and the shockland's two",
    );

    let mut game = cracked();
    take(&mut game, cards::COMMERCIAL_DISTRICT);
    drain_pending(&mut game);

    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.definition == cards::COMMERCIAL_DISTRICT)
            .expect("it arrived")
            .tapped,
        "the surveil land has no such offer to make",
    );
    assert_eq!(
        game.players[0].life, 19,
        "and costs nothing past the Foothills' own life",
    );
}

/// The third of the three, and the one the other two are priced against: a
/// Taiga is a Mountain Forest too, and the only one that arrives untapped,
/// free, and without asking anything.
#[test]
fn the_dual_is_the_third_of_them_and_costs_nothing() {
    let mut game = cracked_over(&[
        cards::STOMPING_GROUND,
        cards::COMMERCIAL_DISTRICT,
        cards::TAIGA,
    ]);
    let mut offered: Vec<_> = game
        .observe(PlayerId::One)
        .decision
        .expect("the search offers what it found")
        .options
        .iter()
        .filter_map(|option| {
            option
                .card
                .and_then(|(_, characteristics)| characteristics.card_definition())
        })
        .collect();
    offered.sort_unstable();
    let mut wanted = vec![
        cards::STOMPING_GROUND,
        cards::COMMERCIAL_DISTRICT,
        cards::TAIGA,
    ];
    wanted.sort_unstable();
    assert_eq!(offered, wanted, "all three read as Mountain Forests");

    let life = game.players[0].life;
    take(&mut game, cards::TAIGA);
    drain_pending(&mut game);

    assert!(
        game.pending_decisions.is_empty(),
        "the dual asks nothing on the way in",
    );
    let taiga = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::TAIGA)
        .expect("it arrived");
    assert!(!taiga.tapped, "and arrives ready to use");
    assert_eq!(
        game.players[0].life, life,
        "for nothing beyond the fetch's own life",
    );
}
