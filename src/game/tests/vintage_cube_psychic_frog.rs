//! Psychic Frog: two mana that eats a hand, eats a graveyard, and draws a
//! card every time it connects.

use super::*;

/// Player One with a Frog that has been out since last turn, `hand` in hand
/// and `graveyard` behind it.
fn staged(hand: &[CardDefinitionId], graveyard: &[CardDefinitionId]) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    for (zone, definitions) in [(ZoneKind::Hand, hand), (ZoneKind::Graveyard, graveyard)] {
        for definition in definitions {
            let card = game
                .build_zone(PlayerId::One, &[*definition])
                .expect("cataloged")
                .into_iter()
                .next()
                .expect("one card");
            match zone {
                ZoneKind::Hand => game.players[0].hand.push(card),
                _ => game.players[0].graveyard.push(card),
            }
        }
    }
    let frog = game
        .put_onto_battlefield(PlayerId::One, cards::PSYCHIC_FROG)
        .expect("cataloged");
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [2, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, frog)
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if !game.pending_decisions.is_empty() {
            return;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

/// Every activation of the Frog on offer right now.
fn activations(game: &Game, frog: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == frog),
        )
        .collect()
}

/// The activation whose ability is the one at `index` on the Frog.
fn activation(game: &Game, frog: GameObjectId, index: u8) -> Option<Action> {
    activations(game, frog).into_iter().find(|action| {
        matches!(
            action,
            Action::ActivateAbility {
                ability: AbilityOrigin::Printed { ability, .. },
                ..
            } if *ability == AbilityId(index)
        )
    })
}

fn the_frog(game: &Game) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::PSYCHIC_FROG)
        .expect("it is on the battlefield")
}

/// Sends the Frog at `defender` and lets combat damage happen.
fn connect(game: &mut Game, frog: GameObjectId, defender: AttackDefender) {
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == frog)
    {
        permanent.attacking = true;
        permanent.attack_defender = Some(defender);
    }
    game.step = Step::DeclareBlockers;
    game.advance_step();
    pass_priority_pair(game);
    settle(game);
}

/// Discarding a card grows it, and the card actually leaves the hand.
#[test]
fn a_discard_puts_a_counter_on_it() {
    let (mut game, frog) = staged(&[cards::MOUNTAIN], &[]);

    let action = activation(&game, frog, 1).expect("a card in hand pays for it");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    assert_eq!(
        the_frog(&game).counters(CounterKind::PlusOnePlusOne),
        1,
        "one counter",
    );
    assert_eq!(game.power(the_frog(&game)), Some(2), "a 2/3 now");
    assert!(game.players[0].hand.is_empty(), "the card was discarded");
    assert_eq!(
        game.players[0].graveyard.len(),
        1,
        "into its owner's graveyard",
    );
}

/// No mana and no tap: with a hand to spend it grows as often as it likes.
#[test]
fn an_empty_hand_cannot_pay_for_the_counter() {
    let (game, frog) = staged(&[], &[]);

    assert!(
        activation(&game, frog, 1).is_none(),
        "there is nothing to discard",
    );
}

/// Three cards out of the graveyard buy flying for the turn.
#[test]
fn three_graveyard_cards_buy_flying() {
    let (mut game, frog) = staged(&[], &[cards::MOUNTAIN, cards::ISLAND, cards::SWAMP]);
    assert!(
        !game.permanent_has_executable_keyword(the_frog(&game), KeywordAbility::Flying),
        "a Frog does not fly on its own",
    );

    let action = activation(&game, frog, 2).expect("three cards pay for it");
    game.apply(PlayerId::One, action).expect("it activates");
    settle(&mut game);

    assert!(
        game.permanent_has_executable_keyword(the_frog(&game), KeywordAbility::Flying),
        "and now it does",
    );
    assert!(
        game.players[0].graveyard.is_empty(),
        "the three left the graveyard",
    );
    assert_eq!(
        game.players[0].exile.len(),
        3,
        "exiled rather than shuffled away",
    );
}

/// Two is not three: the cost is paid in full or not at all.
#[test]
fn two_graveyard_cards_are_not_enough() {
    let (game, frog) = staged(&[], &[cards::MOUNTAIN, cards::ISLAND]);

    assert!(
        activation(&game, frog, 2).is_none(),
        "a partial payment buys nothing",
    );
}

/// Connecting with a player draws a card.
#[test]
fn combat_damage_to_a_player_draws_a_card() {
    let (mut game, frog) = staged(&[], &[]);
    let before = game.players[0].hand.len();

    connect(&mut game, frog, AttackDefender::Player(PlayerId::Two));

    assert_eq!(game.players[1].life, 19, "the Frog got there");
    assert_eq!(
        game.players[0].hand.len(),
        before + 1,
        "and drew a card for it",
    );
}

/// "Or planeswalker": hitting one draws just the same, which is what makes
/// the clause worth writing as one trigger rather than two.
#[test]
fn combat_damage_to_a_planeswalker_draws_a_card() {
    let (mut game, frog) = staged(&[], &[]);
    let mut walker = creature(10_500, cards::DOMRI_RADE, PlayerId::Two);
    walker.set_counters(CounterKind::Loyalty, 3);
    let walker_id = walker.card.id;
    game.battlefield.push(walker);
    let before = game.players[0].hand.len();

    connect(&mut game, frog, AttackDefender::Planeswalker(walker_id));

    assert_eq!(game.players[1].life, 20, "no player was damaged");
    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == walker_id)
            .map(|walker| walker.counters(CounterKind::Loyalty)),
        Some(2),
        "the loyalty took it instead",
    );
    assert_eq!(
        game.players[0].hand.len(),
        before + 1,
        "and the Frog drew all the same",
    );
}

