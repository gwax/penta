//! Walking Ballista: two mana a counter, and every counter is a point of
//! damage on the way back out.

use super::*;

/// Player One with a Ballista in hand and enough mana for X of `x`.
fn staged(x: u16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let ballista = game
        .build_zone(PlayerId::One, &[cards::WALKING_BALLISTA])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = ballista.id;
    game.players[0].hand.push(ballista);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 2 * x);
    game.priority = PlayerId::One;
    (game, id)
}

fn resolve(game: &mut Game) {
    for _ in 0..16 {
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

/// Casts the Ballista for `x` and lets it resolve.
fn cast(game: &mut Game, ballista: GameObjectId, x: u16) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => *card == ballista && choices.x() == x,
            _ => false,
        })
        .unwrap_or_else(|| panic!("a Ballista for X={x} is castable"));
    game.apply(PlayerId::One, action).expect("it is castable");
    resolve(game);
}

fn on_battlefield(game: &Game) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::WALKING_BALLISTA)
}

fn counters(game: &Game) -> u16 {
    on_battlefield(game).map_or(0, |permanent| {
        permanent.counters(CounterKind::PlusOnePlusOne)
    })
}

/// Every ability the Ballista is offering right now.
fn abilities(game: &Game, ballista: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == ballista))
        .collect()
}

/// Shoots the opponent once.
fn shoot(game: &mut Game, ballista: GameObjectId) {
    let shot = abilities(game, ballista)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { targets, .. } => targets
                .iter()
                .any(|selection| selection.targets().contains(&Target::Player(PlayerId::Two))),
            _ => false,
        })
        .expect("a counter on it is a shot at the opponent");
    game.apply(PlayerId::One, shot).expect("it activates");
    resolve(game);
}

/// X counters arrive with it, and the body is the counters.
#[test]
fn it_enters_with_x_counters() {
    let (mut game, ballista) = staged(3);
    cast(&mut game, ballista, 3);

    let permanent = on_battlefield(&game).expect("it resolved");
    assert_eq!(permanent.counters(CounterKind::PlusOnePlusOne), 3);
    assert_eq!(game.power(permanent), Some(3), "a 0/0 with three counters");
}

/// Cast for nothing it is a 0/0 and dies at once, which is what makes X
/// matter rather than being decoration.
#[test]
fn cast_for_nothing_it_dies_immediately() {
    let (mut game, ballista) = staged(0);
    cast(&mut game, ballista, 0);

    assert!(on_battlefield(&game).is_none(), "a 0/0 is a 0/0");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::WALKING_BALLISTA),
    );
}

/// A counter comes off as the cost and a point of damage goes across.
#[test]
fn removing_a_counter_deals_a_damage() {
    let (mut game, ballista) = staged(2);
    cast(&mut game, ballista, 2);
    let id = on_battlefield(&game).expect("it resolved").card.id;

    shoot(&mut game, id);

    assert_eq!(game.players[1].life, 19, "one point");
    assert_eq!(counters(&game), 1, "and one counter left");
}

/// The last counter shoots, and then the Ballista is a 0/0.
#[test]
fn the_last_counter_takes_it_with_it() {
    let (mut game, ballista) = staged(1);
    cast(&mut game, ballista, 1);
    let id = on_battlefield(&game).expect("it resolved").card.id;

    shoot(&mut game, id);

    assert_eq!(game.players[1].life, 19);
    assert!(
        on_battlefield(&game).is_none(),
        "nothing holds a 0/0 up once the counter has paid for a shot",
    );
}

/// Four mana buys another counter, and the Ballista grows.
#[test]
fn four_mana_buys_a_counter() {
    let (mut game, ballista) = staged(1);
    cast(&mut game, ballista, 1);
    let id = on_battlefield(&game).expect("it resolved").card.id;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 4);

    let grow = abilities(&game, id)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { targets, .. } if targets.is_empty()))
        .expect("four mana is on offer");
    game.apply(PlayerId::One, grow).expect("it activates");
    resolve(&mut game);

    assert_eq!(counters(&game), 2);
}

/// With no counters on it there is nothing to shoot with.
#[test]
fn a_ballista_with_no_counters_cannot_shoot() {
    let (mut game, ballista) = staged(1);
    cast(&mut game, ballista, 1);
    let id = on_battlefield(&game).expect("it resolved").card.id;
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == id)
    {
        permanent.set_counters(CounterKind::PlusOnePlusOne, 0);
    }

    assert!(
        abilities(&game, id).iter().all(
            |action| matches!(action, Action::ActivateAbility { targets, .. } if targets.is_empty())
        ),
        "the shot needs a counter to pay with",
    );
}

/// "A casting cost of {X}{X} means that you pay twice X." Five mana is an X
/// of two with one left over, not an X of three.
#[test]
fn the_doubled_x_is_paid_twice_over() {
    let (game, ballista) = staged(3);
    let castable = |game: &Game| {
        let mut values = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .filter_map(|action| match action {
                Action::CastSpell { card, choices, .. } if card == ballista => Some(choices.x()),
                _ => None,
            })
            .collect::<Vec<_>>();
        values.sort_unstable();
        values.dedup();
        values
    };
    assert_eq!(castable(&game), vec![0, 1, 2, 3], "six mana reaches three");

    let mut game = game;
    game.players[0].mana_pool.colorless = 5;

    assert_eq!(
        castable(&game),
        vec![0, 1, 2],
        "and one mana short of six is one counter short of three",
    );
}

