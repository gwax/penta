//! Fixed Eternal Central Old School 93/94 format rules.

pub const STARTING_LIFE: u8 = 20;
pub const OPENING_HAND_SIZE: usize = 7;
pub const MINIMUM_MAIN_DECK_SIZE: usize = 60;
pub const MAXIMUM_SIDEBOARD_SIZE: usize = 15;
pub const MAXIMUM_COPIES: usize = 4;

pub const BANNED_CARDS: &[&str] = &[
    "Bronze Tablet",
    "Contract from Below",
    "Darkpact",
    "Demonic Attorney",
    "Jeweled Bird",
    "Rebirth",
    "Tempest Efreet",
];

pub const RESTRICTED_CARDS: &[&str] = &[
    "Ancestral Recall",
    "Balance",
    "Black Lotus",
    "Braingeyser",
    "Chaos Orb",
    "Channel",
    "Demonic Tutor",
    "Library of Alexandria",
    "Mana Drain",
    "Mind Twist",
    "Mox Emerald",
    "Mox Jet",
    "Mox Pearl",
    "Mox Ruby",
    "Mox Sapphire",
    "Recall",
    "Regrowth",
    "Sol Ring",
    "Time Vault",
    "Time Walk",
    "Timetwister",
    "Wheel of Fortune",
];

#[must_use]
pub fn is_banned(name: &str) -> bool {
    contains_name(BANNED_CARDS, name)
}

#[must_use]
pub fn is_restricted(name: &str) -> bool {
    contains_name(RESTRICTED_CARDS, name)
}

fn contains_name(names: &[&str], candidate: &str) -> bool {
    let candidate = candidate.trim();
    names
        .iter()
        .any(|name| name.eq_ignore_ascii_case(candidate))
}
