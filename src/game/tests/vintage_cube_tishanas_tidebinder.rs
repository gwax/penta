//! Tishana's Tidebinder: a flash body that answers an ability, and leaves
//! the permanent that made it silent for as long as it stands there.

use super::*;

/// Player One holding the Tidebinder with the mana for it, `theirs` on the
/// battlefield under Player Two, and Player Two's turn under way.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut ids = Vec::new();
    for definition in theirs {
        ids.push(
            game.put_onto_battlefield(PlayerId::Two, *definition)
                .expect("cataloged"),
        );
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    drain_pending(&mut game);
    let tidebinder = game
        .build_zone(PlayerId::One, &[cards::TISHANA_S_TIDEBINDER])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = tidebinder.id;
    game.players[0].hand.push(tidebinder);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 3);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::White, 3);
    game.active_player = PlayerId::Two;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::Two;
    (game, id, ids)
}

/// Answers whatever is waiting, always taking the first option offered.
fn settle(game: &mut Game) {
    for _ in 0..24 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .take(decision.minimum.max(1))
                .map(|option| option.id)
                .collect();
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

/// Every activation `source` is offering its controller right now.
fn activations(game: &Game, player: PlayerId, source: GameObjectId) -> Vec<Action> {
    game.legal_actions(player)
        .into_iter()
        .filter(|action| matches!(action, Action::ActivateAbility { source: from, .. } if *from == source))
        .collect()
}

/// Player Two points the Sorcerer at Player One and puts the ability on the
/// stack, leaving priority with Player One.
fn they_ping_you(game: &mut Game, sorcerer: GameObjectId) {
    let action = activations(game, PlayerId::Two, sorcerer)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { targets, .. } => targets
                .iter()
                .any(|selection| selection.targets().contains(&Target::Player(PlayerId::One))),
            _ => false,
        })
        .expect("the Sorcerer can point at you");
    game.apply(PlayerId::Two, action).expect("it activates");
    game.priority = PlayerId::One;
}

/// Casts the Tidebinder, which is legal only because it has flash.
fn flash_it_in(game: &mut Game, tidebinder: GameObjectId) {
    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tidebinder))
        .expect("flash makes it castable on their turn");
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(game);
}

/// The ability is countered: the damage never arrives.
#[test]
fn it_counters_an_activated_ability() {
    let (mut game, tidebinder, ids) = staged(&[cards::PRODIGAL_SORCERER]);
    game.players[0].life = 20;

    they_ping_you(&mut game, ids[0]);
    flash_it_in(&mut game, tidebinder);

    assert_eq!(game.players[0].life, 20, "the ping was countered");
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TISHANA_S_TIDEBINDER),
        "and the body is still a body",
    );
}

/// The creature whose ability was countered has nothing left to activate.
#[test]
fn the_creature_it_answered_loses_its_abilities() {
    let (mut game, tidebinder, ids) = staged(&[cards::PRODIGAL_SORCERER]);
    let sorcerer = ids[0];

    they_ping_you(&mut game, sorcerer);
    flash_it_in(&mut game, tidebinder);
    for permanent in &mut game.battlefield {
        if permanent.card.id == sorcerer {
            permanent.tapped = false;
        }
    }

    assert!(
        activations(&game, PlayerId::Two, sorcerer).is_empty(),
        "an untapped Sorcerer with no abilities cannot ping",
    );
}

/// "For as long as this creature remains": killing it hands the abilities
/// straight back.
#[test]
fn the_abilities_return_when_the_tidebinder_leaves() {
    let (mut game, tidebinder, ids) = staged(&[cards::PRODIGAL_SORCERER]);
    let sorcerer = ids[0];

    they_ping_you(&mut game, sorcerer);
    flash_it_in(&mut game, tidebinder);
    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::TISHANA_S_TIDEBINDER)
        .expect("it resolved")
        .card
        .id;
    game.move_permanents_to_graveyard(&[body]);
    settle(&mut game);
    for permanent in &mut game.battlefield {
        if permanent.card.id == sorcerer {
            permanent.tapped = false;
        }
    }

    assert!(
        !activations(&game, PlayerId::Two, sorcerer).is_empty(),
        "the Sorcerer has its ability back",
    );
}

/// The rider names three types, and an enchantment is not among them.
#[test]
fn an_enchantment_keeps_its_abilities() {
    let (mut game, tidebinder, ids) = staged(&[cards::CIRCLE_OF_PROTECTION_BLUE]);
    let circle = ids[0];

    let action = activations(&game, PlayerId::Two, circle)
        .into_iter()
        .next()
        .expect("the Circle can be activated");
    game.apply(PlayerId::Two, action).expect("it activates");
    game.priority = PlayerId::One;
    flash_it_in(&mut game, tidebinder);
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::White, 3);

    assert!(
        game.damage_preventions.is_empty(),
        "the Circle's ability was countered rather than resolving",
    );
    assert!(
        !activations(&game, PlayerId::Two, circle).is_empty(),
        "an enchantment whose ability was countered keeps it",
    );
}

