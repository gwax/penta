//! Fog: all combat damage this turn is prevented.
//!
//! The engine could already prevent combat damage per permanent, which is
//! enough for a Maze of Ith but not for a Fog: the Fog has no permanent to
//! attach to, and it has to cover creatures that were not on the battlefield
//! when it resolved. So the shield is game state and lives until cleanup.

use super::*;

fn fogged_combat(cast_fog: bool) -> Game {
    let mut game = ready_game();
    game.step = Step::DeclareBlockers;
    let mut attacker = creature(10_000, cards::SEA_SERPENT, PlayerId::One);
    attacker.attacking = true;
    attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    game.battlefield.push(attacker);
    let mut blocker = creature(10_001, cards::SAVANNAH_LIONS, PlayerId::Two);
    blocker.blocking = Some(GameObjectId(10_000));
    game.battlefield.push(blocker);
    if cast_fog {
        game.all_combat_damage_prevented = true;
    }
    game
}

fn resolve_combat_damage(game: &mut Game) {
    game.finish_declaring_blockers();
    game.start_combat_damage();
    game.finish_rules_procedure();
}

#[test]
fn combat_damage_lands_without_a_fog() {
    let mut game = fogged_combat(false);
    resolve_combat_damage(&mut game);
    let serpent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == GameObjectId(10_000));
    assert!(
        serpent.is_some_and(|permanent| permanent.damage > 0),
        "the blocker's damage is marked"
    );
}

#[test]
fn a_fog_prevents_damage_in_both_directions() {
    let mut game = fogged_combat(true);
    resolve_combat_damage(&mut game);
    for id in [GameObjectId(10_000), GameObjectId(10_001)] {
        let permanent = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == id)
            .expect("both combatants survive a Fog");
        assert_eq!(permanent.damage, 0, "{id:?} took no combat damage");
    }
}

/// The shield covers what the attacker would have dealt to the player too.
#[test]
fn a_fog_prevents_damage_to_the_defending_player() {
    let mut game = ready_game();
    game.step = Step::DeclareBlockers;
    let mut attacker = creature(10_000, cards::SEA_SERPENT, PlayerId::One);
    attacker.attacking = true;
    attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
    game.battlefield.push(attacker);
    game.all_combat_damage_prevented = true;
    let before = game.players[PlayerId::Two.index()].life;

    resolve_combat_damage(&mut game);

    assert_eq!(
        game.players[PlayerId::Two.index()].life,
        before,
        "an unblocked attacker deals nothing through a Fog"
    );
}

/// It is a turn-scoped shield, not a permanent one.
#[test]
fn a_fog_does_not_survive_cleanup() {
    let mut game = fogged_combat(true);
    game.finish_cleanup();
    assert!(
        !game.all_combat_damage_prevented,
        "the shield expires with the turn"
    );
}

#[test]
fn every_newly_unblocked_fog_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [cards::FOG, cards::HOLY_DAY, cards::DARKNESS] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            crate::ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}

fn shielded_creature(game: &mut Game) -> GameObjectId {
    let creature = creature(20_000, cards::SAVANNAH_LIONS, PlayerId::One);
    let id = creature.card.id;
    game.battlefield.push(creature);
    id
}

/// A shield waits for damage rather than acting now, and is spent by the
/// damage it covers.
#[test]
fn a_shield_absorbs_up_to_its_amount_and_is_then_gone() {
    let mut game = ready_game();
    let target = shielded_creature(&mut game);
    game.prevention_shields.push(PreventionShield {
        recipient: Target::Permanent(target),
        remaining: Some(2),
    });

    game.damage_target(Some(Target::Permanent(target)), 1);
    let marked = |game: &Game| {
        game.battlefield
            .iter()
            .find(|permanent| permanent.card.id == target)
            .map_or(0, |permanent| permanent.damage)
    };
    assert_eq!(marked(&game), 0, "the first point is prevented");

    game.damage_target(Some(Target::Permanent(target)), 3);
    assert_eq!(
        marked(&game),
        2,
        "one point of the shield was left, so two of the three land"
    );
    assert!(game.prevention_shields.is_empty(), "a spent shield is gone");
}

