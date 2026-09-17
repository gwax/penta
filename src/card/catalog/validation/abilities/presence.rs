// A condition on membership is read before ability discovery, never during
// resolution. Keep unsupported zones and recursive conditions out of this slice.
fn validate_ability_presence(ability: &AbilityDef) -> Result<(), GrantedAbilityValidationError> {
    if ability.presence.is_some() {
        let battlefield = match ability.definition {
            DeclarativeAbilityDef::Activated(definition)
            | DeclarativeAbilityDef::ActivatedMana(definition) => {
                definition.source_zones == [ZoneKind::Battlefield]
            }
            DeclarativeAbilityDef::Triggered(definition)
            | DeclarativeAbilityDef::TriggeredMana(definition) => {
                definition.source_zones == [ZoneKind::Battlefield]
            }
            DeclarativeAbilityDef::Static(definition) => {
                definition.source_zones == [ZoneKind::Battlefield]
                    && definition.defines_colors.is_none()
                    && !ability
                        .declarative_effect()
                        .is_some_and(presence_program_defines_characteristics)
            }
            DeclarativeAbilityDef::Keyword(keyword) => !matches!(
                keyword,
                crate::card::KeywordAbility::Flash
                    | crate::card::KeywordAbility::Convoke
                    | crate::card::KeywordAbility::Delve
                    | crate::card::KeywordAbility::Improvise
                    | crate::card::KeywordAbility::Devoid
                    | crate::card::KeywordAbility::Changeling
                    | crate::card::KeywordAbility::Compleated
                    | crate::card::KeywordAbility::Unleash
                    | crate::card::KeywordAbility::SplitSecond
                    | crate::card::KeywordAbility::Suspend(_)
                    | crate::card::KeywordAbility::Rebound
            ),
            _ => false,
        };
        let mut presence = ability.presence;
        while let Some(group) = presence {
            if !battlefield
                || !super::program_context::ability_presence_condition_supported(*group.condition)
            {
                return Err(
                    GrantedAbilityValidationError::UnsupportedEffectProgramContext {
                        context: "ability presence",
                        operation: "requires a battlefield ability and a nonrecursive static condition",
                    },
                );
            }
            presence = *group.inherited;
        }
    }
    Ok(())
}

// Characteristic-defining programs also run outside the battlefield, where
// conditional battlefield membership has no supported discovery path yet.
fn presence_program_defines_characteristics(effect: EffectDef) -> bool {
    fn applied_defines(effect: AppliedEffectDef) -> bool {
        match effect {
            AppliedEffectDef::Characteristic(CharacteristicOperationDef::PowerToughness(
                crate::card::PowerToughnessOperationDef::Define { .. },
            )) => true,
            AppliedEffectDef::Composite(effects) => effects.iter().copied().any(applied_defines),
            _ => false,
        }
    }
    let local = match effect {
        EffectDef::StaticApply { effect, .. } => applied_defines(effect),
        EffectDef::ConditionalStatic(conditional) => applied_defines(conditional.then.effect),
        _ => false,
    };
    local
        || crate::card::child_effects(effect)
            .into_iter()
            .any(presence_program_defines_characteristics)
}
