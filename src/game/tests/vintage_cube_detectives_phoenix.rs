//! Detective's Phoenix: a hasty flier for three, or an Aura for {R} and six
//! mana value out of the graveyard -- and it comes back as a creature when
//! whatever it was wearing is gone.

use super::*;

/// Player One with the Phoenix in `zone`, `graveyard` behind it, a creature
/// out, and mana to spend.
fn staged(
    in_graveyard: bool,
    graveyard: &[CardDefinitionId],
) -> (Game, GameObjectId, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].graveyard.clear();
    for (index, definition) in graveyard.iter().enumerate() {
        game.players[0].graveyard.push(card(
            96_000 + u32::try_from(index).expect("few cards"),
            *definition,
            PlayerId::One,
        ));
    }
    let phoenix = game
        .build_zone(PlayerId::One, &[cards::DETECTIVES_PHOENIX])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let phoenix_id = phoenix.id;
    if in_graveyard {
        game.players[0].graveyard.push(phoenix);
    } else {
        game.players[0].hand.push(phoenix);
    }
    let bears = creature(96_500, cards::GRIZZLY_BEARS, PlayerId::One);
    let bears_id = bears.card.id;
    game.battlefield.push(bears);
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Red, 3);
    (game, phoenix_id, bears_id)
}

/// Six mana value in four cards, which is what collect evidence 6 takes.
const SIX_MANA_VALUE: [CardDefinitionId; 4] = [
    cards::SERRA_ANGEL,
    cards::GRIZZLY_BEARS,
    cards::LIGHTNING_BOLT,
    cards::MOUNTAIN,
];

fn casts(game: &Game, phoenix: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card, .. } if *card == phoenix))
        .collect()
}

/// The bestow cast: the one that names the creature as a target.
fn bestow_cast(game: &Game, phoenix: GameObjectId, host: GameObjectId) -> Option<Action> {
    casts(game, phoenix).into_iter().find(|action| {
        matches!(
            action,
            Action::CastSpell { choices, .. }
                if choices.targets().iter().any(|slot| {
                    slot.targets().contains(&Target::Permanent(host))
                })
        )
    })
}

fn on_battlefield(game: &Game) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == ObjectKind::Card(cards::DETECTIVES_PHOENIX))
}

/// Cast for its printed cost it is an ordinary 2/2 with flying and haste.
#[test]
fn the_printed_cast_is_a_creature() {
    let (mut game, phoenix, _bears) = staged(false, &[]);

    let cast = casts(&game, phoenix)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { choices, .. } if choices.targets().is_empty())
        })
        .expect("three mana casts it outright");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    let permanent = on_battlefield(&game).expect("it arrived");
    assert_eq!(game.power(permanent), Some(2));
    assert!(
        game.permanent_types(permanent)
            .is_some_and(|types| types.contains(CardType::Creature)),
        "a creature, not an Aura",
    );
}

/// Bestowed out of the graveyard it is an Aura, not a creature, and the
/// creature it went on gets bigger and faster.
#[test]
fn bestow_from_the_graveyard_makes_it_an_aura() {
    let (mut game, phoenix, bears) = staged(true, &SIX_MANA_VALUE);

    let cast = bestow_cast(&game, phoenix, bears).expect("bestow is offered from the graveyard");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    let aura = on_battlefield(&game).expect("it arrived");
    assert_eq!(aura.attached_to, Some(bears), "attached to the creature");
    let types = game.permanent_types(aura).expect("it has types");
    assert!(!types.contains(CardType::Creature), "not a creature");
    assert!(types.contains(CardType::Enchantment));
    assert!(
        game.effective_subtypes(aura).contains(&"Aura"),
        "an Aura while it is attached",
    );

    let host = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears)
        .expect("the creature is still there");
    assert_eq!(game.power(host), Some(4), "+2/+2");
    assert_eq!(game.toughness(host), Some(4));
    assert!(game.has_flying(host), "and flying");
}

/// Collect evidence 6 exiles the cards that paid for it.
#[test]
fn collect_evidence_exiles_what_it_counted() {
    let (mut game, phoenix, bears) = staged(true, &SIX_MANA_VALUE);
    let before = game.players[0].exile.len();

    let cast = bestow_cast(&game, phoenix, bears).expect("bestow is offered");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    let exiled = game.players[0].exile.len() - before;
    assert!(exiled > 0, "something paid the cost");
    let total: u16 = game.players[0]
        .exile
        .iter()
        .filter_map(|card| game.catalog.get(card.definition))
        .map(|definition| definition.rules.printed_mana_cost().mana_value())
        .sum();
    assert!(total >= 6, "the cards exiled add up to six: {total}");
}