/// "If Walking Ballista has been dealt damage ... this limits how many times
/// you'll be able to remove +1/+1 counters from it in a single turn. For
/// example, if it has three +1/+1 counters on it and has been dealt 1 damage
/// this turn, it will be destroyed immediately after you activate the ability
/// a second time and you won't be able to activate it a third time."
#[test]
fn damage_already_on_it_cuts_the_shots_short() {
    let (mut game, ballista) = staged(3);
    cast(&mut game, ballista, 3);
    let id = on_battlefield(&game).expect("it resolved").card.id;
    game.damage_target_from_kind(None, Some(Target::Permanent(id)), 1, false);
    resolve(&mut game);
    assert!(
        on_battlefield(&game).is_some(),
        "one damage on a 3/3 is nothing yet",
    );

    shoot(&mut game, id);
    assert_eq!(counters(&game), 2, "a 2/2 with one damage still stands");

    shoot(&mut game, id);

    assert_eq!(game.players[1].life, 18, "both shots got there");
    assert!(
        on_battlefield(&game).is_none(),
        "and the second one left a 1/1 with lethal damage on it",
    );
    assert!(
        abilities(&game, id).is_empty(),
        "so the third counter never gets to shoot",
    );
}

/// "Any target": every test shoots the player, and a creature or a
/// planeswalker is as good a thing to point at. One point is lethal to a
/// Savannah Lions, and one comes off a Tamiyo's loyalty.
#[test]
fn it_shoots_creatures_and_planeswalkers_too() {
    let (mut game, ballista) = staged(3);
    cast(&mut game, ballista, 3);
    // The card in hand and the permanent are different objects.
    let ballista = on_battlefield(&game).expect("it resolved").card.id;
    game.put_onto_battlefield(PlayerId::Two, cards::SAVANNAH_LIONS)
        .expect("cataloged");
    let mut tamiyo = creature(96_100, cards::TAMIYO_COLLECTOR_OF_TALES, PlayerId::Two);
    tamiyo.add_counters(CounterKind::Loyalty, 5);
    let tamiyo_id = tamiyo.card.id;
    game.battlefield.push(tamiyo);
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    let lions = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::SAVANNAH_LIONS)
        .expect("the Lions are on the battlefield")
        .card
        .id;

    let shot = abilities(&game, ballista)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { targets, .. } => targets
                .iter()
                .any(|selection| selection.targets().contains(&Target::Permanent(lions))),
            _ => false,
        })
        .expect("the Lions are a legal thing to shoot");
    game.apply(PlayerId::One, shot).expect("it activates");
    resolve(&mut game);
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == lions),
        "one point is all a 2/1 has to spare",
    );

    // Resolving that shot handed priority across the table.
    game.priority = PlayerId::One;
    let shot = abilities(&game, ballista)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { targets, .. } => targets
                .iter()
                .any(|selection| selection.targets().contains(&Target::Permanent(tamiyo_id))),
            _ => false,
        })
        .expect("a planeswalker is a legal thing to shoot");
    game.apply(PlayerId::One, shot).expect("it activates");
    resolve(&mut game);

    assert_eq!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == tamiyo_id)
            .expect("she is still there")
            .counters(CounterKind::Loyalty),
        4,
        "and the damage came off her loyalty",
    );
    assert_eq!(counters(&game), 1, "two counters spent, one left");
}

/// Nothing in either ability is a tap, so a Ballista shoots the turn it
/// lands.
#[test]
fn it_shoots_the_turn_it_arrives() {
    let (mut game, ballista) = staged(1);
    cast(&mut game, ballista, 1);
    let ballista = on_battlefield(&game).expect("it resolved").card.id;
    let life = game.players[1].life;
    assert_eq!(
        on_battlefield(&game)
            .expect("it resolved")
            .entered_controller_turn,
        game.turns_started[PlayerId::One.index()],
        "it came down this turn",
    );

    shoot(&mut game, ballista);

    assert_eq!(
        game.players[1].life,
        life - 1,
        "and summoning sickness has nothing to say to a cost with no tap in it",
    );
}

/// Neither ability says when: on their turn, with their spell on the stack,
/// four mana buys a counter and the counter buys a point of damage. That is
/// the line the card is played for, and the only reason the growth half is
/// worth its four.
#[test]
fn both_halves_work_on_their_turn() {
    let (mut game, ballista) = staged(1);
    cast(&mut game, ballista, 1);
    let ballista = on_battlefield(&game).expect("it resolved").card.id;
    assert_eq!(counters(&game), 1, "one counter to start with");

    // Their turn, their spell waiting to resolve.
    game.players[1]
        .hand
        .push(card(120_000, cards::LIGHTNING_BOLT, PlayerId::Two));
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(
            CardInstanceId(120_000),
            vec![Target::Player(PlayerId::One)],
            Vec::new(),
            0,
        ),
    )
    .expect("they cast their Bolt");
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 4);

    let grow = abilities(&game, ballista)
        .into_iter()
        .find(|action| {
            matches!(action, Action::ActivateAbility { targets, .. } if targets.is_empty())
        })
        .expect("four mana buys a counter, whoever's turn it is");
    game.apply(PlayerId::One, grow).expect("it activates");
    drain_pending(&mut game);
    assert_eq!(counters(&game), 2, "a second counter, on their turn");

    game.priority = PlayerId::One;
    let life = game.players[1].life;
    let shot = abilities(&game, ballista)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { targets, .. } => targets
                .iter()
                .any(|selection| selection.targets().contains(&Target::Player(PlayerId::Two))),
            _ => false,
        })
        .expect("and a counter is still a shot");
    game.apply(PlayerId::One, shot).expect("it activates");
    drain_pending(&mut game);

    assert_eq!(
        game.players[1].life,
        life - 1,
        "the shot went across on their turn",
    );
    assert_eq!(counters(&game), 1, "and the counter paid for it");
}
