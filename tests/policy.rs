use osarena::poc;
use osarena::{
    Action, Game, GameResult, HandcraftedPolicy, PlayerId, Policy, RandomPolicy, play_game,
};

const ACTION_LIMIT: usize = 50_000;

#[test]
fn random_policy_is_seeded_and_avoids_conceding() {
    let catalog = poc::catalog().unwrap();
    let game = Game::new(catalog, [poc::goblins(), poc::goblins()], 17).unwrap();
    let observation = game.observe(PlayerId::One);
    let mut first = RandomPolicy::new(99);
    let mut second = RandomPolicy::new(99);

    for _ in 0..20 {
        let first_action = first.choose_action(&observation);
        let second_action = second.choose_action(&observation);
        assert_eq!(first_action, second_action);
        assert!(!matches!(first_action, Some(Action::Concede)));
    }
}

#[test]
fn handcrafted_policy_decisively_beats_random_across_builtin_decks_and_seats() {
    let catalog = poc::catalog().unwrap();
    let decks = [poc::goblins(), poc::sligh(), poc::artifacts()];
    let mut wins = 0;
    let mut decided_games = 0;

    for deck in decks {
        for seed in 0..10 {
            for handcrafted_seat in [PlayerId::One, PlayerId::Two] {
                let mut game =
                    Game::new(catalog.clone(), [deck.clone(), deck.clone()], seed).unwrap();
                let mut handcrafted = HandcraftedPolicy::new(catalog.clone());
                let mut random = RandomPolicy::new(seed ^ 0xa11c_e5ed);
                let result = match handcrafted_seat {
                    PlayerId::One => {
                        play_game(&mut game, &mut handcrafted, &mut random, ACTION_LIMIT)
                    }
                    PlayerId::Two => {
                        play_game(&mut game, &mut random, &mut handcrafted, ACTION_LIMIT)
                    }
                }
                .unwrap();

                if let GameResult::Winner { winner, .. } = result {
                    decided_games += 1;
                    wins += usize::from(winner == handcrafted_seat);
                }
            }
        }
    }

    assert_eq!(decided_games, 60);
    assert!(
        wins >= 54,
        "handcrafted policy won only {wins} of {decided_games} games"
    );
}
