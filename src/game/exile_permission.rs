//! Permission to play a card from exile.
//!
//! Two shapes reach this: a card on an adventure, which its owner may cast
//! later as the creature half and which never lapses; and a card somebody
//! else's effect exiled and handed to a player for a while, which is played
//! for free and expires.

use super::{CardDefinition, Game, GameObjectId, PlayOptionDef, PlayerId};

/// What a card in exile costs the player who may play it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExilePlayCost {
    /// Its own cost, as printed.
    Printed,
    /// Waived entirely (CR 118.5): "you may play those cards without paying
    /// their mana costs".
    Free,
    /// "By paying an amount of {E} equal to its mana value rather than paying
    /// its mana cost." The mana cost goes away and the energy takes its
    /// place, so a card nobody has the energy for is not castable at all.
    EnergyEqualToManaValue,
}

/// One card in exile somebody may play from there.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ExilePlayPermission {
    pub(super) card: GameObjectId,
    /// Who may play it. An adventure returns to its owner; a card taken off
    /// the top of somebody's library is played by whoever took it.
    pub(super) player: PlayerId,
    /// What playing it costs, which need not be what the card prints.
    pub(super) cost: ExilePlayCost,
    /// The turn this permission belongs to, as the turn count of the player
    /// whose turn it was. `None` never lapses, which is what an adventure
    /// means; anything else is gone once that turn is over.
    pub(super) until_end_of_turn: Option<(PlayerId, u32)>,
    /// Whether only the main half of an Adventure card may be played, which
    /// is what "as the creature, never as the adventure again" means
    /// (CR 715.3d).
    pub(super) adventure_return_only: bool,
}

impl ExilePlayCost {
    /// The stable wire label for this cost. Written out rather than derived,
    /// because a `Debug` rendering is not a wire contract.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Printed => "printed",
            Self::Free => "free",
            Self::EnergyEqualToManaValue => "energyEqualToManaValue",
        }
    }

    /// The cost a label names. An unknown one is a refusal rather than a
    /// guess: reading it wrong would let a card be cast for nothing.
    pub(super) fn from_label(label: &str) -> Option<Self> {
        match label {
            "printed" => Some(Self::Printed),
            "free" => Some(Self::Free),
            "energyEqualToManaValue" => Some(Self::EnergyEqualToManaValue),
            _ => None,
        }
    }
}

impl Game {
    /// Whether `player` may presently play `card` from exile with `option`.
    pub(super) fn exile_play_is_permitted(
        &self,
        definition: &CardDefinition,
        option: &PlayOptionDef,
        card: GameObjectId,
        player: PlayerId,
    ) -> bool {
        self.exile_play_permission(card, player)
            .is_some_and(|permission| {
                !permission.adventure_return_only
                    || Self::is_adventure_return_option(definition, option)
            })
    }

    /// The live permission `player` holds over `card`, if any.
    pub(super) fn exile_play_permission(
        &self,
        card: GameObjectId,
        player: PlayerId,
    ) -> Option<ExilePlayPermission> {
        self.exile_play_permissions
            .iter()
            .copied()
            .find(|permission| {
                permission.card == card
                    && permission.player == player
                    && permission.until_end_of_turn.is_none_or(|(owner, turn)| {
                        self.turns_started[owner.index()] == turn && self.active_player == owner
                    })
            })
    }

    /// Records that a card on an adventure may come back as the creature it
    /// is. The permission never lapses: a crown that never moves keeps it,
    /// and so does an adventure nobody takes.
    pub(super) fn permit_adventure_return(&mut self, card: GameObjectId, player: PlayerId) {
        self.exile_play_permissions.push(ExilePlayPermission {
            card,
            player,
            cost: ExilePlayCost::Printed,
            until_end_of_turn: None,
            adventure_return_only: true,
        });
    }

    /// "Until end of turn, you may play those cards without paying their
    /// mana costs."
    pub(super) fn permit_free_play_this_turn(&mut self, card: GameObjectId, player: PlayerId) {
        let active = self.active_player;
        self.exile_play_permissions.push(ExilePlayPermission {
            card,
            player,
            cost: ExilePlayCost::Free,
            until_end_of_turn: Some((active, self.turns_started[active.index()])),
            adventure_return_only: false,
        });
    }

    /// "You may cast that card." Unlike the free play above, the cost is
    /// still owed; what the permission grants is only that exile is a legal
    /// place to cast it from.
    pub(super) fn permit_cast_this_turn(&mut self, card: GameObjectId, player: PlayerId) {
        let active = self.active_player;
        self.exile_play_permissions.push(ExilePlayPermission {
            card,
            player,
            cost: ExilePlayCost::Printed,
            until_end_of_turn: Some((active, self.turns_started[active.index()])),
            adventure_return_only: false,
        });
    }

    /// "You may cast that card by paying an amount of {E} equal to its mana
    /// value rather than paying its mana cost." Nothing states a duration, so
    /// the permission lasts as long as the card sits in exile.
    pub(super) fn permit_energy_cast(&mut self, card: GameObjectId, player: PlayerId) {
        self.exile_play_permissions.push(ExilePlayPermission {
            card,
            player,
            cost: ExilePlayCost::EnergyEqualToManaValue,
            until_end_of_turn: None,
            adventure_return_only: false,
        });
    }

    /// The energy `player` owes to cast `card` from exile, if that is how the
    /// permission they hold over it is paid.
    pub(super) fn exile_energy_cost(&self, card: GameObjectId, player: PlayerId) -> Option<u16> {
        let permission = self.exile_play_permission(card, player)?;
        if permission.cost != ExilePlayCost::EnergyEqualToManaValue {
            return None;
        }
        let (_, instance) = self.card_in_nonbattlefield_zone(card)?;
        Some(
            self.catalog
                .get(instance.definition)?
                .rules
                .printed_mana_cost()
                .mana_value(),
        )
    }

    /// Drops the permission a play has just consumed.
    pub(super) fn consume_exile_play_permission(&mut self, card: GameObjectId) {
        self.exile_play_permissions
            .retain(|permission| permission.card != card);
    }
}
