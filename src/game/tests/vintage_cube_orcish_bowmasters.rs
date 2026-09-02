//! Orcish Bowmasters: an arrow for every draw an opponent did not have
//! coming, and an Army that grows by one each time.

use super::*;

/// Answers every pending decision, pointing anything that must be pointed at
/// the opponent, then resolves whatever is left on the stack.
fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            // Two triggers waiting at once are ordered rather than picked
            // between, and that decision wants every option.
            let options = if decision.minimum > 1 {
                decision.options.iter().map(|option| option.id).collect()
            } else {
                decision
                    .options
                    .iter()
                    .find(|option| option.label == "your opponent")
                    .or_else(|| decision.options.first())
                    .map(|option| vec![option.id])
                    .unwrap_or_default()
            };
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
}

/// Player One's Bowmasters on an otherwise empty battlefield, with its own
/// entry trigger already resolved.
fn staged() -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.put_onto_battlefield(PlayerId::One, cards::ORCISH_BOWMASTERS)
        .expect("cataloged");
    settle(&mut game);
    game
}

fn army(game: &Game) -> Option<&Permanent> {
    game.battlefield.iter().find(|permanent| {
        is_token_with(
            permanent,
            tokens::creature(&["Orc", "Army"], &[ManaColor::Black], 0, 0),
        )
    })
}

fn army_counters(game: &Game) -> u16 {
    army(game).map_or(0, |permanent| {
        permanent.counters(CounterKind::PlusOnePlusOne)
    })
}

/// The entry itself is one of the two ways the ability fires, and the amass
/// that follows it has no Army to find, so it makes one.
#[test]
fn entering_shoots_and_amasses() {
    let game = staged();

    assert_eq!(game.players[1].life, 19, "one arrow at the opponent");
    let army = army(&game).expect("amass made an Army");
    assert_eq!(army.counters(CounterKind::PlusOnePlusOne), 1);
    assert_eq!(game.power(army), Some(1), "a 0/0 with one counter");
}

/// Every later draw is another arrow, and the counters land on the Army that
/// is already there rather than making a second one.
#[test]
fn each_extra_draw_grows_the_same_army() {
    let mut game = staged();

    game.draw_cards(PlayerId::Two, 2);
    settle(&mut game);

    assert_eq!(game.players[1].life, 17, "two more arrows");
    assert_eq!(army_counters(&game), 3);
    assert_eq!(
        game.battlefield
            .iter()
            .filter(|permanent| is_token_with(
                permanent,
                tokens::creature(&["Orc", "Army"], &[ManaColor::Black], 0, 0)
            ))
            .count(),
        1,
        "amass grows an Army it already controls",
    );
}

/// The drawn card's characteristics are available while matching the event,
/// but an ordinary draw does not publish that hidden card into the trigger's
/// resolving context or checkpoint.
#[test]
fn an_unrevealed_draw_does_not_bind_the_drawn_card_to_the_trigger() {
    let mut game = staged();

    let drawn = game
        .draw_card(PlayerId::Two)
        .expect("the opponent has a card to draw");
    let [trigger] = game.pending_triggers.as_slice() else {
        panic!("the extra draw created one Bowmasters trigger");
    };
    assert_eq!(trigger.context.trigger.object, None);
    assert_eq!(trigger.context.trigger.object_controller, None);
    assert_eq!(trigger.context.trigger.event_player, Some(PlayerId::Two));

    let checkpoint = game.checkpoint_json(PlayerId::One);
    assert_eq!(
        checkpoint["pendingTriggers"][0]["context"]["trigger"]["object"],
        serde_json::Value::Null,
    );
    assert!(game.events_for(PlayerId::One).iter().all(|event| {
        !matches!(
            event,
            GameEvent::CardRevealed { card, .. } if *card == drawn
        )
    }));
}

/// Your own draws are not an opponent's, however many you take.
#[test]
fn your_own_draws_are_not_shot_at() {
    let mut game = staged();

    game.draw_cards(PlayerId::One, 3);
    settle(&mut game);

    assert_eq!(game.players[1].life, 19, "no further arrows");
    assert_eq!(army_counters(&game), 1);
}

