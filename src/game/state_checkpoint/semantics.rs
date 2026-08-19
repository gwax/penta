use super::model::{
    AbilityLocator, AppliedEffectLocator, ManaPayloadLocator, ReplacementEffectLocator,
    ScopedEffectSnapshot,
};
use crate::card::BandingQuality;

use super::model_keyword::KeywordSnapshot;
use super::model_prevention::DamagePreventionLocator;
use super::{AbilityOrigin, AbilitySourceRef, Mana, ScopedEffect};
use crate::CardCatalog;
use crate::card::{
    AbilityDef, AbilityOperationDef, AbilityProgramDef, AbilityTargetDef, AddManaEffectDef,
    AppliedEffectDef, BasicLandType, CharacteristicOperationDef, DamagePreventionDef,
    DamageSourceMatcherDef, DeclarativeAbilityDef, EffectDef, KeywordAbility, ManaColor,
    ManaSpendEffectDef, ObjectPredicateDef, ProtectedCreatureType, ReplacementEffectDef,
    SpellAbilityDef,
};

pub(super) fn ability_locator(
    catalog: &CardCatalog,
    mut matches: impl FnMut(&AbilityDef) -> bool,
) -> Option<AbilityLocator> {
    for definition in catalog.definitions() {
        for part in &definition.parts {
            for attached in part.rules.indexed_abilities() {
                let mut nested = Vec::new();
                if locate_ability(&attached.definition, &mut matches, &mut nested) {
                    return Some(AbilityLocator {
                        definition: definition.id.0,
                        part_id: part.id.0,
                        ability_id: attached.id.0,
                        nested,
                    });
                }
            }
        }
    }
    None
}

pub(super) fn catalog_ability(
    catalog: &CardCatalog,
    locator: &AbilityLocator,
) -> Option<AbilityDef> {
    let mut current = *catalog
        .get(crate::CardDefinitionId(locator.definition))?
        .part(crate::CardPartId(locator.part_id))?
        .rules
        .ability(crate::AbilityId(locator.ability_id))?;
    for &index in &locator.nested {
        current = **child_abilities(&current).get(index)?;
    }
    Some(current)
}

pub(super) fn mana_payload_locator(
    catalog: &CardCatalog,
    mana: Mana,
) -> Option<ManaPayloadLocator> {
    if mana.restrictions.is_empty() && mana.spend_effects.is_empty() {
        return None;
    }
    let ability = ability_locator(catalog, |candidate| {
        mana_effects(candidate)
            .iter()
            .any(|effect| mana_effect_matches(*effect, mana))
    })?;
    let definition = catalog_ability(catalog, &ability)?;
    let effect_index = mana_effects(&definition)
        .iter()
        .position(|effect| mana_effect_matches(*effect, mana))?;
    Some(ManaPayloadLocator {
        ability,
        effect_index,
    })
}

pub(super) fn catalog_mana_payload(
    catalog: &CardCatalog,
    locator: &ManaPayloadLocator,
) -> Option<AddManaEffectDef> {
    let ability = catalog_ability(catalog, &locator.ability)?;
    mana_effects(&ability).get(locator.effect_index).copied()
}

pub(super) fn applied_effect_locator(
    catalog: &CardCatalog,
    expected: AppliedEffectDef,
) -> Option<AppliedEffectLocator> {
    let ability = ability_locator(catalog, |candidate| {
        applied_effects(candidate).contains(&expected)
    })?;
    let definition = catalog_ability(catalog, &ability)?;
    let effect_index = applied_effects(&definition)
        .iter()
        .position(|effect| *effect == expected)?;
    Some(AppliedEffectLocator {
        ability,
        effect_index,
    })
}

