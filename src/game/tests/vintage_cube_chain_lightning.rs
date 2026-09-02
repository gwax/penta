//! Chain Lightning: three damage, and an invitation to send it back.

use super::*;

/// Player One casting a Chain Lightning at Player Two, who has two
/// Mountains to answer with and `theirs` beside them.
fn staged(theirs: &[CardDefinitionId]) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for index in 0..2 {
        game.battlefield
            .push(creature(80_000 + index, cards::MOUNTAIN, PlayerId::Two));
    }
    for (index, definition) in theirs.iter().enumerate() {
        game.battlefield.push(creature(
            80_100 + u32::try_from(index).expect("a handful"),
            *definition,
            PlayerId::Two,
        ));
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    let chain = card(80_200, cards::CHAIN_LIGHTNING, PlayerId::One);
    let chain_id = chain.id;
    game.players[0].hand.push(chain);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].mana_pool.red = 1;
    game.apply(
        PlayerId::One,
        cast_action(chain_id, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("one red mana casts it");
    pass_priority_pair(&mut game);
    game
}

/// Pays the {R}{R} and sends the copy back at `target`.
fn pay_and_copy_to(game: &mut Game, player: PlayerId, target: Target) {
    choose_decision_by_label(game, player, "Pay the cost");
    choose_decision_by_label(game, player, "Do it");
    let decision = game
        .observe(player)
        .decision
        .expect("copying the chain offers new targets");
    let option = match &game
        .pending_decisions
        .first()
        .expect("the retarget decision is pending")
        .continuation
    {
        DecisionContinuation::CopyStackObject { target_lists, .. } => target_lists
            .iter()
            .position(|targets| flatten_target_selections(targets) == [target])
            .and_then(|index| u32::try_from(index).ok())
            .expect("the requested chain target is offered"),
        continuation => panic!("unexpected chain continuation: {continuation:?}"),
    };
    game.apply(
        player,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![option],
        },
    )
    .expect("the copy is made");
}

/// "The player putting the copy of the spell on the stack controls that
/// copy." The Chain came from across the table; what goes back belongs to
/// the player who paid for it.
#[test]
fn the_copy_belongs_to_the_player_who_paid_for_it() {
    let mut game = staged(&[]);

    pay_and_copy_to(&mut game, PlayerId::Two, Target::Player(PlayerId::One));

    let copy = game.stack.last().expect("the copy is on the stack");
    assert!(copy.is_copy);
    assert_eq!(
        copy.controller,
        PlayerId::Two,
        "the payer controls it, not the player who cast the original",
    );
    assert_eq!(copy.targets(), vec![Target::Player(PlayerId::One)]);
}

/// "The copy is created on the stack, so it's not cast. Abilities that
/// trigger when a player casts a spell won't trigger." The Dryad watches its
/// controller cast red spells, and a copy its controller made is not one.
#[test]
fn the_copy_is_created_rather_than_cast() {
    let mut game = staged(&[cards::QUIRION_DRYAD]);
    let dryad = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::QUIRION_DRYAD)
        .expect("it is there")
        .card
        .id;

    pay_and_copy_to(&mut game, PlayerId::Two, Target::Player(PlayerId::One));
    drain_pending(&mut game);

    let dryad = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == dryad)
        .expect("still there");
    assert_eq!(
        (game.power(dryad), game.toughness(dryad)),
        (Some(1), Some(1)),
        "a copy put onto the stack was never cast, so nothing triggered",
    );
}

/// The chain: the copy resolving offers its own target's controller the same
/// {R}{R}, which is what the card is named for. Three damage each way, and
/// the second payer is asked in turn.
#[test]
fn the_copy_offers_the_chain_onward() {
    let mut game = staged(&[]);
    // Two Mountains' worth of red for the player it comes back at.
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 2);
    assert_eq!(
        game.players[1].life, 17,
        "the original dealt its three as it resolved",
    );

    pay_and_copy_to(&mut game, PlayerId::Two, Target::Player(PlayerId::One));

    // Let the copy resolve.
    let before = game.players[0].life;
    pass_priority_pair(&mut game);

    assert_eq!(
        game.players[0].life,
        before - 3,
        "and the copy dealt three the other way",
    );
    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the copy's target is asked in turn");
    assert!(
        decision
            .options
            .iter()
            .any(|option| option.label == "Pay the cost"),
        "with two Mountains' worth of red, the chain may go on: {:?}",
        decision.options,
    );
}

/// "If the targeted player or permanent is an illegal target as Chain
/// Lightning tries to resolve, the spell doesn't resolve and none of its
/// effects happen. It can't be copied."
#[test]
fn a_lost_target_takes_the_copy_with_it() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for index in 0..2 {
        game.battlefield
            .push(creature(80_300 + index, cards::MOUNTAIN, PlayerId::Two));
    }
    let bears = creature(80_310, cards::GRIZZLY_BEARS, PlayerId::Two);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    let chain = card(80_320, cards::CHAIN_LIGHTNING, PlayerId::One);
    let chain_id = chain.id;
    game.players[0].hand.push(chain);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].mana_pool.red = 1;
    game.apply(
        PlayerId::One,
        cast_action(chain_id, vec![Target::Permanent(bears_id)], Vec::new(), 0),
    )
    .expect("one red mana casts it");

    // The bear leaves under the spell.
    game.move_permanents_to_graveyard(&[bears_id]);
    game.check_state_based_actions();
    let their_life = game.players[1].life;
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert!(
        game.events.iter().any(|event| matches!(
            event,
            GameEvent::SpellFizzled { definition, .. } if *definition == cards::CHAIN_LIGHTNING
        )),
        "it was countered for having no legal target",
    );
    assert_eq!(
        game.players[1].life, their_life,
        "nothing was dealt to anyone else instead",
    );
    assert!(
        game.observe(PlayerId::Two).decision.is_none(),
        "and nobody was offered the {{R}}{{R}}",
    );
    assert!(game.stack.is_empty(), "with no copy left behind");
}

/// "That player ... *may* pay {R}{R}." The three damage is already dealt by
/// the time the question is asked -- the payment buys the copy and nothing
/// else -- so declining leaves them at seventeen with nothing coming back.
#[test]
fn declining_the_payment_ends_the_chain() {
    let mut game = staged(&[]);
    let life = game.players[PlayerId::Two.index()].life;
    assert_eq!(life, 17, "the damage happened before the offer");

    choose_decision_by_label(&mut game, PlayerId::Two, "Decline");
    drain_pending(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        life,
        "declining costs them nothing further",
    );
    assert!(game.stack.is_empty(), "and nothing came back the other way");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::CHAIN_LIGHTNING),
        "the Chain is in its owner's graveyard, done with",
    );
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| permanent.controller == PlayerId::Two && permanent.tapped)
            .count(),
        0,
        "and their Mountains are untapped: nothing was paid",
    );
}
