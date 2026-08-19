//! Tarkir: Dragonstorm cards cataloged for the Vintage Cube pool.
//!
//! Six cards from this set are in the pool. None is cataloged yet; what
//! stands here are the audits for the ones that have been looked at.

use super::{CardRecord, PrintingRecord};

// TDM 33 — Voice of Victory
// Audit: blocked — Needs two things. Mobilize creates tokens already attacking, which no effect here can do, and then sacrifices exactly those tokens at the next end step, which needs the created tokens bound for a later clause to name. And "your opponents can't cast spells during your turn" is a play restriction conditioned on whose turn it is; a static narrows its recipients by an object predicate, and the recipients here are players, so there is nowhere to put the condition.

// TDM 127 — Tersa Lightshatter
// Audit: blocked — Two of her three abilities need capabilities that are already blocking other cards. "Discard up to two cards, then draw that many" needs a discard whose size the player chooses, where a discard here takes a fixed number; the same gap blocks Mind Bomb in The Dark. And "you may play that card this turn" needs a permission to play one exiled card for a duration, which nothing here can grant and which also blocks Robber of the Rich. Haste alone is not the card.

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
