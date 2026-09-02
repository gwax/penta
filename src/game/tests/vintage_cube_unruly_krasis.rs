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

/// "You can always activate an ability that will cause a creature to adapt.
/// As it resolves, if the creature has a +1/+1 counter on it for any reason,
/// you simply won't put any counters on it." The five mana is spent and the
/// Krasis is the size it already was.
#[test]
fn adapting_twice_is_five_mana_for_nothing() {
    let (mut game, krasis, _lions) = staged(&[]);
    adapt(&mut game, krasis);
    assert_eq!(
        size(&game, krasis),
        (Some(7), Some(7)),
        "4/4 and three counters"
    );

    adapt(&mut game, krasis);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == krasis)
            .expect("it is there")
            .counters(CounterKind::PlusOnePlusOne),
        3,
        "the second adapt adds nothing to what is already there",
    );
    assert_eq!(size(&game, krasis), (Some(7), Some(7)));
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.total(),
        0,
        "and the mana was spent for it all the same",
    );
}

/// "The ability overwrites previous effects that set power and toughness to
/// specific numbers. Effects that otherwise modify them still apply. The
/// same is true for +1/+1 counters." A Lions with a counter on it becomes a
/// 4/4 base and keeps the counter on top.
#[test]
fn the_base_it_sets_is_still_read_under_a_counter() {
    let (mut game, krasis, lions) = staged(&[]);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == lions)
        .expect("it is there")
        .add_counters(CounterKind::PlusOnePlusOne, 1);
    assert_eq!(
        size(&game, lions),
        (Some(3), Some(2)),
        "a 2/1 with a counter on it",
    );

    attack_and_accept(&mut game, krasis);

    assert_eq!(
        size(&game, lions),
        (Some(5), Some(5)),
        "the base became 4/4 and the counter is still worth its +1/+1",
    );
}

/// The trigger is a "you may": declined, the creature it would have named is
/// left the size it was.
#[test]
fn the_trigger_may_be_declined() {
    let (mut game, krasis, lions) = staged(&[]);

    game.step = Step::DeclareAttackers;
    game.declare_attacker(krasis, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    for _ in 0..16 {
        if let Some(decision) = game.observe(PlayerId::One).decision {
            let decline = decision.options.first().expect("an option is offered").id;
            game.apply(
                PlayerId::One,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![decline],
                },
            )
            .expect("declining is one of the answers");
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

    assert_eq!(
        size(&game, lions),
        (Some(2), Some(1)),
        "the Lions is the 2/1 it was printed as",
    );
}

/// X is the Krasis's power as the trigger resolves, counters and all, not
/// the four it was printed with. Adapt first and the Lions is handed the
/// seven the Krasis has grown to -- which is the order this card is played
/// in when there is mana for both.
#[test]
fn adapting_before_the_attack_hands_the_lions_a_seven() {
    let (mut game, krasis, lions) = staged(&[]);
    adapt(&mut game, krasis);
    assert_eq!(size(&game, krasis), (Some(7), Some(7)), "three counters on");

    attack_and_accept(&mut game, krasis);

    assert_eq!(
        size(&game, lions),
        (Some(7), Some(7)),
        "and X was read off the Krasis standing there, not off its printing",
    );
}

/// "Until end of turn." The base it set is rented, so the Lions the Krasis
/// made a 4/4 is the 2/1 it was printed as on the turn after.
#[test]
fn the_base_it_sets_wears_off_with_the_turn() {
    let (mut game, krasis, lions) = staged(&[]);

    attack_and_accept(&mut game, krasis);
    assert_eq!(size(&game, lions), (Some(4), Some(4)), "a 4/4 this turn");

    // Walking the steps rather than jumping the turn: the cleanup step is
    // where an until-end-of-turn effect is let go of.
    let turn = game.turn;
    for _ in 0..80 {
        if game.turn > turn {
            break;
        }
        game.advance_step();
        drain_pending(&mut game);
    }
    assert!(game.turn > turn, "the turn ended");

    assert_eq!(
        size(&game, lions),
        (Some(2), Some(1)),
        "and a Savannah Lions again afterwards",
    );
}

/// Trample is what the size is for: a 4/4 held by a 2/2 sends two past it,
/// and a Krasis that has adapted to a 7/7 first sends five. Nothing else on
/// the card would care how big it is once a chump blocker is on it.
#[test]
fn it_tramples_the_excess_past_a_chump_blocker() {
    for (adapt_first, expected) in [(false, 2), (true, 5)] {
        let (mut game, krasis, _) = staged(&[cards::GRIZZLY_BEARS]);
        let chump = game
            .battlefield
            .iter()
            .find(|permanent| permanent.controller == PlayerId::Two)
            .expect("they have a blocker")
            .card
            .id;
        if adapt_first {
            adapt(&mut game, krasis);
        }
        let life = game.players[PlayerId::Two.index()].life;

        game.step = Step::DeclareAttackers;
        game.declare_attacker(krasis, AttackDefender::Player(PlayerId::Two));
        game.finish_declaring_attackers();
        drain_pending(&mut game);
        game.step = Step::DeclareBlockers;
        game.declare_blocker(chump, krasis);
        game.step = Step::CombatDamage;
        game.deal_combat_damage();
        game.check_state_based_actions();

        assert_eq!(
            game.players[PlayerId::Two.index()].life,
            life - expected,
            "a blocked Krasis sends what the blocker cannot eat through",
        );
        assert!(
            !game
                .battlefield
                .iter()
                .any(|permanent| permanent.card.id == chump),
            "and the blocker took the rest of it",
        );
    }
}
