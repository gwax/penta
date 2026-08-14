use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::model::{DecisionStateSnapshot, DecisionZoneSnapshot};
use super::wire::{array, field, player_from_index, u32_field, usize_field};
use super::{CardDefinitionId, CardInstance, GameObjectId, PlayerId};

pub(super) fn rebind_visible_decision_cards(
    observation: &Value,
    state: Option<&DecisionStateSnapshot>,
    viewer: PlayerId,
    hands: &mut [Vec<CardInstance>; 2],
    libraries: &mut [Vec<CardInstance>; 2],
    outside_game: &mut [Vec<CardInstance>; 2],
) -> Result<(), String> {
    let Some(decision) = observation.get("decision").filter(|value| !value.is_null()) else {
        return Ok(());
    };
    let Some(state) = state else {
        return Ok(());
    };

    let origins = state
        .card_origins
        .iter()
        .map(|origin| (GameObjectId(origin.object_id), *origin))
        .collect::<BTreeMap<_, _>>();
    let visible_cards = visible_decision_cards(decision)?;
    let mut rebound_hands = [BTreeSet::new(), BTreeSet::new()];
    let mut rebound_libraries = [BTreeSet::new(), BTreeSet::new()];
    let mut rebound_outside_game = [BTreeSet::new(), BTreeSet::new()];
    for (object, definition) in visible_cards {
        let Some(origin) = origins.get(&object) else {
            // Public-zone cards already keep their object ids. Only hidden
            // zones are reminted from the supplied hypothesis and therefore
            // have an origin entry in the checkpoint.
            continue;
        };
        let seat = player_from_index(origin.seat)?;
        let (cards, rebound, description, requires_exact_id) = match origin.zone {
            DecisionZoneSnapshot::Library => (
                &mut libraries[seat.index()],
                &mut rebound_libraries[seat.index()],
                "hidden library hypothesis",
                false,
            ),
            DecisionZoneSnapshot::Hand => (
                &mut hands[seat.index()],
                &mut rebound_hands[seat.index()],
                if seat == viewer {
                    "public hand"
                } else {
                    "hidden hand hypothesis"
                },
                seat == viewer,
            ),
            DecisionZoneSnapshot::OutsideGame => (
                &mut outside_game[seat.index()],
                &mut rebound_outside_game[seat.index()],
                "hidden outside-game hypothesis",
                false,
            ),
            _ => continue,
        };
        let index = cards
            .iter()
            .enumerate()
            .find(|(index, card)| {
                card.id == object
                    && (requires_exact_id || card.definition == definition)
                    && !rebound.contains(index)
            })
            .or_else(|| {
                if requires_exact_id {
                    None
                } else {
                    cards.iter().enumerate().rev().find(|(index, card)| {
                        card.definition == definition && !rebound.contains(index)
                    })
                }
            })
            .map(|(index, _)| index)
            .ok_or_else(|| format!("visible decision card is absent from the {description}"))?;
        cards[index].id = object;
        rebound.insert(index);
    }
    Ok(())
}

fn visible_decision_cards(
    decision: &Value,
) -> Result<BTreeMap<GameObjectId, CardDefinitionId>, String> {
    let mut cards = BTreeMap::new();
    for option in array(field(decision, "options")?)? {
        if let Some(card) = option.get("card").filter(|value| !value.is_null()) {
            insert_visible_card(&mut cards, card)?;
        }
        for member in array(field(option, "members")?)? {
            insert_visible_card(&mut cards, member)?;
        }
    }
    Ok(cards)
}

fn insert_visible_card(
    cards: &mut BTreeMap<GameObjectId, CardDefinitionId>,
    value: &Value,
) -> Result<(), String> {
    let object = GameObjectId(u32_field(value, "objectId")?);
    let definition = CardDefinitionId(
        u16::try_from(usize_field(value, "definition")?)
            .map_err(|_| "decision card definition is too large")?,
    );
    if let Some(previous) = cards.insert(object, definition)
        && previous != definition
    {
        return Err("one visible decision card has conflicting definitions".into());
    }
    Ok(())
}
