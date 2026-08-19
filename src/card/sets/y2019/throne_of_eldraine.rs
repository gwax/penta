//! Throne of Eldraine cards cataloged for the Vintage Cube pool.
//!
//! Seven cards from this set are in the pool. None is cataloged yet; what
//! stands here is the audit for the one that has been looked at.

use super::{CardRecord, PrintingRecord};

// ELD 138 — Robber of the Rich
// Audit: blocked — Needs three things. An intervening-if that compares two players' hand sizes rather than a count against a printed number; a permission to cast one exiled card that survives its source leaving the battlefield and is gated on having attacked with a Rogue that turn; and spending mana as though it were mana of any color, which already blocks North Star in Legends.

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
