//! Dark Depths: ten counters, and a 20/20 for whoever gets them off.

use super::*;

/// Dark Depths on the battlefield since last turn, with `mana` colorless in
/// Player One's pool.
fn staged(mana: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let depths = game
        .put_onto_battlefield(PlayerId::One, cards::DARK_DEPTHS)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, mana);
    (game, depths)
}

fn settle(game: &mut Game) {
    for _ in 0..32 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .take(decision.minimum.max(1))
                .map(|option| option.id)
                .collect();
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

fn ice(game: &Game, depths: GameObjectId) -> u16 {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == depths)
        .map_or(0, |permanent| permanent.counters(CounterKind::named("ice")))
}

/// Every removal activation Dark Depths is offering right now.
fn removals(game: &Game, depths: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == depths),
        )
        .collect()
}

fn remove_one(game: &mut Game, depths: GameObjectId) {
    let action = removals(game, depths)
        .into_iter()
        .next()
        .expect("three mana takes a counter off");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(game);
}

fn marit_lage(game: &Game) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Token)
}

/// It arrives counting down from ten.
#[test]
fn it_enters_with_ten_ice_counters() {
    let (game, depths) = staged(0);

    assert_eq!(ice(&game, depths), 10);
}

/// Three mana takes one off, and two mana takes none.
#[test]
fn three_mana_removes_a_counter() {
    let (mut game, depths) = staged(2);

    assert!(
        removals(&game, depths).is_empty(),
        "two mana does not pay for it",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    remove_one(&mut game, depths);

    assert_eq!(ice(&game, depths), 9);
}

/// Nine removals leave a counter, and nothing has triggered.
#[test]
fn it_stays_while_a_counter_remains() {
    let (mut game, depths) = staged(27);
    for _ in 0..9 {
        remove_one(&mut game, depths);
    }

    assert_eq!(ice(&game, depths), 1);
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == depths),
        "still there",
    );
    assert!(marit_lage(&game).is_none(), "and no token yet");
}

/// The tenth removal empties it: the state trigger sacrifices the land and
/// pays out the Avatar.
#[test]
fn emptying_it_trades_the_land_for_marit_lage() {
    let (mut game, depths) = staged(30);
    for _ in 0..10 {
        remove_one(&mut game, depths);
    }

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == depths),
        "the land sacrificed itself",
    );
    let token = marit_lage(&game).expect("Marit Lage is here");
    assert_eq!(game.power(token), Some(20));
    assert_eq!(game.toughness(token), Some(20));
    assert!(game.has_flying(token), "with flying");
    assert!(
        game.permanent_has_executable_keyword(token, KeywordAbility::Indestructible),
        "and indestructible",
    );
    assert_eq!(token.controller, PlayerId::One);
    assert!(
        game.effective_rules(token)
            .is_some_and(|rules| rules.has_supertype(CardSupertype::Legendary)),
        "and legendary, so a second one would not stay",
    );
    assert_eq!(
        game.object_card_name(token.card.id).as_deref(),
        Some("Marit Lage"),
    );
}

/// "If you do": a Dark Depths that has already left the battlefield is not
/// sacrificed, and pays nothing out.
#[test]
fn a_depths_that_is_already_gone_makes_no_token() {
    let (mut game, depths) = staged(30);
    for _ in 0..9 {
        remove_one(&mut game, depths);
    }

    // The tenth activation empties it, and the state trigger goes on the
    // stack. Answer it by taking the land itself before it resolves.
    let action = removals(&game, depths)
        .into_iter()
        .next()
        .expect("the last counter can come off");
    game.apply(PlayerId::One, action).expect("it activates");
    settle_until_trigger_waits(&mut game, depths);
    assert_eq!(
        game.stack.len(),
        1,
        "the state trigger is waiting on the stack",
    );
    assert_eq!(ice(&game, depths), 0, "with the last counter gone");
    game.move_permanents_to_graveyard(&[depths]);
    settle(&mut game);

    assert!(marit_lage(&game).is_none(), "nothing was sacrificed");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::DARK_DEPTHS),
        "the land went where it was sent",
    );
}

/// Lets the removal resolve and stops with the state trigger itself waiting
/// on the stack: the counters are gone and one object is left there.
fn settle_until_trigger_waits(game: &mut Game, depths: GameObjectId) {
    for _ in 0..32 {
        if ice(game, depths) == 0 && game.stack.len() == 1 && game.pending_triggers.is_empty() {
            return;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            return;
        }
    }
}

/// "Dark Depths doesn't have a mana ability. It doesn't tap for colorless
/// mana." A land that makes nothing is the other half of what the ten
/// counters cost.
#[test]
fn it_makes_no_mana_at_all() {
    let (game, depths) = staged(0);

    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(
                action,
                Action::ActivateManaAbility { source, .. } if *source == depths
            )),
        "the only ability it offers costs mana rather than making it",
    );
}

/// "It won't trigger again while the ability is on the stack, but if the
/// ability is countered and Dark Depths is still on the battlefield with no
/// ice counters on it, it will trigger again immediately." Stifling the
/// trigger buys nothing unless the land goes with it.
#[test]
fn a_stifled_state_trigger_fires_again() {
    let (mut game, depths) = staged(30);
    let stifle = card(93_000, cards::STIFLE, PlayerId::Two);
    let stifle_id = stifle.id;
    game.players[1].hand.push(stifle);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 1);
    for _ in 0..9 {
        remove_one(&mut game, depths);
    }

    let action = removals(&game, depths)
        .into_iter()
        .next()
        .expect("the last counter can come off");
    game.apply(PlayerId::One, action).expect("it activates");
    settle_until_trigger_waits(&mut game, depths);
    let trigger = game
        .stack
        .last()
        .expect("the state trigger is waiting on the stack")
        .id;

    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(stifle_id, vec![Target::Spell(trigger)], Vec::new(), 0),
    )
    .expect("a triggered ability is what Stifle names");
    settle(&mut game);

    assert!(
        game.players[1]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::STIFLE),
        "the Stifle resolved and countered that trigger",
    );
    assert!(
        marit_lage(&game).is_some(),
        "and the condition was still true, so it triggered again and paid out",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == depths),
        "the land was sacrificed the second time around",
    );
}