/// Without six mana value in the graveyard there is nothing to pay with.
#[test]
fn a_shallow_graveyard_cannot_pay_for_it() {
    let (game, phoenix, bears) = staged(true, &[cards::LIGHTNING_BOLT, cards::MOUNTAIN]);

    assert!(
        bestow_cast(&game, phoenix, bears).is_none(),
        "one mana value between them is not six",
    );
}

/// CR 702.103c: when the enchanted creature goes, the Phoenix stays and
/// becomes a creature again.
#[test]
fn it_becomes_a_creature_when_its_host_leaves() {
    let (mut game, phoenix, bears) = staged(true, &SIX_MANA_VALUE);
    let cast = bestow_cast(&game, phoenix, bears).expect("bestow is offered");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    game.battlefield
        .retain(|permanent| permanent.card.id != bears);
    game.check_state_based_actions();

    let permanent = on_battlefield(&game).expect("it did not go with the creature");
    assert_eq!(permanent.attached_to, None, "it came loose");
    assert!(
        game.permanent_types(permanent)
            .is_some_and(|types| types.contains(CardType::Creature)),
        "and is a creature again",
    );
    assert_eq!(game.power(permanent), Some(2));
}

/// It bestows from hand too, at the same price: the graveyard permission is
/// a second zone, not the only one.
#[test]
fn bestow_works_from_hand_as_well() {
    let (mut game, phoenix, bears) = staged(false, &SIX_MANA_VALUE);

    let cast = bestow_cast(&game, phoenix, bears).expect("bestow is offered from hand");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    let aura = on_battlefield(&game).expect("it arrived");
    assert_eq!(aura.attached_to, Some(bears));
    assert_eq!(
        game.players[0].mana_pool.red, 2,
        "one red, not three: the bestow cost replaced the printed one",
    );
}

/// The printed cast pays no evidence: the graveyard is untouched.
#[test]
fn the_printed_cast_collects_no_evidence() {
    let (mut game, phoenix, _bears) = staged(false, &SIX_MANA_VALUE);
    let before = game.players[0].graveyard.len();

    let cast = casts(&game, phoenix)
        .into_iter()
        .find(|action| {
            matches!(action, Action::CastSpell { choices, .. } if choices.targets().is_empty())
        })
        .expect("three mana casts it outright");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    assert_eq!(
        game.players[0].graveyard.len(),
        before,
        "nothing was exiled"
    );
    assert!(game.players[0].exile.is_empty());
}

/// "Unlike other Aura spells, an Aura spell with bestow isn't countered if
/// its target is illegal as it begins to resolve. Rather, the effect making
/// it an Aura spell ends... and it resolves and enters the battlefield as an
/// enchantment creature."
#[test]
fn a_bestowed_phoenix_whose_host_dies_arrives_as_a_creature() {
    let (mut game, phoenix, bears) = staged(true, &SIX_MANA_VALUE);

    let cast = bestow_cast(&game, phoenix, bears).expect("bestow is offered from the graveyard");
    game.apply(PlayerId::One, cast).expect("it is cast");

    // The creature it was bestowed on is answered before it resolves.
    game.move_permanents_to_graveyard(&[bears]);
    drain_pending(&mut game);

    let arrived = on_battlefield(&game).expect("it was not countered");
    assert_eq!(arrived.attached_to, None, "with nothing to attach to");
    let types = game.permanent_types(arrived).expect("it has types");
    assert!(
        types.contains(CardType::Creature),
        "it came down as an enchantment creature instead",
    );
    assert!(types.contains(CardType::Enchantment));
    assert!(
        !game.effective_subtypes(arrived).contains(&"Aura"),
        "and it is no longer an Aura",
    );
    assert!(game.has_flying(arrived), "the printed body flies");
}

