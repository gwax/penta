// Scratch answers a long `&self` board read may reuse.
//
// Two sweeps dominate `legal_actions` on a board with a dozen lands and
// several activatable abilities, and both are re-derived per question:
//
//   * `battlefield_trigger_listeners` walks every permanent and emblem asking
//     for its effective abilities, which rebuilds the layer stack for each.
//     `with_triggered_mana_choices` asks it for every mana source that taps,
//     to learn whether tapping wakes a mana trigger.
//   * `mana_ability_activations` rebuilds one permanent's mana abilities.
//     `flexible_mana_sources` asks it for every permanent its controller has,
//     and payment planning runs once per candidate action.
//
// Nesting those inside the payment search made the work grow with the square
// of the board: a Premodern gauntlet pairing that took two seconds at eight
// hundred actions took four minutes at sixteen hundred.
//
// The invariant is the one `land_type_layers/query_memo.rs` documents: a query
// holds `&self`, so the board cannot move underneath it and an answer stays
// good until it returns. Installed only by the two long `&self` reads,
// `legal_actions` and `observe`, and dropped when they return. Where no memo
// is installed both sweeps run as before, so every mutation path is untouched.
// The game's address identifies the board, and the installing borrow keeps it
// alive for the memo's whole life, so a different address is a different game
// rather than a reused allocation.
//
// Thread-local rather than a field, because `Game` stays `Send + Sync` for the
// Python binding and a `RefCell` field would cost that.

use std::rc::Rc;

use super::{BattlefieldTriggerListener, Game, GameObjectId, ManaAbilityActivation, Permanent};

#[derive(Default)]
struct BoardReadMemo {
    game: usize,
    listeners: Option<Rc<Vec<BattlefieldTriggerListener>>>,
    mana_activations: std::collections::HashMap<GameObjectId, Rc<Vec<ManaAbilityActivation>>>,
}

thread_local! {
    static BOARD_READ_MEMO: std::cell::RefCell<Option<BoardReadMemo>> =
        const { std::cell::RefCell::new(None) };
}

/// Drops the memo when the query that installed it returns, panic included.
pub(in crate::game) struct BoardReadMemoGuard {
    installed: bool,
}

impl Drop for BoardReadMemoGuard {
    fn drop(&mut self) {
        if self.installed {
            BOARD_READ_MEMO.with(|memo| *memo.borrow_mut() = None);
        }
    }
}

impl Game {
    /// Lets one `&self` read reuse the board sweeps below. Held by the caller;
    /// answers are discarded when it drops.
    pub(in crate::game) fn hold_board_read_memo(&self) -> BoardReadMemoGuard {
        let game = std::ptr::from_ref(self) as usize;
        let installed = BOARD_READ_MEMO.with(|memo| {
            let mut memo = memo.borrow_mut();
            if memo.is_none() {
                *memo = Some(BoardReadMemo {
                    game,
                    ..BoardReadMemo::default()
                });
                true
            } else {
                false
            }
        });
        BoardReadMemoGuard { installed }
    }

    /// Reads from the memo installed for this board, if there is one.
    fn board_memo<T>(&self, read: impl Fn(&BoardReadMemo) -> Option<T>) -> Option<T> {
        let game = std::ptr::from_ref(self) as usize;
        BOARD_READ_MEMO.with(|memo| {
            memo.borrow()
                .as_ref()
                .filter(|memo| memo.game == game)
                .and_then(read)
        })
    }

    /// Writes to the memo installed for this board, if there is one.
    fn remember_board(&self, write: impl FnOnce(&mut BoardReadMemo)) {
        let game = std::ptr::from_ref(self) as usize;
        BOARD_READ_MEMO.with(|memo| {
            if let Some(memo) = memo.borrow_mut().as_mut()
                && memo.game == game
            {
                write(memo);
            }
        });
    }

    /// The listeners this board has, sweeping only when no memo for it is
    /// installed or the installed one has not been asked yet.
    pub(super) fn battlefield_trigger_listeners(&self) -> Vec<BattlefieldTriggerListener> {
        if let Some(listeners) = self.board_memo(|memo| memo.listeners.clone()) {
            return (*listeners).clone();
        }
        let listeners = Rc::new(self.battlefield_trigger_listeners_uncached());
        self.remember_board(|memo| memo.listeners = Some(Rc::clone(&listeners)));
        (*listeners).clone()
    }

    /// One permanent's mana abilities, under the same memo.
    pub(super) fn mana_ability_activations(
        &self,
        permanent: &Permanent,
    ) -> Vec<ManaAbilityActivation> {
        let key = permanent.card.id;
        if let Some(activations) =
            self.board_memo(|memo| memo.mana_activations.get(&key).map(Rc::clone))
        {
            return (*activations).clone();
        }
        let activations = Rc::new(self.mana_ability_activations_uncached(permanent));
        self.remember_board(|memo| {
            memo.mana_activations.insert(key, Rc::clone(&activations));
        });
        (*activations).clone()
    }
}
