//! Searching, scrying, and the cards that rearrange a library.
//!
//! What these have in common is not a colour or a cost but a question: which
//! half of a partition the card lets you name, and where each half lands.

use super::search_and_reveal::stack_library;
use super::*;

/// Resolves whatever is on the stack, answering nothing.
fn resolve(game: &mut Game) {
    for _ in 0..8 {
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
}

#[test]
fn entomb_puts_the_found_card_into_the_graveyard() {
    let mut game = ready_game();
    game.players[PlayerId::One.index()].library.clear();
    stack_library(
        &mut game,
        &[(50_400, cards::SERRA_ANGEL), (50_401, cards::GRIZZLY_BEARS)],
    );
    let entomb = card(50_402, cards::ENTOMB, PlayerId::One);
    let entomb_id = entomb.id;
    game.players[PlayerId::One.index()].hand.push(entomb);
    game.players[PlayerId::One.index()].mana_pool.black = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == entomb_id))
        .expect("Entomb is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game.observe(PlayerId::One).decision.expect("a search");
    let angel = decision
        .options
        .iter()
        .find(|option| option.card.is_some_and(|(_, id)| id == cards::SERRA_ANGEL))
        .expect("every card in the library is eligible")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![angel],
        },
    )
    .expect("the search is answered");

    assert!(
        game.players[PlayerId::One.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "the found card goes to the graveyard rather than the hand",
    );
}

#[test]
fn vampiric_tutor_leaves_the_card_on_top_and_costs_two_life() {
    let mut game = ready_game();
    game.players[PlayerId::One.index()].library.clear();
    stack_library(
        &mut game,
        &[
            (50_500, cards::GRIZZLY_BEARS),
            (50_501, cards::SERRA_ANGEL),
            (50_502, cards::LIGHTNING_BOLT),
        ],
    );
    let tutor = card(50_503, cards::VAMPIRIC_TUTOR, PlayerId::One);
    let tutor_id = tutor.id;
    game.players[PlayerId::One.index()].hand.push(tutor);
    game.players[PlayerId::One.index()].mana_pool.black = 1;
    let life = game.players[PlayerId::One.index()].life;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tutor_id))
        .expect("Vampiric Tutor is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game.observe(PlayerId::One).decision.expect("a search");
    let angel = decision
        .options
        .iter()
        .find(|option| option.card.is_some_and(|(_, id)| id == cards::SERRA_ANGEL))
        .expect("every card in the library is eligible")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![angel],
        },
    )
    .expect("the search is answered");
    resolve(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()]
            .library
            .last()
            .map(|card| card.definition),
        Some(cards::SERRA_ANGEL),
        "the found card survives the shuffle on top",
    );
    assert_eq!(game.players[PlayerId::One.index()].life, life - 2);
}

#[test]
fn mystical_tutor_offers_only_instants_and_sorceries() {
    let mut game = ready_game();
    game.players[PlayerId::One.index()].library.clear();
    stack_library(
        &mut game,
        &[
            (50_600, cards::GRIZZLY_BEARS),
            (50_601, cards::LIGHTNING_BOLT),
            (50_602, cards::ANCESTRAL_RECALL),
        ],
    );
    let tutor = card(50_603, cards::MYSTICAL_TUTOR, PlayerId::One);
    let tutor_id = tutor.id;
    game.players[PlayerId::One.index()].hand.push(tutor);
    game.players[PlayerId::One.index()].mana_pool.blue = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tutor_id))
        .expect("Mystical Tutor is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game.observe(PlayerId::One).decision.expect("a search");
    let mut offered = decision
        .options
        .iter()
        .filter_map(|option| option.card.map(|(_, definition)| definition))
        .collect::<Vec<_>>();
    offered.sort_unstable();
    let mut expected = vec![cards::LIGHTNING_BOLT, cards::ANCESTRAL_RECALL];
    expected.sort_unstable();
    assert_eq!(offered, expected, "the creature is not an eligible card");
}

