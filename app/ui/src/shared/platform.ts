// The system the app runs on, for the words the interface uses (A-104). Behaviour never depends on it: the backend
// decides what the system can do (AAC, where the key is kept) and says so.

const UA = typeof navigator === "undefined" ? "Macintosh" : navigator.userAgent;

export const IS_MAC = /Mac|iPhone|iPad/.test(UA);
export const IS_WINDOWS = /Windows/.test(UA);

/** "this Mac", or "this computer" elsewhere. */
export const THIS_COMPUTER = IS_MAC ? "this Mac" : "this computer";

/** The system store that keeps the assistant's key (AI-11). */
export const KEYCHAIN = IS_MAC ? "the macOS keychain" : IS_WINDOWS ? "Windows Credential Manager" : "the system keyring";

/** The same, in a short sentence ("Stored in …"). */
export const KEYCHAIN_SHORT = IS_MAC ? "the keychain" : IS_WINDOWS ? "Credential Manager" : "the system keyring";