/// Locates a resolved leaf beneath the ability provenance that created it.
///
/// The runtime source identifies the exact top-level printed clause. Nested
/// abilities still use the first structurally equal path because the runtime
/// does not retain a nested catalog path, but the search never falls back to a
/// different top-level ability: that would make source-relative predicates
/// reconstruct with different semantics.
pub(super) fn resolved_applied_effect_locator(
    catalog: &CardCatalog,
    source: AbilitySourceRef,
    expected: AppliedEffectDef,
) -> Option<AppliedEffectLocator> {
    let (definition, part_id, ability_id) = match source.ability {
        AbilityOrigin::Printed {
            definition,
            part,
            ability,
        } => (definition.0, part.0, ability.0),
        AbilityOrigin::Granted {
            source_definition,
            source_part,
            source_ability,
            ..
        } => (source_definition.0, source_part.0, source_ability.0),
        AbilityOrigin::IntrinsicBasicLand(_) => return None,
    };
    let root = AbilityLocator {
        definition,
        part_id,
        ability_id,
        nested: Vec::new(),
    };
    let root_definition = catalog_ability(catalog, &root)?;
    let mut nested = Vec::new();
    let mut contains = |candidate: &AbilityDef| applied_effects(candidate).contains(&expected);
    if !locate_ability(&root_definition, &mut contains, &mut nested) {
        return None;
    }
    let ability = AbilityLocator { nested, ..root };
    let definition = catalog_ability(catalog, &ability)?;
    let effect_index = applied_effects(&definition)
        .iter()
        .position(|effect| *effect == expected)?;
    Some(AppliedEffectLocator {
        ability,
        effect_index,
    })
}

pub(super) fn applied_effect_locator_matches_source(
    locator: &AppliedEffectLocator,
    source: AbilitySourceRef,
) -> bool {
    let expected = match source.ability {
        AbilityOrigin::Printed {
            definition,
            part,
            ability,
        } => (definition.0, part.0, ability.0),
        AbilityOrigin::Granted {
            source_definition,
            source_part,
            source_ability,
            ..
        } => (source_definition.0, source_part.0, source_ability.0),
        AbilityOrigin::IntrinsicBasicLand(_) => return false,
    };
    (
        locator.ability.definition,
        locator.ability.part_id,
        locator.ability.ability_id,
    ) == expected
}

pub(super) fn catalog_applied_effect(
    catalog: &CardCatalog,
    locator: &AppliedEffectLocator,
) -> Option<AppliedEffectDef> {
    let ability = catalog_ability(catalog, &locator.ability)?;
    applied_effects(&ability).get(locator.effect_index).copied()
}

pub(super) fn resolved_damage_prevention_locator(
    catalog: &CardCatalog,
    source: AbilitySourceRef,
    predicate: ObjectPredicateDef,
) -> Option<DamagePreventionLocator> {
    let expected = DamageSourceMatcherDef::Matching(predicate);
    let (definition, part_id, ability_id) = match source.ability {
        AbilityOrigin::Printed {
            definition,
            part,
            ability,
        } => (definition.0, part.0, ability.0),
        AbilityOrigin::Granted {
            source_definition,
            source_part,
            source_ability,
            ..
        } => (source_definition.0, source_part.0, source_ability.0),
        AbilityOrigin::IntrinsicBasicLand(_) => return None,
    };
    let root = AbilityLocator {
        definition,
        part_id,
        ability_id,
        nested: Vec::new(),
    };
    let root_definition = catalog_ability(catalog, &root)?;
    let mut nested = Vec::new();
    let mut contains = |candidate: &AbilityDef| {
        damage_prevention_defs(candidate)
            .iter()
            .any(|prevention| prevention.matcher.source == expected)
    };
    if !locate_ability(&root_definition, &mut contains, &mut nested) {
        return None;
    }
    let ability = AbilityLocator { nested, ..root };
    let definition = catalog_ability(catalog, &ability)?;
    let effect_index = damage_prevention_defs(&definition)
        .iter()
        .position(|prevention| prevention.matcher.source == expected)?;
    Some(DamagePreventionLocator {
        ability,
        effect_index,
    })
}

pub(super) fn catalog_damage_prevention(
    catalog: &CardCatalog,
    locator: &DamagePreventionLocator,
) -> Option<DamagePreventionDef> {
    let ability = catalog_ability(catalog, &locator.ability)?;
    damage_prevention_defs(&ability)
        .get(locator.effect_index)
        .copied()
}

fn damage_prevention_defs(ability: &AbilityDef) -> Vec<DamagePreventionDef> {
    let mut found = Vec::new();
    match ability.effect.definition {
        AbilityProgramDef::Effects(effect) => {
            collect_damage_prevention_defs(effect, &mut found);
        }
        AbilityProgramDef::Replacement(effect) => {
            for child in replacement_child_effects(effect) {
                collect_damage_prevention_defs(child, &mut found);
            }
        }
    }
    found
}

