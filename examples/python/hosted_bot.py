"""Put a bot online so people can play it.

The whole thing is one loop over one HTTP call. Heartbeating is what "online"
means -- stop, and the bot leaves the list -- and the heartbeat's reply is
where invitations arrive. No WebSocket, no framework, no penta module: this
plays entirely against the server's engine, so it runs anywhere `requests`
does.

    python3 hosted_bot.py --server http://localhost:8787 --name Fizzbot

`choose` is the whole bot. Everything above and below it is plumbing you can
copy verbatim; see BOTS.md for what an observation holds.
"""

import argparse
import time

import requests

# Heartbeat well inside the server's 45-second presence window, so one lost
# packet does not read as "this bot went away".
HEARTBEAT_SECONDS = 10
# How long to wait between polls while it is the opponent's turn to move.
POLL_SECONDS = 0.25


def choose(observation):
    """Pick an action index from one observation. Replace me with a model."""
    actions = observation["legalActions"]
    for index, action in enumerate(actions):
        if action["type"] == "KeepHand":
            return index
    # Anything that develops the board beats passing, and passing beats
    # conceding -- which is legal at every priority and would end the game.
    for wanted in ("PlayLand", "CastSpell", "DeclareAttacker", "ActivateAbility"):
        for index, action in enumerate(actions):
            if action["type"] == wanted:
                return index
    for index, action in enumerate(actions):
        if action["type"] != "Concede":
            return index
    return 0


def play(server, room):
    """Drive the opponent seat of one room until the game ends."""
    print(f"playing {room}")
    while True:
        view = requests.get(f"{server}/_game/{room}/opponent", timeout=30).json()
        if view.get("result"):
            print(f"  finished: {view['result']}")
            return
        if not view.get("deciding"):
            # The human is thinking, or the engine is resolving something.
            time.sleep(POLL_SECONDS)
            continue
        index = choose(view["observation"])
        reply = requests.post(
            f"{server}/_game/{room}/command",
            json={"t": "botAct", "index": index},
            timeout=30,
        )
        if reply.status_code != 200:
            # A refused action leaves the previous observation standing, so
            # the next poll simply asks again.
            print(f"  refused: {reply.text}")
            time.sleep(POLL_SECONDS)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server", default="http://localhost:8787")
    parser.add_argument("--name", default="Fizzbot")
    parser.add_argument("--deck", default="Sligh")
    arguments = parser.parse_args()
    server = arguments.server.rstrip("/")

    registration = requests.post(
        f"{server}/_bots/register",
        json={"name": arguments.name, "deck": arguments.deck},
        timeout=30,
    ).json()
    identifier, token = registration["id"], registration["token"]
    print(f"registered as {arguments.name} ({identifier}) playing {arguments.deck}")
    print("waiting for a challenger…")

    finished = []
    while True:
        reply = requests.post(
            f"{server}/_bots/{identifier}/heartbeat",
            json={"token": token, "done": finished},
            timeout=30,
        ).json()
        finished = []
        for invite in reply.get("invites", []):
            play(server, invite["room"])
            # Reporting it finished is what frees the bot for the next game.
            finished.append(invite["room"])
        if not finished:
            time.sleep(HEARTBEAT_SECONDS)


if __name__ == "__main__":
    main()
