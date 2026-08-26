//! Lutri, the Spellchaser: a flash body that copies the spell you were
//! already casting, and only when it was cast itself.

use super::*;

/// Player One holding Lutri with mana for it, and a Lightning Bolt in hand
/// for something to copy.
fn staged() -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[1].hand.clear();
    let mut ids = Vec::new();
    for definition in [cards::LUTRI_THE_SPELLCHASER, cards::LIGHTNING_BOLT] {
        let card = game
            .build_zone(PlayerId::One, &[definition])
            .expect("cataloged")
            .into_iter()
            .next()
            .expect("one card");
        ids.push(card.id);
        game.players[0].hand.push(card);
    }
    game.put_onto_battlefield(PlayerId::Two, cards::GRIZZLY_BEARS)
        .expect("cataloged");
    drain_pending(&mut game);
    game.turns_started = [4, 4];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.players[1].life = 20;
    for color in [ManaColor::Red, ManaColor::Blue] {
        game.add_unrestricted_mana(PlayerId::One, color, 4);
    }
    (game, ids[0], ids[1])
}

fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            // The copy is offered its own targets; "Keep original targets"
            // is the answer that leaves the copy pointed where the spell it
            // copied was pointed.
            let options = decision
                .options
                .iter()
                .find(|option| option.label == "Keep original targets")
                .map_or_else(
                    || {
                        decision
                            .options
                            .iter()
                            .map(|option| option.id)
                            .take(decision.minimum.max(1))
                            .collect()
                    },
                    |option| vec![option.id],
                );
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
    game.check_state_based_actions();
}

/// Casts the Bolt at the other player and leaves it on the stack.
fn bolt_the_player(game: &mut Game, bolt: GameObjectId) -> GameObjectId {
    game.apply(
        PlayerId::One,
        cast_action(bolt, vec![Target::Player(PlayerId::Two)], Vec::new(), 0),
    )
    .expect("the Bolt is castable");
    game.stack.last().expect("it is on the stack").id
}

/// Casts Lutri. Its copy names its target when the enters trigger goes on
/// the stack rather than as the Otter is cast, so nothing is chosen here.
fn cast_lutri(game: &mut Game, lutri: GameObjectId) {
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == lutri))
        .expect("Lutri is castable");
    game.apply(PlayerId::One, action).expect("it is cast");
}

/// The whole card: a Bolt on the stack becomes two.
#[test]
fn it_copies_the_spell_you_were_casting() {
    let (mut game, lutri, bolt) = staged();
    bolt_the_player(&mut game, bolt);

    cast_lutri(&mut game, lutri);
    settle(&mut game);

    assert_eq!(game.players[1].life, 14, "six damage from two Bolts");
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::LUTRI_THE_SPELLCHASER),
        "and the Otter stayed",
    );
}

/// It has flash, which is what lets it answer a spell at all.
#[test]
fn it_can_be_cast_on_their_turn() {
    let (mut game, lutri, _bolt) = staged();
    game.active_player = PlayerId::Two;
    game.step = Step::Upkeep;
    game.priority = PlayerId::One;

    assert!(
        game.legal_actions(PlayerId::One)
            .iter()
            .any(|action| matches!(action, Action::CastSpell { card, .. } if *card == lutri)),
        "flash means their upkeep is a fine time",
    );
}

/// "Target instant or sorcery spell you control": their Bolt is not a legal
/// target, so with nothing of yours on the stack there is nothing to copy.
#[test]
fn it_will_not_copy_their_spell() {
    let (mut game, lutri, _bolt) = staged();
    let theirs = game
        .build_zone(PlayerId::Two, &[cards::LIGHTNING_BOLT])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let theirs_id = theirs.id;
    game.players[1].hand.push(theirs);
    game.players[1].mana_pool.red = 1;
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(
            theirs_id,
            vec![Target::Player(PlayerId::One)],
            Vec::new(),
            0,
        ),
    )
    .expect("their Bolt is castable");
    game.priority = PlayerId::One;

    cast_lutri(&mut game, lutri);
    settle(&mut game);

    assert_eq!(
        game.players[0].life, 17,
        "their Bolt resolved once: the clause says a spell you control",
    );
}

/// "You may choose new targets for the copy": the copy can be pointed
/// somewhere the original was not.
#[test]
fn the_copy_may_be_retargeted() {
    let (mut game, lutri, bolt) = staged();
    bolt_the_player(&mut game, bolt);
    let bears = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::GRIZZLY_BEARS)
        .expect("their Bear is out")
        .card
        .id;

    cast_lutri(&mut game, lutri);
    for _ in 0..24 {
        let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        else {
            if game.stack.is_empty() && game.pending_triggers.is_empty() {
                break;
            }
            let priority = game.priority;
            if game.apply(priority, Action::PassPriority).is_err() {
                break;
            }
            continue;
        };
        let options = decision
            .options
            .iter()
            .find(|option| option.label == "Copy with targets Grizzly Bears")
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
        .expect("the offered choice is legal");
    }
    game.check_state_based_actions();

    assert_eq!(game.players[1].life, 17, "the original still hit them");
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == bears),
        "and the copy killed the Bear instead",
    );
}

/// "If you cast it": a Lutri that arrives some other way is a 3/2 and
/// nothing more, even with a spell of yours on the stack.
#[test]
fn put_onto_the_battlefield_it_copies_nothing() {
    let (mut game, _lutri, bolt) = staged();
    bolt_the_player(&mut game, bolt);

    game.put_onto_battlefield(PlayerId::One, cards::LUTRI_THE_SPELLCHASER)
        .expect("cataloged");
    settle(&mut game);

    assert_eq!(
        game.players[1].life, 17,
        "one Bolt's worth, so nothing was copied",
    );
}
