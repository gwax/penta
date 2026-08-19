//! The cards the Premodern Angry Hermit list needed.

use super::*;

/// The Druid digs to the first basic land and buries what it passed. A
/// library holding none is emptied instead, which is the deck's whole plan.
#[test]
fn hermit_druid_takes_the_basic_and_buries_the_rest() {
    let dug = |basics: bool| {
        let mut game = ready_game();
        let druid = creature(10_000, cards::HERMIT_DRUID, PlayerId::One);
        game.battlefield.push(druid);
        game.players[PlayerId::One.index()].library.clear();
        // Bottom to top: two spells, then a basic beneath nothing else when
        // the library has one at all.
        if basics {
            game.players[PlayerId::One.index()]
                .library
                .push(card(10_010, cards::SWAMP, PlayerId::One));
        }
        for index in 0..2 {
            game.players[PlayerId::One.index()].library.push(card(
                10_020 + index,
                cards::LIGHTNING_BOLT,
                PlayerId::One,
            ));
        }
        game.players[PlayerId::One.index()].mana_pool.green = 1;
        game.priority = PlayerId::One;

        let druid_id = game.battlefield[0].card.id;
        let activate = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| {
                matches!(action, Action::ActivateAbility { source, .. } if *source == druid_id)
            })
            .expect("one green and an untapped Druid");
        game.apply(PlayerId::One, activate).unwrap();
        drain_pending(&mut game);
        let player = &game.players[PlayerId::One.index()];
        (player.hand.len(), player.graveyard.len(), player.library.len())
    };

    let (hand, graveyard, library) = dug(true);
    assert_eq!(hand, 1, "the Swamp it found");
    assert_eq!(graveyard, 2, "the two Bolts above it");
    assert_eq!(library, 0, "the dig went all the way down");

    let (hand, graveyard, library) = dug(false);
    assert_eq!(hand, 0, "nothing found, so nothing taken");
    assert_eq!(graveyard, 2, "the library empties into the graveyard");
    assert_eq!(library, 0, "and nothing is left");
}

/// Stifle answers an ability and cannot be pointed at a spell.
#[test]
fn stifle_counters_an_ability_but_not_a_spell() {
    let mut game = ready_game();
    let stifle = card(10_000, cards::STIFLE, PlayerId::Two);
    let stifle_id = stifle.id;
    game.players[PlayerId::Two.index()].hand.push(stifle);
    game.players[PlayerId::Two.index()].mana_pool.blue = 1;

    // A spell on the stack alone gives Stifle nothing to name.
    let bolt = card(10_001, cards::LIGHTNING_BOLT, PlayerId::One);
    let bolt_id = bolt.id;
    game.players[PlayerId::One.index()].hand.push(bolt);
    game.players[PlayerId::One.index()].mana_pool.red = 1;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(bolt_id, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("the Bolt is cast");
    assert!(
        !game
            .legal_actions(PlayerId::Two)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == stifle_id)),
        "a spell is not an activated or triggered ability",
    );
}

/// And Stifle answers the ability itself: the Druid taps, and the dig never
/// happens.
#[test]
fn stifle_counters_the_druids_dig() {
    let mut game = ready_game();
    game.battlefield
        .push(creature(10_000, cards::HERMIT_DRUID, PlayerId::One));
    game.players[PlayerId::One.index()].library.clear();
    for index in 0..3 {
        game.players[PlayerId::One.index()].library.push(card(
            10_020 + index,
            cards::LIGHTNING_BOLT,
            PlayerId::One,
        ));
    }
    game.players[PlayerId::One.index()].mana_pool.green = 1;

    let stifle = card(10_001, cards::STIFLE, PlayerId::Two);
    let stifle_id = stifle.id;
    game.players[PlayerId::Two.index()].hand.push(stifle);
    game.players[PlayerId::Two.index()].mana_pool.blue = 1;

    game.priority = PlayerId::One;
    let druid_id = game.battlefield[0].card.id;
    let activate = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == druid_id))
        .expect("one green and an untapped Druid");
    game.apply(PlayerId::One, activate).unwrap();

    let ability = game.stack.last().expect("the dig is on the stack").id;
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(stifle_id, vec![Target::Spell(ability)], Vec::new(), 0),
    )
    .expect("the ability can be named");
    drain_pending(&mut game);

    let player = &game.players[PlayerId::One.index()];
    assert_eq!(player.library.len(), 3, "the dig never happened");
    assert_eq!(player.hand.len(), 0, "and nothing was found");
}

