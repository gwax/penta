/**
 * When a bot counts as online, and when an invitation has gone stale.
 *
 * Presence is a lease rather than a connection: a bot is online because it
 * heartbeated recently, so every question about it is a question about the
 * clock. That makes these rules worth stating once, in one place, where a
 * test can walk right up to each boundary.
 */

/** A bot that has not heartbeated within this window is offline. */
export const PRESENCE_MS = 45_000;

/**
 * An invitation nobody picked up frees the bot again. Long enough for a real
 * game, short enough that a bot which died mid-game is not stuck forever.
 */
export const INVITE_MS = 10 * 60_000;

/**
 * How long a seat has to make one move before it loses on time.
 *
 * A person deserves room to think; a program that has not answered in a
 * minute is not thinking. The clock exists so that neither a wedged bot nor
 * an abandoned tab can hold a room open forever.
 */
export const HUMAN_MOVE_MS = 5 * 60_000;
export const BOT_MOVE_MS = 60_000;

/**
 * The clock budget for whichever seat must act.
 *
 * @param {"human" | "bot"} seat
 */
export function moveBudgetMs(seat) {
  return seat === "bot" ? BOT_MOVE_MS : HUMAN_MOVE_MS;
}

/**
 * @typedef {{ room: string, reason: "challenge" | "event", at: number }} Invite
 * @typedef {{ id: string, name: string, deck: string, lastSeen: number,
 *             invites: Invite[] }} BotRecord
 */

/**
 * Whether a bot's heartbeat is still current.
 *
 * @param {number} lastSeen
 * @param {number} now
 */
export function isOnline(lastSeen, now) {
  return now - lastSeen <= PRESENCE_MS;
}

/**
 * The invitations still worth honouring, oldest first.
 *
 * @param {Invite[]} invites
 * @param {number} now
 */
export function liveInvites(invites, now) {
  return invites.filter((invite) => now - invite.at < INVITE_MS);
}

/**
 * A bot as the outside world sees it: no token, presence and busyness
 * resolved against the clock.
 *
 * @param {BotRecord} bot
 * @param {number} now
 */
export function publicBot(bot, now) {
  return {
    id: bot.id,
    name: bot.name,
    deck: bot.deck,
    online: isOnline(bot.lastSeen, now),
    busy: liveInvites(bot.invites, now).length > 0,
  };
}
