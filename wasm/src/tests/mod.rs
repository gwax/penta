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
