use super::accelerator::Accelerator;
use super::{Binding, HotkeyProvider, HotkeyStatus, PressHandler};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

/// The part of a hotkey backend that grabs and releases keys.
pub trait Registrar {
    fn register(&mut self, accelerator: &Accelerator) -> Result<u32, String>;
    fn unregister(&mut self, accelerator: &Accelerator);
}

pub struct ManagerRegistrar(GlobalHotKeyManager);

impl ManagerRegistrar {
    pub fn new() -> Result<Self, String> {
        GlobalHotKeyManager::new()
            .map(Self)
            .map_err(|e| e.to_string())
    }
}

impl Registrar for ManagerRegistrar {
    fn register(&mut self, accelerator: &Accelerator) -> Result<u32, String> {
        let hotkey = accelerator.to_global_hotkey();
        self.0
            .register(hotkey)
            .map(|()| hotkey.id())
            .map_err(|e| e.to_string())
    }

    fn unregister(&mut self, accelerator: &Accelerator) {
        if let Err(e) = self.0.unregister(accelerator.to_global_hotkey()) {
            tracing::debug!("unregistering {accelerator} failed: {e}");
        }
    }
}

type Routes = Arc<Mutex<HashMap<u32, String>>>;

/// Hotkeys through the `global-hotkey` crate (X11 and Windows).
pub struct GlobalProvider<R: Registrar> {
    pub(crate) registrar: R,
    backend: &'static str,
    routes: Routes,
    on_press: PressHandler,
    bound: Vec<(String, Accelerator)>,
    failed: Vec<(String, String)>,
}

impl<R: Registrar> GlobalProvider<R> {
    pub fn new(registrar: R, backend: &'static str, on_press: PressHandler) -> Self {
        Self {
            registrar,
            backend,
            routes: Routes::default(),
            on_press,
            bound: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// Routes OS hotkey events to the press handler.
    pub fn install_event_handler(&self) {
        let routes = self.routes.clone();
        let on_press = self.on_press.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state() == HotKeyState::Pressed {
                route(&routes, &on_press, event.id());
            }
        }));
    }

    pub fn dispatch(&self, hotkey_id: u32) {
        route(&self.routes, &self.on_press, hotkey_id);
    }
}

fn route(routes: &Routes, on_press: &PressHandler, hotkey_id: u32) {
    let target = routes
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&hotkey_id)
        .cloned();
    if let Some(binding) = target {
        on_press(&binding);
    }
}

impl<R: Registrar> HotkeyProvider for GlobalProvider<R> {
    fn bind(&mut self, bindings: Vec<Binding>) {
        for (_, accelerator) in self.bound.drain(..) {
            self.registrar.unregister(&accelerator);
        }
        self.failed.clear();
        let mut routes = HashMap::new();
        for binding in bindings {
            if self.bound.iter().any(|(_, a)| *a == binding.accelerator) {
                self.failed.push((
                    binding.id,
                    format!("{} is already used by another action", binding.accelerator),
                ));
                continue;
            }
            match self.registrar.register(&binding.accelerator) {
                Ok(id) => {
                    routes.insert(id, binding.id.clone());
                    self.bound.push((binding.id, binding.accelerator));
                }
                Err(e) => self.failed.push((binding.id, e)),
            }
        }
        *self.routes.lock().unwrap_or_else(PoisonError::into_inner) = routes;
    }

    fn status(&self) -> HotkeyStatus {
        HotkeyStatus::Active {
            backend: self.backend,
            failed: self.failed.clone(),
        }
    }

    fn triggers(&self) -> HashMap<String, String> {
        self.bound
            .iter()
            .map(|(id, acc)| (id.clone(), acc.to_string()))
            .collect()
    }
}

#[cfg(test)]
#[path = "global_tests.rs"]
mod tests;
