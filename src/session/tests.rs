use super::*;
use crate::card::cards;
use crate::protocol::{BotGame, Opponent};
use serde_json::json;

fn session(mode: MatchMode) -> ExactSession {
    ExactSession::new(Format::OldSchool9394, "Sligh", "The Deck", 42, mode).unwrap()
}

fn keep_hands(session: &mut ExactSession) {
    for seat in [PlayerId::One, PlayerId::Two] {
        session
            .apply_choice(seat, &json!({"action": {"type": "KeepHand"}}))
            .unwrap();
        session.advance_forced().unwrap();
    }
}

#[test]
fn exact_session_rejects_stale_and_malformed_choices_without_changing_views() {
    let mut session = session(MatchMode::OneConclusion);
    let before = [PlayerId::One, PlayerId::Two].map(|seat| session.observe(seat));
    for choice in [
        json!({"index": -1}),
        json!({"index": 4_294_967_296_u64}),
        json!({"index": 0, "extra": true}),
        json!({"decision": 999, "options": []}),
        json!({"action": {"type": "KeepHand", "index": 0}}),
        json!({"action": {"type": "PassPriority"}}),
    ] {
        assert!(session.apply_choice(PlayerId::One, &choice).is_err());
        assert_eq!(
            before,
            [PlayerId::One, PlayerId::Two].map(|seat| session.observe(seat))
        );
    }
    assert!(
        session
            .apply_choice(PlayerId::Two, &json!({"index": 0}))
            .is_err()
    );
    session
        .apply_choice(PlayerId::One, &json!({"action": {"type": "KeepHand"}}))
        .unwrap();
    assert_eq!(session.decision_seat(), Some(PlayerId::Two));
}

#[test]
fn exact_session_forced_steps_preserve_native_observations_and_real_choices() {
    let mut session = session(MatchMode::OneConclusion);
    let mut native = BotGame::new_with_format(
        Format::OldSchool9394,
        "Sligh",
        "The Deck",
        Opponent::External,
        PlayerId::Two,
        42,
    )
    .unwrap();
    let mut forced_count = 0;
    for _ in 0..160 {
        session.advance_forced().unwrap();
        while let Some(seat) = native.decision_seat() {
            let observation: Value = serde_json::from_str(&native.observe_json(seat)).unwrap();
            if observation["forcedAction"].is_null() {
                break;
            }
            let index = observation["legalActions"]
                .as_array()
                .unwrap()
                .iter()
                .position(|action| {
                    let mut action = action.clone();
                    action.as_object_mut().unwrap().remove("index");
                    action == observation["forcedAction"]
                })
                .unwrap();
            native.act(index).unwrap();
            forced_count += 1;
        }
        for seat in [PlayerId::One, PlayerId::Two] {
            let mut observed = session.observe(seat);
            observed.as_object_mut().unwrap().remove("updates");
            let expected: Value = serde_json::from_str(&native.observe_json(seat)).unwrap();
            assert_eq!(observed, expected);
        }
        let Some(seat) = session.decision_seat() else {
            break;
        };
        let observation = session.observe(seat);
        assert!(observation["forcedAction"].is_null());
        let actions = observation["legalActions"].as_array().unwrap();
        let action = actions
            .iter()
            .find(|action| action["type"] == "DeclareAttacker")
            .or_else(|| {
                actions.iter().find(|action| {
                    !matches!(
                        action["type"].as_str(),
                        Some(
                            "PassPriority"
                                | "ActivateManaAbility"
                                | "TakeMulligan"
                                | "BeginPayment"
                        )
                    )
                })
            })
            .unwrap_or(&actions[0]);
        session
            .apply_choice(seat, &json!({"index": action["index"]}))
            .unwrap();
        native
            .act(usize::try_from(action["index"].as_u64().unwrap()).unwrap())
            .unwrap();
    }
    assert!(forced_count > 0);
}

#[test]
fn exact_session_private_forced_inspection_is_retained_only_for_its_viewer() {
    let mut session = ExactSession::new(
        Format::IsdM14Standard,
        "Greer G/R Aggro",
        "Briksza Naya Midrange",
        42,
        MatchMode::OneConclusion,
    )
    .unwrap();
    keep_hands(&mut session);
    let seat = PlayerId::One;
    session.game.set_hand(seat, &[cards::MOUNTAIN]).unwrap();
    session.game.set_hand(seat.opponent(), &[]).unwrap();
    session.game.set_library(seat, &[cards::MOUNTAIN]).unwrap();
    let domri = session
        .game
        .put_onto_battlefield(seat, cards::DOMRI_RADE)
        .unwrap();
    let action = protocol::protocol_actions(&session.game.observe(seat))
        .into_iter()
        .find(|action| matches!(action, Action::ActivateAbility { source, .. } if *source == domri))
        .unwrap();
    session
        .apply_choice(seat, &json!({"action": protocol::action_json(&action)}))
        .unwrap();
    session.advance_forced().unwrap();
    let own = session.observe(seat);
    let other = session.observe(seat.opponent());
    assert!(own["decision"].is_null());
    assert!(own["updates"].as_array().unwrap().iter().any(|update| {
        update["type"] == "AutomaticDecision" && update.to_string().contains("Mountain")
    }));
    assert!(!other["updates"].to_string().contains("AutomaticDecision"));
    assert!(!other["updates"].to_string().contains("Mountain"));
    assert_eq!(
        own,
        session.observe(seat),
        "reads retain inspection history"
    );
    let accepted = own["legalActions"][0]["index"].clone();
    session
        .apply_choice(seat, &json!({"index": accepted}))
        .unwrap();
    assert!(
        !session.observe(seat)["updates"]
            .to_string()
            .contains("AutomaticDecision")
    );
}

#[test]
fn exact_session_match_concession_enters_private_sideboarding() {
    let mut session = session(MatchMode::FirstToTwoWins);
    let opening = session.observe(PlayerId::One)["decision"].clone();
    session
        .apply_choice(
            PlayerId::One,
            &json!({"decision": opening["id"], "options": [0]}),
        )
        .unwrap();
    keep_hands(&mut session);
    session.concede(PlayerId::Two).unwrap();
    assert!(!session.is_finished());
    assert_eq!(
        session.observe(PlayerId::One)["match"]["wins"],
        json!([1, 0])
    );
    for _ in 0..2 {
        let seat = session.decision_seat().unwrap();
        let observation = session.observe(seat);
        assert_eq!(observation["match"]["stage"], "sideboarding");
        assert!(session.observe(seat.opponent())["decision"].is_null());
        let decision = &observation["decision"];
        let options: Vec<_> = decision["options"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|option| option["zone"] == "Library")
            .map(|option| option["id"].clone())
            .collect();
        session
            .apply_choice(
                seat,
                &json!({"decision": decision["id"], "options": options}),
            )
            .unwrap();
    }
    assert_eq!(session.decision_seat(), Some(PlayerId::Two));
    let decision = session.observe(PlayerId::Two)["decision"].clone();
    session
        .apply_choice(
            PlayerId::Two,
            &json!({"decision": decision["id"], "options": [0]}),
        )
        .unwrap();
    session.concede(PlayerId::Two).unwrap();
    assert!(session.is_finished());
    assert_eq!(
        session.observe(PlayerId::One)["match"]["wins"],
        json!([2, 0])
    );
}
