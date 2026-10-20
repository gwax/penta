//! Seat-local delivery history follows `ExactSession`'s explicit update reset contract.

use penta::PlayerId;
use serde_json::{Value, json};

use crate::play::Failure;
use crate::{Host, HostError, Result, number};

pub(crate) fn appended(
    previous: &[Value; 2],
    current: &[Value; 2],
    manual_seat: Option<PlayerId>,
) -> std::result::Result<[Vec<Value>; 2], Failure> {
    let mut result = [Vec::new(), Vec::new()];
    for seat in [PlayerId::One, PlayerId::Two] {
        let updates = current[seat.index()]["updates"]
            .as_array()
            .ok_or_else(|| Failure::Engine("missing seat updates".into()))?;
        let start = if manual_seat == Some(seat) {
            0
        } else {
            previous[seat.index()]["updates"]
                .as_array()
                .ok_or_else(|| Failure::Engine("missing previous seat updates".into()))?
                .len()
        };
        result[seat.index()] = updates
            .get(start..)
            .ok_or_else(|| Failure::Engine("retained seat updates unexpectedly shrank".into()))?
            .to_vec();
    }
    Ok(result)
}

impl Host {
    pub(crate) fn deliver(&mut self, seat: PlayerId, method: &str, args: &Value) -> Result<Value> {
        if args.get("afterEvent").is_none() {
            return self.pilot(seat, method, args);
        }
        if !matches!(
            method,
            "observe"
                | "wait"
                | "choose"
                | "play"
                | "interact"
                | "yield"
                | "cancelYield"
                | "receipt"
                | "concede"
        ) {
            return Err(HostError::new(
                400,
                "afterEvent is unavailable for this method",
            ));
        }
        let after = number(args, "afterEvent", 0, self.event_logs[seat.index()].len())?;
        let mut args = args.clone();
        args.as_object_mut()
            .expect("validated request object")
            .remove("afterEvent");
        let mut view = self.pilot(seat, method, &args)?;
        let events = &self.event_logs[seat.index()];
        view["events"] = json!({"after":after,"next":events.len(),"items":events[after..]});
        Ok(view)
    }
}

#[cfg(test)]
mod tests;
