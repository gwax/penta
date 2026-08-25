//! Sorin of House Markov: a lifelinking two-drop that turns into a
//! planeswalker on the turn he drinks enough.

use super::*;

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .find(|option| option.label != "Decline")
                .or_else(|| decision.options.first())
                .map(|option| vec![option.id])
                .unwrap_or_default();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the decision accepts what it offered");
            continue;
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

/// Sorin on the battlefield since last turn, on his front face.
fn staged() -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let sorin = game
        .put_onto_battlefield(PlayerId::One, cards::SORIN_OF_HOUSE_MARKOV)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[0].life = 20;
    (game, sorin)
}

/// Sorin after he has turned over, with `loyalty` counters on him.
fn as_planeswalker(loyalty: u16) -> (Game, GameObjectId) {
    let (mut game, _sorin) = staged();
    game.gain_life(PlayerId::One, 3);
    postcombat_main(&mut game);
    let neonate = permanent_named(&game, "Sorin, Ravenous Neonate")
        .expect("he turned over")
        .card
        .id;
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == neonate)
        .expect("he is there")
        .set_counters(CounterKind::Loyalty, loyalty);
    game.step = Step::PrecombatMain;
    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;
    (game, neonate)
}

fn postcombat_main(game: &mut Game) {
    game.step = Step::PostcombatMain;
    game.begin_step_triggers();
    settle(game);
}

fn permanent_named<'a>(game: &'a Game, name: &str) -> Option<&'a Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| game.effective_permanent_name(permanent).as_deref() == Some(name))
}

fn activate(game: &mut Game, sorin: GameObjectId, index: u8, victim: Option<GameObjectId>) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility {
                source,
                ability: AbilityOrigin::Printed { ability, .. },
                targets,
                ..
            } => {
                *source == sorin
                    && *ability == AbilityId(index)
                    && victim.is_none_or(|victim| {
                        targets.iter().any(|selection| {
                            selection.targets().iter().any(
                                |target| matches!(target, Target::Permanent(id) if *id == victim),
                            )
                        })
                    })
            }
            _ => false,
        })
        .expect("the loyalty ability is offered");
    game.apply(PlayerId::One, action).expect("it is activated");
    settle(game);
}

/// Three life in a turn turns him over in the postcombat main phase, and he
/// comes back as a planeswalker with his printed loyalty.
#[test]
fn three_life_turns_him_over() {
    let (mut game, sorin) = staged();
    game.gain_life(PlayerId::One, 3);

    postcombat_main(&mut game);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != sorin),
        "the creature he was is gone",
    );
    let neonate = permanent_named(&game, "Sorin, Ravenous Neonate").expect("he came back");
    assert_eq!(neonate.counters(CounterKind::Loyalty), 3);
    assert_eq!(neonate.controller, PlayerId::One);
}

/// Two is not three, and the trigger checks again as it resolves.
#[test]
fn two_life_leaves_him_a_creature() {
    let (mut game, sorin) = staged();
    game.gain_life(PlayerId::One, 2);

    postcombat_main(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == sorin),
        "he is still the Noble",
    );
}

/// The tally is what was gained rather than where the life total ended up:
/// gaining three and losing three still turns him over.
#[test]
fn losing_the_life_again_does_not_stop_him() {
    let (mut game, _sorin) = staged();
    game.gain_life(PlayerId::One, 3);
    game.players[0].life -= 5;

    postcombat_main(&mut game);

    assert!(
        permanent_named(&game, "Sorin, Ravenous Neonate").is_some(),
        "what the clause reads is the gaining",
    );
}

/// His plus makes a Food, which is the other way he gets to three.
#[test]
fn the_plus_two_makes_food() {
    let (mut game, sorin) = as_planeswalker(3);

    activate(&mut game, sorin, 1, None);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| is_token_with(permanent, tokens::food())),
        "a Food token is there",
    );
}

/// The minus one spends the same tally the front face read.
#[test]
fn the_minus_one_burns_for_the_life_you_gained() {
    let (mut game, sorin) = as_planeswalker(5);
    game.gain_life(PlayerId::One, 4);
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);

    activate(&mut game, sorin, 2, Some(bears));

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != bears),
        "four damage killed the Bear",
    );
}

/// The ultimate takes a creature, makes it a Vampire, and gives it a
/// lifelink counter when you have another white permanent.
#[test]
fn the_ultimate_takes_a_creature_and_marks_it() {
    let (mut game, sorin) = as_planeswalker(6);
    let bears = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    game.put_onto_battlefield(PlayerId::One, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);

    activate(&mut game, sorin, 3, Some(bears));

    let stolen = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears)
        .expect("it is still on the battlefield");
    assert_eq!(stolen.controller, PlayerId::One, "under your control now");
    let subtypes = game.effective_subtypes(stolen);
    assert!(subtypes.contains(&"Bear"), "it is still a Bear");
    assert!(subtypes.contains(&"Vampire"), "and a Vampire as well");
    assert_eq!(stolen.counters(CounterKind::Lifelink), 1);
}

/// "Other than that creature or Sorin": a white creature you just took does
/// not count as the white permanent that pays for its own counter, and
/// neither does Sorin, who is white himself.
#[test]
fn the_counter_needs_a_third_white_permanent() {
    let (mut game, sorin) = as_planeswalker(6);
    let angel = game
        .put_onto_battlefield(PlayerId::Two, cards::SERRA_ANGEL)
        .expect("cataloged");
    drain_pending(&mut game);

    activate(&mut game, sorin, 3, Some(angel));

    let stolen = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == angel)
        .expect("it is still on the battlefield");
    assert_eq!(stolen.controller, PlayerId::One, "you took it all the same");
    assert_eq!(
        stolen.counters(CounterKind::Lifelink),
        0,
        "the only white permanents are Sorin and the creature itself",
    );
}
