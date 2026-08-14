use super::{Format, JsValue, Value, json};
use std::fmt::Write as _;

pub(super) fn card_art_value(art: Option<&penta::CardArt>) -> Value {
    art.map_or(Value::Null, |art| {
        json!({
            "scryfallId": art.scryfall_id,
            "artist": art.artist,
        })
    })
}

pub(super) fn hand_mana_cost_value(card: Option<&penta::CardDefinition>) -> Value {
    card.and_then(penta::CardDefinition::primary_part)
        .and_then(penta::CardPart::mana_cost)
        .map_or(Value::Null, |cost| {
            json!({
                "generic": cost.generic,
                "white": cost.white,
                "blue": cost.blue,
                "black": cost.black,
                "red": cost.red,
                "green": cost.green,
                "hybrid": penta::HybridPair::ALL
                    .into_iter()
                    .filter(|pair| cost.hybrid[pair.index()] > 0)
                    .map(|pair| json!({ "symbol": pair.symbol(), "count": cost.hybrid[pair.index()] }))
                    .collect::<Vec<_>>(),
                "x": cost.variable_x,
            })
        })
}

pub(super) fn mana_cost_label(cost: penta::ManaCost) -> String {
    let mut label = String::new();
    if cost.generic > 0 {
        let _ = write!(label, "{{{}}}", cost.generic);
    }
    if cost.variable_x {
        for _ in 0..cost.x_multiplier.max(1) {
            label.push_str("{X}");
        }
    }
    for (amount, symbol) in [
        (cost.white, "W"),
        (cost.blue, "U"),
        (cost.black, "B"),
        (cost.red, "R"),
        (cost.green, "G"),
    ] {
        for _ in 0..amount {
            let _ = write!(label, "{{{symbol}}}");
        }
    }
    for pair in penta::HybridPair::ALL {
        for _ in 0..cost.hybrid[pair.index()] {
            let _ = write!(label, "{{{}}}", pair.symbol());
        }
    }
    if label.is_empty() {
        label.push_str("{0}");
    }
    label
}

pub(super) struct StackCardPresentation {
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) type_line: String,
    pub(super) implementation_status: penta::ImplementationStatus,
    pub(super) is_land: bool,
    pub(super) mana_cost: Option<penta::ManaCost>,
    pub(super) rules_text: String,
    pub(super) power: Option<i16>,
    pub(super) toughness: Option<i16>,
}

impl StackCardPresentation {
    fn unknown() -> Self {
        Self {
            name: "Unknown card".into(),
            kind: "unknown".into(),
            type_line: String::new(),
            implementation_status: penta::ImplementationStatus::Complete,
            is_land: false,
            mana_cost: None,
            rules_text: String::new(),
            power: None,
            toughness: None,
        }
    }

    pub(super) fn from_rules(
        name: String,
        rules: &penta::CardRules,
        mana_cost: Option<penta::ManaCost>,
    ) -> Self {
        Self {
            name,
            kind: rules.kind_name().to_ascii_lowercase(),
            type_line: rules.type_line(),
            implementation_status: rules.implementation_status(),
            is_land: rules.has_type(penta::CardType::Land),
            mana_cost,
            rules_text: rules.rules_text().into_owned(),
            power: rules.creature_stats().map(|stats| stats.power),
            toughness: rules.creature_stats().map(|stats| stats.toughness),
        }
    }
}

