use super::*;
use crate::ImplementationStatus;

#[test]
fn mogg_fanatic_and_seal_of_fire_pay_sacrifice_before_dealing_damage() {
    for (definition, amount) in [(cards::MOGG_FANATIC, 1), (cards::SEAL_OF_FIRE, 2)] {
        let mut game = ready_game();
        let source = CardInstanceId(10_000);
        game.battlefield
            .push(creature(source.0, definition, PlayerId::One));
        let activation = Action::ActivateAbility {
            source,
            ability: primary_ability(definition),
            targets: activated_targets(Target::Player(PlayerId::Two)),
            cost_object: None,
            x: 0,
        };

        assert!(game.legal_actions(PlayerId::One).contains(&activation));
        game.apply(PlayerId::One, activation).unwrap();
        assert!(
            game.battlefield.is_empty(),
            "the source is sacrificed as a cost"
        );

        pass_priority_pair(&mut game);
        assert_eq!(game.players[PlayerId::Two.index()].life, 20 - amount);
    }
}

#[test]
fn incinerating_jackal_pup_deals_the_same_damage_back() {
    let mut game = ready_game();
    let pup = CardInstanceId(10_000);
    game.battlefield
        .push(creature(pup.0, cards::JACKAL_PUP, PlayerId::One));
    let incinerate = card(10_001, cards::INCINERATE, PlayerId::Two);
    game.players[PlayerId::Two.index()]
        .hand
        .push(incinerate.clone());
    game.players[PlayerId::Two.index()].mana_pool.colorless = 1;
    game.players[PlayerId::Two.index()].mana_pool.red = 1;
    game.priority = PlayerId::Two;

    game.apply(
        PlayerId::Two,
        cast_action(incinerate.id, vec![Target::Permanent(pup)], Vec::new(), 0),
    )
    .unwrap();
    pass_priority_pair(&mut game);
    assert!(game.battlefield.is_empty(), "three damage kills the Pup");

    pass_priority_pair(&mut game);
    assert_eq!(game.players[PlayerId::One.index()].life, 17);
}

#[test]
fn naturalize_destroys_an_artifact_or_enchantment() {
    for target_definition in [cards::BLACK_VISE, cards::SEAL_OF_FIRE] {
        let mut game = ready_game();
        let target = CardInstanceId(10_000);
        game.battlefield
            .push(creature(target.0, target_definition, PlayerId::Two));
        let naturalize = card(10_001, cards::NATURALIZE, PlayerId::One);
        game.players[PlayerId::One.index()]
            .hand
            .push(naturalize.clone());
        game.players[PlayerId::One.index()].mana_pool.colorless = 1;
        game.players[PlayerId::One.index()].mana_pool.green = 1;

        game.apply(
            PlayerId::One,
            cast_action(
                naturalize.id,
                vec![Target::Permanent(target)],
                Vec::new(),
                0,
            ),
        )
        .unwrap();
        pass_priority_pair(&mut game);
        assert!(game.battlefield.is_empty());
    }
}

#[test]
fn hydroblast_and_pyroblast_are_complete_opposite_color_modal_answers() {
    let game = ready_game();
    for (definition, color) in [
        (cards::HYDROBLAST, ManaColor::Red),
        (cards::PYROBLAST, ManaColor::Blue),
    ] {
        let rules = game.catalog.get(definition).unwrap().rules;
        assert_eq!(
            rules.implementation_status(),
            ImplementationStatus::Complete
        );
        let DeclarativeAbilityDef::Spell(wrapper) = rules.ability_clauses()[0].definition else {
            panic!("the blast has a spell wrapper");
        };
        let modes = wrapper.modal().expect("the blast is modal").modes;
        assert_eq!(modes.len(), 2);
        let DeclarativeAbilityDef::Spell(counter_mode) = modes[0].definition else {
            panic!("the first mode is a spell clause");
        };
        assert_eq!(
            counter_mode.targets(),
            &[AbilityTargetDef::exactly_one_spell(
                ObjectPredicateDef::Color(color)
            )]
        );
        assert!(matches!(
            modes[1].effect.definition,
            EffectDef::Destroy {
                can_regenerate: true,
                ..
            }
        ));
    }
}
