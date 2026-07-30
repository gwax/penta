use osarena::card::{CardCatalog, CardDefinition, CardSet};
use osarena::deck::{Deck, DeckError};
use osarena::game::{GameResult, WinReason};
use osarena::{Action, CardDefinitionId, Game, PlayerId};

fn catalog() -> CardCatalog {
    CardCatalog::new([
        CardDefinition {
            id: CardDefinitionId(1),
            name: "Mountain".into(),
            set: CardSet::Alpha,
            is_basic_land: true,
        },
        CardDefinition {
            id: CardDefinitionId(2),
            name: "Lightning Bolt".into(),
            set: CardSet::Alpha,
            is_basic_land: false,
        },
        CardDefinition {
            id: CardDefinitionId(3),
            name: "Black Lotus".into(),
            set: CardSet::Alpha,
            is_basic_land: false,
        },
        CardDefinition {
            id: CardDefinitionId(4),
            name: "Contract from Below".into(),
            set: CardSet::Alpha,
            is_basic_land: false,
        },
    ])
    .unwrap()
}

fn valid_deck(catalog: &CardCatalog) -> osarena::ValidatedDeck {
    let mut main = vec![CardDefinitionId(1); 55];
    main.extend([CardDefinitionId(2); 4]);
    main.push(CardDefinitionId(3));
    Deck {
        main,
        sideboard: Vec::new(),
    }
    .validate(catalog)
    .unwrap()
}

#[test]
fn restricted_cards_are_limited_across_deck_and_sideboard() {
    let catalog = catalog();
    let mut main = vec![CardDefinitionId(1); 58];
    main.extend([CardDefinitionId(3); 2]);
    let error = Deck {
        main,
        sideboard: Vec::new(),
    }
    .validate(&catalog)
    .unwrap_err();

    assert_eq!(
        error,
        DeckError::TooManyCopies {
            card: "Black Lotus".into(),
            count: 2,
            limit: 1,
        }
    );
}

#[test]
fn banned_cards_are_rejected() {
    let catalog = catalog();
    let mut main = vec![CardDefinitionId(1); 59];
    main.push(CardDefinitionId(4));

    assert_eq!(
        Deck {
            main,
            sideboard: Vec::new(),
        }
        .validate(&catalog)
        .unwrap_err(),
        DeckError::BannedCard("Contract from Below".into())
    );
}

#[test]
fn setup_is_deterministic_and_hides_the_opponents_hand() {
    let catalog = catalog();
    let game_a = Game::new([valid_deck(&catalog), valid_deck(&catalog)], 0xdeca_fbad).unwrap();
    let game_b = Game::new([valid_deck(&catalog), valid_deck(&catalog)], 0xdeca_fbad).unwrap();

    assert_eq!(game_a.observe(PlayerId::One), game_b.observe(PlayerId::One));
    let observation = game_a.observe(PlayerId::One);
    assert_eq!(observation.hand.len(), 7);
    assert_eq!(observation.opponent_hand_size, 7);
    assert_eq!(observation.library_sizes, [53, 53]);
}

#[test]
fn concession_ends_the_game() {
    let catalog = catalog();
    let mut game = Game::new([valid_deck(&catalog), valid_deck(&catalog)], 123).unwrap();

    game.apply(PlayerId::One, Action::Concede).unwrap();

    let observation = game.observe(PlayerId::Two);
    assert_eq!(
        observation.result,
        Some(GameResult::Winner {
            winner: PlayerId::Two,
            reason: WinReason::OpponentConceded,
        })
    );
    assert!(observation.legal_actions.is_empty());
}
