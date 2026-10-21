use super::*;

fn setup() -> (Host, String, [String; 2]) {
    let mut host = Host::with_seed(
        Config {
            format: "old-school-93-94".into(),
            p1_deck: "Sligh".into(),
            p2_deck: "The Deck".into(),
            match_mode: "one-conclusion".into(),
        },
        u64::MAX,
    )
    .unwrap();
    let launch = host.launch_info("http://127.0.0.1:1234");
    let organizer = launch["organizerSecret"].as_str().unwrap().to_owned();
    let seats = host.request(&organizer, "seats", &json!({})).unwrap();
    let secrets =
        ["p1", "p2"].map(|seat| seats["seats"][seat]["secret"].as_str().unwrap().to_owned());
    (host, organizer, secrets)
}

fn observe(host: &mut Host, secret: &str) -> Value {
    host.request(secret, "observe", &json!({})).unwrap()
}

#[test]
fn proposal_observation_requires_explicit_discovery_for_legal_menus() {
    let (mut host, organizer, seats) = setup();
    let view = observe(&mut host, &seats[0]);
    assert_eq!(view["apiVersion"], 2);
    assert_eq!(view["observation"]["actionInterface"], "proposal-v1");
    assert!(view["observation"].get("legalActions").is_none());
    assert!(view["observation"]["hand"].is_array());
    assert!(host.request(&organizer, "discover", &json!({})).is_err());
    let discovered = host.request(&seats[0], "discover", &json!({})).unwrap();
    assert_eq!(discovered["apiVersion"], 1);
    assert_eq!(discovered["revision"], view["revision"]);
    assert!(
        discovered["observation"]["legalActions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["type"] == "KeepHand")
    );
    let refreshed = observe(&mut host, &seats[0]);
    assert!(refreshed["observation"].get("legalActions").is_none());
    assert_eq!(refreshed["revision"], view["revision"]);
}

#[test]
fn proposal_interaction_is_revision_checked_idempotent_and_replayable() {
    let (mut host, organizer, seats) = setup();
    let initial = observe(&mut host, &seats[0]);
    let args = json!({"requestId":"proposal-keep", "revision":initial["revision"],
        "choices":[{"proposal":{"action":{"type":"KeepHand"}}}]});
    let accepted = host.request(&seats[0], "interact", &args).unwrap();
    assert_eq!(accepted["receipt"]["accepted"], 1);
    let count = host.journal.len();
    let retry = host.request(&seats[0], "interact", &args).unwrap();
    assert_eq!(retry["receipt"], accepted["receipt"]);
    assert_eq!(host.journal.len(), count);
    let mut stale = args.clone();
    stale["requestId"] = json!("proposal-stale");
    assert_eq!(
        host.request(&seats[0], "interact", &stale)
            .unwrap_err()
            .status,
        409
    );
    let bundle = host.request(&organizer, "export", &json!({})).unwrap();
    assert_eq!(bundle["nativeReplayVersion"], 3);
    let replay = replay_bundle(&bundle).unwrap();
    assert_eq!(replay["observations"], bundle["observations"]);
}

#[test]
fn malformed_proposal_returns_safe_rejection_without_changing_state() {
    let (mut host, _, seats) = setup();
    let initial = observe(&mut host, &seats[0]);
    let rejected = host
        .request(
            &seats[0],
            "interact",
            &json!({
                "requestId":"bad-proposal", "revision":initial["revision"],
                "choices":[{"proposal":{"action":{"type":"CastSpell","card":1,
                    "choices":{"playOptionId":0,"targetSelections":[{"slotId":0,
                        "targets":[{"type":"player","seat":"p2","hidden":"forged"}]}]}}}}]
            }),
        )
        .unwrap();
    assert_eq!(rejected["receipt"]["accepted"], 0);
    let refusal = &rejected["receipt"]["rejection"];
    assert_eq!(refusal["code"], "invalidField");
    assert!(refusal["path"].as_str().unwrap().contains("hidden"));
    assert!(refusal["message"].is_string());
    assert!(host.journal.is_empty());
    let after = observe(&mut host, &seats[0]);
    assert_eq!(after["observation"], initial["observation"]);
}

#[test]
fn proposal_play_retains_typed_refusal_and_retry_receipt() {
    let (mut host, _, seats) = setup();
    let initial = observe(&mut host, &seats[0]);
    let args = json!({"requestId":"refused-play","revision":initial["revision"],
        "choices":[{"proposal":{"action":{"type":"PassPriority"}}}]});
    let rejected = host.request(&seats[0], "play", &args).unwrap();
    assert_eq!(rejected["receipt"]["accepted"], 0);
    assert_eq!(rejected["receipt"]["rejection"]["code"], "illegalAction");
    assert_eq!(
        host.request(&seats[0], "play", &args).unwrap()["receipt"],
        rejected["receipt"]
    );
    assert_eq!(
        observe(&mut host, &seats[0])["observation"],
        initial["observation"]
    );
}

#[test]
fn proposal_complete_and_waiting_contexts_do_not_insert_null_offer_arrays() {
    let (mut host, _, seats) = setup();
    assert_eq!(host.observations[1]["actionContext"]["kind"], "waiting");
    assert!(
        host.observations[1]["actionContext"]
            .get("castOffers")
            .is_none()
    );
    host.request(&seats[0], "concede", &json!({"requestId":"end-game"}))
        .unwrap();
    for seat in seats {
        let view = observe(&mut host, &seat);
        assert_eq!(view["status"], "complete");
        assert_eq!(view["observation"]["actionContext"]["kind"], "complete");
        assert!(
            view["observation"]["actionContext"]
                .get("castOffers")
                .is_none()
        );
    }
}
