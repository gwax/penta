//! Seventh Edition has no unique card definitions.
//!
//! It is the last core set inside the Premodern window, so a card printed
//! only in a Portal set before it becomes legal here.

use super::{CardRecord, PrintingRecord};
use crate::card::sets::y1993::alpha;
use crate::card::sets::y1993::arabian_nights;
use crate::card::sets::y1998::portal_second_age;
use crate::card::sets::y2012::magic_2013;

pub(in crate::card::sets) static CARDS: &[&CardRecord] = &[];

pub(in crate::card::sets) static ADDITIONAL_PRINTINGS: &[PrintingRecord] = &[
    PrintingRecord::reprint(&alpha::DISENCHANT),   // 7ED 13
    PrintingRecord::reprint(&alpha::WRATH_OF_GOD), // 7ED 57
    PrintingRecord::reprint(&alpha::MANA_SHORT),   // 7ED 86
    PrintingRecord::reprint(&portal_second_age::SLEIGHT_OF_HAND), // 7ED 98
    PrintingRecord::reprint(&magic_2013::DURESS),  // 7ED 131
    PrintingRecord::reprint(&portal_second_age::VOLCANIC_HAMMER), // 7ED 226
    PrintingRecord::reprint(&arabian_nights::CITY_OF_BRASS), // 7ED 327
];
