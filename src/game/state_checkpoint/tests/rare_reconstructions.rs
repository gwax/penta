//! Reconstruction of objects and continuations caught mid-flight.
//!
//! Its sibling [`super::rare_states`] builds the payments and replacement
//! choices that have to fail closed; these are the states an ordinary game
//! passes through and a snapshot has to be able to express -- restricted
//! mana still unspent, a spell cast from a graveyard, a run of triggers
//! waiting to be ordered.

use super::super::*;
use super::rare_states::{
    answer_with_first_option, assert_reconstructs, cast_targeting, fill_mana, resolve_top_of_stack,
    staged_game, staged_modern_game,
};
use crate::card::cards;
use crate::game::DecisionContinuation;
use crate::game::tests::{card, creature, mana_ability_for, ready_game};
use crate::{Action, ManaColor};

/// Mishra's Workshop pays for artifacts and nothing else. Unspent restricted
/// mana is the case where the public pool and the engine's units disagree in
/// meaning while agreeing in count, so the units have to travel.
#[test]
fn unspent_restricted_mana_reconstructs() {
    let mut game = staged_game();
    let workshop_id = GameObjectId(10_000);
    game.battlefield.push(creature(
        workshop_id.0,
        crate::card::cards::MISHRA_S_WORKSHOP,
        PlayerId::One,
    ));

    let ability = mana_ability_for(&game, workshop_id, ManaColor::Colorless);
    game.apply(
        PlayerId::One,
        Action::ActivateManaAbility {
            source: workshop_id,
            ability,
            color: ManaColor::Colorless,
            counters_removed: None,
            cost_object: None,
            combination: None,
        },
    )
    .expect("the Workshop taps for mana");

    let restricted = game.players[PlayerId::One.index()]
        .mana
        .iter()
        .filter(|mana| !mana.restrictions.is_empty())
        .count();
    assert_eq!(restricted, 3, "the Workshop makes three restricted mana");
    assert_reconstructs(&game, "an unspent pool of restricted mana");
}

/// A flashback spell is on the stack with an alternative cost already paid and
/// a graveyard exile owed to it after it resolves. The stack object therefore
/// carries state its printed card does not.
#[test]
fn a_spell_cast_from_a_graveyard_reconstructs_while_it_is_on_the_stack() {
    let mut game = staged_modern_game();
    let spell = card(20_000, crate::card::cards::THINK_TWICE, PlayerId::One);
    let spell_id = spell.id;
    game.players[PlayerId::One.index()].graveyard.push(spell);
    fill_mana(&mut game, PlayerId::One, 4);

    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                Action::CastSpell { card, choices, .. }
                    if *card == spell_id && choices.costs().alternative().is_some()
            )
        })
        .expect("Think Twice can be flashed back from the graveyard");
    game.apply(PlayerId::One, action)
        .expect("the flashback cast is legal");

    assert!(
        game.stack
            .last()
            .is_some_and(|object| object.cast_via_flashback),
        "the spell must be marked as cast via flashback"
    );
    assert_reconstructs(&game, "a flashback spell on the stack");
}

/// Fork puts a copy of a spell on the stack and repaints it red. The copy is
/// backed by no card in any zone and its color no longer matches its printed
/// face, so both the copy flag and the override have to survive.
#[test]
fn a_copied_and_recolored_spell_reconstructs_on_the_stack() {
    let mut game = staged_game();
    let bolt = card(20_000, crate::card::cards::LIGHTNING_BOLT, PlayerId::One);
    let bolt_id = bolt.id;
    game.players[PlayerId::One.index()].hand.push(bolt);
    let fork = card(20_001, crate::card::cards::FORK, PlayerId::One);
    let fork_id = fork.id;
    game.players[PlayerId::One.index()].hand.push(fork);
    fill_mana(&mut game, PlayerId::One, 4);

    cast_targeting(
        &mut game,
        PlayerId::One,
        bolt_id,
        Target::Player(PlayerId::Two),
    );
    let spell_on_stack = game.stack.last().expect("the Bolt is on the stack").id;
    cast_targeting(
        &mut game,
        PlayerId::One,
        fork_id,
        Target::Spell(spell_on_stack),
    );
    resolve_top_of_stack(&mut game);
    while game
        .pending_decisions
        .first()
        .is_some_and(|pending| matches!(pending.continuation, DecisionContinuation::Fork { .. }))
    {
        answer_with_first_option(&mut game);
    }

    assert!(
        game.stack.iter().any(|object| object.is_copy),
        "Fork must have left a copy on the stack"
    );
    assert!(
        game.stack
            .iter()
            .any(|object| object.is_copy && object.colors.is_some()),
        "the copy must carry Fork's color override"
    );
    assert_reconstructs(&game, "a copied and recolored spell");
}