/// Scry 2 with both cards kept is an arrangement, not just a filter: the two
/// go back on top in the order they were chosen, and the draw that follows
/// takes whichever was put there first.
#[test]
fn preordain_scries_two_and_lets_you_order_what_stays() {
    let mut game = ready_game();
    game.players[0].library.clear();
    stack_library(
        &mut game,
        &[
            (60_000, cards::LIGHTNING_BOLT),
            (60_001, cards::SERRA_ANGEL),
            (60_002, cards::SAVANNAH_LIONS),
        ],
    );
    let preordain = card(60_003, cards::PREORDAIN, PlayerId::One);
    let preordain_id = preordain.id;
    game.players[0].hand.push(preordain);
    game.players[0].mana_pool.blue = 1;
    game.apply(
        PlayerId::One,
        cast_action(preordain_id, Vec::new(), Vec::new(), 0),
    )
    .expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the scry looks at two");
    assert_eq!(
        decision
            .options
            .iter()
            .map(|option| option.label.clone())
            .collect::<Vec<_>>(),
        vec!["Lightning Bolt".to_owned(), "Serra Angel".to_owned()],
        "only the top two are looked at",
    );

    // Keep both, naming the Angel first so it ends up on top of the Bolt.
    let angel = decision
        .options
        .iter()
        .find(|option| option.label == "Serra Angel")
        .expect("offered")
        .id;
    let bolt = decision
        .options
        .iter()
        .find(|option| option.label == "Lightning Bolt")
        .expect("offered")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![angel, bolt],
        },
    )
    .expect("both may stay on top");

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "the draw takes the card put on top first",
    );
    assert_eq!(
        game.players[0]
            .library
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::SAVANNAH_LIONS, cards::LIGHTNING_BOLT],
        "and the other stays above what was never looked at",
    );
}

/// Sending both to the bottom is the other end of the same choice.
#[test]
fn preordain_can_bury_both_cards_it_looked_at() {
    let mut game = ready_game();
    game.players[0].library.clear();
    stack_library(
        &mut game,
        &[
            (60_100, cards::LIGHTNING_BOLT),
            (60_101, cards::SERRA_ANGEL),
            (60_102, cards::SAVANNAH_LIONS),
        ],
    );
    let preordain = card(60_103, cards::PREORDAIN, PlayerId::One);
    let preordain_id = preordain.id;
    game.players[0].hand.push(preordain);
    game.players[0].mana_pool.blue = 1;
    game.apply(
        PlayerId::One,
        cast_action(preordain_id, Vec::new(), Vec::new(), 0),
    )
    .expect("it is cast");
    pass_priority_pair(&mut game);

    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the scry looks at two");
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: Vec::new(),
        },
    )
    .expect("keeping nothing is allowed");

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::SAVANNAH_LIONS),
        "the draw reaches past both buried cards",
    );
    assert_eq!(
        game.players[0].library.len(),
        2,
        "and both are still in the library, at the bottom",
    );
}