pub(super) fn stack_card_presentation(
    card: Option<&penta::CardDefinition>,
    signature: Option<&penta::CastSignature>,
) -> StackCardPresentation {
    let Some(card) = card else {
        return StackCardPresentation::unknown();
    };
    let canonical = || {
        StackCardPresentation::from_rules(card.name.clone(), &card.rules, card.rules.mana_cost())
    };
    let Some(signature) = signature else {
        return canonical();
    };

    match signature.form() {
        penta::SpellForm::Part(part_id) => card.part(*part_id).map_or_else(canonical, |part| {
            StackCardPresentation::from_rules(part.name.clone(), &part.rules, part.mana_cost())
        }),
        penta::SpellForm::Combined(part_ids) => {
            let Some(parts) = part_ids
                .iter()
                .map(|part_id| card.part(*part_id))
                .collect::<Option<Vec<_>>>()
            else {
                return canonical();
            };
            if parts.is_empty() {
                return canonical();
            }

            let name = parts
                .iter()
                .map(|part| part.name.as_str())
                .collect::<Vec<_>>()
                .join(" // ");
            let kind = join_distinct(
                parts
                    .iter()
                    .map(|part| part.rules.kind_name().to_ascii_lowercase()),
            );
            let type_line = join_distinct(parts.iter().map(|part| part.rules.type_line()));
            let rules_text = parts
                .iter()
                .map(|part| format!("{} — {}", part.name, part.rules.rules_text()))
                .collect::<Vec<_>>()
                .join("\n\n");
            let stats = parts
                .iter()
                .filter_map(|part| part.rules.creature_stats())
                .collect::<Vec<_>>();
            let shared_stats = stats
                .first()
                .copied()
                .filter(|first| stats.iter().all(|stats| stats == first));
            let mana_cost = card
                .play_option(signature.play_option())
                .filter(|option| &option.form == signature.form())
                .and_then(|option| option.mana_cost);

            StackCardPresentation {
                name,
                kind,
                type_line,
                implementation_status: parts
                    .iter()
                    .map(|part| part.rules.implementation_status())
                    .reduce(penta::ImplementationStatus::combine)
                    .unwrap_or_default(),
                is_land: parts
                    .iter()
                    .any(|part| part.rules.has_type(penta::CardType::Land)),
                mana_cost,
                rules_text,
                power: shared_stats.map(|stats| stats.power),
                toughness: shared_stats.map(|stats| stats.toughness),
            }
        }
    }
}

pub(super) const fn implementation_status_name(
    status: penta::ImplementationStatus,
) -> &'static str {
    match status {
        penta::ImplementationStatus::Complete => "complete",
        penta::ImplementationStatus::Partial => "partial",
        penta::ImplementationStatus::MetadataOnly => "metadataOnly",
    }
}

fn join_distinct(values: impl IntoIterator<Item = String>) -> String {
    let mut distinct = Vec::new();
    for value in values {
        if !distinct.contains(&value) {
            distinct.push(value);
        }
    }
    distinct.join(" // ")
}

pub(super) fn deck_by_name(format: Format, name: &str) -> Result<penta::Deck, JsValue> {
    penta::protocol::deck_by_name_for_format(format, name)
        .ok_or_else(|| JsValue::from_str("unknown deck for format"))
}

/// Describes why the game ended from the browser player's seat.
/// `human_lost` selects the second-person phrasing.
pub(super) fn win_reason_text(reason: penta::WinReason, human_lost: bool) -> &'static str {
    match (reason, human_lost) {
        (penta::WinReason::OpponentConceded, false) => "opponent conceded",
        (penta::WinReason::OpponentConceded, true) => "you conceded",
        (penta::WinReason::OpponentLostAllLife, false) => "opponent lost all life",
        (penta::WinReason::OpponentLostAllLife, true) => "you lost all life",
        (penta::WinReason::OpponentTriedToDrawFromEmptyLibrary, false) => {
            "opponent drew from an empty library"
        }
        (penta::WinReason::OpponentTriedToDrawFromEmptyLibrary, true) => {
            "you drew from an empty library"
        }
        (penta::WinReason::OpponentLostToAnEffect, false) => "opponent lost to an effect",
        (penta::WinReason::OpponentLostToAnEffect, true) => "you lost to an effect",
        (penta::WinReason::OpponentRanOutOfTime, false) => "opponent ran out of time",
        (penta::WinReason::OpponentRanOutOfTime, true) => "you ran out of time",
        (penta::WinReason::OpponentPoisoned, false) => "opponent was poisoned",
        (penta::WinReason::OpponentPoisoned, true) => "you were poisoned",
    }
}
