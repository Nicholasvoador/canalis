pub mod home;
pub mod panorama;
pub mod setting;

use super::editor::SettingsEditor;
use super::jobs::Jobs;
use super::media_preview::MediaPreview;
use super::network_history::NetworkHistory;
use crate::status::LiveStatus;

/// Everything a page needs for one frame.
pub struct PageContext<'a> {
    pub editor: &'a mut SettingsEditor,
    pub status: &'a LiveStatus,
    pub network: &'a NetworkHistory,
    pub now_millis: u64,
    pub jobs: &'a mut Jobs,
    pub media_preview: &'a mut MediaPreview,
    /// Set by a page while the user drags something, so saving waits for release.
    pub interacting: bool,
}

impl PageContext<'_> {
    pub fn connected(&self) -> bool {
        self.status.is_connected(self.now_millis)
    }

    pub fn service_running(&self) -> bool {
        self.status.is_service_running(self.now_millis)
    }

    pub fn temperature_unit(&self) -> String {
        self.editor.settings.temperature_unit.clone()
    }
}
