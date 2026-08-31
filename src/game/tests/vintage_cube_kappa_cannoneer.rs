//! Kappa Cannoneer: a six-mana artifact that the rest of your artifacts pay
//! for, grow, and make unblockable.

use super::*;

fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let cannoneer = game
        .put_onto_battlefield(PlayerId::One, cards::KAPPA_CANNONEER)
        .expect("cataloged");
    drain_pending(&mut game);
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, cannoneer)
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// Its own arrival counts: it is already a 5/5 by the time anyone sees it.
#[test]
fn it_grows_on_its_own_arrival() {
    let (game, cannoneer) = staged();
    let turtle = permanent(&game, cannoneer);

    assert_eq!(turtle.counters(CounterKind::PlusOnePlusOne), 1);
    assert_eq!(
        (game.power(turtle), game.toughness(turtle)),
        (Some(5), Some(5))
    );
}

/// Every artifact after it is another counter, creature or not.
#[test]
fn every_artifact_afterwards_grows_it() {
    let (mut game, cannoneer) = staged();

    game.put_onto_battlefield(PlayerId::One, cards::MOX_JET)
        .expect("cataloged");
    drain_pending(&mut game);
    assert_eq!(
        permanent(&game, cannoneer).counters(CounterKind::PlusOnePlusOne),
        2,
        "a Mox is an artifact",
    );

    // Somebody else's artifact is not one you control.
    game.put_onto_battlefield(PlayerId::Two, cards::BLACK_LOTUS)
        .expect("cataloged");
    drain_pending(&mut game);
    assert_eq!(
        permanent(&game, cannoneer).counters(CounterKind::PlusOnePlusOne),
        2,
    );
}

/// The trigger also makes it unblockable for the turn.
#[test]
fn an_artifact_makes_it_unblockable_for_the_turn() {
    let (mut game, cannoneer) = staged();
    game.put_onto_battlefield(PlayerId::One, cards::MOX_JET)
        .expect("cataloged");
    drain_pending(&mut game);

    let blocker = creature(98_000, cards::SERRA_ANGEL, PlayerId::Two);
    game.battlefield.push(blocker);
    game.step = Step::DeclareBlockers;
    game.attackers_declared = true;
    if let Some(attacker) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == cannoneer)
    {
        attacker.attacking = true;
        attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    }

    assert!(
        !game.legal_actions(PlayerId::Two).iter().any(
            |action| matches!(action, Action::DeclareBlocker { attacker, .. } if *attacker == cannoneer)
        ),
        "nothing may block it this turn",
    );
}

/// Improvise and ward are both printed on it.
#[test]
fn it_improvises_and_wards() {
    let (game, cannoneer) = staged();
    let turtle = permanent(&game, cannoneer);

    assert!(game.permanent_has_executable_keyword(turtle, KeywordAbility::Improvise));
    assert!(
        game.effective_rules(turtle)
            .is_some_and(|rules| rules.rules_text().contains("Ward {4}")),
    );
}

/// "Another artifact *you control*": the Mox they play is not one of yours,
/// so the Turtle stays the size it was.
#[test]
fn their_artifact_does_not_grow_it() {
    let (mut game, cannoneer) = staged();
    let before = permanent(&game, cannoneer).counters(CounterKind::PlusOnePlusOne);

    game.put_onto_battlefield(PlayerId::Two, cards::MOX_JET)
        .expect("cataloged");
    drain_pending(&mut game);

    assert_eq!(
        permanent(&game, cannoneer).counters(CounterKind::PlusOnePlusOne),
        before,
        "an artifact across the table is nothing to it",
    );
}

/// "It can't be blocked *this turn*": the counter is permanent and the
/// evasion is not, so a turn later it can be blocked like anything else.
#[test]
fn the_unblockable_lasts_only_that_turn() {
    let (mut game, cannoneer) = staged();
    game.put_onto_battlefield(PlayerId::One, cards::MOX_JET)
        .expect("cataloged");
    drain_pending(&mut game);

    // Their turn and then yours again.
    for _ in 0..2 {
        game.finish_cleanup();
        game.start_next_turn();
        drain_pending(&mut game);
    }
    assert_eq!(game.active_player, PlayerId::One);

    let blocker = creature(98_100, cards::SERRA_ANGEL, PlayerId::Two);
    game.battlefield.push(blocker);
    game.step = Step::DeclareBlockers;
    game.attackers_declared = true;
    if let Some(attacker) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == cannoneer)
    {
        attacker.attacking = true;
        attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
        attacker.tapped = false;
    }

    assert_eq!(
        permanent(&game, cannoneer).counters(CounterKind::PlusOnePlusOne),
        2,
        "the counters the Mox gave it are still there",
    );
    assert!(
        game.legal_actions(PlayerId::Two).iter().any(
            |action| matches!(action, Action::DeclareBlocker { attacker, .. } if *attacker == cannoneer)
        ),
        "but the evasion belonged to the turn the Mox arrived",
    );
}

