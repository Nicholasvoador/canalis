//! Keeps the on-screen settings and the settings file in step: saves edits
//! (once a drag is released) and picks up changes made with the command line.

use crate::settings::{Settings, SettingsStore};
use std::time::SystemTime;

pub struct SettingsEditor {
    store: SettingsStore,
    pub settings: Settings,
    saved: Settings,
    saved_modified: Option<SystemTime>,
    pub error: Option<String>,
}

impl SettingsEditor {
    pub fn new(store: SettingsStore) -> Self {
        let (settings, error) = match store.load() {
            Ok(settings) => (settings, None),
            Err(error) => (Settings::default(), Some(error)),
        };
        let saved_modified = store.modified_time();
        Self { store, saved: settings.clone(), settings, saved_modified, error }
    }

    pub fn path(&self) -> &std::path::Path {
        self.store.path()
    }

    /// Call once per frame. `interacting` defers saving while a slider or point is being dragged.
    pub fn sync(&mut self, interacting: bool) {
        if self.settings != self.saved {
            if !interacting {
                self.save();
            }
        } else {
            self.reload_if_changed_elsewhere();
        }
    }

    fn save(&mut self) {
        match self.store.save(&self.settings) {
            Ok(()) => {
                self.saved = self.settings.clone();
                self.saved_modified = self.store.modified_time();
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn reload_if_changed_elsewhere(&mut self) {
        let modified = self.store.modified_time();
        if modified == self.saved_modified {
            return;
        }
        self.saved_modified = modified;
        if let Ok(settings) = self.store.load() {
            self.saved = settings.clone();
            self.settings = settings;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::tests::temporary_path;

    #[test]
    fn saves_after_interaction_ends() {
        let path = temporary_path("editor-save");
        let mut editor = SettingsEditor::new(SettingsStore::new(path.clone()));
        editor.settings.brightness = 30;
        editor.sync(true);
        assert!(!path.exists());
        editor.sync(false);
        assert_eq!(SettingsStore::new(path).load().unwrap().brightness, 30);
    }

    #[test]
    fn picks_up_changes_from_the_command_line() {
        let path = temporary_path("editor-reload");
        let mut editor = SettingsEditor::new(SettingsStore::new(path.clone()));
        std::thread::sleep(std::time::Duration::from_millis(20));
        SettingsStore::new(path).save(&Settings { brightness: 10, ..Settings::default() }).unwrap();
        editor.sync(false);
        assert_eq!(editor.settings.brightness, 10);
    }
}