/// "Prevent all damage" is never spent, so it holds for the whole turn.
#[test]
fn a_prevent_all_shield_is_not_consumed() {
    let mut game = ready_game();
    let target = shielded_creature(&mut game);
    game.prevention_shields.push(PreventionShield {
        recipient: Target::Permanent(target),
        remaining: None,
    });

    for _ in 0..3 {
        game.damage_target(Some(Target::Permanent(target)), 5);
    }
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == target)
        .expect("the creature survives");
    assert_eq!(permanent.damage, 0, "every point was prevented");
    assert_eq!(game.prevention_shields.len(), 1, "the shield still holds");
}

#[test]
fn a_shield_only_covers_the_recipient_it_names() {
    let mut game = ready_game();
    let shielded = shielded_creature(&mut game);
    let other = creature(20_001, cards::SAVANNAH_LIONS, PlayerId::Two);
    let other_id = other.card.id;
    game.battlefield.push(other);
    game.prevention_shields.push(PreventionShield {
        recipient: Target::Permanent(shielded),
        remaining: Some(5),
    });

    game.damage_target(Some(Target::Permanent(other_id)), 1);
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == other_id)
        .expect("the other creature is on the battlefield");
    assert_eq!(permanent.damage, 1, "an unshielded creature takes damage");
}

/// Shields cover players too, which is what "any target" means.
#[test]
fn a_shield_can_cover_a_player() {
    let mut game = ready_game();
    let before = game.players[PlayerId::Two.index()].life;
    game.prevention_shields.push(PreventionShield {
        recipient: Target::Player(PlayerId::Two),
        remaining: Some(3),
    });

    game.damage_target(Some(Target::Player(PlayerId::Two)), 2);
    assert_eq!(game.players[PlayerId::Two.index()].life, before);
}

#[test]
fn shields_do_not_survive_cleanup() {
    let mut game = ready_game();
    let target = shielded_creature(&mut game);
    game.prevention_shields.push(PreventionShield {
        recipient: Target::Permanent(target),
        remaining: None,
    });
    game.finish_cleanup();
    assert!(game.prevention_shields.is_empty());
}

#[test]
fn every_newly_unblocked_prevention_card_reports_complete_coverage() {
    let catalog = poc::catalog().expect("catalog builds");
    for definition in [
        cards::SAMITE_HEALER,
        cards::INDESTRUCTIBLE_AURA,
        cards::AMULET_OF_KROOG,
    ] {
        let card = catalog.get(definition).expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            crate::ImplementationStatus::Complete,
            "{} should be fully executable",
            card.name,
        );
    }
}

/// A second sweep, prompted by the shields having outlived their audit lines.
/// Seven identities were blocked on "a duration-scoped replacement/prevention
/// effect" that had already been built; the two shapes below are the ones the
/// first sweep never drove -- a shield aimed at a player, and prevention of
/// only the combat damage a creature deals.
mod follow_up {
    use super::*;

    /// Conservator shields its controller, not a permanent. The shield has to
    /// find a player recipient and spend itself on damage aimed there.
    #[test]
    fn conservator_shields_its_controller_and_spends_the_shield() {
        let mut game = ready_game();
        let conservator = creature(10_000, cards::CONSERVATOR, PlayerId::One);
        let conservator_id = conservator.card.id;
        game.battlefield.push(conservator);
        game.players[PlayerId::One.index()].mana_pool.colorless = 3;

        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| {
                matches!(action, Action::ActivateAbility { source, .. } if *source == conservator_id)
            })
            .expect("the ability is affordable");
        game.apply(PlayerId::One, action)
            .expect("the ability activates");
        pass_priority_pair(&mut game);

