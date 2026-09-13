# Engine architecture

This document describes the engine's current runtime abstractions and
invariants. See the [design doctrine](design-doctrine.md) for project
philosophy, [implementing cards](implementing-cards.md) for extension guidance,
[prepared execution](prepared-engine.md) for the optional optimization layer,
[engine interfaces](interfaces.md) for consumer APIs, and
[formats and scope](formats.md) for current coverage.

## Identities and zones

A `CardDefinitionKey` is the natural UUID of the canonical card's debut printing:
the first English-language paper printing when one exists, otherwise the first
paper printing in any language. Decks, catalogs, protocol observations, and
checkpoints use that UUID. `CardDefinitionId` is a compact process-local handle:
the build generates dense IDs for built-in declarations, and custom keys are
interned on entry. Catalog and prepared-program reads use array indices for
built-ins, without UUID hashing or interner locks in gameplay. Ordering and
serialization resolve natural keys, never allocation order. There is no
historical numeric identity table.

`CardPrintingId` is a catalog lookup tuple for a definition, set, and local
variant. For persistent exact-art selection use the printing's `scryfallId`.
Parts, abilities, modes, target slots, costs, and grants are positional references
within their owning definition or instantiated action. They have no independent
global identity; saved references carry their owning natural definition key and
are interpreted against the checkpoint's exact simulation fingerprint.

Bindings are authored as local names. Each effect-resolution context assigns
private numeric slots; cost names are scoped separately to the card part.
Checkpoints retain names and reconstruct slots, so slot allocation order is
not a persistence contract.

The runtime model deliberately separates physical-card lineage from rules
object identity:

- A physical-card ID follows one piece of cardboard and its owner through the
  game. A printing can eventually be attached here without changing the card's
  canonical definition.
- A `GameObjectId` identifies one rules object in its current zone. Moving a
  card from hand to the stack, from the stack to the battlefield, or from the
  battlefield to exile creates a new game object. Effects and targets refer to
  this current incarnation, so an object that leaves and returns is not the
  object that was previously targeted.

Transforming a permanent and phasing it out do not change zones, so both retain
the same `GameObjectId` and permanent state. Shuffling or changing position
within a library likewise does not create a new object. These rules let the
engine distinguish, for example, a Goblin Balloon Brigade card in hand, its
creature spell on the stack, and the creature permanent it becomes without
pretending that they are the same rules object.

An object's characteristics are independent of its physical backing. The
backing is represented conceptually as zero, one, or several physical-card
IDs: a spell copy or token has no physical card, an ordinary card object has
one, and a future melded permanent can have two. A physical card may back at
most one live game object at a time. Physical lineage stays out of player
observations because exposing it would allow a client to track a known card
through a shuffle.

The core runtime uses this separation. Physical cards live in a private
game registry, live objects carry zero-or-more physical backing IDs separately
from their characteristic source, and actions, targets, observations, and
events use `GameObjectId`. The former `CardInstanceId` and `StackObjectId`
names remain deprecated source-compatibility aliases; they no longer identify
physical lineage or a separate stack-ID namespace. Printing IDs are catalog
metadata and are not an art-selection feature in the UI.

Historical events that outlive a zone object carry its immutable card
definition as well as the former object ID. Activated-ability events carry the
ability object's stack ID separately from the source permanent's ID, so a
sacrificed source does not make the resolution record ambiguous.

## Card parts and contextual characteristics

A `CardPart` is an addressable characteristic set. Parts can occupy different
physical faces, share one face, or describe a derived presentation. The
`CardStructure` record separates four independent relationships:

- `normal` selects one part or combines ordered parts outside the stack and
  battlefield;
- `alternatives` associates complete or partial characteristic sets with a
  normal set, without granting permission to cast any of them;
- `faces` records physical single, double, or meld-component backing; and
- `battlefield` describes initial presentation and flip or unlock transitions.

Adventure and Omen insets use the same alternative relationship. Preparation
can associate an inset without offering a cast of that part of the original
card. Partial alternatives specify the fields they replace: prototype changes
mana cost, color, and power/toughness, while a flip presentation preserves mana
cost and color. Catalog construction materializes field inheritance before
runtime characteristic queries and validation.

Play options independently select the part or ordered combination used by a
spell or land play. Alternative and additional costs remain payment choices.
Physical double-faced status does not imply a back-face casting permission;
modal casting does not prevent a permanent from transforming when instructed.
The physical face kind still determines its mana-value convention.

Rooms declare each door's program once. Their combined presentation borrows
both programs in order through `AbilityClauses`, derives the combined cost and
colors, and retains the existing presentation IDs for copying and checkpoints.
The locked presentation and unlock transitions are separate from the printed
doors and the card's normal combined characteristics.

`HasAlternativeCharacteristics(CharacteristicPredicateDef)` queries associated
sets rather than current characteristics or available casts. The characteristic
predicate can combine names, types, subtypes, colors, mana value, and keywords;
it cannot accidentally inspect a controller, zone, or combat state. Event
snapshots retain immutable catalog references to the alternative sets of the
effective copiable presentation. Face-down objects expose no alternative sets,
and later copy effects or zone changes do not rewrite an earlier snapshot.

