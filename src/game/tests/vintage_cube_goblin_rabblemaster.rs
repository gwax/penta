//! Goblin Rabblemaster: a Goblin every turn, a board that has to attack,
//! and a body that grows with the crowd it sends.

use super::*;

/// Player One with a Rabblemaster out since last turn and `others` beside
/// it, on Player One's turn just before combat.
fn staged(
    others: &[CardDefinitionId],
    goblin_tokens: usize,
) -> (Game, GameObjectId, Vec<GameObjectId>) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    let rabblemaster = game
        .put_onto_battlefield(PlayerId::One, cards::GOBLIN_RABBLEMASTER)
        .expect("cataloged");
    let mut friends = Vec::new();
    for definition in others {
        friends.push(
            game.put_onto_battlefield(PlayerId::One, *definition)
                .expect("cataloged"),
        );
    }
    for index in 0..goblin_tokens {
        let permanent = token_permanent(
            90_000 + u32::try_from(index).expect("the fixture has few tokens"),
            tokens::creature(&["Goblin"], &[ManaColor::Red], 1, 1),
            PlayerId::One,
        );
        friends.push(permanent.card.id);
        game.battlefield.push(permanent);
    }
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.turns_started = [1, 1];
    drain_pending(&mut game);
    game.active_player = PlayerId::One;
    game.step = Step::PrecombatMain;
    game.priority = PlayerId::One;
    (game, rabblemaster, friends)
}

/// Passes until the game stops. The card asks nothing of its own, but two
/// Rabblemasters put two triggers up at once and the game wants them
/// ordered, so any waiting question is answered with the first option.
fn settle(game: &mut Game) {
    drain_pending(game);
    game.check_state_based_actions();
}

/// Walks to the declare-attackers step, letting the beginning-of-combat
/// trigger resolve on the way.
fn reach_declare_attackers(game: &mut Game) {
    for _ in 0..24 {
        if game.step == Step::DeclareAttackers && !game.attackers_declared {
            return;
        }
        settle(game);
        if game.step == Step::DeclareAttackers && !game.attackers_declared {
            return;
        }
        let player = game.priority;
        if game.apply(player, Action::PassPriority).is_err() {
            return;
        }
    }
}

fn goblin_tokens(game: &Game) -> usize {
    game.battlefield
        .iter()
        .filter(|permanent| {
            is_token_with(
                permanent,
                token_with_haste(tokens::creature(&["Goblin"], &[ManaColor::Red], 1, 1)),
            )
        })
        .count()
}

fn permanent(game: &Game, id: GameObjectId) -> &Permanent {
    game.battlefield
        .iter()
        .find(|permanent| permanent.card.id == id)
        .expect("it is on the battlefield")
}

/// One Goblin at the beginning of each of your combats, and it can attack
/// the turn it arrives.
#[test]
fn it_makes_a_hasty_goblin_at_the_beginning_of_combat() {
    let (mut game, _rabblemaster, _friends) = staged(&[], 0);
    assert_eq!(goblin_tokens(&game), 0, "nothing yet");

    reach_declare_attackers(&mut game);

    assert_eq!(goblin_tokens(&game), 1, "one Goblin token");
    let token = game
        .battlefield
        .iter()
        .find(|permanent| {
            is_token_with(
                permanent,
                token_with_haste(tokens::creature(&["Goblin"], &[ManaColor::Red], 1, 1)),
            )
        })
        .expect("a token was made");
    assert!(
        game.permanent_has_executable_keyword(token, KeywordAbility::Haste),
        "with haste, which is the only reason it is worth making now",
    );
}

/// "Attack each combat if able" is granted, not printed: the game will not
/// let the attack step finish while an untapped Goblin is sitting home.
#[test]
fn other_goblins_are_made_to_attack() {
    let (mut game, _rabblemaster, friends) = staged(&[], 1);
    let goblin = friends[0];
    reach_declare_attackers(&mut game);

    assert!(
        game.permanent_has_executable_keyword(
            permanent(&game, goblin),
            KeywordAbility::AttacksEachCombatIfAble
        ),
        "the other Goblin was handed the requirement",
    );
    assert!(
        !game
            .legal_actions(PlayerId::One)
            .contains(&Action::FinishDeclaringAttackers),
        "and the step cannot be finished while it stays home",
    );
}

