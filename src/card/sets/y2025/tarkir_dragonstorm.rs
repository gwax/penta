//! Tarkir: Dragonstorm cards cataloged for the Vintage Cube pool.
//!
//! Six cards from this set are in the pool. None is cataloged yet; what
//! stands here is the audit for the one that has been looked at.

use super::{CardRecord, PrintingRecord};

// TDM 127 — Tersa Lightshatter
// Audit: blocked — Two of her three abilities need capabilities that are already blocking other cards. "Discard up to two cards, then draw that many" needs a discard whose size the player chooses, where a discard here takes a fixed number; the same gap blocks Mind Bomb in The Dark. And "you may play that card this turn" needs a permission to play one exiled card for a duration, which nothing here can grant and which also blocks Robber of the Rich. Haste alone is not the card.

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
