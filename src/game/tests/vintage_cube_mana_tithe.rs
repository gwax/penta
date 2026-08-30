//! Mana Tithe: the white Force Spike, and what it costs the player who
//! cannot pay it.

use super::*;

/// Player Two casting a Bolt at Player One with `spare` colourless left
/// over, answered by a Mana Tithe. Returns the game with the Tithe on the
/// stack and the Bolt's controller holding priority.
fn tithed(spare: u16) -> Game {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].life = 20;
    let bolt = card(50_900, cards::LIGHTNING_BOLT, PlayerId::Two);
    let bolt_id = bolt.id;
    game.players[PlayerId::Two.index()].hand.push(bolt);
    game.players[PlayerId::Two.index()].mana_pool.red = 1;
    game.players[PlayerId::Two.index()].mana_pool.colorless = spare;
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(bolt_id, vec![Target::Player(PlayerId::One)], Vec::new(), 0),
    )
    .expect("the Bolt is cast");
    let on_stack = game.stack.last().expect("it is on the stack").id;
    game.apply(PlayerId::Two, Action::PassPriority)
        .expect("priority passes");

    let tithe = card(50_901, cards::MANA_TITHE, PlayerId::One);
    let tithe_id = tithe.id;
    game.players[PlayerId::One.index()].hand.push(tithe);
    // Colourless of your own, to show whose mana the tax is not paid with.
    let pool = &mut game.players[PlayerId::One.index()].mana_pool;
    pool.white += 1;
    pool.colorless += 5;
    game.apply(
        PlayerId::One,
        cast_action(tithe_id, vec![Target::Spell(on_stack)], Vec::new(), 0),
    )
    .expect("the Tithe answers it");
    pass_priority_pair(&mut game);
    game
}

/// Answers whatever is asked by taking the first option offered.
fn settle(game: &mut Game) {
    for _ in 0..8 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision.options.first().map(|option| option.id);
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: options.map(|id| vec![id]).unwrap_or_default(),
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

/// "Unless its controller pays {1}": their mana, not yours. Five colourless
/// on the Tithe's own side buys the Bolt nothing, and a player who spent
/// everything on the spell has no choice left to make.
#[test]
fn the_tax_is_paid_by_the_spell_you_named() {
    let mut game = tithed(0);

    if let Some(decision) = game.observe(PlayerId::Two).decision {
        assert!(
            decision
                .options
                .iter()
                .all(|option| option.label != "Pay the cost"),
            "there is nothing to pay it with",
        );
    }
    settle(&mut game);

    assert!(
        game.players[PlayerId::Two.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::LIGHTNING_BOLT),
        "the Bolt was countered",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].life,
        20,
        "and never dealt its damage",
    );
    assert_eq!(
        game.players[PlayerId::One.index()].mana_pool.colorless,
        5,
        "the Tithe's controller was never asked for anything",
    );
}

/// One spare colourless is exactly the tax, and paying it is the whole
/// difference: the Bolt resolves and the mana is gone.
#[test]
fn one_spare_mana_is_all_it_takes() {
    let mut game = tithed(1);

    let decision = game
        .observe(PlayerId::Two)
        .decision
        .expect("the Tithe asks for its one mana");
    let pay = decision
        .options
        .iter()
        .find(|option| option.label == "Pay the cost")
        .expect("with the mana up, paying is on offer")
        .id;
    game.apply(
        PlayerId::Two,
        Action::ChooseDecision {
            decision: decision.id,
            options: vec![pay],
        },
    )
    .expect("paying is allowed");
    settle(&mut game);

    assert_eq!(
        game.players[PlayerId::One.index()].life,
        17,
        "the Bolt resolved",
    );
    assert_eq!(
        game.players[PlayerId::Two.index()].mana_pool.colorless,
        0,
        "and the tax came out of their pool",
    );
}
