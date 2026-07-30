# Engine design

## API boundary

`Game` is the authoritative state machine. Consumers do not mutate zones,
life, mana, priority, or the stack. They ask for `legal_actions(player)` and
submit one of those values to `apply(player, action)`. `apply` checks legality
again so stale bot decisions fail without changing state.

Bots receive `PlayerObservation`, which contains that player's hand and only
counts for an opponent's hidden zones. `GameEvent` is an omniscient debugging
and replay stream; it is not a bot observation.

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
resolution.

Attacker and blocker declaration are staged to keep legal-action generation
linear rather than enumerating exponential subsets. No player receives
priority until the declaring player submits the corresponding finish action.

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
mulligans, priority-bearing turn steps, cleanup, combat, and its twenty-card
red/artifact corpus.

It deliberately remains narrower than the full Comprehensive Rules. Fireball
has a single-target casting mode, Fork retains copied targets, blockers use
deterministic damage order, and simple non-mana abilities and triggers resolve
atomically. Blood Moon and Red Elemental Blast have no observable legal work
inside the Mountain/red/artifact-only corpus because it contains neither
nonbasic lands nor blue objects.