/// "Up to one": with nothing on the stack it is simply a 3/2.
#[test]
fn it_can_enter_with_nothing_to_counter() {
    let (mut game, tidebinder, _) = staged(&[]);
    game.priority = PlayerId::One;

    flash_it_in(&mut game, tidebinder);

    let body = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::TISHANA_S_TIDEBINDER)
        .expect("it resolved with no target to take");
    assert_eq!(game.power(body), Some(3));
    assert_eq!(game.toughness(body), Some(2));
}

/// The card says "activated or triggered", and a trigger's source is found
/// the same way: the Djinn's upkeep trigger is countered and the Djinn is
/// left with nothing.
#[test]
fn it_counters_a_triggered_ability_too() {
    let (mut game, tidebinder, ids) = staged(&[cards::JUZAM_DJINN]);
    let djinn = ids[0];
    game.players[1].life = 20;
    game.step = Step::Upkeep;
    game.handle_upkeep_triggers();
    if game.stack.is_empty() {
        game.apply(PlayerId::Two, Action::PassPriority)
            .expect("the trigger goes on the stack as priority is offered");
    }
    assert_eq!(game.stack.len(), 1, "their upkeep trigger is waiting");
    game.priority = PlayerId::One;

    flash_it_in(&mut game, tidebinder);

    assert_eq!(game.players[1].life, 20, "the trigger never resolved");

    // A silenced Djinn has nothing to trigger: the same upkeep asked again
    // produces nothing at all.
    game.handle_upkeep_triggers();
    drain_pending(&mut game);

    assert_eq!(
        game.players[1].life, 20,
        "and it does not trigger again either",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == djinn),
        "the Djinn itself is untouched",
    );
}

/// "If the Tidebinder leaves the battlefield before its triggered ability
/// resolves, the target ability will still be countered, but the source of
/// the ability won't lose its abilities at all." The counter is the first
/// half of the effect and lands either way; the silence is the second half
/// and has nothing to hang on.
#[test]
fn a_dead_tidebinder_still_counters_but_silences_nothing() {
    let (mut game, tidebinder, ids) = staged(&[cards::PRODIGAL_SORCERER]);
    let sorcerer = ids[0];
    they_ping_you(&mut game, sorcerer);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tidebinder))
        .expect("flash makes it castable on their turn");
    game.apply(PlayerId::One, cast).expect("it is cast");

    // Far enough for the body to resolve and its trigger to be put on the
    // stack with a target, and no further.
    let mut body = None;
    for _ in 0..16 {
        if let Some(permanent) = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.definition == cards::TISHANA_S_TIDEBINDER)
        {
            body = Some(permanent.card.id);
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
                .take(decision.minimum.max(1))
                .map(|option| option.id)
                .collect();
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
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    let body = body.expect("the Tidebinder resolved");
    assert!(
        !game.stack.objects.is_empty() || !game.pending_triggers.is_empty(),
        "the trigger is still waiting, which is the whole of the case",
    );

    game.move_permanents_to_graveyard(&[body]);
    settle(&mut game);

    assert_eq!(
        game.players[0].life, 20,
        "the ping was countered even though its answer had died",
    );
    for permanent in &mut game.battlefield {
        if permanent.card.id == sorcerer {
            permanent.tapped = false;
        }
    }
    assert!(
        !activations(&game, PlayerId::Two, sorcerer).is_empty(),
        "and the Sorcerer never lost anything",
    );
}

/// "If the affected permanent gains an ability after the effect begins to
/// apply to it, it will keep that ability." The silence removes what was
/// there when it resolved; the Boots strapped on afterwards are later in
/// line and hand their keywords over.
#[test]
fn an_ability_granted_afterwards_survives_the_silence() {
    let (mut game, tidebinder, ids) = staged(&[cards::PRODIGAL_SORCERER, cards::LAVASPUR_BOOTS]);
    let sorcerer = ids[0];
    let boots = ids[1];

    they_ping_you(&mut game, sorcerer);
    flash_it_in(&mut game, tidebinder);
    assert!(
        activations(&game, PlayerId::Two, sorcerer).is_empty(),
        "the Sorcerer is silenced to begin with",
    );

    game.priority = PlayerId::Two;
    let equip = activations(&game, PlayerId::Two, boots)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { targets, .. } => targets
                .iter()
                .any(|selection| selection.targets().contains(&Target::Permanent(sorcerer))),
            _ => false,
        })
        .expect("the Boots can be strapped to the Sorcerer");
    game.apply(PlayerId::Two, equip).expect("it activates");
    settle(&mut game);

    let sorcerer = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == sorcerer)
        .expect("he is still there");
    assert!(
        game.permanent_has_executable_keyword(sorcerer, KeywordAbility::Haste),
        "what the Boots grant arrives after the silence and stays",
    );
    assert_eq!(game.power(sorcerer), Some(2), "and so does their +1/+0");
}

