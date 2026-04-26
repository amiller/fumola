/* tslint:disable */
/* eslint-disable */

/**
 * Stateful fumola interpreter handle. Holds a Core, lets JS register
 * modules by path before evaluating expressions that `import` them.
 *
 * JS usage:
 *     const state = new FumolaState();
 *     state.set_module("handle", handleSourceText);
 *     const json = state.eval('import H "handle"; H.handle("GET", "/", "")');
 */
export class FumolaState {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Evaluate `source`. Returns a JSON-encoded `{ok, output|error}`.
     */
    eval(source: string): string;
    constructor();
    /**
     * Register a module under `local_path`. Pass the file's full source
     * text (must begin with `module { ... }`). Trailing `.fumola` in the
     * path is stripped to match the import-statement convention.
     * Returns "" on success, error string on failure.
     */
    setModule(local_path: string, content: string): string;
}

/**
 * Evaluate a fumola program. Returns a JSON-encoded result string:
 *   `{"ok": true,  "output": "<pretty-printed value>"}` on success
 *   `{"ok": false, "error":  "<debug-formatted error>"}` on failure
 */
export function _eval(source: string): string;

/**
 * Convenience: pretty-print the result (or `Err(...)`) as a plain string.
 */
export function evalString(source: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly _eval: (a: number, b: number) => [number, number];
    readonly evalString: (a: number, b: number) => [number, number];
    readonly __wbg_fumolastate_free: (a: number, b: number) => void;
    readonly fumolastate_new: () => number;
    readonly fumolastate_setModule: (a: number, b: number, c: number, d: number, e: number) => [number, number];
    readonly fumolastate_eval: (a: number, b: number, c: number) => [number, number];
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
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