        assert_eq!(
            game.prevention_shields.len(),
            1,
            "one shield, aimed at a player"
        );
        game.damage_target(Some(Target::Player(PlayerId::One)), 3);
        assert_eq!(
            game.players[PlayerId::One.index()].life,
            i16::from(rules::STARTING_LIFE) - 1,
            "two of the three damage was prevented"
        );
        assert!(
            game.prevention_shields.is_empty(),
            "and the shield was spent doing it"
        );
    }

    /// Horn of Deafening stops what the creature deals without touching what
    /// is dealt to it, which is the distinction between the two combat-damage
    /// prevention effects.
    #[test]
    fn horn_of_deafening_silences_one_attacker_in_one_direction() {
        let mut game = ready_game();
        let horn = creature(10_000, cards::HORN_OF_DEAFENING, PlayerId::One);
        let horn_id = horn.card.id;
        game.battlefield.push(horn);
        game.players[PlayerId::One.index()].mana_pool.colorless = 2;
        let ogre = creature(10_001, cards::SEDGE_TROLL, PlayerId::Two);
        let ogre_id = ogre.card.id;
        game.battlefield.push(ogre);

        let action = game
            .legal_actions(PlayerId::One)
            .into_iter()
            .find(|action| {
                matches!(action, Action::ActivateAbility { source, .. } if *source == horn_id)
            })
            .expect("the ability is affordable");
        game.apply(PlayerId::One, action)
            .expect("the ability activates");
        pass_priority_pair(&mut game);

        let silenced = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == ogre_id)
            .expect("the creature is still on the battlefield");
        assert!(
            silenced.combat_damage_dealt_by_prevented,
            "the creature deals no combat damage this turn"
        );
        assert!(
            !silenced.combat_damage_prevented,
            "but damage dealt to it is untouched"
        );
    }

    #[test]
    fn the_swept_identities_report_complete_coverage() {
        let catalog = poc::catalog().expect("catalog builds");
        for definition in [
            cards::CONSERVATOR,
            cards::OASIS,
            cards::ARGIVIAN_BLACKSMITH,
            cards::KEI_TAKAHASHI,
            cards::LADY_EVANGELA,
            cards::HORN_OF_DEAFENING,
            cards::COMBAT_MEDIC,
        ] {
            let card = catalog.get(definition).expect("the card is cataloged");
            assert_eq!(
                card.rules.implementation_status(),
                crate::ImplementationStatus::Complete,
                "{} should be fully executable",
                card.name,
            );
        }
    }
}

/// A continuous combat-damage prevention, which is what an Aura needs and
/// what the turn-scoped effects could not give it. The flags those set are
/// written once and cleared at cleanup; this is asked afresh every time
/// combat damage is dealt, so the Aura leaving mid-combat stops applying.
mod gaseous_form {
    use super::*;

    fn form_game() -> (Game, GameObjectId, GameObjectId) {
        let mut game = ready_game();
        game.step = Step::DeclareBlockers;
        let mut attacker = creature(10_000, cards::SEDGE_TROLL, PlayerId::One);
        attacker.attacking = true;
        attacker.attack_defender = Some(AttackDefender::Player(PlayerId::Two));
        let attacker_id = attacker.card.id;
        game.battlefield.push(attacker);

        let mut aura = creature(10_001, cards::GASEOUS_FORM, PlayerId::One);
        aura.attached_to = Some(attacker_id);
        let aura_id = aura.card.id;
        game.battlefield.push(aura);
        (game, attacker_id, aura_id)
    }

    #[test]
    fn an_enchanted_attacker_deals_no_combat_damage() {
        let (mut game, _attacker_id, _aura_id) = form_game();
        game.finish_declaring_blockers();
        game.deal_combat_damage();

        assert_eq!(
            game.players[PlayerId::Two.index()].life,
            i16::from(rules::STARTING_LIFE),
            "the enchanted creature's combat damage was prevented"
        );
    }

    /// The same creature, once the Aura is gone, hits for its printed power.
    /// This is the half a turn-scoped flag would get wrong.
    #[test]
    fn removing_the_aura_restores_the_damage_immediately() {
        let (mut game, attacker_id, aura_id) = form_game();
        game.battlefield
            .retain(|permanent| permanent.card.id != aura_id);
        let power = game
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == attacker_id)
            .and_then(|permanent| game.power(permanent))
            .expect("the attacker has power");

        game.finish_declaring_blockers();
        game.deal_combat_damage();

        assert_eq!(
            game.players[PlayerId::Two.index()].life,
            i16::from(rules::STARTING_LIFE) - power,
            "with the Aura gone nothing is prevented"
        );
    }

    #[test]
    fn gaseous_form_reports_complete_coverage() {
        let catalog = poc::catalog().expect("catalog builds");
        let card = catalog
            .get(cards::GASEOUS_FORM)
            .expect("the card is cataloged");
        assert_eq!(
            card.rules.implementation_status(),
            crate::ImplementationStatus::Complete,
        );
    }
}
