// Selecting one card of each type from a revealed group.
//
// Split out of `decision_piles.rs` for the source-size budget along the seam
// the question already has: the file next door asks once for a bounded group,
// and this asks once per card type. Included textually, so the imports here
// are that module's.

impl Game {
    /// Asks about the next card type that any of the remaining cards has,
    /// or settles the selection when none is left. Each type is one optional
    /// pick from the cards still on the table, so a card taken as the
    /// artifact is no longer there to be taken as the creature.
    ///
    /// The engine has no battle card type, and no cataloged card is one, so
    /// the seven types it does have are the whole list.
    pub(super) fn queue_typed_selection(
        &mut self,
        progress: TypedSelectionProgress,
        selection: &'static TopCardSelectionDef,
        object: &StackObject,
        context: EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let source = object.source.unwrap_or(object.id);
        let mut next_type = progress.next_type;
        while let Some(card_type) = CardType::ALL.get(next_type).copied() {
            let eligible = self.typed_selection_candidates(&progress.revealed, card_type, source);
            if eligible.is_empty() {
                next_type += 1;
                continue;
            }
            let mut options = self.card_decision_options(&eligible, DecisionZone::Library);
            // Every card still on the table rides along, the way it does for
            // an ordinary look: the question is about one type, but what is
            // being looked at is the whole group.
            let inspected = progress
                .revealed
                .iter()
                .map(|card| {
                    (
                        card.id,
                        ObjectCharacteristics::card(card.definition, CardPartId::PRIMARY),
                    )
                })
                .collect::<Vec<_>>();
            for option in &mut options {
                option.members.clone_from(&inspected);
            }
            self.queue_decision(
                progress.looker,
                Self::typed_selection_prompt(card_type),
                if selection.reveal_inspected {
                    DecisionVisibility::Public
                } else {
                    DecisionVisibility::Private
                },
                DecisionPreference::HigherCardValue,
                0..=1,
                false,
                options,
                DecisionContinuation::TypedTopCardSelection {
                    progress: TypedSelectionProgress {
                        next_type,
                        ..progress
                    },
                    selection,
                    object: Box::new(object.clone()),
                    context,
                    effect: scoped,
                },
            );
            return;
        }
        self.finish_typed_selection(progress, selection, object, context, scoped);
    }

    /// What one type's pick is asked. Shared with the checkpoint, which
    /// compares the prompt it rebuilds against the one the observation
    /// carries.
    pub(super) fn typed_selection_prompt(card_type: CardType) -> String {
        let name = card_type.name().to_lowercase();
        let article = if name.starts_with(['a', 'e', 'i', 'o', 'u']) {
            "an"
        } else {
            "a"
        };
        format!("Put {article} {name} card from among them into your hand")
    }

    /// The remaining cards that have one card type, which is what that
    /// type's pick may name.
    pub(super) fn typed_selection_candidates(
        &self,
        revealed: &[CardInstance],
        card_type: CardType,
        source: GameObjectId,
    ) -> Vec<CardInstance> {
        revealed
            .iter()
            .filter(|card| {
                self.card_object_matches(
                    ObjectPredicateDef::HasType(card_type),
                    card,
                    ZoneKind::Library,
                    source,
                )
            })
            .cloned()
            .collect()
    }

    /// Places both halves once every type has been asked about, and runs
    /// whatever follows the selection.
    pub(super) fn finish_typed_selection(
        &mut self,
        progress: TypedSelectionProgress,
        selection: &'static TopCardSelectionDef,
        object: &StackObject,
        context: EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let hider = object.source.unwrap_or(object.id);
        let (count, mana_value) =
            self.selected_card_totals(&progress.taken, selection.counted, hider);
        self.finish_top_card_selection_from(
            progress.player,
            progress.taken,
            progress.revealed,
            selection,
            Some(hider),
        );
        if let Some(then) = selection.then {
            let mut context = context;
            context.matched_count = Some(count);
            context.matched_mana_value = Some(mana_value);
            self.resolve_nested_effect_before_later(scoped.with_effect(*then), object, context);
        }
    }
}
