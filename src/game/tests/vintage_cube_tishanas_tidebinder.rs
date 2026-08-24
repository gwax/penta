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