/// "(Mana abilities can't be targeted.)" Nothing in the record excludes
/// them, because a mana ability never uses the stack and so is never a stack
/// object to name. The Sorcerer beside it is the control: the same
/// Tidebinder, entering after a real activation, is asked what to counter.
#[test]
fn a_mana_ability_is_not_something_to_counter() {
    for tap_the_island in [true, false] {
        let (mut game, tidebinder, ids) = staged(&[cards::PRODIGAL_SORCERER, cards::ISLAND]);
        if tap_the_island {
            let island = ids[1];
            game.apply(
                PlayerId::Two,
                Action::ActivateManaAbility {
                    source: island,
                    ability: mana_ability_for(&game, island, ManaColor::Blue),
                    color: ManaColor::Blue,
                    counters_removed: None,
                    cost_object: None,
                    combination: None,
                    triggered_mana: None,
                },
            )
            .expect("they tap their Island");
            assert!(
                game.stack.is_empty(),
                "a mana ability does not use the stack",
            );
            game.priority = PlayerId::One;
        } else {
            they_ping_you(&mut game, ids[0]);
        }

        let cast = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tidebinder))
            .expect("flash makes it castable on their turn");
        game.apply(PlayerId::One, cast).expect("it is cast");
        pass_priority_pair(&mut game);

        let asked = game
            .observe(PlayerId::One)
            .decision
            .is_some_and(|decision| !decision.options.is_empty());
        assert_eq!(
            asked, !tap_the_island,
            "an activation on the stack is what the trigger has to name, and \
             mana they made is not one",
        );
    }
}

/// "Checks to see if the source of the ability is an artifact, creature, or
/// planeswalker as it resolves." A Sorcerer that dies under the trigger is
/// no permanent at all by then, so the ability is countered and there is
/// nothing left to silence -- and the Tidebinder is still standing.
#[test]
fn a_source_that_dies_first_is_countered_and_nothing_else() {
    let (mut game, tidebinder, ids) = staged(&[cards::PRODIGAL_SORCERER]);
    let sorcerer = ids[0];
    game.players[0].life = 20;
    they_ping_you(&mut game, sorcerer);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tidebinder))
        .expect("flash makes it castable on their turn");
    game.apply(PlayerId::One, cast).expect("it is cast");
    // As far as the trigger being on the stack with its target named.
    for _ in 0..16 {
        if game
            .battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TISHANA_S_TIDEBINDER)
            && !game.stack.is_empty()
        {
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
                .take(decision.minimum.max(1))
                .map(|option| option.id)
                .collect();
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
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    game.move_permanents_to_graveyard(&[sorcerer]);
    game.check_state_based_actions();
    settle(&mut game);

    assert_eq!(
        game.players[0].life, 20,
        "the ability was countered whatever became of its source",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TISHANA_S_TIDEBINDER),
        "and the Tidebinder is unbothered by having silenced nobody",
    );
}

/// "Loses all abilities" is all of them, not only the one that was
/// countered: a Shivan Dragon whose firebreathing is answered is left
/// without its wings as well.
#[test]
fn the_silence_takes_the_keywords_too() {
    let (mut game, tidebinder, ids) = staged(&[cards::SHIVAN_DRAGON]);
    let dragon = ids[0];
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    assert!(
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == dragon)
            .is_some_and(|permanent| game.has_flying(permanent)),
        "it flies to begin with",
    );

    let pump = activations(&game, PlayerId::Two, dragon)
        .into_iter()
        .next()
        .expect("one red buys it a point of power");
    game.apply(PlayerId::Two, pump).expect("it activates");
    game.priority = PlayerId::One;
    flash_it_in(&mut game, tidebinder);

    let dragon = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == dragon)
        .expect("it is still there");
    assert!(
        !game.has_flying(dragon),
        "the silence took the flying with the firebreathing",
    );
    assert_eq!(
        (game.power(dragon), game.toughness(dragon)),
        (Some(5), Some(5)),
        "and the pump it was buying never resolved",
    );
}