fn collect_damage_prevention_defs(effect: EffectDef, found: &mut Vec<DamagePreventionDef>) {
    if let EffectDef::PreventDamage { prevention, .. } = effect {
        found.push(prevention);
    }
    for child in child_effects(effect) {
        collect_damage_prevention_defs(child, found);
    }
}

pub(super) fn applied_effects(ability: &AbilityDef) -> Vec<AppliedEffectDef> {
    let mut found = Vec::new();
    match ability.effect.definition {
        AbilityProgramDef::Effects(effect) => {
            collect_applied_effects_from_effect(effect, &mut found);
        }
        AbilityProgramDef::Replacement(effect) => {
            for child in replacement_child_effects(effect) {
                collect_applied_effects_from_effect(child, &mut found);
            }
        }
    }
    for mana in mana_effects(ability) {
        for spend in mana.spend_effects {
            if let ManaSpendEffectDef::ApplyToPaidSpell(effect) = *spend {
                collect_applied_effect(effect, &mut found);
            }
        }
    }
    found
}

fn collect_applied_effects_from_effect(effect: EffectDef, found: &mut Vec<AppliedEffectDef>) {
    // Every effect that carries a rider, not just the one that is nothing but
    // a rider: a damage clause with one attached leaves a resolved effect on
    // the battlefield that has to be locatable again.
    match effect {
        EffectDef::Apply {
            effect: applied, ..
        }
        | EffectDef::DealDamageAndApply { applied, .. } => collect_applied_effect(applied, found),
        _ => {}
    }
    for child in child_effects(effect) {
        collect_applied_effects_from_effect(child, found);
    }
}

fn collect_applied_effect(effect: AppliedEffectDef, found: &mut Vec<AppliedEffectDef>) {
    found.push(effect);
    if let AppliedEffectDef::Composite(children) = effect {
        for child in children {
            collect_applied_effect(*child, found);
        }
    }
}

pub(super) fn scoped_effect_snapshot(
    ability: &AbilityDef,
    effect: ScopedEffect,
) -> Option<ScopedEffectSnapshot> {
    let mut path = Vec::new();
    let found = match ability.effect.definition {
        AbilityProgramDef::Effects(definition) => {
            locate_effect(definition, effect.effect, &mut path)
        }
        AbilityProgramDef::Replacement(replacement) => replacement_child_effects(replacement)
            .into_iter()
            .enumerate()
            .any(|(index, root)| {
                path.push(index);
                if locate_effect(root, effect.effect, &mut path) {
                    true
                } else {
                    path.pop();
                    false
                }
            }),
    };
    found.then_some(ScopedEffectSnapshot {
        path,
        target_base: effect.target_base,
    })
}

pub(super) fn catalog_scoped_effect(
    catalog: &CardCatalog,
    ability: &AbilityLocator,
    snapshot: &ScopedEffectSnapshot,
) -> Option<ScopedEffect> {
    let ability = catalog_ability(catalog, ability)?;
    let (mut effect, path) = match ability.effect.definition {
        AbilityProgramDef::Effects(effect) => (effect, snapshot.path.as_slice()),
        AbilityProgramDef::Replacement(replacement) => {
            let (&root, path) = snapshot.path.split_first()?;
            (*replacement_child_effects(replacement).get(root)?, path)
        }
    };
    for &index in path {
        effect = *child_effects(effect).get(index)?;
    }
    Some(ScopedEffect {
        effect,
        target_base: snapshot.target_base,
    })
}

#[cfg(test)]
pub(super) fn replacement_effect_locator(
    catalog: &CardCatalog,
    expected: ReplacementEffectDef,
) -> Option<ReplacementEffectLocator> {
    let ability = ability_locator(catalog, |candidate| {
        replacement_effects(candidate)
            .into_iter()
            .any(|effect| effect == expected)
    })?;
    let definition = catalog_ability(catalog, &ability)?;
    let effect_index = replacement_effects(&definition)
        .into_iter()
        .position(|effect| effect == expected)?;
    Some(ReplacementEffectLocator {
        ability,
        effect_index,
    })
}

