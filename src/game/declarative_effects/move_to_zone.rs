//! Moving objects between zones, and everything an arrival on the
//! battlefield carries with it.
//!
//! Split out of the parent module for the source-size budget: what belongs
//! here is one clause whose parameters all describe the same arrival.

use super::{
    ArrivalAttachment, ArrivalAttachmentDef, BattlefieldArrival, EffectResolutionContext, Game,
    ScopedEffect, StackObject, Target, ZoneKind, ZoneMoveCause, ZonePlacement,
};
use crate::card::{AppliedEffectDef, EffectRecipientDef, PlayerRelation};

/// How a permanent this effect moves arrives, when it arrives at all.
/// "Under your control" and "attach this to it" both belong to the arrival:
/// what enters is a new object, so neither can wait for a later step.
fn battlefield_arrival(
    object: &StackObject,
    arriving_controller: Option<crate::PlayerId>,
    attachment: Option<ArrivalAttachment>,
) -> Option<BattlefieldArrival> {
    if arriving_controller.is_none() && attachment.is_none() {
        return None;
    }
    let arrival = BattlefieldArrival::under(arriving_controller.unwrap_or(object.controller));
    Some(match attachment {
        Some(ArrivalAttachment::SourceToArrival(source)) => arrival.attaching(source),
        Some(ArrivalAttachment::ArrivalToHost(host)) => arrival.attached_to(host),
        None => arrival,
    })
}

/// One authored "put this there" clause, gathered so the resolution takes an
/// arrival rather than six loose parameters.
#[derive(Clone, Copy)]
pub(super) struct MoveToZoneClause {
    pub(super) recipient: EffectRecipientDef,
    pub(super) zone: ZoneKind,
    pub(super) controller: Option<PlayerRelation>,
    pub(super) placement: ZonePlacement,
    pub(super) arrival_effect: Option<&'static AppliedEffectDef>,
    pub(super) attachment: Option<ArrivalAttachmentDef>,
}

impl Game {
    pub(super) fn resolve_move_to_zone(
        &mut self,
        clause: MoveToZoneClause,
        object: &StackObject,
        context: &EffectResolutionContext,
        scoped: ScopedEffect,
    ) {
        let MoveToZoneClause {
            recipient,
            zone,
            controller,
            placement,
            arrival_effect,
            attachment,
        } = clause;
        let attachment = attachment.and_then(|attachment| match attachment {
            ArrivalAttachmentDef::SourceToArrival => {
                object.source.map(ArrivalAttachment::SourceToArrival)
            }
            ArrivalAttachmentDef::ArrivalToHost(reference) => self
                .object_reference_target(reference, object, context, scoped)
                .and_then(|target| match target {
                    Target::Permanent(host) => Some(ArrivalAttachment::ArrivalToHost(host)),
                    _ => None,
                }),
        });
        let arriving_controller = controller.map(|relation| {
            if self.player_relation_matches(
                object.controller,
                relation,
                object.controller,
                context.trigger,
            ) {
                object.controller
            } else {
                object.controller.opponent()
            }
        });
        for target in self.effect_recipients(recipient, object, context, scoped) {
            let arrived = self.move_target_to_zone(
                target,
                zone,
                ZoneMoveCause::Effect {
                    controller: object.controller,
                },
                // "Under your control" and "attach this to it" both belong to
                // the arrival: a permanent that enters is a new object, so
                // neither can wait for a later step.
                battlefield_arrival(object, arriving_controller, attachment),
                placement,
            );
            // Applied as the move happens: the identity a permanent gets on
            // arrival is not the one the card had in the graveyard it came
            // from, so a later effect would have nothing to name.
            if let (Some(effect), Some(arrived)) = (arrival_effect, arrived) {
                self.apply_arrival_effect(arrived, *effect, object, context, scoped);
            }
        }
    }
}
