//! macOS: run as an "accessory" app, so tally has a menu-bar icon but no Dock icon
//! and no app menu.

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

/// Must be called on the main thread after the event loop has started (winit sets the
/// policy to Regular while launching, so calling it earlier has no effect).
pub fn set_accessory() {
    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("set_accessory called off the main thread");
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    if !app.setActivationPolicy(NSApplicationActivationPolicy::Accessory) {
        tracing::warn!("macOS refused the accessory activation policy");
    }
}