/// The card an opponent is handed in their own draw step is spared, and the
/// next one in that same step is not.
#[test]
fn the_first_draw_of_their_draw_step_is_spared() {
    let mut game = staged();
    game.active_player = PlayerId::Two;
    game.step = Step::Draw;
    game.draw_step_draw_taken = [false; 2];

    game.draw_cards(PlayerId::Two, 1);
    settle(&mut game);
    assert_eq!(game.players[1].life, 19, "the turn-based draw is spared");
    assert_eq!(army_counters(&game), 1);

    game.draw_cards(PlayerId::Two, 1);
    settle(&mut game);
    assert_eq!(game.players[1].life, 18, "the second one is not");
    assert_eq!(army_counters(&game), 2);
}

/// The exemption belongs to the drawing player's own draw step. A draw taken
/// during your draw step is an ordinary draw for them.
#[test]
fn their_draw_during_your_draw_step_is_not_spared() {
    let mut game = staged();
    game.active_player = PlayerId::One;
    game.step = Step::Draw;
    game.draw_step_draw_taken = [false; 2];

    game.draw_cards(PlayerId::Two, 1);
    settle(&mut game);

    assert_eq!(game.players[1].life, 18);
    assert_eq!(army_counters(&game), 2);
}

/// "If a spell or ability causes an opponent to put cards into their hand
/// without specifically using the word draw, it's not a card drawn." Their
/// bear bounced back to hand is a card in hand and no arrow at all.
#[test]
fn a_card_returned_to_their_hand_is_not_a_draw() {
    let mut game = staged();
    let theirs = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    let counters = army_counters(&game);
    let life = game.players[PlayerId::Two.index()].life;
    let held = game.players[PlayerId::Two.index()].hand.len();

    let theft = game
        .build_zone(PlayerId::One, &[cards::BRAZEN_BORROWER])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let theft_id = theft.id;
    game.players[PlayerId::One.index()].hand.push(theft);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.priority = PlayerId::One;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == theft_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(theirs))
            }
            _ => false,
        })
        .expect("Petty Theft bounces a nonland permanent of theirs");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].hand.len(),
        held + 1,
        "the bear is in their hand",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        life,
        "and nobody was shot for putting it there",
    );
    assert_eq!(
        army_counters(&game),
        counters,
        "so the Army is the size it was",
    );
}

/// Flash is what makes the entry trigger an answer: cast on their turn with
/// their own draw spell on the stack, the Orc arrives first and the arrow
/// lands before anything of theirs resolves.
#[test]
fn flash_lets_it_arrive_on_their_turn() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    let bowmasters = card(101_000, cards::ORCISH_BOWMASTERS, PlayerId::One);
    let bowmasters_id = bowmasters.id;
    game.players[PlayerId::One.index()].hand.push(bowmasters);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    let life = game.players[PlayerId::Two.index()].life;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == bowmasters_id))
        .expect("flash casts it on their turn");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        life - 1,
        "the entry trigger shot them on their own turn",
    );
    assert_eq!(
        army_counters(&game),
        1,
        "and amassed an Orc while it was at it"
    );
}

/// "Some spells and abilities that amass Orcs may require targets. If each
/// target chosen is an illegal target as that spell or ability tries to
/// resolve, it won't resolve. You won't amass Orcs." The arrow and the Army
/// are one trigger, so answering what it points at costs them both.
#[test]
fn an_answered_target_costs_the_amass_as_well_as_the_arrow() {
    let mut game = staged();
    let bear = game
        .put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    settle(&mut game);
    let counters = army_counters(&game);
    let life = game.players[PlayerId::Two.index()].life;
    assert_eq!(counters, 1, "the Army the entry trigger made");

    // A draw of theirs puts the arrow on the stack; point it at the bear
    // rather than at them.
    game.draw_cards(PlayerId::Two, 1);
    // The trigger waits off the stack until priority moves; its targets are
    // chosen as it goes on, which is the moment to name the bear.
    for _ in 0..12 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let aimed = decision
                .options
                .iter()
                .find(|option| option.card.is_some_and(|(id, _)| id == bear))
                .map(|option| vec![option.id])
                .expect("the bear is one of the offered targets");
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: aimed,
                },
            )
            .expect("naming it is legal");
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    assert!(
        !game.stack.is_empty(),
        "the trigger is waiting with the bear named",
    );

    // Answer the bear before the trigger resolves.
    game.move_permanents_to_graveyard(&[bear]);
    settle(&mut game);
    game.check_state_based_actions();

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        life,
        "no arrow landed: its only target was gone",
    );
    assert_eq!(
        army_counters(&game),
        counters,
        "and the amass went with it, being the same trigger",
    );
}
