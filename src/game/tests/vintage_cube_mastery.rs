//! Baleful Mastery: two prices for one exile, and the rider on the cheap one.

use super::*;

fn staged() -> (Game, CardInstanceId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    let victim = creature(89_000, cards::SERRA_ANGEL, PlayerId::Two);
    let victim_id = victim.card.id;
    game.battlefield.push(victim);
    let mastery = card(89_001, cards::BALEFUL_MASTERY, PlayerId::One);
    let mastery_id = mastery.id;
    game.players[0].hand.push(mastery);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    (game, mastery_id, victim_id)
}

fn cast_mastery(game: &mut Game, mastery: CardInstanceId, discounted: bool) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == mastery && choices.costs().alternative().is_some() == discounted
            }
            _ => false,
        })
        .unwrap_or_else(|| panic!("a cast with discounted={discounted} is offered"));
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_priority_pair(game);
    drain_pending(game);
}

/// Cast for its printed cost it exiles and gives nothing back.
#[test]
fn the_full_price_exiles_and_pays_nothing() {
    let (mut game, mastery, victim) = staged();
    let their_hand = game.players[1].hand.len();

    cast_mastery(&mut game, mastery, false);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != victim),
        "the creature is gone",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "exiled rather than destroyed",
    );
    assert_eq!(
        game.players[1].hand.len(),
        their_hand,
        "and they drew nothing",
    );
}

/// Cast for the discount it still exiles, and the opponent draws.
#[test]
fn the_discount_exiles_and_hands_them_a_card() {
    let (mut game, mastery, victim) = staged();
    let their_hand = game.players[1].hand.len();

    cast_mastery(&mut game, mastery, true);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != victim),
        "the creature is gone either way",
    );
    assert_eq!(
        game.players[1].hand.len(),
        their_hand + 1,
        "and the discount cost them a card",
    );
    assert_eq!(
        game.players[0].mana_pool.total(),
        2,
        "two of the four mana are left over",
    );
}

/// It answers a planeswalker as readily as a creature.
#[test]
fn it_exiles_a_planeswalker_too() {
    let mut game = ready_game();
    game.battlefield.clear();
    let walker = game
        .put_onto_battlefield(PlayerId::Two, cards::JACE_MEMORY_ADEPT)
        .expect("cataloged");
    drain_pending(&mut game);
    let mastery = card(89_010, cards::BALEFUL_MASTERY, PlayerId::One);
    let mastery_id = mastery.id;
    game.players[0].hand.push(mastery);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);

    cast_mastery(&mut game, mastery_id, false);

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != walker),
        "the planeswalker is gone",
    );
}

/// "The mana value of a spell on the stack is determined by its mana cost,
/// not any alternative costs you used to pay for it." Cast for {1}{B} it is
/// still a four-mana spell, which a Spell Blast has to pay four for.
#[test]
fn the_alternative_cost_does_not_change_its_mana_value() {
    let (mut game, mastery, _victim) = staged();
    let blast = card(89_100, cards::SPELL_BLAST, PlayerId::Two);
    let blast_id = blast.id;
    game.players[1].hand.push(blast);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Colorless, 4);

    let cheap = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == mastery && choices.costs().alternative().is_some()
            }
            _ => false,
        })
        .expect("the discounted cast is offered");
    game.apply(PlayerId::One, cheap).expect("it is cast");
    let on_stack = game.stack.last().expect("it is on the stack").id;
    game.priority = PlayerId::Two;

    let blasts = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. }
                if card == blast_id
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Spell(on_stack)) =>
            {
                Some(choices.x())
            }
            _ => false.then_some(0),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        blasts,
        vec![4],
        "X must be four: the printed cost is what the stack reads",
    );
}

/// "Exile target creature": what it takes does not come back, and
/// indestructible is no answer to it.
#[test]
fn it_exiles_an_indestructible_creature() {
    let mut game = ready_game();
    game.battlefield.clear();
    let juggernaut = game
        .put_onto_battlefield(PlayerId::Two, cards::DARKSTEEL_JUGGERNAUT)
        .expect("cataloged");
    drain_pending(&mut game);
    let mastery = card(89_200, cards::BALEFUL_MASTERY, PlayerId::One);
    let mastery_id = mastery.id;
    game.players[0].hand.push(mastery);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    game.priority = PlayerId::One;

    cast_mastery(&mut game, mastery_id, false);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == juggernaut),
        "exile asks nothing about destruction",
    );
    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::DARKSTEEL_JUGGERNAUT),
        "and it is in exile, not a graveyard",
    );
}

/// "If an effect increases or decreases the cost of spells you cast, that
/// cost increase is applied to the alternative cost you chose to pay. In
/// that case, the cost was still paid for the purposes of the effect, even
/// if you paid more." A Thorn of Amethyst makes the cheap half {2}{B}, and
/// the opponent still draws for it.
#[test]
fn a_tax_raises_the_alternative_cost_without_undoing_it() {
    let (mut game, mastery, victim) = staged();
    game.put_onto_battlefield(PlayerId::One, cards::THORN_OF_AMETHYST)
        .expect("cataloged");
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    game.empty_mana_pools();
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Black, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    let held = game.players[1].hand.len();

    assert!(
        !game.legal_actions(PlayerId::One).iter().any(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == mastery && choices.costs().alternative().is_some())
        }),
        "two mana no longer pays the alternative cost",
    );

    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);
    cast_mastery(&mut game, mastery, true);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == victim),
        "the Angel is exiled all the same",
    );
    assert_eq!(
        game.players[1].hand.len(),
        held + 1,
        "and the cost counts as paid, so they draw for it",
    );
}

