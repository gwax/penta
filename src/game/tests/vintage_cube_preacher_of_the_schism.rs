//! Preacher of the Schism: two clauses about one attack, each asking who is
//! ahead on life.

use super::*;

/// Her on the battlefield since last turn, with the two life totals set.
fn staged(yours: i16, theirs: i16) -> (Game, GameObjectId) {
    let mut game = ready_game();
    game.battlefield.clear();
    game.players[0].hand.clear();
    game.players[0].library.clear();
    for (index, definition) in [cards::MOUNTAIN, cards::FOREST].into_iter().enumerate() {
        let id = 273_000 + u32::try_from(index).expect("two cards");
        game.players[0]
            .library
            .push(card(id, definition, PlayerId::One));
    }
    let preacher = game
        .put_onto_battlefield(PlayerId::One, cards::PREACHER_OF_THE_SCHISM)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }
    game.players[0].life = yours;
    game.players[1].life = theirs;
    game.turns_started = [5, 5];
    game.turn = 9;
    game.active_player = PlayerId::One;
    game.priority = PlayerId::One;
    (game, preacher)
}

fn settle(game: &mut Game) {
    for _ in 0..32 {
        if let Some(decision) = game
            .pending_decisions
            .first()
            .map(|pending| pending.observation.clone())
        {
            let options = decision
                .options
                .iter()
                .map(|option| option.id)
                .take(decision.minimum.max(1))
                .collect();
            game.apply(
                decision.player,
                Action::ChooseDecision {
                    decision: decision.id,
                    options,
                },
            )
            .expect("the offered choice is legal");
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

/// She attacks the other player, and whatever triggers is carried through.
fn attack(game: &mut Game, preacher: GameObjectId) {
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(preacher, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    settle(game);
}

fn tokens(game: &Game) -> Vec<(i16, i16)> {
    game.battlefield
        .iter()
        .filter(|permanent| !matches!(permanent.card.definition, ObjectKind::Card(_)))
        .map(|permanent| {
            (
                game.power(permanent).unwrap_or_default(),
                game.toughness(permanent).unwrap_or_default(),
            )
        })
        .collect()
}

/// Deathtouch is printed on her, and it is hers whatever the life totals do.
#[test]
fn she_has_deathtouch() {
    let (game, preacher) = staged(20, 20);
    let permanent = game
        .battlefield
        .iter()
        .find(|permanent| permanent.card.id == preacher)
        .expect("she is there");

    assert!(game.permanent_has_executable_keyword(permanent, KeywordAbility::Deathtouch));
    assert_eq!(game.power(permanent), Some(2));
    assert_eq!(game.toughness(permanent), Some(4));
}

/// Attacking the player who is ahead makes the token. Being behind on life
/// yourself means the second clause says nothing.
#[test]
fn attacking_the_leader_makes_a_lifelinking_vampire() {
    let (mut game, preacher) = staged(10, 20);

    attack(&mut game, preacher);

    assert_eq!(tokens(&game), vec![(1, 1)], "one 1/1 arrived");
    let token = game
        .battlefield
        .iter()
        .find(|permanent| !matches!(permanent.card.definition, ObjectKind::Card(_)))
        .expect("the token is there");
    assert!(
        game.permanent_has_executable_keyword(token, KeywordAbility::Lifelink),
        "and it has lifelink",
    );
    assert_eq!(
        game.players[0].hand.len(),
        0,
        "you are not ahead, so no card"
    );
    assert_eq!(game.players[0].life, 10, "and nothing was paid");
}

/// Being ahead yourself draws a card and costs a life, and the player you
/// attacked is behind, so no token.
#[test]
fn attacking_while_ahead_draws_and_costs_a_life() {
    let (mut game, preacher) = staged(20, 10);

    attack(&mut game, preacher);

    assert!(tokens(&game).is_empty(), "they are not the leader");
    assert_eq!(game.players[0].hand.len(), 1, "you drew");
    assert_eq!(game.players[0].life, 19, "and paid a life for it");
}

/// Tied for most life counts for both clauses: one attack, two triggers.
#[test]
fn a_tie_counts_for_both_clauses() {
    let (mut game, preacher) = staged(20, 20);

    attack(&mut game, preacher);

    assert_eq!(tokens(&game), vec![(1, 1)]);
    assert_eq!(game.players[0].hand.len(), 1);
    assert_eq!(game.players[0].life, 19);
}

/// The condition belongs to the attack rather than being an intervening if:
/// life that moves while the trigger waits on the stack does not undo it.
#[test]
fn the_condition_is_read_where_the_attack_happened() {
    let (mut game, preacher) = staged(20, 10);
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(preacher, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    // Both triggers are waiting; the lead changes hands before either one
    // resolves.
    game.players[0].life = 5;
    game.players[1].life = 30;
    settle(&mut game);

    assert_eq!(
        game.players[0].hand.len(),
        1,
        "she attacked while ahead, and that is settled",
    );
    assert!(
        tokens(&game).is_empty(),
        "and they were behind when she attacked",
    );
}

/// She attacks a planeswalker the other player controls instead.
fn attack_planeswalker(game: &mut Game, preacher: GameObjectId, walker: GameObjectId) {
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(preacher, AttackDefender::Planeswalker(walker));
    game.finish_declaring_attackers();
    settle(game);
}

/// Puts a planeswalker under the other player.
fn their_planeswalker(game: &mut Game) -> GameObjectId {
    let mut walker = creature(273_500, cards::VRASKA_THE_UNSEEN, PlayerId::Two);
    walker.set_counters(CounterKind::Loyalty, 5);
    let id = walker.card.id;
    game.battlefield.push(walker);
    id
}

/// "Attacks the player with the most life" means the player. Attacking a
/// planeswalker they control is attacking them (CR 506.3b) and still not
/// what the clause asks for, so no Vampire arrives.
#[test]
fn attacking_their_planeswalker_makes_no_vampire() {
    let (mut game, preacher) = staged(10, 20);
    let walker = their_planeswalker(&mut game);

    attack_planeswalker(&mut game, preacher, walker);

    assert!(
        tokens(&game).is_empty(),
        "the leader was not the one attacked",
    );
}

/// The other clause says only "attacks", so a planeswalker is as good as a
/// player for it: she draws and pays while she is the one ahead.
#[test]
fn attacking_a_planeswalker_still_draws_while_you_are_ahead() {
    let (mut game, preacher) = staged(20, 10);
    let walker = their_planeswalker(&mut game);
    let hand = game.players[0].hand.len();

    attack_planeswalker(&mut game, preacher, walker);

    assert_eq!(game.players[0].hand.len(), hand + 1, "a card was drawn");
    assert_eq!(game.players[0].life, 19, "and a life paid for it");
    assert!(
        tokens(&game).is_empty(),
        "and the clause that names a player still said nothing",
    );
}

/// "Create a 1/1 white Vampire creature token with lifelink" and nothing
/// more: it arrives untapped and out of the attack, which is what separates
/// it from the tokens that say "tapped and attacking".
#[test]
fn the_vampire_arrives_untapped_and_out_of_the_attack() {
    let (mut game, preacher) = staged(10, 20);

    attack(&mut game, preacher);

    let token = game
        .battlefield
        .iter()
        .find(|permanent| !matches!(permanent.card.definition, ObjectKind::Card(_)))
        .expect("the Vampire arrived");
    assert!(!token.tapped, "untapped");
    assert!(!token.attacking, "and not part of the attack it came from");
    let life = game.players[1].life;
    game.step = Step::DeclareBlockers;
    game.finish_declaring_blockers();
    settle(&mut game);
    game.deal_combat_damage();
    game.check_state_based_actions();

    assert_eq!(
        game.players[1].life,
        life - 2,
        "so only the Preacher's two got through",
    );
}

/// Deathtouch is not only a keyword on her: a Grave Titan that blocks her
/// takes two damage and dies of it, and its own six kills her back.
#[test]
fn her_deathtouch_kills_what_blocks_her() {
    let (mut game, preacher) = staged(10, 20);
    let titan = game
        .put_onto_battlefield(PlayerId::Two, cards::GRAVE_TITAN)
        .expect("cataloged");
    drain_pending(&mut game);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
    }

    attack(&mut game, preacher);
    game.step = Step::DeclareBlockers;
    game.declare_blocker(titan, preacher);
    game.finish_declaring_blockers();
    settle(&mut game);
    game.deal_combat_damage();
    game.check_state_based_actions();

    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != titan),
        "two deathtouch damage is lethal to a 6/6",
    );
    assert!(
        game.battlefield
            .iter()
            .all(|permanent| permanent.card.id != preacher),
        "and the six it dealt back is lethal to a 2/4, so both go",
    );
}

/// "You'll create a Vampire token even if the player she attacked doesn't
/// have the most life as the ability resolves." The other side of the same
/// ruling: they were ahead when she swung, they are behind by the time it
/// resolves, and the Vampire arrives anyway.
#[test]
fn the_token_arrives_even_after_the_lead_changes_hands() {
    let (mut game, preacher) = staged(10, 20);
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.declare_attacker(preacher, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();

    // The trigger is waiting; they fall behind before it resolves.
    game.players[1].life = 1;
    settle(&mut game);

    assert_eq!(
        tokens(&game),
        vec![(1, 1)],
        "they were the leader when she attacked, and that is what it read",
    );
    assert_eq!(
        game.players[0].hand.len(),
        0,
        "and you were behind then, whatever you are now",
    );
}

/// The token's lifelink is not just a keyword on it: the Vampire connecting
/// next turn is a point of damage and a point of life.
#[test]
fn the_vampires_lifelink_pays_out_when_it_connects() {
    let (mut game, preacher) = staged(10, 20);
    attack(&mut game, preacher);
    let vampire = game
        .battlefield
        .iter()
        .find(|permanent| !matches!(permanent.card.definition, ObjectKind::Card(_)))
        .expect("the Vampire is there")
        .card
        .id;

    // She goes, so the only damage in the next combat is the Vampire's.
    game.battlefield
        .retain(|permanent| permanent.card.id != preacher);
    for permanent in &mut game.battlefield {
        permanent.entered_controller_turn = 0;
        permanent.tapped = false;
    }
    game.turn += 2;
    game.turns_started = [7, 6];
    game.step = Step::DeclareAttackers;
    game.attackers_declared = false;
    game.priority = PlayerId::One;
    let before = game.players[0].life;

    game.declare_attacker(vampire, AttackDefender::Player(PlayerId::Two));
    game.finish_declaring_attackers();
    settle(&mut game);
    game.step = Step::CombatDamage;
    game.deal_combat_damage();
    settle(&mut game);

    assert_eq!(game.players[1].life, 19, "one damage got through");
    assert_eq!(
        game.players[0].life,
        before + 1,
        "and lifelink turned it into a life",
    );
}
