//! Spell Pierce: a one-mana tax that answers everything except a creature.
//!
//! The soft-counter trio is compared side by side in `vintage_cube_spells`;
//! what lives here is the Pierce on its own, where those comparisons have
//! nothing to say.

use super::*;

/// A controller with nothing to pay with has nothing to decide: the Pierce
/// counters the spell without a real choice being offered.
#[test]
fn spell_pierce_counters_a_controller_who_cannot_pay() {
    let mut game = ready_game();
    game.battlefield.clear();
    let ponder = card(50_840, cards::PONDER, PlayerId::Two);
    let ponder_id = ponder.id;
    game.players[PlayerId::Two.index()].hand.push(ponder);
    game.players[PlayerId::Two.index()].mana_pool.blue = 1;
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    game.apply(
        PlayerId::Two,
        cast_action(ponder_id, Vec::new(), Vec::new(), 0),
    )
    .expect("they cast their sorcery");
    let on_stack = game.stack.last().expect("it is on the stack").id;
    game.apply(PlayerId::Two, Action::PassPriority).unwrap();

    let pierce = card(50_841, cards::SPELL_PIERCE, PlayerId::One);
    let pierce_id = pierce.id;
    game.players[PlayerId::One.index()].hand.push(pierce);
    game.players[PlayerId::One.index()].mana_pool.blue = 1;
    game.apply(
        PlayerId::One,
        cast_action(pierce_id, vec![Target::Spell(on_stack)], Vec::new(), 0),
    )
    .expect("the Pierce answers it");
    pass_priority_pair(&mut game);

    // With an empty pool there is nothing to pay the two with, so whatever
    // is asked, only one answer is available.
    for _ in 0..4 {
        let Some(decision) = game.observe(PlayerId::Two).decision else {
            break;
        };
        assert!(
            decision
                .options
                .iter()
                .all(|option| option.label != "Pay the cost"),
            "an empty pool cannot be offered the tax",
        );
        game.apply(
            PlayerId::Two,
            Action::ChooseDecision {
                decision: decision.id,
                options: vec![decision.options[0].id],
            },
        )
        .expect("the one answer is legal");
    }
    pass_priority_pair(&mut game);

    assert!(
        game.players[PlayerId::Two.index()]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::PONDER),
        "the sorcery was countered for want of two mana",
    );
}

/// "Target noncreature spell" does not say whose: your own Ponder is as
/// legal a target as theirs, which is how a Pierce answers a Force of Will
/// pitched at your own spell.
#[test]
fn spell_pierce_may_name_your_own_spell() {
    let mut game = ready_game();
    game.battlefield.clear();
    let ponder = card(50_850, cards::PONDER, PlayerId::One);
    let ponder_id = ponder.id;
    game.players[PlayerId::One.index()].hand.push(ponder);
    game.players[PlayerId::One.index()].mana_pool.blue = 2;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        cast_action(ponder_id, Vec::new(), Vec::new(), 0),
    )
    .expect("you cast your own sorcery");
    let on_stack = game.stack.last().expect("it is on the stack").id;

    let pierce = card(50_851, cards::SPELL_PIERCE, PlayerId::One);
    let pierce_id = pierce.id;
    game.players[PlayerId::One.index()].hand.push(pierce);

    assert!(
        game.legal_actions(PlayerId::One).into_iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if card == pierce_id
                    && choices
                        .targets()
                        .iter()
                        .any(|selection| selection.targets().contains(&Target::Spell(on_stack))))
        }),
        "your own noncreature spell is a legal target for it",
    );
}

/// The tax is all of {2} or none of it: a controller holding one mana is no
/// better off than one holding none, and pays nothing on the way to the
/// graveyard. The pair either side of the boundary is what pins it -- two
/// mana is offered the choice, one is not.
#[test]
fn one_mana_short_is_no_payment_at_all() {
    for (held, offered) in [(1, false), (2, true)] {
        let mut game = ready_game();
        game.battlefield.clear();
        let bolt = card(50_860, cards::LIGHTNING_BOLT, PlayerId::Two);
        let bolt_id = bolt.id;
        game.players[PlayerId::Two.index()].hand.push(bolt);
        game.players[PlayerId::Two.index()].mana_pool.red = 1;
        game.players[PlayerId::One.index()].life = 20;
        game.priority = PlayerId::Two;
        game.apply(
            PlayerId::Two,
            cast_action(bolt_id, vec![Target::Player(PlayerId::One)], Vec::new(), 0),
        )
        .expect("they cast something to answer");
        let on_stack = game.stack.last().expect("it is on the stack").id;
        game.apply(PlayerId::Two, Action::PassPriority).unwrap();

        let pierce = card(50_861, cards::SPELL_PIERCE, PlayerId::One);
        let pierce_id = pierce.id;
        game.players[PlayerId::One.index()].hand.push(pierce);
        game.players[PlayerId::One.index()].mana_pool.blue = 1;
        game.apply(
            PlayerId::One,
            cast_action(pierce_id, vec![Target::Spell(on_stack)], Vec::new(), 0),
        )
        .expect("the Pierce answers it");
        // What is left in the pool after the Bolt was paid for.
        game.players[PlayerId::Two.index()].mana_pool.colorless = held;
        pass_priority_pair(&mut game);

        // One mana short there is nothing to decide, so the engine asks
        // nothing at all rather than offering a tax that cannot be paid.
        let decision = game.observe(PlayerId::Two).decision;
        assert_eq!(
            decision.as_ref().is_some_and(|decision| decision
                .options
                .iter()
                .any(|option| option.label == "Pay the cost")),
            offered,
            "holding {held} of the two",
        );
        if offered {
            continue;
        }
        for _ in 0..4 {
            let Some(decision) = game.observe(PlayerId::Two).decision else {
                break;
            };
            let only = decision.options[0].id;
            game.apply(
                PlayerId::Two,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![only],
                },
            )
            .expect("the one answer is legal");
        }
        assert_eq!(
            game.players[PlayerId::Two.index()].mana_pool.colorless,
            held,
            "the mana it could not spend was not taken from it",
        );
        pass_priority_pair(&mut game);
        drain_pending(&mut game);

        assert_eq!(
            game.players[PlayerId::One.index()].life,
            20,
            "the Bolt was countered for want of one more mana",
        );
    }
}