/// Locates a replacement operation beneath the exact printed ability that
/// supplied the suspended prospective-event procedure.
pub(super) fn resolved_replacement_effect_locator(
    catalog: &CardCatalog,
    source: AbilitySourceRef,
    expected: ReplacementEffectDef,
) -> Option<ReplacementEffectLocator> {
    let (definition, part_id, ability_id) = match source.ability {
        AbilityOrigin::Printed {
            definition,
            part,
            ability,
        } => (definition.0, part.0, ability.0),
        AbilityOrigin::Granted {
            source_definition,
            source_part,
            source_ability,
            ..
        } => (source_definition.0, source_part.0, source_ability.0),
        AbilityOrigin::IntrinsicBasicLand(_) => return None,
    };
    let root = AbilityLocator {
        definition,
        part_id,
        ability_id,
        nested: Vec::new(),
    };
    let root_definition = catalog_ability(catalog, &root)?;
    let mut nested = Vec::new();
    let mut contains = |candidate: &AbilityDef| {
        replacement_effects(candidate)
            .into_iter()
            .any(|effect| effect == expected)
    };
    if !locate_ability(&root_definition, &mut contains, &mut nested) {
        return None;
    }
    let ability = AbilityLocator { nested, ..root };
    let definition = catalog_ability(catalog, &ability)?;
    let effect_index = replacement_effects(&definition)
        .into_iter()
        .position(|effect| effect == expected)?;
    Some(ReplacementEffectLocator {
        ability,
        effect_index,
    })
}

pub(super) fn replacement_effect_locator_matches_source(
    locator: &ReplacementEffectLocator,
    source: AbilitySourceRef,
) -> bool {
    let expected = match source.ability {
        AbilityOrigin::Printed {
            definition,
            part,
            ability,
        } => (definition.0, part.0, ability.0),
        AbilityOrigin::Granted {
            source_definition,
            source_part,
            source_ability,
            ..
        } => (source_definition.0, source_part.0, source_ability.0),
        AbilityOrigin::IntrinsicBasicLand(_) => return false,
    };
    (
        locator.ability.definition,
        locator.ability.part_id,
        locator.ability.ability_id,
    ) == expected
}

pub(super) fn catalog_replacement_effect(
    catalog: &CardCatalog,
    locator: &ReplacementEffectLocator,
) -> Option<ReplacementEffectDef> {
    let ability = catalog_ability(catalog, &locator.ability)?;
    replacement_effects(&ability)
        .get(locator.effect_index)
        .copied()
}

pub(super) fn replacement_effects(ability: &AbilityDef) -> Vec<ReplacementEffectDef> {
    let mut effects = Vec::new();
    if let AbilityProgramDef::Replacement(effect) = ability.effect.definition {
        collect_replacement_effects(effect, &mut effects);
    }
    effects
}

fn collect_replacement_effects(
    effect: ReplacementEffectDef,
    found: &mut Vec<ReplacementEffectDef>,
) {
    found.push(effect);
    match effect {
        ReplacementEffectDef::Sequence(effects) => {
            for effect in effects {
                collect_replacement_effects(*effect, found);
            }
        }
        ReplacementEffectDef::Conditional {
            if_true, if_false, ..
        } => {
            for effect in if_true.iter().chain(if_false.iter()) {
                collect_replacement_effects(*effect, found);
            }
        }
        ReplacementEffectDef::PayOr {
            if_paid,
            if_declined,
            ..
        } => {
            for effect in if_paid.iter().chain(if_declined.iter()) {
                collect_replacement_effects(*effect, found);
            }
        }
        ReplacementEffectDef::ReplaceEventWithNothing
        | ReplacementEffectDef::MoveToZone(_)
        | ReplacementEffectDef::Perform(_)
        | ReplacementEffectDef::ModifyBattlefieldEntry(_)
        | ReplacementEffectDef::MultiplyEventAmount(_)
        | ReplacementEffectDef::Choose(_)
        | ReplacementEffectDef::CopyEntering { .. } => {}
    }
}

fn locate_effect(current: EffectDef, needle: EffectDef, path: &mut Vec<usize>) -> bool {
    if current == needle {
        return true;
    }
    for (index, child) in child_effects(current).into_iter().enumerate() {
        path.push(index);
        if locate_effect(child, needle, path) {
            return true;
        }
        path.pop();
    }
    false
}

