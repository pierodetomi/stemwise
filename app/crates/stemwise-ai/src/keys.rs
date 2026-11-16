//! The OpenRouter key (AI-11, P-005): kept in the system keychain (Keychain on macOS, Credential Manager or
//! Secret Service elsewhere) behind an interface, never in the project, the logs or the settings file. Tests use
//! the in-memory store: no test touches the real keychain.

use std::sync::Mutex;

pub trait KeyStore: Send + Sync {
    fn get(&self) -> Option<String>;
    fn set(&self, key: &str) -> Result<(), String>;
    fn delete(&self) -> Result<(), String>;
    /// Whether a key is saved, without needing its value when the store can tell otherwise.
    fn has(&self) -> bool {
        self.get().is_some()
    }
}

const SERVICE: &str = "Stemwise";
const ACCOUNT: &str = "openrouter";

/// The system keychain.
pub struct SystemKeyStore;

impl KeyStore for SystemKeyStore {
    fn get(&self) -> Option<String> {
        keyring::Entry::new(SERVICE, ACCOUNT).ok()?.get_password().ok().filter(|k| !k.trim().is_empty())
    }

    fn set(&self, key: &str) -> Result<(), String> {
        keyring::Entry::new(SERVICE, ACCOUNT).and_then(|e| e.set_password(key.trim())).map_err(|e| e.to_string())
    }

