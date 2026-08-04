/* tslint:disable */
/* eslint-disable */

/**
 * Browser-owned game facade. JavaScript only selects legal action indexes;
 * rules and bot decisions remain inside the Rust engine.
 */
export class WebGame {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Applies one action from the current state's action list.
     *
     * # Errors
     *
     * Returns a JavaScript error when the game is not waiting for the human,
     * the index is stale, the action is rejected, or the bot cannot finish.
     */
    act(action_index: number): void;
    /**
     * Declares every currently legal attacker, then finishes attacker declaration.
     *
     * # Errors
     *
     * Returns a JavaScript error unless the human is declaring attackers or
     * the engine rejects one of the otherwise-legal actions.
     */
    attack_all(): void;
    /**
     * Takes back every attacker declared so far this combat.
     *
     * # Errors
     *
     * Returns a JavaScript error when the attack has already been committed.
     */
    cancel_attackers(): void;
    /**
     * Submits the selected option IDs for the current generic decision.
     *
     * The selection is validated by the engine, so the browser does not need
     * to receive an eagerly-expanded action for every possible combination.
     *
     * # Errors
     *
     * Returns a JavaScript error when the game is not waiting for the human,
     * the JSON is malformed, or the engine rejects the selection.
     */
    choose_decision(decision: number, options_json: string): void;
    /**
     * Commits a complete set of blocker assignments selected by the browser UI.
     *
     * Assignments are encoded as JSON pairs of `[blocker_id, attacker_id]` so
     * the UI can rearrange arrows freely before making one atomic submission.
     *
     * # Errors
     *
     * Returns a JavaScript error unless the human is declaring blockers or an
     * assignment is duplicated, malformed, or no longer legal.
     */
    finalize_blocks(assignments_json: string): void;
    /**
     * Creates a mirror-format game and advances until the human must decide.
     *
     * # Errors
     *
     * Returns a JavaScript error when a deck or policy name is unknown, game
     * construction fails, or the bot cannot reach a human decision.
     */
    constructor(human_deck: string, bot_deck: string, bot_policy: string, human_first: boolean, seed: number);
    /**
     * Enables or disables the browser's smooth automatic priority yields.
     * Enables or disables routine UI priority passing.
     *
     * # Errors
     *
     * Returns an error if advancing the facade encounters an invalid engine action.
     */
    set_autopass(enabled: boolean): void;
    /**
     * Enables or disables a human-interface stop for one displayed phase.
     * The rules engine still exposes every individual step.
     * Sets or clears a UI phase stop.
     *
     * # Errors
     *
     * Returns an error if advancing the facade encounters an invalid engine action.
     */
    set_phase_stop(phase: string, enabled: boolean): void;
    /**
     * Returns the human-visible game state as JSON.
     */
    state_json(): string;
    /**
     * Rewinds the most recent manual mana ability while it is still safe to do so.
     *
     * # Errors
     *
     * Returns a JavaScript error when there is no reversible mana activation.
     */
    undo_mana(): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_webgame_free: (a: number, b: number) => void;
    readonly webgame_act: (a: number, b: number) => [number, number];
    readonly webgame_attack_all: (a: number) => [number, number];
    readonly webgame_cancel_attackers: (a: number) => [number, number];
    readonly webgame_choose_decision: (a: number, b: number, c: number, d: number) => [number, number];
    readonly webgame_finalize_blocks: (a: number, b: number, c: number) => [number, number];
    readonly webgame_new: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly webgame_set_autopass: (a: number, b: number) => [number, number];
    readonly webgame_set_phase_stop: (a: number, b: number, c: number, d: number) => [number, number];
    readonly webgame_state_json: (a: number) => [number, number];
    readonly webgame_undo_mana: (a: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