/// A creature is neither a player nor a planeswalker: a Frog that gets
/// blocked deals its damage and draws nothing.
#[test]
fn damage_to_a_blocking_creature_draws_nothing() {
    let (mut game, frog) = staged(&[], &[]);
    let mut blocker = creature(10_500, cards::GRIZZLY_BEARS, PlayerId::Two);
    blocker.blocking = vec![frog];
    game.battlefield.push(blocker);
    let before = game.players[0].hand.len();

    connect(&mut game, frog, AttackDefender::Player(PlayerId::Two));

    assert_eq!(game.players[1].life, 20, "nothing got through");
    assert_eq!(
        game.players[0].hand.len(),
        before,
        "and a blocked Frog draws nothing",
    );
}

/// "Until end of turn": the wings are rented, and the counters are not. A
/// Frog that grew and flew keeps the body it bought and loses the flying
/// when the turn ends.
#[test]
fn the_flying_wears_off_and_the_counter_does_not() {
    let (mut game, frog) = staged(
        &[cards::MOUNTAIN],
        &[cards::MOUNTAIN, cards::ISLAND, cards::SWAMP],
    );

    let grow = activation(&game, frog, 1).expect("a card in hand pays for the counter");
    game.apply(PlayerId::One, grow).expect("it activates");
    settle(&mut game);
    game.priority = PlayerId::One;
    let fly = activation(&game, frog, 2).expect("three cards pay for the flying");
    game.apply(PlayerId::One, fly).expect("it activates");
    settle(&mut game);

    assert!(game.permanent_has_executable_keyword(the_frog(&game), KeywordAbility::Flying));
    assert_eq!(game.power(the_frog(&game)), Some(2), "a 1/2 that grew once");

    game.step = Step::Cleanup;
    game.finish_cleanup();
    game.start_next_turn();

    assert!(
        !game.permanent_has_executable_keyword(the_frog(&game), KeywordAbility::Flying),
        "the wings were only for the turn",
    );
    assert_eq!(
        game.power(the_frog(&game)),
        Some(2),
        "and the counter is still a counter",
    );
}

/// The play the card is built around: neither ability costs mana or a tap,
/// so both are available in the middle of combat. A 1/2 Frog blocked by a
/// 2/2 is dead and kills nothing; two cards out of hand after blockers are
/// declared make it a 3/4 that eats the blocker and walks away.
#[test]
fn discarding_after_blockers_wins_the_fight() {
    let (mut game, frog) = staged(&[cards::MOUNTAIN, cards::ISLAND], &[]);
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    assert_eq!(
        (game.power(the_frog(&game)), game.toughness(the_frog(&game))),
        (Some(1), Some(2)),
        "a 1/2 walks into a 2/2 and loses",
    );

    // Attack, and let them block before anything is spent.
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == frog)
        .expect("it is there")
        .attacking = true;
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == frog)
        .expect("it is there")
        .attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    game.step = Step::DeclareBlockers;
    game.declare_blocker(bears, frog);
    game.finish_declaring_blockers();

    // Blockers are in and the Frog is still a 1/2. Two discards fix that.
    for round in 0..2 {
        // Settling the first activation hands priority on; the attacking
        // player takes it back to spend the second card.
        game.priority = PlayerId::One;
        let action = activation(&game, frog, 1)
            .unwrap_or_else(|| panic!("a card in hand pays for round {round}"));
        game.apply(PlayerId::One, action).expect("it activates");
        settle(&mut game);
    }
    assert_eq!(
        (game.power(the_frog(&game)), game.toughness(the_frog(&game))),
        (Some(3), Some(4)),
        "two counters, mid-combat, for two cards",
    );
    assert!(
        game.players[PlayerId::One.index()].hand.is_empty(),
        "which is the whole hand",
    );

    game.deal_combat_damage();
    game.check_state_based_actions();

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != bears),
        "three damage is more than a bear can take",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == frog),
        "and two is less than the Frog now has",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        20,
        "it was blocked, so nothing got through to draw a card for",
    );
}

/// "Exile three cards from *your* graveyard": a graveyard across the table
/// is no fuel, however full it is. Five of their cards buy nothing, and the
/// third of your own is what buys the wings.
#[test]
fn their_graveyard_does_not_pay_for_the_wings() {
    let (mut game, frog) = staged(&[], &[cards::MOUNTAIN, cards::ISLAND]);
    game.players[1].graveyard.clear();
    for index in 0..5u32 {
        game.players[1]
            .graveyard
            .push(card(105_400 + index, cards::GRIZZLY_BEARS, PlayerId::Two));
    }

    assert!(
        activation(&game, frog, 2).is_none(),
        "two of yours and five of theirs is still two",
    );

    game.players[0]
        .graveyard
        .push(card(105_500, cards::FOREST, PlayerId::One));
    assert!(
        activation(&game, frog, 2).is_some(),
        "and the third of your own is what pays",
    );

    let fly = activation(&game, frog, 2).expect("just checked");
    game.apply(PlayerId::One, fly).expect("it activates");
    settle(&mut game);

    assert!(
        game.permanent_has_executable_keyword(the_frog(&game), KeywordAbility::Flying),
        "the Frog is flying",
    );
    assert!(
        game.players[0].graveyard.is_empty(),
        "on three cards out of your own graveyard",
    );
    assert_eq!(game.players[1].graveyard.len(), 5, "with theirs untouched");
}
