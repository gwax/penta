//! Watery Grave: two basic land types on a card that is not basic, which is
//! the half of the shock-land ruling the family test cannot show.

use super::*;

/// "Unlike most dual lands, this land has two basic land types. It's not
/// basic, so cards such as District Guide can't find it." The two halves of
/// that ruling pull in opposite directions and only the first is tested
/// above: a Rampant Growth wants the Basic supertype, which a Watery Grave
/// has never had however many basic types it carries.
#[test]
fn a_shock_land_has_basic_types_without_being_basic() {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[PlayerId::One.index()].hand.clear();
    game.players[PlayerId::One.index()].library.clear();
    for (index, definition) in [cards::WATERY_GRAVE, cards::ISLAND].into_iter().enumerate() {
        game.players[PlayerId::One.index()].library.push(card(
            41_800 + u32::try_from(index).expect("two cards"),
            definition,
            PlayerId::One,
        ));
    }
    let growth = card(41_810, cards::RAMPANT_GROWTH, PlayerId::One);
    let growth_id = growth.id;
    game.players[PlayerId::One.index()].hand.push(growth);
    game.turns_started = [5, 5];
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Green, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 1);

    let cast = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::CastSpell { card, .. } if *card == growth_id))
        .expect("two mana casts it");
    game.apply(PlayerId::One, cast).expect("it is cast");
    pass_until_decision(&mut game);

    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("the search asks what to take");
    let offered = decision
        .options
        .iter()
        .filter_map(|option| option.card.and_then(|(_, card)| card.card_definition()))
        .collect::<Vec<_>>();

    assert_eq!(
        offered,
        vec![cards::ISLAND],
        "the Island is basic and the Watery Grave is only a Swamp Island",
    );
}
