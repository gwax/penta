fn static_damage_matcher_supported(matcher: DamageEventMatcherDef) -> bool {
    let source = match matcher.source {
        DamageSourceMatcherDef::Any
        | DamageSourceMatcherDef::Group(_)
        | DamageSourceMatcherDef::AffectedObject => true,
        DamageSourceMatcherDef::Object(reference) | DamageSourceMatcherDef::Except(reference) => {
            static_damage_object_reference_supported(reference)
        }
        DamageSourceMatcherDef::Matching(predicate) => static_object_predicate_supported(predicate),
    };
    let recipient = match matcher.recipient {
        DamageRecipientMatcherDef::Any | DamageRecipientMatcherDef::AffectedObject => true,
        DamageRecipientMatcherDef::Recipients(recipients) => recipients
            .object_reference()
            .is_some_and(static_damage_object_reference_supported),
        DamageRecipientMatcherDef::MatchingObject(predicate) => {
            static_object_predicate_supported(predicate)
        }
        DamageRecipientMatcherDef::PlayerAndCreaturesControlledBy(_)
        | DamageRecipientMatcherDef::PlayerOrPlaneswalker => false,
    };
    source && recipient
}

fn static_damage_object_reference_supported(reference: ObjectRefDef) -> bool {
    matches!(
        reference,
        ObjectRefDef::Source | ObjectRefDef::AttachedToSource
    )
}
