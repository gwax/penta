# Engine design

## API boundary

`Game` is the authoritative state machine. Consumers do not mutate zones,
life, mana, priority, or the stack. They ask for `legal_actions(player)` and
submit one of those values to `apply(player, action)`. `apply` checks legality
again so stale bot decisions fail without changing state.

Bots receive `PlayerObservation`, which contains that player's hand and only
counts for an opponent's hidden zones. `GameEvent` is an omniscient debugging
and replay stream; it is not a bot observation.

A bot runner asks `decision_player()` who must act, observes that player, and
submits one of the observation's legal actions:

```rust
while let Some(player) = game.decision_player() {
    let observation = game.observe(player);
    let action = bots[player.index()].choose_action(&observation);
    game.apply(player, action)?;
}
```

The decision player is normally the player with priority, but differs during
mulligans, blocker declaration, restricted untaps, cleanup discards, and
triggered or combat-damage choices.

## Identities and zones

A `CardDefinitionId` identifies a kind of card in the catalog. A
`CardInstanceId` identifies one card object during a game. Moving a card
between library, hand, stack, battlefield, and graveyard preserves its
instance ID and owner. Permanents separately track their controller.

## Priority and atomic actions

Exactly one player has priority while a game is running. Concession is always
legal; other actions are generated only for the priority player.

- A non-pass action resets the consecutive-pass count.
- The first priority pass gives priority to the opponent.
- Two passes with a nonempty stack resolve its top object.
- Two passes with an empty stack advance the turn step.
- After a resolution or step change, the active player receives priority.

Mana abilities resolve immediately and do not use the stack. The current
casting API requires mana to be floated before casting; integrating mana
activation into a compound casting choice can be added without changing spell
resolution. Chaos Orb's non-mana activated ability uses the stack and is
identified separately from spells in `StackObservation`; its chosen permanent
is exposed as a choice rather than a target.

Attacker and blocker declaration are staged to keep legal-action generation
linear rather than enumerating exponential subsets. No player receives
priority until the declaring player submits the corresponding finish action.
When an attacker is blocked by multiple creatures, its controller explicitly
divides its damage among them. A trampling attacker can also assign damage to
the defending player once lethal damage has been assigned to every blocker.
This follows the current rules, which removed combat damage assignment order
in the [Foundations rules update][foundations-update].

Spell actions carry a list of targets. Fireball enumerates affordable,
distinct target combinations, charges one additional generic mana for each
target beyond the first, and divides X evenly on resolution. After Fork
resolves, its controller chooses legal targets for the copy or keeps the
original targets. Spell actions also carry explicit sacrifices for additional
costs such as Goblin Grenade.

## Determinism and replay

All random choices use the engine-owned, versioned PRNG. A dependency upgrade
therefore cannot change the meaning of an existing seed. A replay can be
reconstructed from the format/card version, decks, seed, and submitted action
sequence. Events provide a convenient derived trace for debugging and UI use.

## Card behavior

Card metadata lives in `CardCatalog`; executable behavior is selected by the
closed `CardBehavior` enum. Every card in the POC catalog has a behavior,
kind, mana cost, color, and creature characteristics where applicable.
Unsupported cards can exist in other catalogs and hidden zones but do not
generate cast actions. This makes partial coverage explicit and keeps
arbitrary card code out of serialized game state.

As the corpus grows, behavior should be factored into reusable primitives
(damage, draw, destroy, continuous restrictions, triggers) rather than one
large bespoke function per printed card.

## Rules boundary

The format is Eternal Central 93/94: current Magic rules plus the EC
exceptions, notably phase-boundary mana burn. The POC implements London
mulligans, priority-bearing turn steps, cleanup, combat, three fixed powered
red decks, and its forty-card red/artifact corpus.

It deliberately remains narrower than the full Comprehensive Rules. Fireball
and Fork expose their full targeting decisions, and attackers expose current
combat damage assignment decisions. Simple non-mana abilities and triggers
generally resolve atomically. Chaos Orb's activation uses the stack and
deterministically destroys its chosen permanent rather than simulating EC's
physical card flip; removing the Orb before resolution nullifies the ability.
Off-color Moxen produce colorless mana because the corpus contains no non-red
colored costs. Red Elemental Blast has no observable legal work because the
corpus contains no blue objects.

[foundations-update]: https://magic.wizards.com/en/news/announcements/foundations-update-bulletin
