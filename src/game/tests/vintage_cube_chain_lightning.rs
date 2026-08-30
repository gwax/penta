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
