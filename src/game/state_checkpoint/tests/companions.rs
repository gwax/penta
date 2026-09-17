use super::*;
use crate::card::cards;
use crate::game::tests::companions::{choose, new_game};

#[test]
fn companion_choice_designation_and_used_access_round_trip() {
    let mut game = new_game([
        vec![cards::LURRUS_OF_THE_DREAM_DEN, cards::ZIRDA_THE_DAWNWAKER],
        vec![],
    ]);
    let (_, mut rebuilt) = rebuild_current_checkpoint(&game, PlayerId::One, 14);
    assert!(rebuilt.players[0].hand.is_empty());
    choose(
        &mut rebuilt,
        PlayerId::One,
        Some(cards::LURRUS_OF_THE_DREAM_DEN),
    );
    choose(
        &mut game,
        PlayerId::One,
        Some(cards::LURRUS_OF_THE_DREAM_DEN),
    );
    for viewer in [PlayerId::One, PlayerId::Two] {
        let (_, restored) = rebuild_current_checkpoint(&game, viewer, 15);
        assert_eq!(restored.players[0].companion, game.players[0].companion);
    }
    game.pregame = None;
    game.step = crate::Step::PrecombatMain;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    let chosen = game.players[0].companion.unwrap();
    game.apply(PlayerId::One, Action::TakeCompanion { card: chosen.card })
        .unwrap();
    let (_, restored) = rebuild_current_checkpoint(&game, PlayerId::Two, 16);
    assert!(restored.players[0].companion.unwrap().used);
    assert!(restored.observe(PlayerId::One).companions.is_empty());
}

#[test]
fn companion_brought_in_by_another_effect_does_not_spend_its_special_action() {
    let mut game = new_game([vec![cards::LURRUS_OF_THE_DREAM_DEN], vec![]]);
    choose(
        &mut game,
        PlayerId::One,
        Some(cards::LURRUS_OF_THE_DREAM_DEN),
    );
    let moved = game.players[0].sideboard.remove(0);
    let (moved, _) = game.zone_change_card(moved);
    game.players[0].hand.push(moved);
    let (_, restored) = rebuild_current_checkpoint(&game, PlayerId::Two, 18);
    assert!(!restored.players[0].companion.unwrap().used);
    assert!(restored.observe(PlayerId::One).companions.is_empty());
}

#[test]
fn companion_selection_round_trips_between_the_two_reveals() {
    let mut game = new_game([
        vec![cards::LURRUS_OF_THE_DREAM_DEN],
        vec![cards::LUTRI_THE_SPELLCHASER],
    ]);
    choose(
        &mut game,
        PlayerId::One,
        Some(cards::LURRUS_OF_THE_DREAM_DEN),
    );
    let (_, mut restored) = rebuild_current_checkpoint(&game, PlayerId::Two, 19);
    assert_eq!(restored.players[0].companion, game.players[0].companion);
    assert!(restored.players.iter().all(|player| player.hand.is_empty()));
    choose(
        &mut restored,
        PlayerId::Two,
        Some(cards::LUTRI_THE_SPELLCHASER),
    );
    assert!(restored.players.iter().all(|player| player.hand.len() == 7));
}
