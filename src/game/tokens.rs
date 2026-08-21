//! Creating tokens.
//!
//! A token is an ordinary permanent whose card has no printed existence, so
//! everything about it -- entry replacements, triggers, arriving tapped --
//! goes through the same battlefield entry path as a card. What lives here is
//! only the part that differs: minting the object.

use super::{
    CardDefinitionId, CardSet, CharacteristicSource, CounterKind, EntryCompletion, Game,
    GameObjectId, PendingBattlefieldEntry, Permanent, PlayerId, ZoneKind,
};

impl Game {
    /// Whether a definition is a token rather than a printed card.
    pub(super) fn is_token(&self, definition: CardDefinitionId) -> bool {
        self.catalog
            .get(definition)
            .is_some_and(|card| card.debut_set == CardSet::Token)
    }

    /// Puts one token onto the battlefield under `controller`.
    ///
    /// A token is a real permanent built from a catalog definition that no
    /// format allows, so it can be looked up and rendered like any other card
    /// while never being deck-legal.
    pub(super) fn create_token(&mut self, controller: PlayerId, token: CardDefinitionId) {
        self.create_token_from(controller, token, None);
    }

    /// Puts one token onto the battlefield, remembering which permanent's
    /// ability made it. Only the cards that later refer to their own tokens
    /// pass a creator; for everything else the link is dead weight.
    pub(super) fn create_token_from(
        &mut self,
        controller: PlayerId,
        token: CardDefinitionId,
        creator: Option<GameObjectId>,
    ) {
        self.create_token_arriving(controller, token, creator, false, None, None);
    }

    /// Creates one token whose committed battlefield incarnation becomes the
    /// host of `source`. The binding sits on the entry event rather than this
    /// resolving instruction because replacement effects can replace or
    /// otherwise delay the prospective token entry.
    pub(super) fn create_attached_token(
        &mut self,
        controller: PlayerId,
        token: CardDefinitionId,
        source: GameObjectId,
    ) {
        let Some(definition) = self.catalog.get(token) else {
            return;
        };
        let presented = definition.primary_part_id();
        let card = self.unbacked_object(token, controller, CharacteristicSource::Card(token));
        let permanent = Permanent::entering(
            card,
            presented,
            controller,
            self.turns_started[controller.index()],
        );
        self.enqueue_battlefield_entry(PendingBattlefieldEntry {
            permanent,
            from: ZoneKind::Stack,
            completion: EntryCompletion::AttachSource { source },
            redirected_to: None,
        });
    }

    /// The same, for a token whose card says it arrives tapped.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn create_token_arriving(
        &mut self,
        controller: PlayerId,
        token: CardDefinitionId,
        creator: Option<GameObjectId>,
        tapped: bool,
        attacking: Option<crate::AttackDefender>,
        counters: Option<(CounterKind, u16)>,
    ) -> Option<GameObjectId> {
        let definition = self.catalog.get(token)?;
        let presented = definition.primary_part_id();
        // A token has no physical card behind it, which is exactly what an
        // unbacked object is.
        let card = self.unbacked_object(token, controller, CharacteristicSource::Card(token));
        let mut permanent = Permanent::entering(
            card,
            presented,
            controller,
            self.turns_started[controller.index()],
        );
        permanent.created_by = creator;
        // Set before entry replacements run, the same point an as-enters
        // clause would set it.
        permanent.tapped = tapped;
        // "Create an Incubator token with X +1/+1 counters on it" is one
        // instruction: the counters are on it as it arrives rather than put
        // there afterwards, so an enters trigger reading its power sees them.
        if let Some((kind, amount)) = counters {
            permanent.add_counters(kind, amount);
        }
        let prospective = permanent.card.id;
        self.enqueue_battlefield_entry(PendingBattlefieldEntry {
            permanent,
            from: ZoneKind::Stack,
            completion: match attacking {
                Some(defender) => EntryCompletion::Attacking { defender },
                None => EntryCompletion::None,
            },
            redirected_to: None,
        });
        // A permanent takes a fresh identity as it actually arrives, so what
        // the caller gets back is the object that ended up on the
        // battlefield rather than the prospective one it was handed. An
        // entry still waiting on a decision has no successor yet, and gives
        // back the only id there is.
        Some(
            self.successors
                .get(&prospective)
                .copied()
                .unwrap_or(prospective),
        )
    }
}
