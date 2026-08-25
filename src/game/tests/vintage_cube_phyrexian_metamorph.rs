//! Phyrexian Metamorph: four mana, or three and two life, for the best
//! artifact or creature on the table.

use super::*;

/// The Metamorph in hand with `theirs` on the other side and enough mana to
/// cast it the ordinary way.
fn staged(theirs: &[CardDefinitionId]) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let mut ids = Vec::new();
    for definition in theirs {
        ids.push(
            game.put_onto_battlefield(PlayerId::Two, *definition)
                .expect("cataloged"),
        );
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [5, 5];
    drain_pending(&mut game);
    let metamorph = game
        .build_zone(PlayerId::One, &[cards::PHYREXIAN_METAMORPH])
        .expect("cataloged")
        .into_iter()
        .next()
        .expect("one card");
    let id = metamorph.id;
    game.players[0].hand.push(metamorph);
    game.players[0].life = 20;
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, id, ids)
}

fn casts(game: &Game, metamorph: GameObjectId) -> Vec<Action> {
    game.legal_actions(PlayerId::One)
        .into_iter()
        .filter(|action| matches!(action, Action::CastSpell { card, .. } if *card == metamorph))
        .collect()
}

/// Casts it and answers the copy choice with `copied`, or takes the
/// enter-as-itself option when nothing is named.
fn cast_copying(game: &mut Game, metamorph: GameObjectId, copied: Option<GameObjectId>) {
    let cast = casts(game, metamorph)
        .into_iter()
        .next()
        .expect("the mana is there");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..12 {
        if let Some(decision) = game.observe(PlayerId::One).decision {
            let option = match copied {
                Some(copied) => {
                    decision
                        .options
                        .iter()
                        .find(|option| option.card.is_some_and(|(id, _)| id == copied))
                        .expect("the permanent is on the menu")
                        .id
                }
                None => {
                    decision
                        .options
                        .iter()
                        .find(|option| option.label == "Enter as itself")
                        .expect("entering as itself is always offered")
                        .id
                }
            };
            game.apply(
                PlayerId::One,
                Action::ChooseDecision {
                    decision: decision.id,
                    options: vec![option],
                },
            )
            .expect("the choice is legal");
            continue;
        }
        if game.stack.is_empty() && game.pending_triggers.is_empty() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }
    game.check_state_based_actions();
}

fn the_metamorph(game: &Game) -> Option<&Permanent> {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.definition == cards::PHYREXIAN_METAMORPH)
}

/// It copies a creature and adds artifact to what it copied.
#[test]
fn it_copies_a_creature_and_is_also_an_artifact() {
    let (mut game, metamorph, theirs) = staged(&[cards::SERRA_ANGEL]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);

    cast_copying(&mut game, metamorph, Some(theirs[0]));

    let copy = the_metamorph(&game).expect("it entered");
    assert_eq!(game.power(copy), Some(4), "a copy of the Angel");
    assert_eq!(game.toughness(copy), Some(4));
    assert!(game.has_flying(copy), "with the Angel's abilities");
    let types = game.permanent_types(copy).expect("it has types");
    assert!(types.contains(CardType::Creature));
    assert!(
        types.contains(CardType::Artifact),
        "and an artifact in addition to its other types",
    );
}

/// An artifact is just as good a choice, which is what separates it from
/// Clone.
#[test]
fn it_copies_an_artifact_too() {
    let (mut game, metamorph, theirs) = staged(&[cards::JAYEMDAE_TOME]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);

    cast_copying(&mut game, metamorph, Some(theirs[0]));

    let copy = the_metamorph(&game).expect("it entered");
    assert_eq!(
        game.effective_rules(copy).map(|rules| rules.mana_cost()),
        game.catalog
            .get(cards::JAYEMDAE_TOME)
            .map(|definition| definition.rules.mana_cost()),
        "it is the Tome in every copiable respect",
    );
}

/// A land is not on the menu, and neither is the Metamorph itself.
#[test]
fn it_copies_only_artifacts_and_creatures() {
    let (mut game, metamorph, theirs) = staged(&[cards::MOUNTAIN, cards::GRIZZLY_BEARS]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);
    let cast = casts(&game, metamorph)
        .into_iter()
        .next()
        .expect("the mana is there");
    game.apply(PlayerId::One, cast).expect("it is cast");
    for _ in 0..8 {
        if game.observe(PlayerId::One).decision.is_some() {
            break;
        }
        let priority = game.priority;
        if game.apply(priority, Action::PassPriority).is_err() {
            break;
        }
    }

    let decision = game
        .observe(PlayerId::One)
        .decision
        .expect("entering asks what to copy");
    let offered = decision
        .options
        .iter()
        .filter_map(|option| option.card.map(|(id, _)| id))
        .collect::<Vec<_>>();

    assert!(offered.contains(&theirs[1]), "the creature is a choice");
    assert!(!offered.contains(&theirs[0]), "and the land is not");
}

/// Copying nothing leaves a 0/0, which dies where it stands.
#[test]
fn entering_as_itself_is_a_zero_zero() {
    let (mut game, metamorph, _) = staged(&[cards::GRIZZLY_BEARS]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Blue, 1);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);

    cast_copying(&mut game, metamorph, None);

    assert!(
        the_metamorph(&game).is_none(),
        "a 0/0 with nothing copied is put into the graveyard",
    );
    assert!(
        game.players[0]
            .graveyard
            .iter()
            .any(|card| card.definition == cards::PHYREXIAN_METAMORPH),
    );
}

/// The Phyrexian pip is the reason it is playable off one Island: three
/// colourless and two life gets it down.
#[test]
fn the_phyrexian_pip_can_be_paid_with_life() {
    let (mut game, metamorph, theirs) = staged(&[cards::SERRA_ANGEL]);
    game.add_unrestricted_mana(PlayerId::One, ManaColor::Colorless, 3);

    assert!(
        !casts(&game, metamorph).is_empty(),
        "three colourless and a life payment is a legal cast",
    );

    cast_copying(&mut game, metamorph, Some(theirs[0]));

    assert_eq!(game.players[0].life, 18, "two life for the pip");
    assert!(the_metamorph(&game).is_some(), "and the copy is out");
}