/// Passes until something waits on an answer, which is where a ward lives.
fn settle_to_decision(game: &mut Game) {
    for _ in 0..24 {
        if game.observe(PlayerId::One).decision.is_some()
            || game.observe(PlayerId::Two).decision.is_some()
        {
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

/// Answers the waiting ward decision, paying or declining, and reports the
/// labels it was offered.
fn answer_ward(game: &mut Game, pay: bool) -> Vec<String> {
    let decision = game
        .observe(PlayerId::Two)
        .decision
        .expect("the targeting player was asked about the ward cost");
    let labels = decision
        .options
        .iter()
        .map(|option| option.label.clone())
        .collect::<Vec<_>>();
    let option = decision
        .options
        .iter()
        .find(|option| (option.label != "Decline") == pay)
        .unwrap_or_else(|| panic!("paying {pay} is offered: {labels:?}"))
        .id;
    game.apply(
        PlayerId::Two,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![option],
        },
    )
    .expect("the answer is legal");
    settle_to_decision(game);
    labels
}

/// Player Two bolts the Turtle with `spare` colourless left over for the
/// ward.
fn bolt_the_turtle(game: &mut Game, cannoneer: GameObjectId, spare: u16) {
    game.players[1]
        .hand
        .push(card(93_400, cards::LIGHTNING_BOLT, PlayerId::Two));
    game.players[1].mana_pool.red = 1;
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Colorless, spare);
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(
            CardInstanceId(93_400),
            vec![Target::Permanent(cannoneer)],
            Vec::new(),
            0,
        ),
    )
    .expect("the Bolt is castable");
    settle_to_decision(game);
}

/// Ward {4} in the doing rather than in the rules text: declining the four
/// counters the spell, and the Turtle takes nothing.
#[test]
fn a_declined_ward_counters_their_spell() {
    let (mut game, cannoneer) = staged();
    bolt_the_turtle(&mut game, cannoneer, 4);

    answer_ward(&mut game, false);
    game.check_state_based_actions();

    assert_eq!(
        permanent(&game, cannoneer).damage,
        0,
        "the Bolt never resolved",
    );
    assert_eq!(
        game.players[1].mana_pool.colorless, 4,
        "and the four they had is still theirs",
    );
}

/// Paying it lets the spell through, and the four mana is gone.
#[test]
fn a_paid_ward_lets_the_bolt_through() {
    let (mut game, cannoneer) = staged();
    bolt_the_turtle(&mut game, cannoneer, 4);

    answer_ward(&mut game, true);
    game.check_state_based_actions();

    assert_eq!(
        permanent(&game, cannoneer).damage,
        3,
        "three damage on a 5/5 that lives through it",
    );
    assert_eq!(
        game.players[1].mana_pool.colorless, 0,
        "and the ward took its four",
    );
}

/// Improvise in the doing: five artifacts stand in for the {5}, so one blue
/// mana is the whole of what a six-drop costs.
#[test]
fn improvise_lets_the_artifacts_pay_the_generic() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for definition in [
        cards::HOWLING_MINE,
        cards::ICY_MANIPULATOR,
        cards::DARKSTEEL_PLATE,
        cards::MANIFOLD_KEY,
        cards::JADE_STATUE,
    ] {
        game.put_onto_battlefield(PlayerId::One, definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    let held = card(93_500, cards::KAPPA_CANNONEER, PlayerId::One);
    let held_id = held.id;
    game.players[0].hand.push(held);
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == held_id)),
        "five untapped artifacts and a blue mana pay for it",
    );

    for permanent in &mut game.battlefield {
        permanent.tapped = true;
    }

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .all(|action| !matches!(action, Action::CastSpell { card, .. } if *card == held_id)),
        "and artifacts already tapped help with nothing",
    );
}

/// "Each artifact you tap pays for {1}": the generic half and nothing else,
/// so a board of artifacts and no blue mana leaves a six-drop uncastable.
#[test]
fn improvise_never_pays_the_coloured_pip() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    for definition in [
        cards::HOWLING_MINE,
        cards::ICY_MANIPULATOR,
        cards::DARKSTEEL_PLATE,
        cards::MANIFOLD_KEY,
        cards::JADE_STATUE,
        cards::SOL_RING,
    ] {
        game.put_onto_battlefield(PlayerId::One, definition)
            .expect("cataloged");
    }
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    let held = card(93_600, cards::KAPPA_CANNONEER, PlayerId::One);
    let held_id = held.id;
    game.players[0].hand.push(held);
    game.turns_started[PlayerId::One.index()] = 5;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .all(|action| !matches!(action, Action::CastSpell { card, .. } if *card == held_id)),
        "six artifacts are six generic and no blue at all",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 5);
    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .all(|action| !matches!(action, Action::CastSpell { card, .. } if *card == held_id)),
        "and colourless mana is no more blue than an artifact is",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == held_id)),
        "one blue is the whole of what improvise cannot cover",
    );
}

/// Ward is "whenever this becomes the target of a spell or ability an
/// opponent controls": your own Bolt aimed at your own Turtle asks for
/// nothing.
#[test]
fn your_own_spell_pays_no_ward() {
    let (mut game, cannoneer) = staged();
    let bolt = card(93_700, cards::LIGHTNING_BOLT, PlayerId::One);
    let bolt_id = bolt.id;
    game.players[PlayerId::One.index()].hand.push(bolt);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 1);

    game.apply(
        PlayerId::One,
        cast_action(bolt_id, vec![Target::Permanent(cannoneer)], Vec::new(), 0),
    )
    .expect("your own creature is a legal target for your own Bolt");
    drain_pending(&mut game);
    game.check_state_based_actions();

    assert!(
        game.pending_decisions.is_empty(),
        "nobody was asked for four mana",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.total(),
        0,
        "the red paid for the Bolt and nothing else was spent",
    );
    let turtle = permanent(&game, cannoneer);
    assert_eq!(turtle.damage, 3, "and the Bolt resolved, ward or no ward");
}
