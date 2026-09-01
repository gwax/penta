//! Touch the Spirit Realm: an answer that gives the thing back when it
//! leaves, and a channel cost that blinks one of yours instead.

use super::*;

/// The enchantment in hand with `board` on the battlefield.
fn staged(board: &[(CardDefinitionId, PlayerId)]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut ids = Vec::new();
    for (index, (definition, controller)) in board.iter().enumerate() {
        let permanent = creature(
            107_000 + u32::try_from(index).expect("few permanents"),
            *definition,
            *controller,
        );
        ids.push(permanent.card.id);
        game.battlefield.push(permanent);
    }
    let touch = game
        .build_zone(PlayerId::One, &[cards::TOUCH_THE_SPIRIT_REALM])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let touch_id = touch.id;
    game.players[0].hand.push(touch);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 3);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, touch_id, ids)
}

/// Answers whatever is waiting, naming `target` when one is wanted.
fn settle(game: &mut Game, target: Option<GameObjectId>) {
    for _ in 0..16 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = match target {
                Some(wanted) => decision
                    .options
                    .iter()
                    .filter(|option| option.card.is_some_and(|(object, _)| object == wanted))
                    .map(|option| option.id)
                    .take(1)
                    .collect(),
                None => Vec::new(),
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
}

fn on_battlefield(game: &Game, definition: CardDefinitionId) -> bool {
    game.battlefield
        .iter()
        .any(|permanent| permanent.card.definition == definition)
}

/// Cast, it exiles something and keeps it until the enchantment goes.
#[test]
fn it_holds_the_thing_it_took() {
    let (mut game, touch, ids) = staged(&[(cards::SERRA_ANGEL, PlayerId::Two)]);
    let angel = ids[0];

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == touch))
        .expect("three mana buys it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game, Some(angel));

    assert!(!on_battlefield(&game, cards::SERRA_ANGEL), "it is exiled");

    let enchantment = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::TOUCH_THE_SPIRIT_REALM)
        .expect("the enchantment resolved")
        .card
        .id;
    game.move_permanents_to_graveyard(&[enchantment]);
    settle(&mut game, None);

    assert!(
        on_battlefield(&game, cards::SERRA_ANGEL),
        "and comes back when the enchantment leaves",
    );
}

/// Channelled from hand, it blinks the thing back at the end of the turn.
#[test]
fn channelling_it_blinks_something() {
    let (mut game, touch, ids) = staged(&[(cards::GRIZZLY_BEARS, PlayerId::One)]);
    let bears = ids[0];

    let channel = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == touch))
        .expect("two mana and the card itself buys the channel");
    game.apply(PlayerId::One, channel).expect("it activates");
    settle(&mut game, Some(bears));

    assert!(!on_battlefield(&game, cards::GRIZZLY_BEARS), "exiled first");
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::TOUCH_THE_SPIRIT_REALM),
        "the card itself was discarded to pay",
    );

    game.step = Step::End;
    game.begin_step_triggers();
    settle(&mut game, None);

    assert!(
        on_battlefield(&game, cards::GRIZZLY_BEARS),
        "and back at the beginning of the next end step",
    );
}

/// The channel ability is a card in hand's, not a permanent's.
#[test]
fn the_channel_is_offered_only_from_hand() {
    let (mut game, touch, _) = staged(&[(cards::GRIZZLY_BEARS, PlayerId::One)]);
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == touch))
        .expect("three mana buys it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game, None);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::White, 3);

    let enchantment = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::TOUCH_THE_SPIRIT_REALM)
        .expect("it resolved")
        .card
        .id;
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == enchantment)),
        "the enchantment on the battlefield channels nothing",
    );
}

