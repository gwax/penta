use crate::card::{
    CardCatalog, CardDefinition, CardProperty, CardRequirement, CardType, DeckCards,
    DeckRequirementDef, DeclarativeAbilityDef, KeywordAbility,
};
use crate::card::{CharacteristicContext, applicable_part_ids};
use crate::{CardDefinitionId, Format};
use std::collections::HashSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeckRequirementViolation {
    UnknownCard(CardDefinitionId),
    InvalidCharacteristics(CardDefinitionId),
    UnsupportedCard(String),
    NotInSideboard(CardDefinitionId),
    NotACompanion(CardDefinitionId),
    CompanionBanned {
        card: String,
        format: Format,
    },
    CardDoesNotMeetRequirement {
        card: String,
        requirement: CardRequirement,
    },
    DuplicateName(String),
    TooFewCards {
        actual: usize,
        minimum: usize,
    },
}

impl std::fmt::Display for DeckRequirementViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCard(id) => write!(f, "unknown card {id:?}"),
            Self::UnsupportedCard(name) => write!(
                f,
                "cannot check starting-deck characteristics for unsupported card {name}"
            ),
            Self::InvalidCharacteristics(id) => {
                write!(f, "invalid card characteristics for {id:?}")
            }
            Self::NotInSideboard(id) => write!(f, "companion {id:?} is not in the sideboard"),
            Self::CompanionBanned { card, format } => {
                write!(f, "{card} is banned as a companion in {format:?}")
            }
            Self::NotACompanion(id) => write!(f, "card {id:?} has no companion ability"),
            Self::CardDoesNotMeetRequirement { card, requirement } => {
                write!(f, "{card} does not meet {requirement:?}")
            }
            Self::DuplicateName(name) => {
                write!(f, "starting deck repeats non-distinct name {name}")
            }
            Self::TooFewCards { actual, minimum } => write!(
                f,
                "starting deck has {actual} cards; requires at least {minimum}"
            ),
        }
    }
}
impl std::error::Error for DeckRequirementViolation {}

pub(crate) fn validate_companion_requirement(
    candidate: CardDefinitionId,
    catalog: &CardCatalog,
    starting_deck: impl IntoIterator<Item = CardDefinitionId>,
    format: Format,
) -> Result<(), DeckRequirementViolation> {
    let definition = catalog
        .get(candidate)
        .ok_or(DeckRequirementViolation::UnknownCard(candidate))?;
    let companion = definition
        .companion()
        .ok_or(DeckRequirementViolation::NotACompanion(candidate))?;
    if format.commander_definition().is_some_and(|rules| {
        rules
            .companion_only_banned_cards
            .contains(&definition.name.as_str())
    }) {
        return Err(DeckRequirementViolation::CompanionBanned {
            card: definition.name.clone(),
            format,
        });
    }
    validate_deck_requirement(companion.requirement, catalog, starting_deck, format)
}

