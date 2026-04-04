/**
 * Shared HMR (Hot Module Replacement) types for utoopack.
 * Used by both NAPI (Node.js) and browser-based implementations.
 */
// ============================================================================
// HMR Actions (Server -> Client)
// ============================================================================
export var HMR_ACTIONS_SENT_TO_BROWSER;
(function (HMR_ACTIONS_SENT_TO_BROWSER) {
    HMR_ACTIONS_SENT_TO_BROWSER["RELOAD"] = "reload";
    HMR_ACTIONS_SENT_TO_BROWSER["CLIENT_CHANGES"] = "clientChanges";
    HMR_ACTIONS_SENT_TO_BROWSER["SERVER_ONLY_CHANGES"] = "serverOnlyChanges";
    HMR_ACTIONS_SENT_TO_BROWSER["SYNC"] = "sync";
    HMR_ACTIONS_SENT_TO_BROWSER["BUILT"] = "built";
    HMR_ACTIONS_SENT_TO_BROWSER["BUILDING"] = "building";
    HMR_ACTIONS_SENT_TO_BROWSER["TURBOPACK_MESSAGE"] = "turbopack-message";
    HMR_ACTIONS_SENT_TO_BROWSER["TURBOPACK_CONNECTED"] = "turbopack-connected";
})(HMR_ACTIONS_SENT_TO_BROWSER || (HMR_ACTIONS_SENT_TO_BROWSER = {}));