/// "If you copy a Mastery spell and the alternative cost was paid, the copy
/// will resolve as though the cost was paid." The Fork's copy exiles a
/// second creature and hands them a second card.
#[test]
fn a_copy_of_the_cheap_half_hands_them_another_card() {
    let (mut game, mastery, victim) = staged();
    let second = creature(89_100, cards::GRIZZLY_BEARS, PlayerId::Two);
    let second_id = second.card.id;
    game.battlefield.push(second);
    game.players[0]
        .hand
        .push(card(89_101, cards::FORK, PlayerId::One));
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 2);
    let held = game.players[1].hand.len();

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| match action {
            Action::CastSpell { card, choices, .. } => {
                *card == mastery
                    && choices.costs().alternative().is_some()
                    && choices
                        .iter_targets()
                        .any(|target| *target == Target::Permanent(victim))
            }
            _ => false,
        })
        .expect("the cheap half is offered at the Angel");
    game.apply(PlayerId::One, cast).expect("it is cast");

    let on_stack = game.stack.last().expect("it is waiting").id;
    let fork = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, choices, .. }
                if *card == CardInstanceId(89_101)
                    && choices.iter_targets().any(|target| *target == Target::Spell(on_stack)))
        })
        .expect("the Fork can name it");
    game.apply(PlayerId::One, fork).expect("it is cast");

    // The copy resolves first, so it is the one that must be pointed
    // somewhere else: its retarget is offered as whole target lists rather
    // than as one card among several.
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let option = match &game
                .pending_decisions
                .first()
                .expect("it is pending")
                .continuation
            {
                DecisionContinuation::CopyStackObject { target_lists, .. } => target_lists
                    .iter()
                    .position(|targets| {
                        flatten_target_selections(targets) == [Target::Permanent(second_id)]
                    })
                    .and_then(|index| u32::try_from(index).ok()),
                _ => decision.options.first().map(|option| option.id),
            };
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: option.map(|id| vec![id]).unwrap_or_default(),
                },
            )
            .expect("the offered choice is legal");
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
    drain_pending(&mut game);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == victim),
        "the original exiled what it named",
    );
    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == second_id),
        "and the copy exiled the one it was pointed at",
    );
    assert_eq!(
        game.players[1].hand.len(),
        held + 2,
        "and the copy resolved as though the cheap cost had been paid for it too",
    );
}

/// "If you choose to pay one alternative cost, you can't pay any other
/// alternative costs. For example, if an effect lets you cast a Mastery
/// spell without paying its mana cost, you can't also choose to pay its
/// given alternative cost." Cast off an Omniscience with an empty pool, the
/// {1}{B} cost was not the cost that was paid, so nobody draws.
#[test]
fn a_free_cast_is_not_the_cheap_cast() {
    let (mut game, mastery, _victim) = staged();
    game.players[0].mana_pool = ManaPool::default();
    game.battlefield
        .push(creature(89_100, cards::OMNISCIENCE, PlayerId::One));
    let their_hand = game.players[1].hand.len();

    let casts = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card, .. } if *card == mastery))
        .collect::<Vec<_>>();
    assert_eq!(
        casts.len(),
        1,
        "an empty pool pays for the free cast and nothing else: {casts:?}",
    );
    game.apply(PlayerId::One, casts.into_iter().next().expect("one cast"))
        .expect("it is cast");
    pass_priority_pair(&mut game);
    drain_pending(&mut game);

    assert!(
        game.players[1]
            .exile
            .iter()
            .any(|card| card.definition == cards::SERRA_ANGEL),
        "the exile happened either way",
    );
    assert_eq!(
        game.players[1].hand.len(),
        their_hand,
        "and the rider rides on the cost that was actually paid",
    );
}

/// An instant, which is what the four mana is really buying: both halves are
/// on offer in their combat, and the cheap one answers an attacker that has
/// already been declared -- the Angel is exiled before it deals its four.
#[test]
fn either_half_answers_an_attacker_at_instant_speed() {
    let (mut game, mastery, victim) = staged();
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    drain_pending(&mut game);

    // Their turn, and the Angel is coming across.
    game.active_player = PlayerId::Two;
    game.turns_started = [5, 6];
    game.step = Step::DeclareAttackers;
    game.priority = PlayerId::Two;
    game.declare_attacker(victim, AttackDefender::Player(PlayerId::One));
    game.finish_declaring_attackers();
    drain_pending(&mut game);
    game.priority = PlayerId::One;
    let life = game.players[0].life;

    let offered: Vec<_> = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .filter_map(|action| match action {
            Action::CastSpell { card, choices, .. } if card == mastery => {
                Some(choices.costs().alternative().is_some())
            }
            _ => None,
        })
        .collect();
    assert!(
        offered.contains(&true) && offered.contains(&false),
        "both halves are castable on their turn: it is an instant",
    );

    cast_mastery(&mut game, mastery, true);

    assert!(
        !game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == victim),
        "the attacker was exiled mid-combat",
    );

    game.finish_declaring_blockers();
    game.deal_combat_damage();
    assert_eq!(
        game.players[0].life, life,
        "and dealt none of its four for it",
    );
}