/// The Rabblemaster is a Goblin, but "other" excludes it: it is never made
/// to attack by its own clause.
#[test]
fn the_rabblemaster_is_not_made_to_attack_by_itself() {
    let (mut game, rabblemaster, _friends) = staged(&[], 0);
    reach_declare_attackers(&mut game);

    assert!(
        !game.permanent_has_executable_keyword(
            permanent(&game, rabblemaster),
            KeywordAbility::AttacksEachCombatIfAble
        ),
        "\"other\" leaves the Rabblemaster out",
    );
}

/// A creature that is not a Goblin is left alone, however friendly.
#[test]
fn a_nongoblin_is_left_alone() {
    let (mut game, _rabblemaster, friends) = staged(&[cards::SAVANNAH_LIONS], 0);
    let lions = friends[0];
    reach_declare_attackers(&mut game);

    assert!(
        !game.permanent_has_executable_keyword(
            permanent(&game, lions),
            KeywordAbility::AttacksEachCombatIfAble
        ),
        "a Savannah Lions is nobody's Goblin",
    );
}

/// The attack trigger counts the crowd: two other attacking Goblins make a
/// 2/2 into a 4/2.
#[test]
fn it_grows_by_one_for_each_other_attacking_goblin() {
    let (mut game, rabblemaster, friends) = staged(&[], 2);
    reach_declare_attackers(&mut game);

    let _ = friends;
    // Everything goes, which is the point of the card: the two Goblin
    // tokens staged here, the one its own combat trigger just made, and the
    // Rabblemaster itself.
    while let Some(action) = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::DeclareAttacker { .. }))
    {
        game.apply(PlayerId::One, action).expect("it attacks");
    }
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("every Goblin that had to attack did");
    settle(&mut game);

    assert_eq!(
        game.power(permanent(&game, rabblemaster)),
        Some(5),
        "a 2/2 plus one for each of the three other attacking Goblins",
    );
    assert_eq!(
        game.toughness(permanent(&game, rabblemaster)),
        Some(2),
        "and +1/+0 leaves the toughness alone",
    );
}

/// Attacking alone is worth nothing: the count is of *other* Goblins.
#[test]
fn attacking_alone_grows_it_by_nothing() {
    let (mut game, rabblemaster, _friends) = staged(&[], 0);
    reach_declare_attackers(&mut game);
    // The token its combat trigger just made would have to attack too, so
    // it is taken off the board: what is being measured is a Rabblemaster
    // with no company.
    game.battlefield.retain(|permanent| {
        !is_token_with(
            permanent,
            token_with_haste(tokens::creature(&["Goblin"], &[ManaColor::Red], 1, 1)),
        )
    });
    let action = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| {
            matches!(action, Action::DeclareAttacker { attacker, .. } if *attacker == rabblemaster)
        })
        .expect("it can attack");
    game.apply(PlayerId::One, action).expect("it attacks");
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("nothing else had to attack");
    settle(&mut game);

    assert_eq!(
        game.power(permanent(&game, rabblemaster)),
        Some(2),
        "still a 2/2",
    );
}

/// "Although Goblin Rabblemaster doesn't force itself to attack, if you
/// control two of them, they'll force each other to attack if able."
#[test]
fn two_of_them_force_each_other() {
    let (mut game, rabblemaster, friends) = staged(&[cards::GOBLIN_RABBLEMASTER], 0);
    let other = friends[0];
    reach_declare_attackers(&mut game);

    for (id, description) in [(rabblemaster, "the first"), (other, "the second")] {
        assert!(
            game.permanent_has_executable_keyword(
                permanent(&game, id),
                KeywordAbility::AttacksEachCombatIfAble
            ),
            "{description} Rabblemaster is another Rabblemaster's other Goblin",
        );
    }
}

