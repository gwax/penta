//! Seventh Edition has no unique card definitions.
//!
//! It is the last core set inside the Premodern window, so a card printed
//! only in a Portal set before it becomes legal here.

use super::{CardRecord, PrintingRecord};
use crate::card::sets::y1998::portal_second_age;

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[
    PrintingRecord::reprint(&portal_second_age::VOLCANIC_HAMMER), // 7ED 226
];