pub(super) fn child_effects(effect: EffectDef) -> Vec<EffectDef> {
    match effect {
        EffectDef::Sequence(effects) => effects.to_vec(),
        EffectDef::Randomized {
            on_success,
            on_failure,
            ..
        } => vec![*on_success, *on_failure],
        EffectDef::Choose(choice) => vec![*choice.then],
        EffectDef::PayOr(payment) => payment
            .if_paid
            .iter()
            .chain(payment.otherwise.iter())
            .copied()
            .copied()
            .collect(),
        EffectDef::SplitIntoPiles(partition) => vec![*partition.then],
        EffectDef::May {
            effect: otherwise, ..
        }
        | EffectDef::ReplaceNextDrawThisTurn {
            effect: otherwise, ..
        }
        | EffectDef::IfCondition {
            then: otherwise, ..
        } => vec![*otherwise],
        EffectDef::IfFormat {
            then, otherwise, ..
        } => vec![*then, *otherwise],
        EffectDef::SacrificeOfChoice {
            then: Some(effect), ..
        } => vec![*effect],
        EffectDef::LookAtTopAndSelect { selection, .. } => {
            selection.then.into_iter().copied().collect()
        }
        EffectDef::SearchZone {
            then: Some(then), ..
        }
        | EffectDef::ChooseCardName { then, .. }
        | EffectDef::BindMatching { then, .. } => {
            vec![*then]
        }
        EffectDef::Discard {
            then: Some(follow_up),
            ..
        } => vec![*follow_up.effect],
        EffectDef::RevealAtRandomFromHand { then, .. } => vec![*then],
        _ => Vec::new(),
    }
}

pub(super) fn replacement_child_effects(effect: ReplacementEffectDef) -> Vec<EffectDef> {
    match effect {
        ReplacementEffectDef::Sequence(effects) => effects
            .iter()
            .flat_map(|effect| replacement_child_effects(*effect))
            .collect(),
        ReplacementEffectDef::Perform(effect) => vec![*effect],
        ReplacementEffectDef::Conditional {
            if_true, if_false, ..
        } => if_true
            .iter()
            .chain(if_false.iter())
            .flat_map(|effect| replacement_child_effects(*effect))
            .collect(),
        ReplacementEffectDef::PayOr {
            if_paid,
            if_declined,
            ..
        } => if_paid
            .iter()
            .chain(if_declined.iter())
            .flat_map(|effect| replacement_child_effects(*effect))
            .collect(),
        ReplacementEffectDef::ReplaceEventWithNothing
        | ReplacementEffectDef::MoveToZone(_)
        | ReplacementEffectDef::ModifyBattlefieldEntry(_)
        | ReplacementEffectDef::MultiplyEventAmount(_)
        | ReplacementEffectDef::Choose(_)
        | ReplacementEffectDef::CopyEntering { .. } => Vec::new(),
    }
}

fn mana_effect_matches(effect: AddManaEffectDef, mana: Mana) -> bool {
    effect.restrictions == mana.restrictions
        && effect.spend_effects == mana.spend_effects
        && match effect.mana {
            crate::card::ManaSelectionDef::One(color) => color == mana.color,
            crate::card::ManaSelectionDef::Choice(colors) => colors.contains(&mana.color),
        }
}

pub(super) fn mana_effects(ability: &AbilityDef) -> Vec<AddManaEffectDef> {
    let mut effects = Vec::new();
    match ability.effect.definition {
        AbilityProgramDef::Effects(effect) => collect_mana_effects(effect, &mut effects),
        AbilityProgramDef::Replacement(effect) => {
            for child in replacement_child_effects(effect) {
                collect_mana_effects(child, &mut effects);
            }
        }
    }
    effects
}

fn collect_mana_effects(effect: EffectDef, found: &mut Vec<AddManaEffectDef>) {
    if let EffectDef::AddMana(mana) = effect {
        found.push(mana);
    }
    for child in child_effects(effect) {
        collect_mana_effects(child, found);
    }
}

