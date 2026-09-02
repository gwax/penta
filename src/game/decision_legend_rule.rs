//! The question the legend rule has to ask.
//!
//! CR 704.5j lets the controller of two or more same-named legendary
//! permanents choose which one to keep, and same-named permanents are not
//! interchangeable: counters, attachments, damage, and tapped status all
//! separate them. A Thespian's Stage that has copied its controller's own
//! Dark Depths is the clearest case -- the copy carries no ice counters and
//! is the one worth keeping, even though it is tapped from paying for the
//! ability.

use super::{
    DecisionContinuation, DecisionOption, DecisionPreference, DecisionVisibility, DecisionZone,
    Game, PlayerId,
};
use crate::ids::GameObjectId;

impl Game {
    /// Asks which of `candidates` to keep. The rest go to the graveyard when
    /// the answer comes back, and the first option is the one an automated
    /// policy takes: the untapped body, then the newest, which is the choice
    /// the rule made for everyone before it was a choice at all.
    pub(super) fn queue_legend_rule_choice(
        &mut self,
        player: PlayerId,
        candidates: &[GameObjectId],
    ) {
        if candidates.len() < 2 {
            return;
        }
        let name = candidates
            .first()
            .and_then(|first| self.permanent_card_name(*first))
            .map_or_else(|| "the legend".to_string(), std::borrow::Cow::into_owned);
        let options = candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let permanent = self
                    .battlefield
                    .iter()
                    .find(|permanent| permanent.card.id == *candidate);
                let tapped = permanent.is_some_and(|permanent| permanent.tapped);
                DecisionOption {
                    id: u32::try_from(index).unwrap_or(u32::MAX),
                    label: if tapped {
                        format!("Keep the tapped {name}")
                    } else {
                        format!("Keep the untapped {name}")
                    },
                    card: permanent
                        .map(|permanent| (*candidate, Self::effective_rules_source(permanent))),
                    members: Vec::new(),
                    ability_text: None,
                    zone: DecisionZone::Battlefield,
                }
            })
            .collect();
        self.queue_decision(
            player,
            format!("Choose which {name} to keep"),
            DecisionVisibility::Public,
            DecisionPreference::PreferOption(0),
            1..=1,
            false,
            options,
            DecisionContinuation::LegendRule {
                player,
                candidates: candidates.to_vec(),
            },
        );
    }

    /// Puts every candidate but the one chosen into the graveyard.
    pub(super) fn finish_legend_rule_choice(
        &mut self,
        kept: Option<GameObjectId>,
        candidates: &[GameObjectId],
    ) {
        let kept = kept.or_else(|| candidates.first().copied());
        let doomed = candidates
            .iter()
            .copied()
            .filter(|candidate| Some(*candidate) != kept)
            .filter(|candidate| {
                self.battlefield
                    .iter()
                    .any(|permanent| permanent.card.id == *candidate)
            })
            .collect::<Vec<_>>();
        self.move_permanents_to_graveyard(&doomed);
    }
}