/// Evaluate pregame requirements against card characteristics outside the game.
/// # Errors
/// Returns the first offending card or collection constraint; unknown cards never disappear.
pub fn validate_deck_requirement(
    requirement: DeckRequirementDef,
    catalog: &CardCatalog,
    starting_deck: impl IntoIterator<Item = CardDefinitionId>,
    format: Format,
) -> Result<(), DeckRequirementViolation> {
    let definitions = starting_deck
        .into_iter()
        .map(|id| {
            catalog
                .get(id)
                .ok_or(DeckRequirementViolation::UnknownCard(id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let count = definitions.len();
    // Counting identities requires no unavailable characteristics. Requirements
    // about individual cards construct their outside-game view lazily.
    let cards = definitions.into_iter().map(DeckCard::new);
    match requirement {
        DeckRequirementDef::MinimumSizeAboveFormat(extra) => {
            let minimum = format
                .commander_definition()
                .map_or(format.rules().minimum_main_deck_size, |rules| {
                    rules.total_deck_size
                })
                .saturating_add(extra);
            if count < minimum {
                return Err(DeckRequirementViolation::TooFewCards {
                    actual: count,
                    minimum,
                });
            }
        }
        DeckRequirementDef::Each {
            cards: filter,
            requirement,
        } => {
            for card in cards {
                let card = card?;
                if card.matches(filter) && !card.meets(requirement) {
                    return Err(DeckRequirementViolation::CardDoesNotMeetRequirement {
                        card: card.definition.name.clone(),
                        requirement,
                    });
                }
            }
        }
        DeckRequirementDef::Distinct {
            cards: filter,
            property: CardProperty::Name,
        } => {
            let mut seen = HashSet::new();
            for card in cards {
                let card = card?;
                if !card.matches(filter) {
                    continue;
                }
                for part in &card.parts {
                    if !seen.insert(part.name.as_str()) {
                        return Err(DeckRequirementViolation::DuplicateName(part.name.clone()));
                    }
                }
            }
        }
    }
    Ok(())
}

struct DeckCard<'a> {
    definition: &'a CardDefinition,
    parts: Vec<&'a crate::card::CardPart>,
    types: crate::card::CardTypeSet,
    subtypes: crate::card::SubtypeSet,
}
impl<'a> DeckCard<'a> {
    fn new(definition: &'a CardDefinition) -> Result<Self, DeckRequirementViolation> {
        use crate::card::{
            AppliedEffectDef, CardTypeSet, CharacteristicOperationDef, SetOperationDef, SubtypeSet,
        };
        if definition.implementation_status() == crate::card::ImplementationStatus::Unsupported {
            return Err(DeckRequirementViolation::UnsupportedCard(
                definition.name.clone(),
            ));
        }
        let parts: Vec<_> = applicable_part_ids(definition, &CharacteristicContext::OutsideGame)
            .map_err(|_| DeckRequirementViolation::InvalidCharacteristics(definition.id))?
            .into_iter()
            .map(|id| definition.part(id).expect("validated part"))
            .collect();
        let mut types = CardTypeSet::empty();
        let mut subtypes = SubtypeSet::EMPTY;
        for part in &parts {
            types = types.union(part.rules.types());
            subtypes = subtypes.union(part.rules.subtype_set());
            if part.rules.has_keyword(KeywordAbility::Changeling) {
                subtypes = subtypes.union(SubtypeSet::from_names(crate::card::CREATURE_TYPES));
            }
            for effect in crate::card::self_characteristic_effects(&part.rules, None) {
                match effect {
                    AppliedEffectDef::Characteristic(CharacteristicOperationDef::CardTypes(
                        operation,
                    )) => {
                        types = match operation {
                            SetOperationDef::Add(added) => types.union(added),
                            SetOperationDef::Set(set) => set,
                            SetOperationDef::Remove(removed) => CardType::DISPLAY_ORDER
                                .into_iter()
                                .filter(|kind| types.contains(*kind) && !removed.contains(*kind))
                                .fold(CardTypeSet::empty(), CardTypeSet::with),
                        };
                    }
                    AppliedEffectDef::Characteristic(CharacteristicOperationDef::Subtypes(
                        operation,
                    )) => {
                        subtypes = match operation {
                            SetOperationDef::Add(added) => subtypes.union(added),
                            SetOperationDef::Set(set) => set,
                            SetOperationDef::Remove(removed) => subtypes.difference(removed),
                        };
                    }
                    _ => {}
                }
            }
        }
        Ok(Self {
            definition,
            parts,
            types,
            subtypes,
        })
    }
    fn has_type(&self, kind: CardType) -> bool {
        self.types.contains(kind)
    }
    fn matches(&self, filter: DeckCards) -> bool {
        match filter {
            DeckCards::All => true,
            DeckCards::Permanents => self.types.is_permanent(),
            DeckCards::Nonlands => !self.has_type(CardType::Land),
            DeckCards::OfType(kind) => self.has_type(kind),
        }
    }
    fn meets(&self, requirement: CardRequirement) -> bool {
        match requirement {
            CardRequirement::DistinctManaSymbols => {
                let mut symbols = std::collections::BTreeSet::new();
                self.parts
                    .iter()
                    .filter_map(|part| part.rules.mana_cost())
                    .all(|cost| {
                        cost.to_string()
                            .split('}')
                            .filter(|symbol| !symbol.is_empty())
                            .all(|symbol| symbols.insert(symbol.to_owned()))
                    })
            }
            CardRequirement::ManaValueAtMost(value) => self.definition.card_mana_value() <= value,
            CardRequirement::ManaValueAtLeast(value) => self.definition.card_mana_value() >= value,
            CardRequirement::HasActivatedAbility => self.parts.iter().any(|part| {
                (part.rules.has_type(CardType::Land)
                    && ["Plains", "Island", "Swamp", "Mountain", "Forest"]
                        .iter()
                        .any(|kind| part.rules.has_subtype(kind)))
                    || part.rules.ability_clauses().iter().any(|ability| {
                        matches!(
                            ability.definition,
                            DeclarativeAbilityDef::Activated(_)
                                | DeclarativeAbilityDef::ActivatedMana(_)
                        )
                    })
            }),
            CardRequirement::HasAnySubtype(types) => types.iter().any(|kind| {
                crate::card::Subtype::from_name(kind)
                    .is_some_and(|kind| self.subtypes.contains(kind))
            }),
        }
    }
}

#[cfg(test)]
mod companion_tests {
    use super::*;
    use crate::{Deck, card::cards};

    fn deck(main: Vec<CardDefinitionId>, companion: CardDefinitionId) -> Deck {
        Deck {
            main,
            sideboard: vec![companion],
            commanders: vec![],
        }
    }

    #[test]
    fn starting_deck_includes_commanders_but_allows_another_copy_of_the_companion() {
        let catalog = crate::poc::catalog().unwrap();
        let mut list = deck(vec![cards::MOUNTAIN], cards::LURRUS_OF_THE_DREAM_DEN);
        assert!(
            list.validate_companion(&catalog, cards::LURRUS_OF_THE_DREAM_DEN, Format::Cedh)
                .is_ok()
        );
        list.commanders.push(cards::KERUGA_THE_MACROSAGE);
        assert!(matches!(
            list.validate_companion(&catalog, cards::LURRUS_OF_THE_DREAM_DEN, Format::Cedh),
            Err(DeckRequirementViolation::CardDoesNotMeetRequirement { .. })
        ));
        let list = deck(
            vec![cards::KERUGA_THE_MACROSAGE],
            cards::KERUGA_THE_MACROSAGE,
        );
        assert!(
            list.validate_companion(&catalog, cards::KERUGA_THE_MACROSAGE, Format::VintageCube)
                .is_ok()
        );
    }

    #[test]
    fn zirda_uses_intrinsic_land_abilities_and_the_front_face() {
        let catalog = crate::poc::catalog().unwrap();
        let mut list = deck(
            vec![cards::MOUNTAIN, cards::INDATHA_TRIOME],
            cards::ZIRDA_THE_DAWNWAKER,
        );
        assert!(
            list.validate_companion(&catalog, cards::ZIRDA_THE_DAWNWAKER, Format::VintageCube)
                .is_ok()
        );
        list.main.push(cards::GROWING_RITES_OF_ITLIMOC);
        assert!(
            list.validate_companion(&catalog, cards::ZIRDA_THE_DAWNWAKER, Format::VintageCube)
                .is_err()
        );
    }

    #[test]
    fn mana_values_use_combined_split_cards_and_only_primary_adventure_characteristics() {
        let catalog = crate::poc::catalog().unwrap();
        let mut list = deck(
            vec![cards::FIRE_ICE, cards::BONECRUSHER_GIANT],
            cards::KERUGA_THE_MACROSAGE,
        );
        assert!(
            list.validate_companion(&catalog, cards::KERUGA_THE_MACROSAGE, Format::VintageCube)
                .is_ok()
        );
        list.main.push(cards::WALKING_BALLISTA);
        assert!(
            list.validate_companion(&catalog, cards::KERUGA_THE_MACROSAGE, Format::VintageCube)
                .is_err()
        );
        let list = deck(
            vec![cards::WALKING_BALLISTA],
            cards::LURRUS_OF_THE_DREAM_DEN,
        );
        assert!(
            list.validate_companion(
                &catalog,
                cards::LURRUS_OF_THE_DREAM_DEN,
                Format::VintageCube
            )
            .is_ok()
        );
    }

    #[test]
    fn subtype_requirements_honor_changeling_and_ignore_noncreatures() {
        let base = crate::poc::catalog().unwrap();
        let changeling = CardDefinitionId::from_uuid("00000000-0000-0000-0000-000000019998");
        let mut definitions = base.definitions().into_iter().cloned().collect::<Vec<_>>();
        definitions.push(crate::CardDefinition::new(
            changeling,
            "Test Changeling",
            crate::card::sets::alpha::SET,
            crate::CardRules::new_creature(crate::mana_cost!("{1}"), &["Shapeshifter"], 1, 1)
                .with_abilities(&const { [crate::card::abilities::changeling()] }),
        ));
        let catalog = CardCatalog::new(definitions).unwrap();
        let mut list = deck(
            vec![changeling, cards::SAVANNAH_LIONS, cards::LIGHTNING_BOLT],
            cards::KAHEERA_THE_ORPHANGUARD,
        );
        assert!(
            list.validate_companion(
                &catalog,
                cards::KAHEERA_THE_ORPHANGUARD,
                Format::VintageCube
            )
            .is_ok()
        );
        list.main.push(cards::GRIST_THE_HUNGER_TIDE);
        assert!(
            list.validate_companion(
                &catalog,
                cards::KAHEERA_THE_ORPHANGUARD,
                Format::VintageCube
            )
            .is_err(),
            "Grist is an Insect creature outside the game"
        );
        list.main.pop();
        list.main.push(cards::GRIZZLY_BEARS);
        let failure = list
            .validate_companion(
                &catalog,
                cards::KAHEERA_THE_ORPHANGUARD,
                Format::VintageCube,
            )
            .unwrap_err();
        assert!(failure.to_string().contains("Grizzly Bears"));
    }

    #[test]
    fn minimum_size_uses_the_format_and_counts_both_commanders() {
        let catalog = crate::poc::catalog().unwrap();
        let mut list = deck(vec![cards::MOUNTAIN; 59], cards::YORION_SKY_NOMAD);
        assert!(
            list.validate_companion(&catalog, cards::YORION_SKY_NOMAD, Format::VintageCube)
                .is_err()
        );
        list.main.push(cards::MOUNTAIN);
        assert!(
            list.validate_companion(&catalog, cards::YORION_SKY_NOMAD, Format::VintageCube)
                .is_ok()
        );
        assert!(
            list.validate_companion(&catalog, cards::YORION_SKY_NOMAD, Format::Legacy)
                .is_err()
        );
        list.main.resize(80, cards::MOUNTAIN);
        assert!(
            list.validate_companion(&catalog, cards::YORION_SKY_NOMAD, Format::Legacy)
                .is_ok()
        );
        list.main.resize(117, cards::MOUNTAIN);
        list.commanders = vec![cards::THRASIOS_TRITON_HERO, cards::TYMNA_THE_WEAVER];
        assert!(matches!(
            list.validate_companion(&catalog, cards::YORION_SKY_NOMAD, Format::Cedh),
            Err(DeckRequirementViolation::TooFewCards {
                actual: 119,
                minimum: 120
            })
        ));
    }

    #[test]
    fn duplicate_names_and_unknown_definitions_report_failures() {
        let catalog = crate::poc::catalog().unwrap();
        let mut list = deck(
            vec![cards::MOUNTAIN, cards::MOUNTAIN, cards::LIGHTNING_BOLT],
            cards::LUTRI_THE_SPELLCHASER,
        );
        assert!(
            list.validate_companion(&catalog, cards::LUTRI_THE_SPELLCHASER, Format::VintageCube)
                .is_ok()
        );
        list.main.push(cards::LIGHTNING_BOLT);
        assert_eq!(
            list.validate_companion(&catalog, cards::LUTRI_THE_SPELLCHASER, Format::VintageCube),
            Err(DeckRequirementViolation::DuplicateName(
                "Lightning Bolt".into()
            ))
        );
        let unknown = CardDefinitionId::from_uuid("00000000-0000-0000-0000-000000019999");
        let list = deck(vec![unknown], cards::LURRUS_OF_THE_DREAM_DEN);
        assert_eq!(
            list.validate_companion(
                &catalog,
                cards::LURRUS_OF_THE_DREAM_DEN,
                Format::VintageCube
            ),
            Err(DeckRequirementViolation::UnknownCard(unknown))
        );
    }
}

#[cfg(test)]
mod companion_policy_tests {
    use super::*;
    #[test]
    fn companion_only_bans_do_not_change_the_card_ability_or_deck_requirement() {
        let catalog = crate::poc::catalog().unwrap();
        let candidate = crate::card::cards::LUTRI_THE_SPELLCHASER;
        for format in [Format::Cedh, Format::DuelCommander] {
            assert!(matches!(
                validate_companion_requirement(
                    candidate,
                    &catalog,
                    [crate::card::cards::MOUNTAIN],
                    format
                ),
                Err(DeckRequirementViolation::CompanionBanned { .. })
            ));
        }
        assert!(
            validate_companion_requirement(
                candidate,
                &catalog,
                [crate::card::cards::MOUNTAIN],
                Format::VintageCube
            )
            .is_ok()
        );
    }
}