fn locate_ability(
    ability: &AbilityDef,
    matches: &mut impl FnMut(&AbilityDef) -> bool,
    path: &mut Vec<usize>,
) -> bool {
    if matches(ability) {
        return true;
    }
    for (index, child) in child_abilities(ability).into_iter().enumerate() {
        path.push(index);
        if locate_ability(child, matches, path) {
            return true;
        }
        path.pop();
    }
    false
}

pub(super) fn child_abilities(ability: &AbilityDef) -> Vec<&AbilityDef> {
    let mut children = Vec::new();
    if let DeclarativeAbilityDef::Spell(SpellAbilityDef::Modal(modal)) = ability.definition {
        children.extend(modal.modes);
    }
    match ability.effect.definition {
        AbilityProgramDef::Effects(effect) => collect_effect_abilities(effect, &mut children),
        AbilityProgramDef::Replacement(effect) => {
            for child in replacement_child_effects(effect) {
                collect_effect_abilities(child, &mut children);
            }
        }
    }
    children
}

pub(super) const fn ability_target_defs(ability: &AbilityDef) -> &'static [AbilityTargetDef] {
    match ability.definition {
        DeclarativeAbilityDef::Spell(spell) => spell.targets(),
        DeclarativeAbilityDef::ActivatedMana(activated)
        | DeclarativeAbilityDef::Activated(activated) => activated.targets,
        DeclarativeAbilityDef::TriggeredMana(triggered)
        | DeclarativeAbilityDef::Triggered(triggered) => triggered.targets,
        DeclarativeAbilityDef::Static(_)
        | DeclarativeAbilityDef::Replacement(_)
        | DeclarativeAbilityDef::AlternativeCast(_)
        | DeclarativeAbilityDef::SpecialAction(_)
        | DeclarativeAbilityDef::Keyword(_)
        | DeclarativeAbilityDef::Legacy => &[],
    }
}

fn collect_effect_abilities(effect: EffectDef, abilities: &mut Vec<&'static AbilityDef>) {
    match effect {
        EffectDef::Apply { effect, .. } | EffectDef::StaticApply { effect, .. } => {
            collect_applied_abilities(effect, abilities);
        }
        EffectDef::DealDamageAndApply { applied, .. } => {
            collect_applied_abilities(applied, abilities);
        }
        EffectDef::InstallTrigger(installed) => abilities.push(installed.ability),
        _ => {}
    }
    for child in child_effects(effect) {
        collect_effect_abilities(child, abilities);
    }
}
fn collect_applied_abilities(effect: AppliedEffectDef, abilities: &mut Vec<&'static AbilityDef>) {
    match effect {
        AppliedEffectDef::Composite(effects) => {
            for effect in effects {
                collect_applied_abilities(*effect, abilities);
            }
        }
        AppliedEffectDef::Characteristic(CharacteristicOperationDef::Abilities(
            AbilityOperationDef::Add(ability),
        )) => abilities.push(ability),
        AppliedEffectDef::Rule(_) | AppliedEffectDef::Characteristic(_) => {}
    }
}

