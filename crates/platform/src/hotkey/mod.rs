pub mod accelerator;
pub mod global;
#[cfg(target_os = "linux")]
mod portal;

pub use accelerator::{Accelerator, AcceleratorError, Key};

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub id: String,
    pub description: String,
    pub accelerator: Accelerator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyStatus {
    Starting,
    Active {
        backend: &'static str,
        failed: Vec<(String, String)>,
    },
    Unavailable(String),
}

pub type PressHandler = Arc<dyn Fn(&str) + Send + Sync>;

pub trait HotkeyProvider {
    /// Replaces every registered binding with `bindings`.
    fn bind(&mut self, bindings: Vec<Binding>);
    fn status(&self) -> HotkeyStatus;
    /// Human-readable trigger per binding id, as the backend reports it.
    fn triggers(&self) -> HashMap<String, String>;
}

struct UnavailableProvider(String);

impl HotkeyProvider for UnavailableProvider {
    fn bind(&mut self, _bindings: Vec<Binding>) {}

    fn status(&self) -> HotkeyStatus {
        HotkeyStatus::Unavailable(self.0.clone())
    }

    fn triggers(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

fn global_provider(on_press: PressHandler) -> Box<dyn HotkeyProvider> {
    match global::ManagerRegistrar::new() {
        Ok(registrar) => {
            let provider = global::GlobalProvider::new(registrar, "system", on_press);
            provider.install_event_handler();
            Box::new(provider)
        }
        Err(e) => Box::new(UnavailableProvider(format!(
            "Global hotkeys could not be started: {e}"
        ))),
    }
}

/// Picks the hotkey backend for this session. Must be called on the main
/// thread, because on Windows hotkey messages go to the creating thread.
pub fn create_hotkeys(on_press: PressHandler) -> Box<dyn HotkeyProvider> {
    #[cfg(target_os = "linux")]
    if std::env::var("XDG_SESSION_TYPE").is_ok_and(|t| t.eq_ignore_ascii_case("wayland")) {
        return Box::new(portal::PortalProvider::start(on_press));
    }
    global_provider(on_press)
}
