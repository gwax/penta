//! Chronicles has no unique card definitions.
//!
//! It reprinted cards from the sets before it, which is what brings some of
//! them inside the Premodern window.

use super::{CardRecord, PrintingRecord};
use crate::card::sets::y1994::the_dark;

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[
    PrintingRecord::reprint(&the_dark::TORMODS_CRYPT), // CHR 109
];
