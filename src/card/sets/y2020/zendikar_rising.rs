//! Zendikar Rising cards cataloged for the Vintage Cube pool.
//!
//! Five cards from this set are in the pool. None is cataloged yet; what
//! stands here is the audit for the one that has been looked at.

use super::{CardRecord, PrintingRecord};

// ZNR 85 — Thieving Skydiver
// Audit: blocked — Kicker here is a spell cast for more mana with different instructions, and the kicked clause has to carry those instructions. This card's kicker changes nothing about how the spell resolves; it changes whether a triggered ability fires afterwards and what that ability may target, which the kicked alternative has no way to say. It also needs a minimum on X, since casts are enumerated from zero and "X can't be 0" would otherwise let an unkicked-sized cast steal a nothing-cost artifact.

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[];
