"""Smoke test for the Python bindings: full games, determinism, self-play.

Run via scripts/check-bindings.sh, which builds the module and puts it on
the path first.
"""

import json

import penta

print("engine", penta.engine_version(), "protocol", penta.protocol_version())
assert "Sligh" in penta.deck_names()
catalog = {c["definition"]: c for c in json.loads(penta.catalog())["cards"]}
assert any(c["name"] == "Lightning Bolt" for c in catalog.values())

def pass_bot(obs):
    prefer = ["KeepHand", "ChooseDecision", "PassPriority", "FinishDeclaringAttackers",
              "FinishDeclaringBlockers", "AssignCombatDamage", "DiscardCards",
              "BottomCards", "ChooseUntap"]
    actions = obs["legalActions"]
    for kind in prefer:
        for action in actions:
            if action["type"] == kind:
                return action["index"]
    return 0

# vs handcrafted
game = penta.Game("Sligh", "The Deck", opponent="handcrafted", seed=7)
steps = 0
while game.result() is None:
    obs = json.loads(game.observe())
    game.act(pass_bot(obs))
    steps += 1
    assert steps < 100000
print("vs handcrafted:", game.result(), "in", steps, "decisions")

# determinism
def run(seed):
    g = penta.Game("Goblins", "Sligh", opponent="random", seed=seed)
    trace = []
    while g.result() is None:
        obs = json.loads(g.observe())
        trace.append(len(obs["legalActions"]))
        g.act(pass_bot(obs))
    return g.result(), trace
a, b = run(99), run(99)
assert a == b, "same seed, same game"
print("determinism ok:", a[0], "over", len(a[1]), "decisions")

# self-play: one loop drives both seats
game = penta.Game("Goblins", "White Weenie", opponent="external", seed=13)
steps = 0
while game.result() is None:
    seat = game.decision_seat()
    obs = json.loads(game.observe(seat))
    assert obs["seat"] == seat
    game.act(pass_bot(obs))
    steps += 1
    assert steps < 200000
print("self-play:", game.result(), "in", steps, "decisions")

# hidden info: p1 never sees p2's hand
game = penta.Game("Sligh", "The Deck", opponent="handcrafted", seed=3)
obs = json.loads(game.observe())
assert "opponentHandSize" in obs and isinstance(obs["opponentHandSize"], int)
print("smoke test passed")