/// The sacrifice is a cost, so it happens on casting and cannot be dodged by
/// answering the spell. What comes back is any land, which is the point: a
/// Forest becomes whatever the deck actually wanted.
#[test]
fn crop_rotation_trades_a_land_on_the_battlefield_for_any_land_in_the_library() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].library.clear();
    stack_library(
        &mut game,
        &[
            (63_000, cards::LIGHTNING_BOLT),
            (63_001, cards::GAEAS_CRADLE),
            (63_002, cards::TAIGA),
        ],
    );
    let forest = creature(63_003, cards::FOREST, PlayerId::One);
    let forest_id = forest.card.id;
    game.battlefield.push(forest);

    let rotation = card(63_004, cards::CROP_ROTATION, PlayerId::One);
    let rotation_id = rotation.id;
    game.players[0].hand.push(rotation);
    game.players[0].mana_pool.green = 1;

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, sacrifices, .. }
                if *card == rotation_id && sacrifices.contains(&forest_id))
        })
        .expect("the land on the battlefield pays for it");
    game.apply(PlayerId::One, action).expect("it is cast");
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != forest_id),
        "the sacrifice is a cost, paid on casting",
    );
    pass_priority_pair(&mut game);

    let decision = game.observe(PlayerId::One).decision.expect("a search");
    let mut offered = decision
        .options
        .iter()
        .filter_map(|option| option.card.map(|(_, definition)| definition))
        .collect::<Vec<_>>();
    offered.sort_unstable();
    let mut lands = vec![cards::GAEAS_CRADLE, cards::TAIGA];
    lands.sort_unstable();
    assert_eq!(offered, lands, "any land card, and only lands");

    let cradle = decision
        .options
        .iter()
        .find(|option| option.card.is_some_and(|(_, id)| id == cards::GAEAS_CRADLE))
        .expect("offered")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![cradle],
        },
    )
    .expect("the search is answered");

    let found = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GAEAS_CRADLE)
        .expect("it arrived on the battlefield");
    assert!(!found.tapped, "and it arrives untapped");
}

/// With no land to sacrifice there is no way to pay, so the spell is not
/// castable at all.
#[test]
fn crop_rotation_needs_a_land_to_give_up() {
    let mut game = ready_game();
    game.battlefield.clear();
    let rotation = card(63_100, cards::CROP_ROTATION, PlayerId::One);
    let rotation_id = rotation.id;
    game.players[0].hand.push(rotation);
    game.players[0].mana_pool.green = 1;

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == rotation_id)),
        "the mana alone does not pay for it",
    );
}

/// Two bounds at once: an instant or a sorcery, and cheap. Demonic Tutor sits
/// exactly on the line at two and is eligible; Wrath of God is the same card
/// type and one too expensive.
#[test]
fn spellseeker_finds_a_cheap_instant_or_sorcery_and_nothing_else() {
    let mut game = ready_game();
    game.players[0].library.clear();
    stack_library(
        &mut game,
        &[
            (72_000, cards::LIGHTNING_BOLT),
            (72_001, cards::DEMONIC_TUTOR),
            (72_002, cards::WRATH_OF_GOD),
            (72_003, cards::SERRA_ANGEL),
            (72_004, cards::FOREST),
        ],
    );

    game.put_onto_battlefield(PlayerId::One, cards::SPELLSEEKER)
        .expect("cataloged");

    // The search is optional; the last option accepts it.
    let offer = loop {
        if let Some(decision) = game.observe(PlayerId::One).decision {
            break decision;
        }
        let player = game.priority;
        assert!(
            game.apply(player, Action::PassPriority).is_ok(),
            "the enters trigger is waiting",
        );
    };
    let accept = offer.options.last().expect("accepting is offered").id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: offer.id,
            options: vec![accept],
        },
    )
    .expect("the search is accepted");

    let search = game
        .observe(PlayerId::One)
        .decision
        .expect("the library search follows");
    let mut offered = search
        .options
        .iter()
        .filter_map(|option| option.card.map(|(_, definition)| definition))
        .collect::<Vec<_>>();
    offered.sort_unstable();
    let mut expected = vec![cards::LIGHTNING_BOLT, cards::DEMONIC_TUTOR];
    expected.sort_unstable();
    assert_eq!(
        offered, expected,
        "a four-mana sorcery, a creature and a land are all out",
    );

    let tutor = search
        .options
        .iter()
        .find(|option| {
            option
                .card
                .is_some_and(|(_, id)| id == cards::DEMONIC_TUTOR)
        })
        .expect("offered")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: search.id,
            options: vec![tutor],
        },
    )
    .expect("the search is answered");

    assert!(
        game.players[0]
            .hand
            .iter()
            .any(|card| card.definition == cards::DEMONIC_TUTOR),
        "the found card goes to hand",
    );
}