Visibility remains separate from applicability. A player may inspect a face
without that face contributing current characteristics. Physical lineage stays
separate from copiable values, including for token copies and meld components.
`CardDefinition.rules` remains the primary compatibility view; current queries
use contextual parts and effective runtime presentations.

A selected spell program combines its clauses, chosen modes, and ordered splice
contributions with their target scopes. A splice donor remains a separate
object; it is not a physical face or part of the host's physical backing.
Spell resolution and copying retain the locked form and selected program.

The model does not itself implement every mechanism that can use these
relationships. Preparation's designation and linked exiled-copy lifetime, for
example, require their own runtime program. Cards remain wholly unsupported
until their complete behavior is executable.

The native refactor preserves the established catalog JSON structure tags as a
derived compatibility projection. Native relationships and play options are
the authority; wire layout labels do not drive rules execution.

## Priority and stack actions

Exactly one player has priority while a game is running. Concession is always
legal; other actions are generated only for the priority player.

- A non-pass action resets the consecutive-pass count.
- The first priority pass gives priority to the opponent.
- Two passes with a nonempty stack resolve its top object.
- Two passes with an empty stack advance the turn step.
- After a resolution or step change, the active player receives priority.

Activated and triggered mana abilities resolve immediately and do not use the
stack. This is an explicit ability category, not something inferred from its
effect: an ability that produces mana can still be an ordinary activated
ability and use the stack. Other supported activated abilities create stack
objects with their source, clause origin, text, targets, and effect frozen at
activation. Removing or changing the source does not erase that independent
ability object. Every supported non-mana activated and triggered ability uses
this shared lifecycle and the declarative effect runtime.

Committed events capture matching triggered abilities from the objects that
declare them. The active player's simultaneous triggers are handled before the
nonactive player's; each player explicitly chooses the first-resolving-first
order of their own group and, when needed, places targeted triggers one at a
time with targets selected. After every pending trigger is on the stack,
priority returns to the player who was about to receive it. A trigger stack
object freezes its source object ID and event context; resolution consults the
live incarnation when it remains available or the engine's retained
last-known-information snapshot after it leaves. The source may therefore
disappear before resolution without losing required information.

Spell actions consider both floating mana and usable untapped mana sources.
Applying a spell action deterministically activates only the additional
sources needed to pay its cost, preferring colorless sources for generic costs
and avoiding excess production where possible. The read-only
`mana_sources_for_action` helper exposes that payment preview to UI clients
without cloning a complete game state. Explicit mana actions remain legal for
callers that intentionally want to float mana. Chaos Orb's non-mana activated
ability uses the stack and is identified separately from spells in
`StackObservation`; its permanent is chosen through the separate non-targeting
resolution path described below.

Attacker and blocker declaration are staged to keep legal-action generation
linear rather than enumerating exponential subsets. No player receives
priority until the declaring player submits the corresponding finish action.
When an attacker is blocked by multiple creatures, its controller explicitly
divides its damage among them. A trampling attacker can also assign damage to
the defending player once lethal damage has been assigned to every blocker.
This follows the current rules, which removed combat damage assignment order
in the [Foundations rules update][foundations-update].

Targets and choices are separate rules constructs. Targets are bound to stable
slots when a spell or ability is put on the stack, are constrained by targeting
restrictions such as hexproof, shroud, and protection, and are rechecked as the
object resolves. A declarative `Choose(ChooseDef)` effect instead asks its named
player during resolution, binds the selected object or object set in the typed
resolution context, and resumes its nested continuation. It does not create a
target slot, trigger target-fizzle rules, or re-run target legality. Chaos Orb
uses that non-targeting path, while the same operation can choose a spell on the
stack or a card in another zone when its candidate query permits one.

Spell choices bind targets to stable target slots. A variable-cardinality slot
can enumerate any number of distinct targets; a semantic additional cost may
multiply mana by an arithmetic target-count quantity, and an effect may divide
a value by the recipient count with an explicit rounding direction. Different slots
are independent, so two instructions may choose the same object. After Fork resolves, its
controller chooses legal replacement values for the existing slots or keeps
the original targets. Spell actions also carry explicit payment objects for
costs such as Goblin Grenade's sacrifice.

## Play options, modes, and cast signatures

The casting model keeps several choices separate because they obey different
rules:

- A play option selects what is being played: a split-card half, both halves
  with fuse, an Adventure spell, or one side of a modal double-faced card. It
  also says whether the action casts a spell or plays a land.
- A rules-text mode selects an effect branch, such as one of Izzet Charm's
  three instructions. A card can have one ordinary play option and several
  modes.
- Alternative and additional cost choices describe how the selected form is
  paid for. They do not become extra faces or modes.
- X, targets, and any required divisions are further choices made for the
  particular spell.

After validation, those choices form an immutable cast signature on the stack:
the selected play option and spell form, chosen modes, cost choices, X, and
target-slot assignments. Authored effects refer to clause-local target
positions; casting assigns runtime slots by flattening the selected parts and
mode occurrences in order. The resolver carries each effect's offset into that
flat list, so a modal spell retains the exact target schema that was chosen
rather than regenerating one from the canonical card definition.

