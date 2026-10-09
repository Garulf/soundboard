use super::{Binding, HotkeyProvider, HotkeyStatus, PressHandler};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut, Shortcut};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

const BACKEND: &str = "desktop portal";

#[derive(Default)]
struct PortalState {
    status: Option<HotkeyStatus>,
    triggers: HashMap<String, String>,
}

type SharedState = Arc<Mutex<PortalState>>;

fn update(state: &SharedState, f: impl FnOnce(&mut PortalState)) {
    f(&mut state.lock().unwrap_or_else(PoisonError::into_inner));
}

fn unavailable(reason: impl std::fmt::Display) -> HotkeyStatus {
    HotkeyStatus::Unavailable(format!(
        "This desktop does not offer global shortcuts to apps ({reason}). \
         You can bind a desktop shortcut to the remote API with curl instead."
    ))
}

/// Global shortcuts through the xdg-desktop-portal GlobalShortcuts interface,
/// the only way to receive hotkeys on Wayland.
pub struct PortalProvider {
    tx: UnboundedSender<Vec<Binding>>,
    state: SharedState,
}

impl PortalProvider {
    pub fn start(on_press: PressHandler) -> Self {
        let (tx, rx) = unbounded_channel();
        let state = SharedState::default();
        let thread_state = state.clone();
        let spawned = std::thread::Builder::new()
            .name("hotkey-portal".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                match runtime {
                    Ok(runtime) => runtime.block_on(run(rx, on_press, thread_state)),
                    Err(e) => update(&thread_state, |s| s.status = Some(unavailable(e))),
                }
            });
        if let Err(e) = spawned {
            update(&state, |s| s.status = Some(unavailable(e)));
        }
        Self { tx, state }
    }
}

impl HotkeyProvider for PortalProvider {
    fn bind(&mut self, bindings: Vec<Binding>) {
        let _ = self.tx.send(bindings);
    }

    fn status(&self) -> HotkeyStatus {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .status
            .clone()
            .unwrap_or(HotkeyStatus::Starting)
    }

    fn triggers(&self) -> HashMap<String, String> {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .triggers
            .clone()
    }
}

fn trigger_map(shortcuts: &[Shortcut]) -> HashMap<String, String> {
    shortcuts
        .iter()
        .map(|s| (s.id().to_string(), s.trigger_description().to_string()))
        .collect()
}

async fn run(mut rx: UnboundedReceiver<Vec<Binding>>, on_press: PressHandler, state: SharedState) {
    let portal = match GlobalShortcuts::new().await {
        Ok(portal) => portal,
        Err(e) => return update(&state, |s| s.status = Some(unavailable(e))),
    };
    let (mut activated, mut changed) = match (
        portal.receive_activated().await,
        portal.receive_shortcuts_changed().await,
    ) {
        (Ok(a), Ok(c)) => (a, c),
        (Err(e), _) | (_, Err(e)) => {
            return update(&state, |s| s.status = Some(unavailable(e)));
        }
    };
    update(&state, |s| {
        s.status = Some(HotkeyStatus::Active {
            backend: BACKEND,
            failed: Vec::new(),
        })
    });

    let mut session = None;
    loop {
        tokio::select! {
            bindings = rx.recv() => {
                let Some(mut bindings) = bindings else { break };
                while let Ok(newer) = rx.try_recv() {
                    bindings = newer;
                }
                if let Some(old) = session.take() {
                    let old: ashpd::desktop::Session<GlobalShortcuts> = old;
                    let _ = old.close().await;
                }
                if bindings.is_empty() {
                    update(&state, |s| s.triggers.clear());
                    continue;
                }
                match bind(&portal, &bindings).await {
                    Ok((new_session, triggers)) => {
                        session = Some(new_session);
                        update(&state, |s| {
                            s.triggers = triggers;
                            s.status = Some(HotkeyStatus::Active { backend: BACKEND, failed: Vec::new() });
                        });
                    }
                    Err(e) => {
                        tracing::warn!("binding portal shortcuts failed: {e}");
                        update(&state, |s| s.status = Some(unavailable(e)));
                    }
                }
            }
            Some(event) = activated.next() => on_press(event.shortcut_id()),
            Some(event) = changed.next() => {
                let triggers = trigger_map(event.shortcuts());
                update(&state, |s| s.triggers = triggers);
            }
        }
    }
}

async fn bind(
    portal: &GlobalShortcuts,
    bindings: &[Binding],
) -> Result<
    (
        ashpd::desktop::Session<GlobalShortcuts>,
        HashMap<String, String>,
    ),
    ashpd::Error,
> {
    let session = portal.create_session(Default::default()).await?;
    let shortcuts: Vec<NewShortcut> = bindings
        .iter()
        .map(|b| {
            let trigger = b.accelerator.to_portal_trigger();
            NewShortcut::new(&b.id, &b.description).preferred_trigger(trigger.as_str())
        })
        .collect();
    let response = portal
        .bind_shortcuts(&session, &shortcuts, None, Default::default())
        .await?
        .response()?;
    Ok((session, trigger_map(response.shortcuts())))
}
