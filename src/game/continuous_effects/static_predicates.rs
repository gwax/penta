impl Game {
    /// Matches the common characteristic predicates used by live static
    /// effects without eagerly assembling every characteristic layer.
    ///
    /// The general trigger matcher still owns the complete predicate
    /// vocabulary. Falling back to its snapshot preserves that behavior for
    /// predicates that need values such as power, keywords, or battlefield
    /// relationships; simple type, color, and subtype queries only compute
    /// the layers they actually inspect.
    fn static_object_predicate_matches(
        &self,
        predicate: ObjectPredicateDef,
        source: &Permanent,
        affected: &Permanent,
        prospective: Option<&Permanent>,
    ) -> bool {
        self.static_object_predicate_matches_lazily(predicate, source, affected, prospective)
            .unwrap_or_else(|| {
                self.trigger_object_matches(
                    predicate,
                    &prospective.map_or_else(
                        || self.trigger_event_object(affected),
                        |prospective| {
                            self.trigger_event_object_with_prospective(affected, prospective)
                        },
                    ),
                    source.card.id,
                    false,
                )
            })
    }

    /// `None` means that this predicate needs the complete trigger snapshot.
    /// Composite predicates retain useful short-circuit answers: one known
    /// false arm decides `All`, and one known true arm decides `AnyOf`, even
    /// when a different arm would require the fallback.
    fn static_object_predicate_matches_lazily(
        &self,
        predicate: ObjectPredicateDef,
        source: &Permanent,
        affected: &Permanent,
        prospective: Option<&Permanent>,
    ) -> Option<bool> {
        match predicate {
            ObjectPredicateDef::Any => Some(true),
            ObjectPredicateDef::Source => Some(source.card.id == affected.card.id),
            ObjectPredicateDef::Token => Some(self.is_token(affected.card.definition)),
            ObjectPredicateDef::Tapped => Some(affected.tapped),
            ObjectPredicateDef::HasType(card_type) => self
                .permanent_types(affected)
                .map(|types| types.contains(card_type)),
            ObjectPredicateDef::HasAnyBasicLandType(land_types) => {
                let is_land = self.permanent_types(affected)?.contains(CardType::Land);
                if !is_land {
                    return Some(false);
                }
                let subtypes = prospective.map_or_else(
                    || self.effective_subtypes(affected),
                    |prospective| self.effective_subtypes_with_prospective(affected, prospective),
                );
                Some(
                    land_types
                        .iter()
                        .any(|land_type| subtypes.contains(&land_type.subtype())),
                )
            }
            // A static recipient is always a battlefield permanent, never a
            // spell, matching the `is_spell = false` general matcher call.
            ObjectPredicateDef::Spell | ObjectPredicateDef::NoncreatureSpell => Some(false),
            ObjectPredicateDef::Color(color) => {
                let rules = self.effective_rules(affected)?;
                let colors = self.effective_colors(affected, rules);
                Some(color.color_index().is_some_and(|index| colors[index]))
            }
            ObjectPredicateDef::ColorCount(count) => {
                let rules = self.effective_rules(affected)?;
                let colors = self.effective_colors(affected, rules);
                Some(colors.iter().filter(|present| **present).count() == usize::from(count))
            }
            ObjectPredicateDef::Subtype(subtype) => {
                let subtypes = prospective.map_or_else(
                    || self.effective_subtypes(affected),
                    |prospective| self.effective_subtypes_with_prospective(affected, prospective),
                );
                Some(subtypes.contains(&subtype))
            }
            // Supertypes currently have no continuous operation. Read the
            // same copied/printed rules used to construct a trigger snapshot.
            ObjectPredicateDef::Supertype(supertype) => self
                .effective_rules(affected)
                .map(|rules| rules.has_supertype(supertype)),
            ObjectPredicateDef::All(predicates) => self.static_composite_predicate_matches_lazily(
                predicates,
                source,
                affected,
                prospective,
                false,
            ),
            ObjectPredicateDef::AnyOf(predicates) => self
                .static_composite_predicate_matches_lazily(
                    predicates,
                    source,
                    affected,
                    prospective,
                    true,
                ),
            ObjectPredicateDef::Not(predicate) => self
                .static_object_predicate_matches_lazily(*predicate, source, affected, prospective)
                .map(|matches| !matches),
            ObjectPredicateDef::ManaValueAtMost(_)
            | ObjectPredicateDef::ManaValueEqualTo(_)
            | ObjectPredicateDef::ManaValueAtMostValue(_)
            | ObjectPredicateDef::PowerAtLeast(_)
            | ObjectPredicateDef::PowerExactly(_)
            | ObjectPredicateDef::ToughnessExactly(_)
            | ObjectPredicateDef::ToughnessLessThan(_)
            | ObjectPredicateDef::PowerGreaterThan(_)
            | ObjectPredicateDef::ToughnessGreaterThan(_)
            | ObjectPredicateDef::PowerLessThan(_)
            | ObjectPredicateDef::HasCounter(_)
            | ObjectPredicateDef::ControlledBy(_)
            | ObjectPredicateDef::DebutSet(_)
            | ObjectPredicateDef::SharesNameWithSource
            | ObjectPredicateDef::AttackingOrBlocking
            | ObjectPredicateDef::HasKeyword(_)
            | ObjectPredicateDef::HasNonManaActivatedAbility
            | ObjectPredicateDef::Attacking
            | ObjectPredicateDef::AttachedToSource
            | ObjectPredicateDef::Blocking
            | ObjectPredicateDef::BlockedBySource
            | ObjectPredicateDef::BlockingSource
            | ObjectPredicateDef::Enchanted
            | ObjectPredicateDef::AttachedTo(_)
            | ObjectPredicateDef::AttackedThisTurn
            | ObjectPredicateDef::AttackedDuringControllersLastTurn
            | ObjectPredicateDef::Special(_) => None,
        }
    }

    fn static_composite_predicate_matches_lazily(
        &self,
        predicates: &[ObjectPredicateDef],
        source: &Permanent,
        affected: &Permanent,
        prospective: Option<&Permanent>,
        decisive_match: bool,
    ) -> Option<bool> {
        let mut needs_snapshot = false;
        for predicate in predicates {
            match self.static_object_predicate_matches_lazily(
                *predicate,
                source,
                affected,
                prospective,
            ) {
                Some(matches) if matches == decisive_match => return Some(decisive_match),
                Some(_) => {}
                None => needs_snapshot = true,
            }
        }
        (!needs_snapshot).then_some(!decisive_match)
    }
}