pub(super) const fn keyword_snapshot(keyword: KeywordAbility) -> KeywordSnapshot {
    match keyword {
        KeywordAbility::Flying => KeywordSnapshot::Flying,
        KeywordAbility::Trample => KeywordSnapshot::Trample,
        KeywordAbility::Haste => KeywordSnapshot::Haste,
        KeywordAbility::FirstStrike => KeywordSnapshot::FirstStrike,
        KeywordAbility::DoubleStrike => KeywordSnapshot::DoubleStrike,
        KeywordAbility::Banding => KeywordSnapshot::Banding,
        KeywordAbility::BandsWithOther(BandingQuality::LegendaryCreatures) => {
            KeywordSnapshot::BandsWithOtherLegendaryCreatures
        }
        KeywordAbility::BandsWithOther(BandingQuality::WolvesOfTheHunt) => {
            KeywordSnapshot::BandsWithOtherWolvesOfTheHunt
        }
        KeywordAbility::Vigilance => KeywordSnapshot::Vigilance,
        KeywordAbility::Defender => KeywordSnapshot::Defender,
        KeywordAbility::Deathtouch => KeywordSnapshot::Deathtouch,
        KeywordAbility::Lifelink => KeywordSnapshot::Lifelink,
        KeywordAbility::Reach => KeywordSnapshot::Reach,
        KeywordAbility::Flash => KeywordSnapshot::Flash,
        KeywordAbility::Hexproof => KeywordSnapshot::Hexproof,
        KeywordAbility::Shroud => KeywordSnapshot::Shroud,
        KeywordAbility::Unleash => KeywordSnapshot::Unleash,
        KeywordAbility::Intimidate => KeywordSnapshot::Intimidate,
        KeywordAbility::Menace => KeywordSnapshot::Menace,
        KeywordAbility::Undying => KeywordSnapshot::Undying,
        KeywordAbility::Indestructible => KeywordSnapshot::Indestructible,
        KeywordAbility::AttacksEachCombatIfAble => KeywordSnapshot::AttacksEachCombatIfAble,
        KeywordAbility::LegendaryLandwalk => KeywordSnapshot::LegendaryLandwalk,
        KeywordAbility::Landwalk(BasicLandType::Plains) => KeywordSnapshot::Plainswalk,
        KeywordAbility::Landwalk(BasicLandType::Island) => KeywordSnapshot::Islandwalk,
        KeywordAbility::Landwalk(BasicLandType::Swamp) => KeywordSnapshot::Swampwalk,
        KeywordAbility::Landwalk(BasicLandType::Mountain) => KeywordSnapshot::Mountainwalk,
        KeywordAbility::Landwalk(BasicLandType::Forest) => KeywordSnapshot::Forestwalk,
        KeywordAbility::ProtectionFrom(ManaColor::White) => KeywordSnapshot::ProtectionFromWhite,
        KeywordAbility::ProtectionFrom(ManaColor::Blue) => KeywordSnapshot::ProtectionFromBlue,
        KeywordAbility::ProtectionFrom(ManaColor::Black) => KeywordSnapshot::ProtectionFromBlack,
        KeywordAbility::ProtectionFrom(ManaColor::Red) => KeywordSnapshot::ProtectionFromRed,
        KeywordAbility::ProtectionFrom(ManaColor::Green) => KeywordSnapshot::ProtectionFromGreen,
        KeywordAbility::ProtectionFrom(ManaColor::Colorless) => {
            KeywordSnapshot::ProtectionFromColorless
        }
        KeywordAbility::ProtectionFromCreatureType(ProtectedCreatureType::Zombie) => {
            KeywordSnapshot::ProtectionFromZombies
        }
        KeywordAbility::ProtectionFromCreatureType(ProtectedCreatureType::Vampire) => {
            KeywordSnapshot::ProtectionFromVampires
        }
        KeywordAbility::ProtectionFromCreatureType(ProtectedCreatureType::Werewolf) => {
            KeywordSnapshot::ProtectionFromWerewolves
        }
        KeywordAbility::ProtectionFromCreatures => KeywordSnapshot::ProtectionFromCreatures,
        KeywordAbility::ProtectionFromMulticolored => KeywordSnapshot::ProtectionFromMulticolored,
    }
}

