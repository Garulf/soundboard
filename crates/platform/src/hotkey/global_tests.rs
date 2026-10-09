use super::*;
use crate::hotkey::{Binding, HotkeyStatus};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Default)]
struct FakeRegistrar {
    refused: Vec<Accelerator>,
    registered: Vec<Accelerator>,
}

impl Registrar for FakeRegistrar {
    fn register(&mut self, accelerator: &Accelerator) -> Result<u32, String> {
        if self.refused.contains(accelerator) {
            return Err("already grabbed by another application".into());
        }
        self.registered.push(*accelerator);
        Ok(accelerator.to_global_hotkey().id())
    }

    fn unregister(&mut self, accelerator: &Accelerator) {
        self.registered.retain(|a| a != accelerator);
    }
}

fn binding(id: &str, acc: &str) -> Binding {
    Binding {
        id: id.into(),
        description: id.into(),
        accelerator: acc.parse().unwrap(),
    }
}

fn provider(refused: &[&str]) -> (GlobalProvider<FakeRegistrar>, Arc<AtomicU32>) {
    let presses = Arc::new(AtomicU32::new(0));
    let counter = presses.clone();
    let registrar = FakeRegistrar {
        refused: refused.iter().map(|s| s.parse().unwrap()).collect(),
        registered: Vec::new(),
    };
    let provider = GlobalProvider::new(
        registrar,
        "test",
        Arc::new(move |_: &str| {
            counter.fetch_add(1, Ordering::SeqCst);
        }),
    );
    (provider, presses)
}

fn failed(provider: &GlobalProvider<FakeRegistrar>) -> Vec<String> {
    match provider.status() {
        HotkeyStatus::Active { failed, .. } => failed.into_iter().map(|(id, _)| id).collect(),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn duplicate_accelerator_fails_only_the_second_binding() {
    let (mut provider, _) = provider(&[]);
    provider.bind(vec![
        binding("a", "Ctrl+1"),
        binding("b", "Ctrl+1"),
        binding("c", "Ctrl+2"),
    ]);
    assert_eq!(failed(&provider), ["b"]);
    assert_eq!(provider.registrar.registered.len(), 2);
}

#[test]
fn refused_key_is_reported_and_others_still_bind() {
    let (mut provider, _) = provider(&["Ctrl+1"]);
    provider.bind(vec![binding("a", "Ctrl+1"), binding("b", "Ctrl+2")]);
    assert_eq!(failed(&provider), ["a"]);
    assert_eq!(
        provider.registrar.registered,
        vec!["Ctrl+2".parse().unwrap()]
    );
}

#[test]
fn rebinding_replaces_previous_registrations() {
    let (mut provider, _) = provider(&[]);
    provider.bind(vec![binding("a", "Ctrl+1")]);
    provider.bind(vec![binding("b", "Ctrl+2")]);
    assert_eq!(
        provider.registrar.registered,
        vec!["Ctrl+2".parse().unwrap()]
    );
}

#[test]
fn press_dispatches_to_the_bound_id() {
    let (mut provider, presses) = provider(&[]);
    provider.bind(vec![binding("a", "Ctrl+1")]);
    let id = "Ctrl+1"
        .parse::<Accelerator>()
        .unwrap()
        .to_global_hotkey()
        .id();
    provider.dispatch(id);
    provider.dispatch(12345);
    assert_eq!(presses.load(Ordering::SeqCst), 1);
}

#[test]
fn triggers_show_bound_accelerators() {
    let (mut provider, _) = provider(&[]);
    provider.bind(vec![binding("a", "ctrl+shift+f1")]);
    assert_eq!(
        provider.triggers().get("a").map(String::as_str),
        Some("Ctrl+Shift+F1")
    );
}