/// "If, during your declare attackers step, a creature that must attack if
/// able ... hasn't been under your control continuously since the turn
/// began (and doesn't have haste), then it doesn't attack." The
/// requirement is still on it; being unable is what excuses it.
#[test]
fn a_goblin_that_arrived_this_turn_stays_home() {
    let (mut game, _rabblemaster, _friends) = staged(&[], 0);
    let newcomer = token_permanent(
        90_500,
        tokens::creature(&["Goblin"], &[ManaColor::Red], 1, 1),
        PlayerId::One,
    );
    let newcomer_id = newcomer.card.id;
    game.battlefield.push(newcomer);
    game.battlefield
        .iter_mut()
        .find(|permanent| permanent.card.id == newcomer_id)
        .expect("it was just pushed")
        .entered_controller_turn = game.turns_started[PlayerId::One.index()];
    reach_declare_attackers(&mut game);

    assert!(
        game.permanent_has_executable_keyword(
            permanent(&game, newcomer_id),
            KeywordAbility::AttacksEachCombatIfAble
        ),
        "it is a Goblin and it was handed the requirement",
    );
    while let Some(action) = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::DeclareAttacker { .. }))
    {
        game.apply(PlayerId::One, action).expect("it attacks");
    }
    assert!(
        !permanent(&game, newcomer_id).attacking,
        "no haste and no history, so it never had the chance",
    );
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("a Goblin that cannot attack does not hold the step open");
}

/// "The number of attacking Goblins is counted as the last ability
/// resolves, and the bonus is locked in at that time." Two of the crowd
/// die afterwards and the Rabblemaster is no smaller for it.
#[test]
fn the_bonus_is_locked_in_and_does_not_follow_the_board() {
    let (mut game, rabblemaster, _friends) = staged(&[], 2);
    send_everything(&mut game);
    assert_eq!(
        game.power(permanent(&game, rabblemaster)),
        Some(5),
        "two staged Goblins and the one its combat trigger made",
    );

    let doomed = attacking_goblins_other_than(&game, rabblemaster);
    assert_eq!(doomed.len(), 3, "the whole crowd is there to be thinned");
    for id in &doomed[..2] {
        game.destroy_permanent(*id);
    }
    settle(&mut game);

    assert_eq!(
        game.power(permanent(&game, rabblemaster)),
        Some(5),
        "the count was taken once and is not recounted",
    );
}

/// The same rule read forwards: a Goblin removed before the trigger
/// resolves was never part of the count.
#[test]
fn a_goblin_removed_in_response_is_never_counted() {
    let (mut game, rabblemaster, _friends) = staged(&[], 2);
    declare_everything(&mut game);
    let doomed = attacking_goblins_other_than(&game, rabblemaster);
    assert_eq!(doomed.len(), 3, "three of them made it to the attack");
    game.destroy_permanent(doomed[0]);
    settle(&mut game);

    assert_eq!(
        game.power(permanent(&game, rabblemaster)),
        Some(4),
        "two other attacking Goblins were there when the count was taken",
    );
}

/// Every Goblin the board can send, up to the point where the attack
/// trigger is waiting to resolve.
fn declare_everything(game: &mut Game) {
    reach_declare_attackers(game);
    while let Some(action) = game
        .legal_actions(PlayerId::One)
        .into_iter()
        .find(|action| matches!(action, Action::DeclareAttacker { .. }))
    {
        game.apply(PlayerId::One, action).expect("it attacks");
    }
    game.apply(PlayerId::One, Action::FinishDeclaringAttackers)
        .expect("every Goblin that had to attack did");
}

/// The same, with the attack trigger resolved.
fn send_everything(game: &mut Game) {
    declare_everything(game);
    settle(game);
}

fn attacking_goblins_other_than(game: &Game, rabblemaster: GameObjectId) -> Vec<GameObjectId> {
    game.battlefield
        .iter()
        .filter(|permanent| permanent.attacking && permanent.card.id != rabblemaster)
        .map(|permanent| permanent.card.id)
        .collect()
}
