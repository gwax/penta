use super::*;
use crate::{Config, replay_bundle};

fn host() -> Host {
    Host::with_seed(
        Config {
            format: "old-school-93-94".into(),
            p1_deck: "Sligh".into(),
            p2_deck: "The Deck".into(),
            match_mode: "one-conclusion".into(),
        },
        1,
    )
    .unwrap()
}

#[test]
fn reset_contract_preserves_identical_events_and_private_automatic_inspections() {
    let repeated = json!({"type":"CardDrawn","seat":"p1"});
    let inspection = json!({"type":"AutomaticDecision","cards":["private-card"]});
    let old = [json!({"updates":[repeated]}), json!({"updates":[repeated]})];
    let new = [
        json!({"updates":[repeated,repeated,inspection]}),
        json!({"updates":[repeated,repeated]}),
    ];
    let events =
        appended(&old, &new, Some(PlayerId::One)).unwrap_or_else(|_| panic!("valid reset"));
    assert_eq!(
        events[0],
        vec![repeated.clone(), repeated.clone(), inspection.clone()]
    );
    assert_eq!(events[1], vec![repeated.clone()]);
    let automatic = appended(&old, &new, None).unwrap_or_else(|_| panic!("valid append"));
    assert_eq!(automatic[0], vec![repeated.clone(), inspection]);
    assert_eq!(automatic[1], vec![repeated]);
    assert!(appended(&new, &old, None).is_err());
}

#[test]
fn delivery_keeps_full_manual_batch_history_and_cursor_does_not_change_retry() {
    let mut host = host();
    let token = host.seats[0].clone();
    let initial = host.event_logs[0].clone();
    let mut args = json!({"requestId":"batch","revision":host.revision,"afterEvent":0,
        "choices":[{"action":{"type":"TakeMulligan"}},{"action":{"type":"KeepHand"}}]});
    let view = host.request(&token, "interact", &args).unwrap();
    assert_eq!(view["receipt"]["accepted"], 2);
    let mut expected = initial;
    expected.extend(
        view["receipt"]["updates"]
            .as_array()
            .unwrap()
            .iter()
            .cloned(),
    );
    assert_eq!(view["events"]["items"], json!(expected));
    let journal = host.journal.clone();
    args["afterEvent"] = view["events"]["next"].clone();
    let retry = host.request(&token, "interact", &args).unwrap();
    assert_eq!(retry["receipt"], view["receipt"]);
    assert_eq!(retry["events"]["items"], json!([]));
    assert_eq!(host.journal, journal);
    let all = host
        .request(&token, "observe", &json!({"afterEvent":0}))
        .unwrap();
    assert_eq!(all["events"]["items"], json!(expected));
    let receipt = host
        .request(
            &token,
            "receipt",
            &json!({"requestId":"batch","afterEvent":0}),
        )
        .unwrap();
    assert_eq!(receipt["events"], all["events"]);
    replay_bundle(&host.export()).unwrap();
}

#[test]
fn delivery_rejects_invalid_cursors_before_writes_and_keeps_legacy_views() {
    let mut host = host();
    let token = host.seats[0].clone();
    for cursor in [
        json!(-1),
        json!(0.5),
        json!("0"),
        json!(null),
        json!(usize::MAX),
    ] {
        let args = json!({"requestId":"invalid","revision":host.revision,"afterEvent":cursor,
            "choices":[{"action":{"type":"KeepHand"}}]});
        let before = host.export();
        assert_eq!(host.request(&token, "play", &args).unwrap_err().status, 400);
        assert_eq!(host.journal.len(), 0);
        assert_eq!(host.export()["events"], before["events"]);
        assert!(host.receipts[0].is_empty());
    }
    assert!(
        host.request(&token, "observe", &json!({}))
            .unwrap()
            .get("events")
            .is_none()
    );
    let organizer = host.organizer.clone();
    assert!(
        host.request(&organizer, "status", &json!({"afterEvent":0}))
            .is_err()
    );
}

#[test]
fn delivery_is_seat_local_even_while_waiting() {
    let mut host = host();
    host.event_logs[0].push(json!({"type":"AutomaticDecision","cards":["p1-private"]}));
    host.event_logs[1].push(json!({"type":"AutomaticDecision","cards":["p2-private"]}));
    for seat in [PlayerId::One, PlayerId::Two] {
        let token = host.seats[seat.index()].clone();
        let view = host
            .request(&token, "observe", &json!({"afterEvent":0}))
            .unwrap();
        assert_eq!(
            view["events"]["items"],
            json!(host.event_logs[seat.index()])
        );
        assert!(
            !view["events"]
                .to_string()
                .contains(if seat == PlayerId::One {
                    "p2-private"
                } else {
                    "p1-private"
                })
        );
        if seat == PlayerId::Two {
            assert_eq!(view["status"], "waiting");
        }
    }
}

#[test]
fn delivery_appends_only_new_events_from_automatic_yield_passes() {
    let mut host = host();
    for seat in [PlayerId::One, PlayerId::Two] {
        let token = host.seats[seat.index()].clone();
        let args = json!({"requestId":format!("keep-{}",seat.index()),"revision":host.revision,
            "choices":[{"action":{"type":"KeepHand"}}]});
        host.request(&token, "play", &args).unwrap();
    }
    let seat = host.session.decision_seat().unwrap();
    let token = host.seats[seat.index()].clone();
    let after = host.event_logs[seat.index()].len();
    let retained = host.observations[seat.index()]["updates"]
        .as_array()
        .unwrap()
        .len();
    let args = json!({"requestId":"yield","revision":host.revision,"afterEvent":after,
        "until":[{"type":"nextStep","step":"End"}],"maxPasses":64});
    let view = host.request(&token, "yield", &args).unwrap();
    assert!(view["yield"]["passes"].as_u64().unwrap() > 0);
    let expected = &host.observations[seat.index()]["updates"]
        .as_array()
        .unwrap()[retained..];
    assert_eq!(view["events"]["items"], json!(expected));
    assert_eq!(view["events"]["next"], after + expected.len());
    replay_bundle(&host.export()).unwrap();
}
