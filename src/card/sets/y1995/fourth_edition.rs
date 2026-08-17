//! Fourth Edition has no unique card definitions.
//!
//! It is the set the Premodern window opens on, so a card whose only earlier
//! printings predate that window becomes legal here.

use super::{CardRecord, PrintingRecord};
use crate::card::sets::y1993::alpha;

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[
    PrintingRecord::reprint(&alpha::RED_ELEMENTAL_BLAST), // 4ED 218
];
