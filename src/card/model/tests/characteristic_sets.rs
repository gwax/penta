use crate::card::{
    CardCatalog, CardDefinition, CardPart, CardRules, CardStructure, CharacteristicContext,
    CharacteristicField, CharacteristicPredicateDef, SpellForm, abilities, applicable_part_ids,
    sets,
};
use crate::{CardDefinitionId, CardPartId, CardType, ManaColor};

fn definition(
    normal: &CardRules,
    alternative: &CardRules,
    structure: CardStructure,
) -> CardDefinition {
    let mut card = CardDefinition::new(
        CardDefinitionId::from_uuid("00000000-0000-0000-0000-00000009a000"),
        "Normal set",
        sets::alpha::SET,
        *normal,
    );
    card.parts.push(CardPart::new(
        CardPartId(1),
        "Alternative set",
        *alternative,
    ));
    card.structure = structure;
    card
}

#[test]
fn a_preparation_style_alternative_exists_without_a_cast_option() {
    let normal = CardRules::new_creature(mana_cost!("{3}{U}"), &["Wizard"], 3, 3);
    let alternative = CardRules::new_sorcery(mana_cost!("{R}"));
    let card = definition(
        &normal,
        &alternative,
        CardStructure::single(CardPartId::PRIMARY)
            .with_alternative(CardPartId::PRIMARY, CardPartId(1)),
    );
    let id = card.id;
    let catalog = CardCatalog::new([card]).unwrap();
    let card = catalog.get(id).unwrap();
    assert_eq!(
        card.structure
            .alternatives_for(CardPartId::PRIMARY)
            .collect::<Vec<_>>(),
        [CardPartId(1)]
    );
    assert!(
        CharacteristicPredicateDef::HasType(CardType::Sorcery)
            .matches(card.part(CardPartId(1)).unwrap())
    );
    assert_eq!(
        applicable_part_ids(card, &CharacteristicContext::Graveyard).unwrap(),
        [CardPartId::PRIMARY]
    );
    assert!(
        applicable_part_ids(
            card,
            &CharacteristicContext::Stack {
                form: SpellForm::Part(CardPartId(1))
            }
        )
        .is_err()
    );
    assert_eq!(card.play_options.len(), 1);
}

#[test]
fn prototype_fields_inherit_name_types_and_rules_from_the_normal_set() {
    let normal = CardRules::new_creature(mana_cost!("{7}"), &["Golem"], 7, 7)
        .with_abilities(&const { [abilities::flying()] });
    let alternative = CardRules::new_creature(mana_cost!("{2}{U}"), &[], 3, 3);
    let card = definition(
        &normal,
        &alternative,
        CardStructure::single(CardPartId::PRIMARY).with_alternative_fields(
            CardPartId::PRIMARY,
            CardPartId(1),
            Some(vec![
                CharacteristicField::ManaCost,
                CharacteristicField::Color,
                CharacteristicField::PowerToughness,
            ]),
        ),
    );
    let id = card.id;
    let catalog = CardCatalog::new([card]).unwrap();
    let card = catalog.get(id).unwrap();
    let alternative = card.part(CardPartId(1)).unwrap();
    assert_eq!(alternative.name, "Normal set");
    assert!(alternative.rules.has_subtype("Golem"));
    assert!(alternative.rules.has_keyword(crate::KeywordAbility::Flying));
    assert!(CharacteristicPredicateDef::Color(ManaColor::Blue).matches(alternative));
    assert_eq!(alternative.rules.creature_stats().unwrap().power, 3);
    assert_eq!(alternative.rules.printed_mana_cost().mana_value(), 3);
    assert_eq!(card.card_mana_value(), 7);
}

#[test]
fn flip_fields_preserve_mana_cost_and_color_but_replace_the_other_characteristics() {
    let normal = CardRules::new_creature(mana_cost!("{2}{R}"), &["Human"], 2, 1);
    let alternative = CardRules::new_creature(mana_cost!("{0}"), &["Spirit"], 4, 4)
        .with_abilities(&const { [abilities::flying()] });
    let card = definition(
        &normal,
        &alternative,
        CardStructure::flip(CardPartId::PRIMARY, CardPartId(1)),
    );
    let id = card.id;
    let catalog = CardCatalog::new([card]).unwrap();
    let flipped = catalog.get(id).unwrap().part(CardPartId(1)).unwrap();
    assert_eq!(flipped.name, "Alternative set");
    assert_eq!(flipped.rules.mana_cost(), Some(mana_cost!("{2}{R}")));
    assert!(CharacteristicPredicateDef::Color(ManaColor::Red).matches(flipped));
    assert!(flipped.rules.has_subtype("Spirit"));
    assert!(!flipped.rules.has_subtype("Human"));
    assert_eq!(flipped.rules.creature_stats().unwrap().power, 4);
}

#[test]
fn characteristic_relationship_cycles_and_ambiguous_parents_are_rejected() {
    let rules = CardRules::new_creature(mana_cost!("{1}"), &["Golem"], 1, 1);
    for ambiguous in [false, true] {
        let mut structure = CardStructure::single(CardPartId::PRIMARY)
            .with_alternative(CardPartId::PRIMARY, CardPartId(1));
        structure
            .alternatives
            .push(crate::card::AlternativeCharacteristics {
                normal: CardPartId(1),
                alternative: if ambiguous {
                    CardPartId(1)
                } else {
                    CardPartId::PRIMARY
                },
                fields: None,
            });
        assert!(matches!(
            CardCatalog::new([definition(&rules, &rules, structure)]),
            Err(crate::card::CatalogError::InvalidCharacteristicStructure { .. })
        ));
    }
}
