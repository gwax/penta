use super::*;
use crate::ManaColor;
use crate::card::KeywordAbility;

#[test]
fn catalog_semantics_rehydrate_an_animation_without_a_card_name_switch() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let key = animation_json(&crate::card::abilities::MISHRAS_FACTORY_ANIMATION);
    let rebuilt = catalog_animation(&catalog, &key).expect("animation is cataloged");
    assert_eq!(*rebuilt, crate::card::abilities::MISHRAS_FACTORY_ANIMATION);
}

#[test]
fn every_runtime_keyword_has_a_stable_checkpoint_round_trip() {
    let mut keywords = vec![
        KeywordAbility::Flying,
        KeywordAbility::Trample,
        KeywordAbility::Haste,
        KeywordAbility::FirstStrike,
        KeywordAbility::DoubleStrike,
        KeywordAbility::Banding,
        KeywordAbility::Vigilance,
        KeywordAbility::Defender,
        KeywordAbility::Deathtouch,
        KeywordAbility::Lifelink,
        KeywordAbility::Reach,
        KeywordAbility::Flash,
        KeywordAbility::Hexproof,
        KeywordAbility::Shroud,
        KeywordAbility::Intimidate,
        KeywordAbility::Undying,
        KeywordAbility::Indestructible,
        KeywordAbility::AttacksEachCombatIfAble,
        KeywordAbility::Mountainwalk,
        KeywordAbility::Forestwalk,
    ];
    keywords.extend(
        [
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
            ManaColor::Colorless,
        ]
        .map(KeywordAbility::ProtectionFrom),
    );
    for keyword in keywords {
        assert_eq!(parse_keyword(&keyword_json(keyword)), Ok(keyword));
    }
}

#[test]
fn checkpoint_redacts_opposing_hidden_object_ids() {
    let catalog = crate::poc::catalog().expect("catalog builds");
    let deck = crate::Deck {
        main: vec![crate::card::cards::MOUNTAIN; 60],
        sideboard: Vec::new(),
    };
    let mut game = Game::new(catalog, [deck.clone(), deck], 41).expect("game starts");
    let hidden = game.players[PlayerId::Two.index()].hand[0].id;
    game.drawn_this_turn[PlayerId::Two.index()] = vec![hidden];
    game.miracle_window = Some(hidden);

    let checkpoint = game.checkpoint_json(PlayerId::One);
    assert_eq!(checkpoint["drawnThisTurn"][1], json!([]));
    assert!(checkpoint["miracleWindow"].is_null());

    let own = game.players[PlayerId::One.index()].hand[0].id;
    game.drawn_this_turn[PlayerId::One.index()] = vec![own];
    game.miracle_window = Some(own);
    let checkpoint = game.checkpoint_json(PlayerId::One);
    assert_eq!(checkpoint["drawnThisTurn"][0], json!([own.0]));
    assert_eq!(checkpoint["miracleWindow"], own.0);
}