/// "If a permanent with bestow enters the battlefield by any method other
/// than being cast, it will be an enchantment creature. You can't choose to
/// pay the bestow cost and have it become an Aura." Reanimated, it is a
/// bird and not an Aura, however many creatures are standing beside it.
#[test]
fn a_phoenix_that_was_never_cast_arrives_as_a_bird() {
    let (mut game, _phoenix, bears) = staged(true, &SIX_MANA_VALUE);

    game.put_onto_battlefield(PlayerId::One, cards::DETECTIVES_PHOENIX)
        .expect("cataloged");
    drain_pending(&mut game);

    let permanent = on_battlefield(&game).expect("it arrived");
    assert_eq!(permanent.attached_to, None, "attached to nothing");
    let types = game.permanent_types(permanent).expect("it has types");
    assert!(
        types.contains(CardType::Creature),
        "an enchantment creature"
    );
    assert!(types.contains(CardType::Enchantment));
    assert_eq!(game.power(permanent), Some(2), "its own 2/2 body");
    assert_eq!(
        game.power(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == bears)
                .expect("the creature is still there")
        ),
        Some(2),
        "and the creature beside it was never enchanted",
    );
}

/// "Enchanted creature gets +2/+2 and has flying and haste." The file reads
/// the first two; haste is the half that turns a Bears played this turn
/// into an attacker.
#[test]
fn the_host_gains_haste_as_well() {
    let (mut game, phoenix, bears) = staged(true, &SIX_MANA_VALUE);
    // The creature arrived this turn, so haste is the only thing that could
    // let it attack.
    if let Some(permanent) = game
        .battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == bears)
    {
        permanent.entered_controller_turn = game.turns_started[PlayerId::One.index()];
    }
    assert!(
        !game.permanent_has_executable_keyword(
            game.battlefield
                .iter()
                .find(|permanent| permanent.card.id == bears)
                .expect("it is there"),
            KeywordAbility::Haste,
        ),
        "a Bears has no haste of its own",
    );

    let cast = bestow_cast(&game, phoenix, bears).expect("bestow is offered");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);

    let host = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == bears)
        .expect("the creature is still there");
    assert!(
        game.permanent_has_executable_keyword(host, KeywordAbility::Haste),
        "the Aura hands its haste over with the rest",
    );
    assert!(game.has_flying(host), "and its flying");
}

/// "Opponents can't try to remove cards from your graveyard to stop you from
/// collecting evidence": the cards are exiled as the spell is announced,
/// while it is still on the stack and before anybody has priority.
#[test]
fn the_evidence_is_collected_on_announcement() {
    let (mut game, phoenix, bears) = staged(true, &SIX_MANA_VALUE);
    let cast = bestow_cast(&game, phoenix, bears).expect("bestow is offered");
    game.apply(PlayerId::One, cast).expect("it is cast");

    assert_eq!(game.stack.len(), 1, "the Phoenix has not resolved yet");
    let collected: u16 = game.players[0]
        .exile
        .iter()
        .filter_map(|card| game.catalog.get(card.definition))
        .map(|definition| definition.rules.printed_mana_cost().mana_value())
        .sum();
    assert!(
        collected >= 6,
        "and six mana value has already left the graveyard for exile: {collected}",
    );
}

/// "An Aura with bestow remains untapped when it becomes unattached ... It
/// can attack on the turn it becomes unattached if it's been under your
/// control continuously, even as an Aura, since your most recent turn
/// began." The Phoenix that loses its host is an attacker that turn.
#[test]
fn the_unattached_phoenix_is_untapped_and_may_attack() {
    let (mut game, phoenix, bears) = staged(true, &SIX_MANA_VALUE);
    let cast = bestow_cast(&game, phoenix, bears).expect("bestow is offered");
    game.apply(PlayerId::One, cast).expect("it is cast");
    drain_pending(&mut game);
    // It has been here since before this turn, Aura or not.
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == bears)
        .expect("the host is there")
        .tapped = true;
    let bird = on_battlefield(&game).expect("the Aura is there").card.id;
    assert!(
        !game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == bird)
            .expect("it is there")
            .tapped,
        "a tapped host does not tap the Aura on it",
    );

    game.battlefield
        .retain(|permanent| permanent.card.id != bears);
    game.check_state_based_actions();
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;

    let creature = on_battlefield(&game).expect("it stayed behind");
    assert!(!creature.tapped, "and it is untapped as a creature too");
    assert!(
        game.legal_actions(PlayerId::One)
            .into_iter()
            .any(|action| matches!(
                action,
                Action::DeclareAttacker { attacker, .. } if attacker == bird
            )),
        "so it may attack the turn it came loose",
    );
}