    fn delete(&self) -> Result<(), String> {
        match keyring::Entry::new(SERVICE, ACCOUNT).and_then(|e| e.delete_credential()) {
            Ok(()) => Ok(()),
            Err(e) if e.to_string().to_lowercase().contains("no entry") => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// The keychain as the app uses it (A-052): read at most once per session, and only when a call to the provider needs
/// the key. Whether a key is saved is answered from a small marker file in the app's data folder (no secret in it),
/// so starting the app, opening a project or showing the assistant's state never touches the keychain. Each keychain
/// read can ask for the login password while the app is not signed with a Developer ID.
pub struct LazyKeyStore {
    inner: Box<dyn KeyStore>,
    /// `has-key.json`: `true` or `false`. Absent (a key saved by an earlier version): the first `has` reads once.
    marker: std::path::PathBuf,
    /// The value read in this session: `None` not read yet, `Some(None)` no key.
    cache: Mutex<Option<Option<String>>>,
}

impl LazyKeyStore {
    pub fn new(inner: Box<dyn KeyStore>, app_data: &std::path::Path) -> Self {
        LazyKeyStore { inner, marker: app_data.join("has-key.json"), cache: Mutex::new(None) }
    }

    fn mark(&self, has: bool) {
        if let Some(dir) = self.marker.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&self.marker, if has { "true" } else { "false" });
    }

    fn marked(&self) -> Option<bool> {
        serde_json::from_str(&std::fs::read_to_string(&self.marker).ok()?).ok()
    }
}

impl KeyStore for LazyKeyStore {
    fn get(&self) -> Option<String> {
        let mut c = self.cache.lock().ok()?;
        if let Some(v) = c.as_ref() {
            return v.clone();
        }
        let v = self.inner.get();
        if self.marked() != Some(v.is_some()) {
            self.mark(v.is_some());
        }
        *c = Some(v.clone());
        v
    }

    fn set(&self, key: &str) -> Result<(), String> {
        self.inner.set(key)?;
        self.mark(true);
        if let Ok(mut c) = self.cache.lock() {
            *c = Some(Some(key.trim().to_string()));
        }
        Ok(())
    }

    fn delete(&self) -> Result<(), String> {
        self.inner.delete()?;
        self.mark(false);
        if let Ok(mut c) = self.cache.lock() {
            *c = Some(None);
        }
        Ok(())
    }

    fn has(&self) -> bool {
        if let Some(v) = self.cache.lock().ok().and_then(|c| c.clone()) {
            return v.is_some();
        }
        self.marked().unwrap_or_else(|| self.get().is_some())
    }
}

/// The key in a file of the app's data folder, readable by the user only (A-072): for builds whose signature does
/// not let them read the keychain silently (`signing::keychain_is_silent`). Never in the project, the logs or the
/// settings file.
pub struct FileKeyStore {
    path: std::path::PathBuf,
}

impl FileKeyStore {
    pub fn new(app_data: &std::path::Path) -> Self {
        FileKeyStore { path: app_data.join("openrouter.key") }
    }
}

impl KeyStore for FileKeyStore {
    fn get(&self) -> Option<String> {
        let k = std::fs::read_to_string(&self.path).ok()?;
        (!k.trim().is_empty()).then(|| k.trim().to_string())
    }

    fn set(&self, key: &str) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        // Written beside and renamed, so the file never exists with wider permissions or half a key.
        let tmp = self.path.with_extension("key.tmp");
        let _ = std::fs::remove_file(&tmp);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut f = options.open(&tmp).map_err(|e| e.to_string())?;
        std::io::Write::write_all(&mut f, key.trim().as_bytes()).map_err(|e| e.to_string())?;
        drop(f);
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }

    fn delete(&self) -> Result<(), String> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Where the app keeps the key, told to the user wherever the key is mentioned (AI-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum KeyPlace {
    Keychain,
    File,
}

/// The app's key store (A-072): the system keychain when it can be read without a password prompt, a private file
/// otherwise. `STEMWISE_KEY_STORE` (`keychain` or `file`) forces one of the two. Neither store is read here.
pub fn app_key_store(app_data: &std::path::Path, keychain_is_silent: bool) -> (Box<dyn KeyStore>, KeyPlace) {
    let forced = std::env::var("STEMWISE_KEY_STORE").ok();
    let keychain = match forced.as_deref() {
        Some("keychain") => true,
        Some("file") => false,
        _ => keychain_is_silent,
    };
    if keychain {
        (Box::new(LazyKeyStore::new(Box::new(SystemKeyStore), app_data)), KeyPlace::Keychain)
    } else {
        (Box::new(FileKeyStore::new(app_data)), KeyPlace::File)
    }
}

/// In-memory store (tests and the browser-less CLI).
#[derive(Default)]
pub struct MemoryKeyStore(Mutex<Option<String>>);

impl KeyStore for MemoryKeyStore {
    fn get(&self) -> Option<String> {
        self.0.lock().ok()?.clone()
    }

    fn set(&self, key: &str) -> Result<(), String> {
        *self.0.lock().map_err(|e| e.to_string())? = Some(key.trim().to_string());
        Ok(())
    }

    fn delete(&self) -> Result<(), String> {
        *self.0.lock().map_err(|e| e.to_string())? = None;
        Ok(())
    }
}

/// Shows a key safely (settings screen): the first characters and the last four.
pub fn masked(key: &str) -> String {
    let k = key.trim();
    if k.len() <= 10 {
        return "•".repeat(k.len());
    }
    format!("{}…{}", &k[..6], &k[k.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: AI-11
    #[test]
    fn keys_are_stored_behind_the_interface_and_shown_masked() {
        let s = MemoryKeyStore::default();
        assert!(s.get().is_none());
        s.set("  sk-or-v1-abcdef0123456789  ").unwrap();
        assert_eq!(s.get().as_deref(), Some("sk-or-v1-abcdef0123456789"));
        assert_eq!(masked("sk-or-v1-abcdef0123456789"), "sk-or-…6789");
        s.delete().unwrap();
        assert!(s.get().is_none());
    }

    /// A store that counts its reads (each read of the real keychain may ask for the password).
    struct Counting(MemoryKeyStore, std::sync::Arc<std::sync::atomic::AtomicUsize>);

    impl KeyStore for Counting {
        fn get(&self) -> Option<String> {
            self.1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.0.get()
        }
        fn set(&self, key: &str) -> Result<(), String> {
            self.0.set(key)
        }
        fn delete(&self) -> Result<(), String> {
            self.0.delete()
        }
    }

    /// A lazy store over a counting one holding `key`, and its read counter.
    fn lazy(
        dir: &std::path::Path,
        key: Option<&str>,
    ) -> (LazyKeyStore, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        let n = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let inner = Counting(MemoryKeyStore::default(), n.clone());
        if let Some(k) = key {
            inner.0.set(k).unwrap();
        }
        (LazyKeyStore::new(Box::new(inner), dir), n)
    }

    /// req: AI-11
    #[test]
    fn the_keychain_is_read_at_most_once_and_never_to_know_whether_a_key_is_saved() {
        use std::sync::atomic::Ordering::SeqCst;
        let dir = tempfile::tempdir().unwrap();
        let key = "sk-or-v1-abcdef0123456789";
        // A key saved by an earlier version (no marker): one read, then the marker answers.
        let (s, n) = lazy(dir.path(), Some(key));
        assert!(s.has() && s.has());
        assert_eq!(s.get().as_deref(), Some(key));
        assert_eq!(n.load(SeqCst), 1);
        // The next session: the marker says there is a key; nothing is read until a call needs it, then once.
        let (s, n) = lazy(dir.path(), Some(key));
        for _ in 0..10 {
            assert!(s.has());
        }
        assert_eq!(n.load(SeqCst), 0);
        s.get();
        s.get();
        assert_eq!(n.load(SeqCst), 1);
        // Removing it is remembered without reading.
        s.delete().unwrap();
        assert!(!s.has() && s.get().is_none());
        let (s, n) = lazy(dir.path(), None);
        assert!(!s.has());
        assert_eq!(n.load(SeqCst), 0);
    }
    /// req: AI-11
    #[test]
    fn the_key_file_is_private_replaced_whole_and_removed_on_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileKeyStore::new(&dir.path().join("data"));
        assert!(store.get().is_none() && !store.has());
        store.delete().unwrap();
        store.set("  sk-or-first\n").unwrap();
        assert_eq!(store.get().as_deref(), Some("sk-or-first"));
        store.set("sk-or-second").unwrap();
        assert_eq!(store.get().as_deref(), Some("sk-or-second"));
        let file = dir.path().join("data/openrouter.key");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o600, "only the user reads it");
        }
        // Nothing else is left beside it.
        let names: Vec<_> =
            std::fs::read_dir(file.parent().unwrap()).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names.len(), 1, "{names:?}");
        store.delete().unwrap();
        assert!(!file.exists() && !store.has());
    }

    /// req: AI-11
    #[test]
    fn the_store_follows_the_signature_and_choosing_it_reads_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (_, place) = app_key_store(dir.path(), true);
        assert_eq!(place, KeyPlace::Keychain);
        let (store, place) = app_key_store(dir.path(), false);
        assert_eq!(place, KeyPlace::File);
        store.set("sk-or-x").unwrap();
        assert!(dir.path().join("openrouter.key").exists());
        assert_eq!(serde_json::to_string(&KeyPlace::File).unwrap(), "\"file\"");
    }
}
