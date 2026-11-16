//! Whether the running app can use the system keychain without the user being asked for a password (AI-11, A-072).
//!
//! On macOS a keychain item belongs to the code identity that created it. A build signed with a certificate issued
//! by Apple (Developer ID, Apple Development, Mac App Store) keeps the same identity from one build to the next, so
//! the app reads its own item silently. An unsigned or ad-hoc signed build has an identity that changes at every
//! build: macOS then asks for the login password, twice per read (once for the item, once for its partition list).
//! Such builds keep the key in a file instead (`keys::FileKeyStore`).
//!
//! This is the only place that asks the system about the signature: macOS-specific code behind one function, with
//! a fallback elsewhere (NFR-8).

/// True when the system keychain can be read without a password prompt.
#[cfg(target_os = "macos")]
pub fn keychain_is_silent() -> bool {
    use security_framework::os::macos::code_signing::{Flags, SecCode, SecRequirement};
    // Satisfied by any signature whose certificate chain ends at Apple's root: never by an ad-hoc signature.
    let Ok(requirement) = "anchor apple generic".parse::<SecRequirement>() else { return false };
    SecCode::for_self(Flags::NONE).is_ok_and(|code| code.check_validity(Flags::NONE, &requirement).is_ok())
}

/// Elsewhere the system store (Credential Manager, Secret Service) does not tie an item to a build.
#[cfg(not(target_os = "macos"))]
pub fn keychain_is_silent() -> bool {
    true
}

#[cfg(test)]
mod tests {
    /// req: AI-11
    #[test]
    fn asking_about_the_signature_never_fails() {
        // The answer depends on how the test binary was signed; the question itself must always have one.
        eprintln!("keychain_is_silent = {}", super::keychain_is_silent());
    }
}
