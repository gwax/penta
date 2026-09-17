//! One-shot zone changes whose duration creates an immediate return, not a triggered ability.
use super::{
    BattlefieldArrival, BattlefieldExitCompletion, EffectResolutionContext, Game, ScopedEffect,
    StackObject, Target, ZoneMoveCause,
};
use crate::card::{EffectRecipientDef, ZoneKind, ZonePlacement};

impl Game {
    pub(super) fn resolve_duration_exile(
        &mut self,
        recipient: EffectRecipientDef,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let source = object.source.unwrap_or(object.id);
        if !self
            .battlefield
            .iter()
            .any(|permanent| permanent.card.id == source)
        {
            return;
        }
        let mut permanents = Vec::new();
        for target in self.effect_recipients(recipient, object, context, scoped) {
            match target {
                Target::Permanent(card) => permanents.push(card),
                Target::Card(card) => {
                    let Some((from, _)) = self.card_in_nonbattlefield_zone(card) else {
                        continue;
                    };
                    if from == ZoneKind::Exile {
                        continue;
                    }
                    if let Some((moved, ZoneKind::Exile)) = self.move_card_from_nonbattlefield_zone(
                        card,
                        from,
                        ZoneKind::Exile,
                        ZoneMoveCause::Effect {
                            controller: object.controller,
                        },
                        None,
                    ) {
                        self.duration_exiles.push((source, moved.id, from));
                    }
                }
                _ => {}
            }
        }
        if !permanents.is_empty() {
            self.move_permanents_to_zone_then(
                &permanents,
                ZoneKind::Exile,
                ZonePlacement::Top,
                Some(BattlefieldExitCompletion::ExileUntilSourceLeaves { source }),
            );
        }
        self.return_expired_duration_exiles();
    }

    pub(super) fn return_expired_duration_exiles(&mut self) {
        if self.duration_exiles.is_empty() {
            return;
        }
        let records = std::mem::take(&mut self.duration_exiles);
        let mut returning = Vec::new();
        for (source, card, zone) in records {
            let Some((ZoneKind::Exile, instance)) = self.card_in_nonbattlefield_zone(card) else {
                continue;
            };
            let owner = instance.owner;
            if self
                .battlefield
                .iter()
                .any(|permanent| permanent.card.id == source)
            {
                self.duration_exiles.push((source, card, zone));
            } else {
                returning.push((card, zone, owner));
            }
        }
        if returning.is_empty() {
            return;
        }
        // Remove the duration records before moving any cards: the returns can themselves
        // cause zone events, entries, or another duration to end.
        self.entering_together(|game| {
            for (card, zone, owner) in returning {
                game.move_card_from_nonbattlefield_zone(
                    card,
                    ZoneKind::Exile,
                    zone,
                    ZoneMoveCause::Rules,
                    (zone == ZoneKind::Battlefield).then(|| BattlefieldArrival::under(owner)),
                );
            }
        });
    }
}
