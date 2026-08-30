//! Unruly Krasis: a body that lends its own size, and an adapt that only
//! ever fills an empty creature.

use super::*;

/// The Krasis and a Savannah Lions under Player One, with `theirs` opposite.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let krasis = creature(78_000, cards::UNRULY_KRASIS, PlayerId::One);
    let krasis_id = krasis.card.id;
    game.battlefield.push(krasis);
    let lions = creature(78_001, cards::SAVANNAH_LIONS, PlayerId::One);
    let lions_id = lions.card.id;
    game.battlefield.push(lions);
    for (index, definition) in theirs.iter().enumerate() {
        game.battlefield.push(creature(
            78_100 + u32::try_from(index).expect("a handful"),
            *definition,
            PlayerId::Two,
        ));
    }
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, krasis_id, lions_id)
}

/// Activates adapt, which the Krasis always offers.
fn adapt(game: &mut Game, krasis: GameObjectId) {
    game.players[0].mana_pool.green = 1;
    game.players[0].mana_pool.blue = 1;
    game.players[0].mana_pool.colorless = 3;
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(
            |action| matches!(action, Action::ActivateAbility { source, .. } if *source == krasis),
        )
        .expect("adapt is always offered");
    game.apply(PlayerId::One, action).expect("it activates");
    drain_pending(game);
}

fn size(game: &Game, id: GameObjectId) -> (Option<i16>, Option<i16>) {
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("still there");
    (game.power(permanent), game.toughness(permanent))
}

/// Attacks with the Krasis and answers the trigger by taking the last
/// option offered at every question, which is the affirmative one.
fn attack_and_accept(game: &mut Game, krasis: GameObjectId) {
    game.step = Step::DeclareAttackers;
    game.declare_attacker(krasis, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    for _ in 0..16 {
        if let Some(decision) = game.observe(PlayerId::One).decision {
            let accept = decision.options.last().expect("an option is offered").id;
            game.apply(
                PlayerId::One,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![accept],
                },
            )
            .expect("the offered option is legal");
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
}

/// "The value of X is determined only once, as the ability resolves. Further
/// changes to this creature's power that turn won't cause the target's base
/// power and toughness to change."
#[test]
fn x_is_read_once_and_not_again() {
    let (mut game, krasis, lions) = staged(&[]);

    attack_and_accept(&mut game, krasis);
    assert_eq!(size(&game, lions), (Some(4), Some(4)), "the Krasis's size");

    adapt(&mut game, krasis);

    assert_eq!(size(&game, krasis), (Some(7), Some(7)), "it grew");
    assert_eq!(
        size(&game, lions),
        (Some(4), Some(4)),
        "and the Lions kept the four X was worth when it resolved",
    );
}

/// "Another target creature you control": not the Krasis itself, and not
/// one of theirs however inviting.
#[test]
fn the_trigger_names_another_creature_of_yours() {
    let (mut game, krasis, lions) = staged(&[cards::GRIZZLY_BEARS]);

    game.step = Step::DeclareAttackers;
    game.declare_attacker(krasis, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    for _ in 0..8 {
        if game.observe(PlayerId::One).decision.is_some() {
            break;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            break;
        }
    }

    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the trigger asks for its target");
    let offered = decision
        .options
        .iter()
        .filter_map(|option| option.card.map(|(object, _)| object))
        .collect::<Vec<_>>();

    assert_eq!(
        offered,
        vec![lions],
        "the Lions and nobody else -- not itself, and not their Bear",
    );
}

/// "If a creature somehow loses all of its +1/+1 counters, it can adapt
/// again and get more." The blocked case is what the keyword is known for;
/// this is the other side of the same condition.
#[test]
fn losing_the_counters_opens_adapt_again() {
    let (mut game, krasis, _lions) = staged(&[]);

    adapt(&mut game, krasis);
    assert_eq!(size(&game, krasis), (Some(7), Some(7)), "three counters");

    for permanent in &mut game.battlefield {
        if permanent.card.id == krasis {
            permanent.set_counters(CounterKind::PlusOnePlusOne, 0);
        }
    }
    assert_eq!(size(&game, krasis), (Some(4), Some(4)), "and then none");

    adapt(&mut game, krasis);

    assert_eq!(
        size(&game, krasis),
        (Some(7), Some(7)),
        "an empty creature adapts as readily as it did the first time",
    );
}
