use super::*;

mod actions;
mod autopass;
mod snapshots;

fn assert_nested_card_art(card: &Value) {
    assert!(card.get("scryfallId").is_none());
    assert!(card.get("artist").is_none());

    let art = card["art"].as_object().expect("card art is an object");
    assert_eq!(art.len(), 2);
    assert!(art["scryfallId"].as_str().is_some_and(|id| id.len() == 36));
    assert!(
        art["artist"]
            .as_str()
            .is_some_and(|artist| !artist.is_empty())
    );
}

fn act_matching(game: &mut WebGame, predicate: impl Fn(&Action) -> bool) {
    let action_index = game
        .session
        .observe(game.human)
        .legal_actions
        .iter()
        .position(predicate)
        .expect("matching legal action");
    game.act(action_index).expect("legal action succeeds");
}

fn apply_engine_action(game: &mut Game, predicate: impl Fn(&Action) -> bool) {
    let player = game.decision_player().expect("game has a decision player");
    let action = game
        .observe(player)
        .legal_actions
        .into_iter()
        .find(predicate)
        .expect("matching engine action");
    game.apply(player, action).expect("engine action succeeds");
}

fn advance_engine_quietly_until(game: &mut Game, stop: impl Fn(&PlayerObservation) -> bool) {
    for _ in 0..200 {
        let player = game.decision_player().expect("game remains in progress");
        let observation = game.observe(player);
        if stop(&observation) {
            return;
        }
        let action = observation
            .legal_actions
            .into_iter()
            .find(|action| {
                matches!(
                    action,
                    Action::PassPriority
                        | Action::FinishDeclaringAttackers
                        | Action::FinishDeclaringBlockers
                        | Action::DiscardCards { .. }
                        | Action::ChooseUntap { .. }
                )
            })
            .expect("a quiet action advances the test game");
        game.apply(player, action).expect("quiet action succeeds");
    }
    panic!("test game did not reach the requested state");
}

fn choices_targeting(target: Target) -> penta::CastChoices {
    penta::CastChoices::default().with_targets(vec![penta::TargetSelection::single(
        penta::TargetSlotId(0),
        target,
    )])
}

mod external_opponent {
    use super::*;

    fn hosted_external() -> WebGame {
        WebGame::new("Sligh", "Goblins", "External", true, 77, None).expect("game starts")
    }

    fn parsed(json: &str) -> serde_json::Value {
        serde_json::from_str(json).expect("observation is JSON")
    }

    /// Picks the first action the driver would consider a real play, the way
    /// the socket bots in the protocol tests do.
    fn driver_index(observation: &serde_json::Value) -> u32 {
        let actions = observation["legalActions"]
            .as_array()
            .expect("legalActions is an array");
        assert!(!actions.is_empty(), "the driver has something to do");
        u32::try_from(
            actions
                .iter()
                .position(|action| {
                    !matches!(action["type"].as_str(), Some("PassPriority" | "Concede"))
                })
                .unwrap_or(0),
        )
        .expect("index fits")
    }

    #[test]
    fn an_external_game_waits_for_the_driver_instead_of_inventing_a_policy() {
        let mut game = hosted_external();
        // The human settles their hand first.
        game.act(
            parsed(&game.state_json())["actions"]
                .as_array()
                .unwrap()
                .iter()
                .position(|action| action["label"] == "Keep this hand")
                .expect("keep is offered"),
        )
        .expect("keep applies");
        // Now the engine is parked on the opponent's mulligan, not playing it.
        assert!(game.opponent_is_deciding(), "the driver's seat is waiting");
        let observation = parsed(&game.opponent_observe_json().expect("external"));
        assert!(
            observation["legalActions"]
                .as_array()
                .is_some_and(|actions| !actions.is_empty()),
            "the driver sees its legal actions"
        );
    }

    #[test]
    fn the_driver_plays_by_protocol_index_and_the_human_gets_the_beats() {
        // The opponent goes first, so their whole first turn is a driver
        // window: mulligan, land, spells. The human should watch it happen
        // the way they watch a built-in opponent -- as beats and log lines --
        // even though every choice arrived from outside by index.
        let mut game =
            WebGame::new("Sligh", "Goblins", "External", false, 77, None).expect("game starts");
        let mut saw_beats = false;
        for _ in 0..4_000 {
            if game.opponent_is_deciding() {
                let observation = parsed(&game.opponent_observe_json().expect("external"));
                game.opponent_act(driver_index(&observation))
                    .expect("the driver's index is legal");
                continue;
            }
            let state = parsed(&game.state_json());
            saw_beats |= state["opponentActions"]
                .as_array()
                .is_some_and(|beats| !beats.is_empty());
            if state["events"].as_array().is_some_and(|events| {
                events.iter().any(|line| {
                    line.as_str()
                        .is_some_and(|line| line.starts_with("Opponent played"))
                })
            }) {
                break;
            }
            let actions = state["actions"].as_array().expect("actions").clone();
            let index = actions
                .iter()
                .position(|action| action["label"] == "Keep this hand")
                .or_else(|| actions.iter().position(|action| action["kind"] == "pass"))
                .unwrap_or(0);
            game.act(index).expect("the human's index is legal");
        }
        let state = parsed(&game.state_json());
        assert!(
            state["events"].as_array().is_some_and(|events| {
                events.iter().any(|line| {
                    line.as_str()
                        .is_some_and(|line| line.starts_with("Opponent played"))
                })
            }),
            "the driver's land shows up in the human's log"
        );
        assert!(saw_beats, "and the window produced beats to watch");
    }

    #[test]
    fn a_built_in_opponent_refuses_the_driver_entry_points() {
        let mut game =
            WebGame::new("Sligh", "Goblins", "Handcrafted", true, 77, None).expect("game starts");
        assert!(!game.opponent_is_deciding());
        assert!(game.opponent_observe_json().is_err());
        assert!(game.opponent_act(0).is_err());
    }

    #[test]
    fn the_driver_cannot_act_out_of_turn() {
        let mut game = hosted_external();
        // The human has not kept yet, so the opponent holds nothing.
        assert!(!game.opponent_is_deciding());
        assert!(game.opponent_act(0).is_err());
    }
}

#[test]
fn an_external_game_never_prints_the_seed() {
    // The seed reconstructs both libraries, and in an external game the
    // opponent is real. The built-in-policy snapshot keeps its courtesy line;
    // the external one must not have it anywhere.
    let external =
        WebGame::new("Sligh", "Goblins", "External", true, 4242, None).expect("game starts");
    assert!(
        !external.state_json().contains("seed"),
        "an external game's snapshot mentions no seed"
    );
    let local =
        WebGame::new("Sligh", "Goblins", "Handcrafted", true, 4242, None).expect("game starts");
    assert!(
        local.state_json().contains("Game started · seed 4242"),
        "a built-in game still shows its courtesy line"
    );
}