/// "Up to one target activated or triggered ability" does not say whose: a
/// Tidebinder flashed in on your own turn may answer your own trigger, which
/// is a thing you do only to silence your own permanent on purpose.
#[test]
fn it_may_counter_your_own_ability() {
    let (mut game, tidebinder, _) = staged(&[]);
    let sorcerer = game
        .put_onto_battlefield(PlayerId::One, cards::PRODIGAL_SORCERER)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;

    let ping = activations(&game, PlayerId::One, sorcerer)
        .into_iter()
        .find(|action| match action {
            Action::ActivateAbility { targets, .. } => targets
                .iter()
                .any(|selection| selection.targets().contains(&Target::Player(PlayerId::Two))),
            _ => false,
        })
        .expect("your own Sorcerer can point across the table");
    game.apply(PlayerId::One, ping).expect("it activates");
    let life = game.players[1].life;

    flash_it_in(&mut game, tidebinder);

    assert_eq!(
        game.players[1].life, life,
        "your own ping was countered by your own Merfolk",
    );
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == sorcerer)
        .expect("he is there")
        .tapped = false;
    assert!(
        activations(&game, PlayerId::One, sorcerer).is_empty(),
        "and your own Sorcerer is the one left silent",
    );
}

/// The rider names three types and the coverage above only ever exercises
/// the creature. A planeswalker's loyalty ability is an activated ability
/// like any other: countered, and Jace is left with no abilities to
/// activate at all.
#[test]
fn a_planeswalkers_loyalty_ability_is_answered_and_silences_him() {
    let (mut game, tidebinder, theirs) = staged(&[cards::JACE_THE_MIND_SCULPTOR]);
    let jace = theirs[0];
    let before = game.players[1].hand.len();

    let activate = activations(&game, PlayerId::Two, jace)
        .into_iter()
        .next()
        .expect("he has loyalty abilities to spend");
    game.apply(PlayerId::Two, activate).expect("it activates");
    game.priority = PlayerId::One;

    flash_it_in(&mut game, tidebinder);

    assert_eq!(
        game.players[1].hand.len(),
        before,
        "whatever he was doing, it did not resolve",
    );
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == jace),
        "he is still standing",
    );
    assert!(
        activations(&game, PlayerId::Two, jace).is_empty(),
        "with nothing left to activate",
    );
}

/// The artifact half of the same rider: a Relic of Sauron whose draw is
/// countered keeps its body and loses everything it does, mana ability
/// included.
#[test]
fn an_artifacts_ability_is_answered_and_silences_it() {
    let (mut game, tidebinder, theirs) = staged(&[cards::RELIC_OF_SAURON]);
    let relic = theirs[0];
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Colorless, 3);
    let draw = activations(&game, PlayerId::Two, relic)
        .into_iter()
        .find(|action| !matches!(action, Action::ActivateManaAbility { .. }))
        .expect("three mana and a tap draws two");
    game.apply(PlayerId::Two, draw).expect("it activates");
    game.priority = PlayerId::One;
    let before = game.players[1].hand.len();

    flash_it_in(&mut game, tidebinder);

    assert_eq!(game.players[1].hand.len(), before, "the draw was countered");
    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.id == relic),
        "the artifact is still there",
    );
    assert!(
        !game
            .legal_actions(PlayerId::Two)
            .iter()
            .any(|action| matches!(
                action,
                Action::ActivateManaAbility { source, .. } if *source == relic
            )),
        "and its mana ability went with the rest",
    );
}

/// "Counter up to one target activated or triggered ability": a spell is
/// neither. A Lightning Bolt on the stack is no target for it, and the
/// Tidebinder arrives with nothing to answer.
#[test]
fn a_spell_on_the_stack_is_not_something_it_may_counter() {
    let (mut game, tidebinder, _theirs) = staged(&[]);
    game.players[1]
        .hand
        .push(card(120_900, cards::LIGHTNING_BOLT, PlayerId::Two));
    game.add_unrestricted_mana(PlayerId::Two, ManaColor::Red, 1);
    let bolt = game
        .legal_actions(PlayerId::Two)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { card, .. } if *card == CardInstanceId(120_900))
        })
        .expect("one red casts it");
    game.apply(PlayerId::Two, bolt).expect("it is cast");
    game.priority = PlayerId::One;
    let spell = game.stack.last().expect("the Bolt is on the stack").id;

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == tidebinder))
        .expect("flash makes it castable in response");
    assert!(
        !matches!(&cast, Action::CastSpell { choices, .. }
            if choices.iter_targets().any(|target| *target == Target::Spell(spell))),
        "the Bolt is no target for it",
    );
    game.apply(PlayerId::One, cast).expect("it is cast");
    settle(&mut game);

    assert!(
        game.battlefield
            .iter()
            .any(|permanent| permanent.card.definition == cards::TISHANA_S_TIDEBINDER),
        "the Tidebinder arrived all the same",
    );
    assert_eq!(
        game.players[0].life, 17,
        "and the Bolt it could not touch resolved",
    );
}
