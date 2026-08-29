//! The question a creature that arrives attacking has to answer.
//!
//! An effect that puts a creature onto the battlefield attacking lets its
//! controller choose what it is attacking as it enters (CR 506.3d). With no
//! planeswalker across the table there is only one answer, so nothing is
//! asked; with one, each arriving attacker is asked separately, and a batch
//! can be split between the player and their planeswalkers.

use super::{
    DecisionContinuation, DecisionOption, DecisionPreference, DecisionVisibility, DecisionZone,
    Game, PlayerId,
};
use crate::card::CardType;
use crate::ids::GameObjectId;

impl Game {
    /// Asks what an arriving attacker is attacking. A creature put onto the
    /// battlefield attacking has its defender chosen by the controller of
    /// the effect (CR 506.3d), which only makes a difference when the
    /// defending player has a planeswalker to send it at instead. One
    /// question per creature, so a batch can be split between defenders.
    pub(super) fn queue_arriving_attacker_defender(
        &mut self,
        player: PlayerId,
        defending: PlayerId,
        attackers: &[GameObjectId],
    ) {
        let Some((first, rest)) = attackers.split_first() else {
            return;
        };
        let planeswalkers = self
            .battlefield
            .iter()
            .filter(|permanent| permanent.controller == defending)
            .filter(|permanent| {
                self.permanent_types(permanent)
                    .is_some_and(|types| types.contains(CardType::Planeswalker))
            })
            .map(|permanent| permanent.card.id)
            .collect::<Vec<_>>();
        if planeswalkers.is_empty() {
            return;
        }
        let name = self
            .battlefield
            .iter()
            .find(|permanent| permanent.card.id == *first)
            .and_then(|permanent| self.permanent_card_name(permanent.card.id))
            .map_or_else(|| "the creature".to_string(), std::borrow::Cow::into_owned);
        let mut options = vec![DecisionOption {
            id: 0,
            label: format!("{name} attacks the player"),
            card: None,
            members: Vec::new(),
            ability_text: None,
            zone: DecisionZone::Battlefield,
        }];
        options.extend(planeswalkers.iter().enumerate().map(|(index, walker)| {
            let walker_name = self.permanent_card_name(*walker).map_or_else(
                || "a planeswalker".to_string(),
                std::borrow::Cow::into_owned,
            );
            DecisionOption {
                id: u32::try_from(index + 1).unwrap_or(u32::MAX),
                label: format!("{name} attacks {walker_name}"),
                card: self
                    .battlefield
                    .iter()
                    .find(|candidate| candidate.card.id == *walker)
                    .map(|candidate| (*walker, Self::effective_rules_source(candidate))),
                members: Vec::new(),
                ability_text: None,
                zone: DecisionZone::Battlefield,
            }
        }));
        self.queue_decision(
            player,
            "Choose what the arriving attacker is attacking",
            DecisionVisibility::Public,
            DecisionPreference::PreferOption(0),
            1..=1,
            false,
            options,
            DecisionContinuation::ArrivingAttackerDefender {
                player,
                defending,
                attackers: std::iter::once(*first)
                    .chain(rest.iter().copied())
                    .collect(),
            },
        );
    }
}
