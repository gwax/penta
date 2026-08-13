use serde_json::{Value, json};

use crate::{CardDefinitionId, CardPartId, GameObjectId};

use super::super::{CharacteristicSource, ContinuousEffectTimestamp, Game, Permanent};
use super::stack::parse_ability_origin;
use super::{array, card, field, seat_index, seat_value, u8_field, u32_field, usize_field};

pub(super) fn emblem_checkpoint_json(emblem: &Permanent) -> Value {
    json!({
        "objectId": emblem.card.id.0,
        "definition": emblem.card.definition.0,
        "owner": emblem.card.owner.index(),
        "presentedPartId": emblem.presented.0,
        "timestamp": emblem.timestamp.0,
        "enteredControllerTurn": emblem.entered_controller_turn,
    })
}

pub(super) fn parse_emblems(
    observation: &Value,
    checkpoint: &Value,
    game: &Game,
) -> Result<Vec<Permanent>, String> {
    let visible = array(field(observation, "emblems")?)?;
    let raw = array(field(checkpoint, "emblems")?)?;
    if visible.len() != raw.len() {
        return Err("checkpoint emblems do not match observation".into());
    }
    visible
        .iter()
        .zip(raw)
        .map(|(shown, state)| {
            let id = GameObjectId(u32_field(shown, "objectId")?);
            if id.0 != u32_field(state, "objectId")? {
                return Err("checkpoint emblem id does not match observation".into());
            }
            let definition = CardDefinitionId(
                u16::try_from(usize_field(state, "definition")?)
                    .map_err(|_| "emblem definition is too large")?,
            );
            let owner = seat_index(field(state, "owner")?)?;
            let controller = seat_value(field(shown, "controller")?)?;
            let presented = CardPartId(u8_field(state, "presentedPartId")?);
            let mut emblem = Permanent::entering(
                card(id, definition, owner, &game.catalog)?,
                presented,
                controller,
                u32_field(state, "enteredControllerTurn")?,
            );
            emblem.card.characteristics = CharacteristicSource::Ability(definition);
            emblem.timestamp = ContinuousEffectTimestamp(
                field(state, "timestamp")?
                    .as_u64()
                    .ok_or("emblem timestamp must be u64")?,
            );
            emblem.emblem_source = Some(parse_ability_origin(field(shown, "sourceAbility")?)?);
            Ok(emblem)
        })
        .collect()
}