/// Triggers that have fired but not yet been placed live in the game itself,
/// with the context each captured when it fired. The broad audit never sees
/// this, because placement normally follows capture without a boundary in
/// between, so the `pendingTriggers` half of the snapshot is only exercised
/// from a position built on purpose.
#[test]
fn triggers_that_have_fired_but_not_yet_been_placed_reconstruct() {
    let mut game = staged_game();
    for id in 10_000..10_002 {
        let mut vault = creature(id, crate::card::cards::MANA_VAULT, PlayerId::One);
        vault.tapped = true;
        game.battlefield.push(vault);
    }
    game.step = crate::Step::Upkeep;
    game.handle_upkeep_triggers();

    assert_eq!(
        game.pending_triggers.len(),
        2,
        "both vaults must have captured a trigger"
    );
    assert_reconstructs(&game, "triggers captured but not yet placed");
}

/// Simultaneous triggers wait, unordered, in a decision that owns them. They
/// are on no stack and belong to no object, and each carries the context it
/// captured when it fired, so the ordering decision is where a snapshot has
/// the most to lose.
#[test]
fn simultaneous_triggers_waiting_to_be_ordered_reconstruct() {
    let mut game = staged_game();
    for id in 10_000..10_002 {
        let mut vault = creature(id, crate::card::cards::MANA_VAULT, PlayerId::One);
        vault.tapped = true;
        game.battlefield.push(vault);
    }
    game.step = crate::Step::Upkeep;
    game.handle_upkeep_triggers();
    game.finish_rules_procedure();

    assert!(
        matches!(
            game.pending_decisions
                .first()
                .map(|pending| &pending.continuation),
            Some(DecisionContinuation::TriggerOrder { .. })
        ),
        "two upkeep triggers must ask for an order, not {:?}",
        game.pending_decisions
            .first()
            .map(|pending| &pending.continuation)
    );
    assert_reconstructs(&game, "simultaneous triggers awaiting an order");
}

/// A text change rewrites a printed word for the rest of the game. It is
/// neither a characteristic nor an effect on a stack, and the permanent that
/// carries it has to come back reading the same way.
#[test]
fn an_indefinite_text_change_reconstructs_while_choosing_and_after() {
    let mut game = staged_game();
    let land_id = GameObjectId(12_000);
    game.battlefield.push(creature(
        land_id.0,
        crate::card::cards::PLATEAU,
        PlayerId::Two,
    ));
    let hack = card(11_000, crate::card::cards::MAGICAL_HACK, PlayerId::One);
    let hack_id = hack.id;
    game.players[PlayerId::One.index()].hand.push(hack);
    fill_mana(&mut game, PlayerId::One, 4);

    cast_targeting(
        &mut game,
        PlayerId::One,
        hack_id,
        Target::Permanent(land_id),
    );
    resolve_top_of_stack(&mut game);

    assert!(
        matches!(
            game.pending_decisions
                .first()
                .map(|pending| &pending.continuation),
            Some(DecisionContinuation::BasicLandTypeTextChange { .. })
        ),
        "Magical Hack must ask which word to rewrite, not {:?}",
        game.pending_decisions
            .first()
            .map(|pending| &pending.continuation)
    );
    assert_reconstructs(&game, "a text change choosing its words");

    answer_with_first_option(&mut game);
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| !permanent.text_changes.is_empty()),
        "the choice must leave a rewritten permanent behind"
    );
    assert_reconstructs(&game, "a permanent carrying an indefinite text change");
}