pub(super) const fn parse_keyword(value: KeywordSnapshot) -> KeywordAbility {
    match value {
        KeywordSnapshot::Flying => KeywordAbility::Flying,
        KeywordSnapshot::Trample => KeywordAbility::Trample,
        KeywordSnapshot::Haste => KeywordAbility::Haste,
        KeywordSnapshot::FirstStrike => KeywordAbility::FirstStrike,
        KeywordSnapshot::DoubleStrike => KeywordAbility::DoubleStrike,
        KeywordSnapshot::Banding => KeywordAbility::Banding,
        KeywordSnapshot::BandsWithOtherLegendaryCreatures => {
            KeywordAbility::BandsWithOther(BandingQuality::LegendaryCreatures)
        }
        KeywordSnapshot::BandsWithOtherWolvesOfTheHunt => {
            KeywordAbility::BandsWithOther(BandingQuality::WolvesOfTheHunt)
        }
        KeywordSnapshot::Vigilance => KeywordAbility::Vigilance,
        KeywordSnapshot::Defender => KeywordAbility::Defender,
        KeywordSnapshot::Deathtouch => KeywordAbility::Deathtouch,
        KeywordSnapshot::Lifelink => KeywordAbility::Lifelink,
        KeywordSnapshot::Reach => KeywordAbility::Reach,
        KeywordSnapshot::Flash => KeywordAbility::Flash,
        KeywordSnapshot::Hexproof => KeywordAbility::Hexproof,
        KeywordSnapshot::Shroud => KeywordAbility::Shroud,
        KeywordSnapshot::Unleash => KeywordAbility::Unleash,
        KeywordSnapshot::Intimidate => KeywordAbility::Intimidate,
        KeywordSnapshot::Menace => KeywordAbility::Menace,
        KeywordSnapshot::Undying => KeywordAbility::Undying,
        KeywordSnapshot::Indestructible => KeywordAbility::Indestructible,
        KeywordSnapshot::AttacksEachCombatIfAble => KeywordAbility::AttacksEachCombatIfAble,
        KeywordSnapshot::LegendaryLandwalk => KeywordAbility::LegendaryLandwalk,
        KeywordSnapshot::Plainswalk => KeywordAbility::Landwalk(BasicLandType::Plains),
        KeywordSnapshot::Islandwalk => KeywordAbility::Landwalk(BasicLandType::Island),
        KeywordSnapshot::Swampwalk => KeywordAbility::Landwalk(BasicLandType::Swamp),
        KeywordSnapshot::Mountainwalk => KeywordAbility::Landwalk(BasicLandType::Mountain),
        KeywordSnapshot::Forestwalk => KeywordAbility::Landwalk(BasicLandType::Forest),
        KeywordSnapshot::ProtectionFromWhite => KeywordAbility::ProtectionFrom(ManaColor::White),
        KeywordSnapshot::ProtectionFromBlue => KeywordAbility::ProtectionFrom(ManaColor::Blue),
        KeywordSnapshot::ProtectionFromBlack => KeywordAbility::ProtectionFrom(ManaColor::Black),
        KeywordSnapshot::ProtectionFromRed => KeywordAbility::ProtectionFrom(ManaColor::Red),
        KeywordSnapshot::ProtectionFromGreen => KeywordAbility::ProtectionFrom(ManaColor::Green),
        KeywordSnapshot::ProtectionFromCreatures => KeywordAbility::ProtectionFromCreatures,
        KeywordSnapshot::ProtectionFromMulticolored => KeywordAbility::ProtectionFromMulticolored,
        KeywordSnapshot::ProtectionFromZombies => {
            KeywordAbility::ProtectionFromCreatureType(ProtectedCreatureType::Zombie)
        }
        KeywordSnapshot::ProtectionFromVampires => {
            KeywordAbility::ProtectionFromCreatureType(ProtectedCreatureType::Vampire)
        }
        KeywordSnapshot::ProtectionFromWerewolves => {
            KeywordAbility::ProtectionFromCreatureType(ProtectedCreatureType::Werewolf)
        }
        KeywordSnapshot::ProtectionFromColorless => {
            KeywordAbility::ProtectionFrom(ManaColor::Colorless)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{EffectRecipientDef, ResolvedEffectDurationDef, ValueDef};

    static GRANTED: AbilityDef = AbilityDef::not_implemented(
        "A nested ability.",
        "Only structural checkpoint traversal matters in this fixture.",
    );
    static APPLIED: [AppliedEffectDef; 2] = [
        AppliedEffectDef::add_ability(&GRANTED),
        AppliedEffectDef::set_base_power_toughness(ValueDef::Constant(3), ValueDef::Constant(3)),
    ];
    static PERFORM: EffectDef = EffectDef::Apply {
        recipient: EffectRecipientDef::Source,
        effect: AppliedEffectDef::Composite(&APPLIED),
        duration: ResolvedEffectDurationDef::UntilEndOfTurn,
    };
    static PROGRAM: [ReplacementEffectDef; 1] = [ReplacementEffectDef::Perform(&PERFORM)];
    static OUTER: AbilityDef = AbilityDef::replacement(
        "Perform nested definitions while replacing an event.",
        ReplacementEffectDef::Sequence(&PROGRAM),
    );

    #[test]
    fn checkpoint_semantic_walkers_descend_replacement_programs() {
        assert_eq!(child_abilities(&OUTER), vec![&GRANTED]);
        assert!(applied_effects(&OUTER).contains(&APPLIED[1]));
    }
}
