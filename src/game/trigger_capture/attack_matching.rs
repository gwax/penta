// Whether a committed attack event answers a printed attack matcher.
//
// Split from the capture logic because the two questions are different: one
// is about a creature that attacked, the other about the declaration as a
// whole. Included textually into `trigger_capture.rs`, so the imports here
// are the parent module's.

impl Game {
    /// "Whenever you attack." Counted rather than matched: the declaration is
    /// the event, and the predicate says which of the creatures in it count
    /// toward the size the clause asks for.
    fn attack_declaration_matches(
        &self,
        attacker: ObjectPredicateDef,
        declaration: AttackDeclarationRangeDef,
        attackers: &[TriggerEventObject],
        source: GameObjectId,
        controller: Option<PlayerId>,
    ) -> bool {
        let matching = attackers
            .iter()
            .filter(|object| {
                self.trigger_object_matches_for_controller(
                    attacker, object, source, false, controller,
                )
            })
            .count();
        let matching = u8::try_from(matching).unwrap_or(u8::MAX);
        matching >= declaration.minimum
            && declaration
                .maximum
                .is_none_or(|maximum| matching <= maximum)
    }

    /// "Whenever this creature attacks", and the clauses that narrow it by
    /// how big the declaration was or how many times this creature has
    /// attacked. Both numbers are facts of the event rather than conditions
    /// to recheck while the trigger is placed.
    fn attacker_matches(
        &self,
        matcher: AttackEventMatcherDef,
        object: &TriggerEventObject,
        declaration_size: u8,
        attack_number: u8,
        source: GameObjectId,
        controller: Option<PlayerId>,
    ) -> bool {
        declaration_size >= matcher.declaration.minimum
            && matcher
                .declaration
                .maximum
                .is_none_or(|maximum| declaration_size <= maximum)
            && matcher
                .attack_number
                .is_none_or(|number| attack_number == number)
            && self.trigger_object_matches_for_controller(
                matcher.attacker,
                object,
                source,
                false,
                controller,
            )
    }
}
