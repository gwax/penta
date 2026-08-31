//! Oath of Druids: two mana that puts something enormous onto the
//! battlefield for free, for the deck that lets the other player go first.

use super::*;

/// The Oath under Player One, with `library` stacked so the last entry is on
/// top of Player One's library.
fn staged(library: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    game.players[0].graveyard.clear();
    for definition in library {
        let card = game
            .build_zone(PlayerId::One, &[*definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[0].library.push(card);
    }
    game.put_onto_battlefield(PlayerId::One, cards::OATH_OF_DRUIDS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.turns_started = [5, 5];
    game.turn = 9;
    game
}

/// Runs `player`'s upkeep, taking every offer.
fn upkeep_of(game: &mut Game, player: PlayerId, accept: bool) {
    game.active_player = player;
    game.step = Step::Upkeep;
    game.priority = player;
    game.handle_upkeep_triggers();
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .filter(|option| {
                    if accept {
                        !option.label.contains("Decline")
                    } else {
                        option.label.contains("Decline")
                    }
                })
                .map(|option| option.id)
                .take(1)
                .collect::<Vec<_>>();
            let options = if options.len() < decision.minimum {
                decision
                    .options
                    .iter()
                    .map(|option| option.id)
                    .take(decision.minimum)
                    .collect()
            } else {
                options
            };
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the offered choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == definition)
}

/// Behind on creatures, the upkeep player digs to the first creature card:
/// it arrives, and everything above it is buried.
#[test]
fn being_behind_puts_a_creature_onto_the_battlefield() {
    let mut game = staged(&[cards::SERRA_ANGEL, cards::LIGHTNING_BOLT, cards::MOUNTAIN]);
    game.battlefield
        .push(creature(200_000, cards::GRIZZLY_BEARS, PlayerId::Two));

    upkeep_of(&mut game, PlayerId::One, true);

    assert!(
        on_battlefield(&game, cards::SERRA_ANGEL),
        "the Angel arrived"
    );
    assert_eq!(
        game.players[0]
            .graveyard
            .iter()
            .map(|card| card.definition)
            .collect::<Vec<_>>(),
        vec![cards::MOUNTAIN, cards::LIGHTNING_BOLT],
        "and everything above it is buried",
    );
    assert!(game.players[0].library.is_empty());
}

/// Level on creatures, nothing happens: the condition is a comparison, not a
/// count of your own.
#[test]
fn being_level_does_nothing() {
    let mut game = staged(&[cards::SERRA_ANGEL, cards::MOUNTAIN]);
    game.battlefield
        .push(creature(200_100, cards::GRIZZLY_BEARS, PlayerId::Two));
    game.battlefield
        .push(creature(200_101, cards::GRIZZLY_BEARS, PlayerId::One));

    upkeep_of(&mut game, PlayerId::One, true);

    assert!(!on_battlefield(&game, cards::SERRA_ANGEL));
    assert_eq!(game.players[0].library.len(), 2, "nothing was revealed");
}

/// It is each player's upkeep, not the controller's: the other player digs
/// out of their own library when they are the one behind.
#[test]
fn the_other_players_upkeep_asks_about_them() {
    let mut game = staged(&[cards::MOUNTAIN]);
    game.players[1].library.clear();
    for definition in [cards::SERRA_ANGEL, cards::LIGHTNING_BOLT] {
        let card = game
            .build_zone(PlayerId::Two, &[definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        game.players[1].library.push(card);
    }
    game.battlefield
        .push(creature(200_200, cards::GRIZZLY_BEARS, PlayerId::One));

    upkeep_of(&mut game, PlayerId::Two, true);

    let angel = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SERRA_ANGEL)
        .expect("their Angel arrived");
    assert_eq!(angel.controller, PlayerId::Two, "under their control");
    assert_eq!(game.players[1].graveyard.len(), 1);
    assert_eq!(
        game.players[0].library.len(),
        1,
        "and the Oath's controller revealed nothing",
    );
}

/// The dig is optional.
#[test]
fn declining_leaves_the_library_alone() {
    let mut game = staged(&[cards::SERRA_ANGEL, cards::MOUNTAIN]);
    game.battlefield
        .push(creature(200_300, cards::GRIZZLY_BEARS, PlayerId::Two));

    upkeep_of(&mut game, PlayerId::One, false);

    assert!(!on_battlefield(&game, cards::SERRA_ANGEL));
    assert_eq!(game.players[0].library.len(), 2);
}

/// The first decision the upkeep raises, without answering it.
fn first_decision(game: &mut Game, player: PlayerId) -> DecisionObservation {
    game.active_player = player;
    game.step = Step::Upkeep;
    game.priority = player;
    game.handle_upkeep_triggers();
    for _ in 0..8 {
        if let Some(pending) = game.pending_decisions.first() {
            return pending.observation.clone();
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    panic!("the upkeep raised nothing to answer");
}

/// "That player chooses target player": the choice belongs to whoever's
/// upkeep it is, not to whoever controls the Oath.
#[test]
fn the_upkeep_player_chooses_the_target() {
    let mut game = staged(&[cards::SERRA_ANGEL]);
    game.battlefield
        .push(creature(200_300, cards::GRIZZLY_BEARS, PlayerId::One));

    let decision = first_decision(&mut game, PlayerId::Two);

    assert_eq!(
        decision.player,
        PlayerId::Two,
        "their upkeep, so their choice, even though the Oath is not theirs",
    );
    assert_eq!(
        decision.options.len(),
        1,
        "one opponent, and they are ahead on creatures",
    );
    assert_eq!(
        decision.options[0].label, "your opponent",
        "and named from the seat being asked",
    );
}

/// Nobody with more creatures is nobody to target, so the ability leaves the
/// stack without asking anything else.
#[test]
fn a_level_board_offers_no_target() {
    let mut game = staged(&[cards::SERRA_ANGEL]);
    game.battlefield
        .push(creature(200_400, cards::GRIZZLY_BEARS, PlayerId::One));
    game.battlefield
        .push(creature(200_401, cards::GRIZZLY_BEARS, PlayerId::Two));

    game.active_player = PlayerId::One;
    game.step = Step::Upkeep;
    game.priority = PlayerId::One;
    game.handle_upkeep_triggers();

    assert!(
        game.pending_decisions.is_empty(),
        "no legal target is nothing to ask about",
    );
    assert!(game.stack.is_empty(), "and nothing waiting to resolve");
}

/// The comparison is against the player choosing rather than against the
/// Oath's controller: player one being ahead means player one's own upkeep
/// finds nobody to name, whoever owns the enchantment.
#[test]
fn the_comparison_is_made_from_the_choosing_seat() {
    let mut game = staged(&[cards::SERRA_ANGEL]);
    game.battlefield
        .push(creature(200_500, cards::GRIZZLY_BEARS, PlayerId::One));
    game.battlefield
        .push(creature(200_501, cards::GRIZZLY_BEARS, PlayerId::One));
    game.battlefield
        .push(creature(200_502, cards::GRIZZLY_BEARS, PlayerId::Two));

    // Player one is ahead two creatures to one, so their own upkeep names
    // nobody and reveals nothing.
    upkeep_of(&mut game, PlayerId::One, true);
    assert!(!on_battlefield(&game, cards::SERRA_ANGEL));
    assert_eq!(game.players[0].library.len(), 1, "nothing was revealed");

    // Player two is behind, so theirs finds player one.
    let decision = first_decision(&mut game, PlayerId::Two);
    assert_eq!(decision.player, PlayerId::Two);
    assert_eq!(decision.options.len(), 1);
}

/// "The ability doesn't resolve if it's no longer true at that time." The
/// comparison is part of the targeting requirement, so a target whose lead
/// is gone by resolution is an illegal target and the whole ability is
/// removed from the stack.
#[test]
fn a_lead_lost_in_response_takes_the_ability_with_it() {
    let mut game = staged(&[cards::SERRA_ANGEL]);
    let theirs = creature(200_600, cards::GRIZZLY_BEARS, PlayerId::Two);
    let theirs_id = theirs.card.id;
    game.battlefield.push(theirs);

    // Player One is behind one creature to none, so their upkeep names
    // Player Two.
    let decision = first_decision(&mut game, PlayerId::One);
    assert_eq!(decision.player, PlayerId::One);
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![decision.options[0].id],
        },
    )
    .expect("naming the only legal target is legal");

    // The lead goes away before the ability resolves.
    game.move_permanents_to_graveyard(&[theirs_id]);
    drain_pending(&mut game);

    assert!(
        !on_battlefield(&game, cards::SERRA_ANGEL),
        "the ability did nothing at all",
    );
    assert_eq!(
        game.players[0].library.len(),
        1,
        "and nothing was revealed on the way",
    );
}

/// "The ability can only target an opponent of the current player." Being
/// behind your own board is not a thing that can happen, and the check is
/// that the offer names the other seat rather than your own.
#[test]
fn the_only_target_it_ever_offers_is_the_opponent() {
    let mut game = staged(&[cards::SERRA_ANGEL]);
    game.battlefield
        .push(creature(200_700, cards::GRIZZLY_BEARS, PlayerId::Two));
    game.battlefield
        .push(creature(200_701, cards::GRIZZLY_BEARS, PlayerId::Two));

    let decision = first_decision(&mut game, PlayerId::One);

    assert_eq!(
        decision
            .options
            .iter()
            .map(|option| option.label.as_str())
            .collect::<Vec<_>>(),
        vec!["your opponent"],
        "their seat and no other, even with two creatures to name it for",
    );
}

/// "Until they reveal a creature card": a library with none in it is
/// revealed to the bottom, and everything revealed is buried. Nothing
/// arrives, and milling out is not itself a loss.
#[test]
fn a_library_with_no_creature_is_buried_whole() {
    let mut game = staged(&[cards::MOUNTAIN, cards::LIGHTNING_BOLT, cards::PONDER]);
    game.battlefield
        .push(creature(200_500, cards::GRIZZLY_BEARS, PlayerId::Two));

    upkeep_of(&mut game, PlayerId::One, true);
    game.check_state_based_actions();

    assert!(
        game.players[0].library.is_empty(),
        "the whole library was revealed looking for a creature",
    );
    let mut buried = game.players[0]
        .graveyard
        .iter()
        .map(|card| card.definition)
        .collect::<Vec<_>>();
    buried.sort_unstable();
    let mut expected = vec![cards::MOUNTAIN, cards::LIGHTNING_BOLT, cards::PONDER];
    expected.sort_unstable();
    assert_eq!(buried, expected, "and all of it is in the graveyard");
    assert_eq!(
        game.result, None,
        "an empty library is not a loss until it is drawn from",
    );
}

/// What the Oath does is put a creature onto the battlefield, which is not
/// casting it: a Containment Priest answers the Angel it finds.
#[test]
fn what_it_finds_was_never_cast() {
    let mut game = staged(&[cards::SERRA_ANGEL, cards::MOUNTAIN]);
    // Two of theirs, because the Priest is a creature of yours and the Oath
    // only fires for the player who is behind.
    for instance in [200_600, 200_601] {
        game.battlefield
            .push(creature(instance, cards::GRIZZLY_BEARS, PlayerId::Two));
    }
    game.put_onto_battlefield(PlayerId::One, cards::CONTAINMENT_PRIEST)
        .expect("cataloged");
    drain_pending(&mut game);

    upkeep_of(&mut game, PlayerId::One, true);
    game.check_state_based_actions();

    assert!(
        !on_battlefield(&game, cards::SERRA_ANGEL),
        "the Priest answers a creature that arrives without being cast",
    );
    assert!(
        game.players[0]
            .exile
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "and it is exiled rather than buried with the rest",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::MOUNTAIN),
        "the Mountain above it is buried either way",
    );
}