Copying a spell creates a new game object with no physical backing and copies
the cast signature. It therefore retains the selected split/MDFC/alternate
form, rules modes, X, cost decisions, and targets. The copy is not cast and
does not pay those costs. A copy effect such as Fork may explicitly replace
legal target assignments, but it cannot choose different modes or a different
spell form.

The catalog types and runtime now share this model. `Action::CastSpell` carries
one authoritative `CastChoices` value rather than parallel mode/form/target/X
fields, and a validated cast stores a `CastSignature` on its stack object.
Existing single-faced cards receive a default play option and positional
target slots, so their behavior is unchanged while structured cards use their
declared options, modes, and target slots. Fork copies that signature and can
replace only the target values in its existing slots. Sacrificed, discarded,
or tapped objects remain payment records outside the signature because a copy
does not pay those costs again.

Izzet Charm and Turn // Burn exercise this structured catalog and validation
path, including ordered modes, fused forms, and independent target slots.
Ability implementation coverage remains separate from casting structure, so a
catalog can represent a form without offering an action that would resolve as
a silent no-op.

Catalog construction rejects ambiguous structured metadata: duplicate local
IDs, missing or out-of-structure parts, invalid mode and target bounds, and
cyclic or ambiguous alternative-characteristic relationships. Each play option
supplies its own ordered characteristic expression; the card's normal combined
expression does not grant or restrict combined casting.

The identity model also leaves room for objects backed by multiple physical
cards. The future design is recorded separately in
[composite objects and meld](design-notes/composite-objects.md).

## Determinism and replay

All random choices use the engine-owned PRNG. A build-time
`simulationFingerprint` hashes the production engine source and dependency
resolution, card catalog, repository deck data, and pinned toolchain. It is a
conservative guard for artifacts built from those inputs: non-behavioral edits
can change it, and build provenance outside those covered inputs must still be
controlled when exact reproducibility matters.

The browser command journal carries an independent `replayVersion`. Replay
acceptance requires that format version and the same simulation fingerprint;
the bot-wire `protocolVersion` and package `engineVersion` are recorded as
provenance but do not decide whether the commands can be replayed. This keeps a
wire-compatible rules change from replaying under silently different semantics
and lets a package release move without inventing a bot-wire break.

Observation reconstruction follows the same split. Its nested `checkpoint`
object has its own format `version` and repeats the simulation
fingerprint. Changing checkpoint bookkeeping moves that version or capability,
not the ordinary bot protocol epoch. Events remain a convenient derived trace
for debugging and UI use.

## Card model and behavior

Each built-in canonical card is declared once in the `CARDS` registry of its
preferred representative set module, under the set's release-year module. That
is normally the first English-language paper set, falling back to the first
paper set only when no English printing exists. Its `CardRecord` keeps identity
and its rules together: name, cost, types, creature stats, and ordered ability
clauses can all be understood at the card's declaration. Double-faced records
use `CardRecord::new_dfc` or
`CardRecord::new_mdfc` to declare both named faces together and derive their
parts, topology, and play options. Other structured cards attach a
`CardComposition`; an ordinary record receives an equivalent one-part
composition automatically.

An `AbilityDef` owns one rules-text clause together with its explicit timing
category, costs, targets, and structured effect.
The displayed card text is the clauses' text joined in printed order with
newlines, so presentation and execution do not duplicate Oracle text. Clause
IDs are assigned from that order when definitions are attached to a card part.
A `CardRules` definition is either complete and wholly declarative or is a
whole-card `Unsupported` sentinel with no executable clauses or creature body.

A set module supplies the debut set for each canonical record in its `CARDS`
registry. Its `ADDITIONAL_PRINTINGS` registry points back to those records for
reprints or additional variants in that set. Each printing records its exact
art UUID and artist. The resulting
`CardPrintingId` combines the canonical definition, set, and variant, so
alternate art can be distinguished while sharing one runtime `CardDefinition`
and its rules. Format legality considers all known printings: a nonbasic card
is legal when at least one printing belongs to the format's allowed sets,
regardless of which printing is selected for presentation. Browser games can
show debut art or the earliest artwork-bearing printing allowed by their
format. A format can also admit exact card identities for dated promos whose
physical promo set spans the format boundary; it does not treat every card in
that set as legal.

Many executable effects use reusable declarative primitives or constructors in
`card::abilities`. Card definitions do not carry an alternate execution
selector: executable clauses are composed from the shared declarative model.
Unsupported cards can exist in other catalogs and hidden zones but do not generate play options that
would resolve as silent no-ops. This makes partial coverage explicit and keeps
arbitrary card code out of serialized game state.

Implementation choices and extension boundaries are documented in
[implementing cards](implementing-cards.md). Current format-specific rules and
support limitations belong in [formats and scope](formats.md).

[foundations-update]: https://magic.wizards.com/en/news/announcements/foundations-update-bulletin