/// A phased-out permanent is public information, so it survives a checkpoint
/// the same way anything else on the board does -- and comes back phased
/// out rather than as an ordinary permanent.
#[test]
fn a_phased_out_permanent_reconstructs_as_phased_out() {
    let mut game = ready_game();
    game.battlefield.push(creature(
        10_000,
        crate::card::cards::BLACK_VISE,
        PlayerId::One,
    ));
    let vise = game.battlefield[0].card.id;
    game.phase_out(vise);
    assert!(game.battlefield.is_empty(), "it left the battlefield");

    let (_wire, rebuilt) = super::super::tests::rebuild_current_checkpoint(&game, PlayerId::One, 7);
    assert!(
        rebuilt.battlefield.is_empty(),
        "and the rebuilt game does not have it on the battlefield either",
    );
    assert_eq!(
        rebuilt
            .phased_out
            .iter()
            .map(|permanent| permanent.card.definition)
            .collect::<Vec<_>>(),
        vec![crate::card::cards::BLACK_VISE],
        "it came back waiting to phase in",
    );
}

/// A Dreadnought's entry cost is answered one creature at a time, and the
/// resolution that wants it is still in flight while it is asked -- so the
/// checkpoint has to carry that resolution along with how much is owed.
#[test]
fn a_run_of_sacrifices_reconstructs_mid_payment() {
    let mut game = ready_game();
    for index in 0..3 {
        game.battlefield.push(creature(
            10_010 + index,
            crate::card::cards::SERRA_ANGEL,
            PlayerId::One,
        ));
    }
    let dreadnought = card(
        10_000,
        crate::card::cards::PHYREXIAN_DREADNOUGHT,
        PlayerId::One,
    );
    let dreadnought_id = dreadnought.id;
    game.players[PlayerId::One.index()].hand.push(dreadnought);
    game.players[PlayerId::One.index()].mana_pool.colorless = 1;
    game.priority = PlayerId::One;
    game.apply(
        PlayerId::One,
        crate::game::tests::cast_action(dreadnought_id, Vec::new(), Vec::new(), 0),
    )
    .expect("one mana casts it");
    crate::game::tests::pass_until_decision(&mut game);

    let offer = game
        .observe(PlayerId::One)
        .decision
        .expect("the payer is asked whether to pay");
    let pay = offer
        .options
        .iter()
        .find(|option| option.id != 0)
        .expect("paying is on offer")
        .id;
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: offer.id,
            options: vec![pay],
        },
    )
    .expect("paying is legal");

    let (_wire, rebuilt) =
        super::super::tests::rebuild_current_checkpoint(&game, PlayerId::One, 11);
    let remaining = match rebuilt
        .pending_decisions
        .first()
        .map(|pending| &pending.continuation)
    {
        Some(DecisionContinuation::SacrificeToTotalPower { remaining, .. }) => *remaining,
        other => panic!("the run of sacrifices came back as {other:?}"),
    };
    assert_eq!(remaining, 12, "and it still owes the whole twelve");
}

/// A trigger that divides a fixed total asks twice: which targets, and then
/// how much each takes. A game saved between those two questions has to come
/// back asking the second one, with the same splits on offer.
#[test]
fn a_pending_trigger_division_reconstructs_and_resumes() {
    let mut game = staged_game();
    game.battlefield.clear();
    game.battlefield
        .push(creature(30_000, cards::SERRA_ANGEL, PlayerId::Two));
    game.battlefield
        .push(creature(30_001, cards::SERRA_ANGEL, PlayerId::Two));
    game.put_onto_battlefield(PlayerId::One, cards::FURY)
        .expect("Fury is cataloged");

    let targets = loop {
        if let Some(decision) = game.observe(PlayerId::One).decision {
            break decision;
        }
        let player = game.priority;
        game.apply(player, Action::PassPriority)
            .expect("the enters trigger is waiting on its targets");
    };
    let both = targets
        .options
        .iter()
        .filter(|option| option.label == "Serra Angel")
        .map(|option| option.id)
        .collect::<Vec<_>>();
    assert_eq!(both.len(), 2);
    game.apply(
        PlayerId::One,
        Action::ChooseDecision {
            decision: targets.id,
            options: both,
        },
    )
    .expect("naming two targets is legal");

    assert!(
        game.observe(PlayerId::One)
            .decision
            .is_some_and(|decision| decision.prompt.contains("divide")),
        "the division is what is pending",
    );
    assert_reconstructs(&game, "a pending trigger division");
}