/// "If Touch the Spirit Realm leaves the battlefield before its first
/// ability resolves, the target permanent won't be exiled." The exile and
/// the return are one ability, so an enchantment answered underneath its own
/// trigger takes nothing with it (CR 610.3b).
#[test]
fn an_enchantment_answered_in_response_exiles_nothing() {
    let (mut game, touch, ids) = staged(&[(cards::SERRA_ANGEL, PlayerId::Two)]);
    let angel = ids[0];

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == touch))
        .expect("three mana buys it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    // Let the enchantment itself resolve and its trigger go on the stack,
    // naming the Angel, then answer the enchantment underneath it.
    for _ in 0..8 {
        let enchanted = game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TOUCH_THE_SPIRIT_REALM);
        if enchanted && !game.stack.is_empty() {
            break;
        }
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .filter(|option| option.card.is_some_and(|(object, _)| object == angel))
                .map(|option| option.id)
                .take(1)
                .collect::<Vec<_>>();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the trigger names the Angel");
            continue;
        }
        let player = game.priority;
        game.apply(player, Action::PassPriority)
            .expect("the trigger goes on the stack");
    }
    let enchantment = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::TOUCH_THE_SPIRIT_REALM)
        .expect("the enchantment resolved")
        .card
        .id;
    game.move_permanents_to_graveyard(&[enchantment]);
    settle(&mut game, None);

    assert!(
        on_battlefield(&game, cards::SERRA_ANGEL),
        "the Angel was never exiled",
    );
    assert!(
        game.players[PlayerId::Two.index()].exile.is_empty(),
        "so nothing waits on an enchantment that is gone",
    );
}

/// "If a token is exiled this way, it will cease to exist and won't return
/// to the battlefield." The blink is a one-way door for anything that was
/// never a card.
#[test]
fn a_token_it_blinks_never_comes_back() {
    let (mut game, touch, _) = staged(&[]);
    let token = token_permanent(
        107_500,
        tokens::creature(&["Goblin"], &[ManaColor::Red], 1, 1),
        PlayerId::One,
    );
    let token_id = token.card.id;
    game.battlefield.push(token);

    let channel = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == touch))
        .expect("a token is an artifact-or-creature it may name");
    game.apply(PlayerId::One, channel).expect("it activates");
    settle(&mut game, Some(token_id));

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != token_id),
        "the token was exiled",
    );

    game.step = Step::End;
    game.begin_step_triggers();
    settle(&mut game, None);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != token_id),
        "and a token that left the battlefield has ceased to exist",
    );
    assert!(
        game.battlefield.is_empty(),
        "nothing came back in its place either",
    );
}

/// "If a channel ability requires a target, you may not activate it without
/// a target just to discard the card." An empty board offers no channel at
/// all, however much mana is up.
#[test]
fn an_empty_board_offers_no_channel() {
    let (game, touch, _) = staged(&[]);

    assert!(
        game.legal_actions(PlayerId::One).into_iter().all(
            |action| !matches!(action, Action::ActivateAbility { source, .. } if source == touch)
        ),
        "there is nothing to name, so there is nothing to activate",
    );
}

/// "Any counters on the exiled permanent will cease to exist. When the card
/// returns to the battlefield, it will be a new object." What comes back is
/// the printed creature rather than the one that left.
#[test]
fn counters_do_not_come_back_with_it() {
    let (mut game, touch, ids) = staged(&[(cards::GRIZZLY_BEARS, PlayerId::One)]);
    let bears = ids[0];
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == bears)
        .expect("it is there")
        .set_counters(CounterKind::PlusOnePlusOne, 3);

    let channel = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == touch))
        .expect("the channel is offered");
    game.apply(PlayerId::One, channel).expect("it activates");
    settle(&mut game, Some(bears));

    game.step = Step::End;
    game.begin_step_triggers();
    settle(&mut game, None);

    let returned = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("it came back");
    assert_eq!(
        returned.counters(CounterKind::PlusOnePlusOne),
        0,
        "the counters did not come back with it",
    );
    assert_eq!(
        (game.power(returned), game.toughness(returned)),
        (Some(2), Some(2)),
        "so what returned is a plain 2/2",
    );
}