/// Shallow Grave takes the newest creature card, not the oldest, and the
/// creature it returns leaves again at the end of the turn.
#[test]
fn shallow_grave_returns_the_top_creature_and_exiles_it_at_end_of_turn() {
    let mut game = ready_game();
    // Oldest to newest: a Bolt between two creatures, so "the top creature
    // card" is the second creature rather than the last card.
    game.players[PlayerId::One.index()]
        .graveyard
        .push(card(10_010, cards::GOBLIN_LACKEY, PlayerId::One));
    game.players[PlayerId::One.index()]
        .graveyard
        .push(card(10_011, cards::PSYCHATOG, PlayerId::One));
    game.players[PlayerId::One.index()]
        .graveyard
        .push(card(10_012, cards::LIGHTNING_BOLT, PlayerId::One));

    let grave = card(10_000, cards::SHALLOW_GRAVE, PlayerId::One);
    let grave_id = grave.id;
    game.players[PlayerId::One.index()].hand.push(grave);
    let pool = &mut game.players[PlayerId::One.index()].mana_pool;
    pool.black = 1;
    pool.colorless = 1;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(grave_id, Vec::new(), Vec::new(), 0),
    )
    .expect("two mana casts it");
    drain_pending(&mut game);

    let returned = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::PSYCHATOG)
        .expect("the newest creature card came back");
    assert!(
        game.permanent_has_executable_keyword(returned, KeywordAbility::Haste),
        "it can attack the turn it arrives",
    );
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.definition != cards::GOBLIN_LACKEY),
        "the older creature stayed in the graveyard",
    );

    // Reaching the end step is what fires the delayed clause; setting the
    // field would skip the step beginning.
    for _ in 0..8 {
        if game.step == Step::End {
            break;
        }
        game.advance_step();
    }
    assert_eq!(game.step, Step::End, "the turn reached its end step");
    // Driving steps directly skips the procedure that puts captured triggers
    // on the stack, so run it before letting the stack resolve.
    game.finish_rules_procedure();
    pass_until_decision(&mut game);
    drain_pending(&mut game);
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.definition != cards::PSYCHATOG),
        "and it is exiled at the beginning of the end step",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].exile.len(),
        1,
        "exiled rather than buried",
    );
}

/// Reflecting Pool reads the lands you control, not the opponent's, and a
/// type is a type: colourless counts where Fellwar Stone's colours do not.
#[test]
fn reflecting_pool_borrows_from_your_own_lands() {
    let mut game = ready_game();
    game.battlefield
        .push(creature(10_000, cards::REFLECTING_POOL, PlayerId::One));
    game.battlefield
        .push(creature(10_001, cards::FOREST, PlayerId::One));
    // An opponent's Island is not one of yours.
    game.battlefield
        .push(creature(10_002, cards::ISLAND, PlayerId::Two));
    game.priority = PlayerId::One;

    let pool_id = game.battlefield[0].card.id;
    let colors: Vec<ManaColor> = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility { source, color, .. } if source == pool_id => Some(color),
            _ => None,
        })
        .collect();
    assert_eq!(
        colors,
        vec![ManaColor::Green],
        "the Forest lends green and the opponent's Island lends nothing",
    );

    // Ancient Tomb makes colourless, which "any type" accepts.
    game.battlefield
        .push(creature(10_003, cards::ANCIENT_TOMB, PlayerId::One));
    let mut colors: Vec<ManaColor> = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::ActivateManaAbility { source, color, .. } if source == pool_id => Some(color),
            _ => None,
        })
        .collect();
    colors.sort_unstable();
    colors.dedup();
    assert!(
        colors.contains(&ManaColor::Colorless),
        "a type, not a colour: {colors:?}",
    );
}
