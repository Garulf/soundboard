use soundboard_core::{Command, ControllerHandle, RemoteSettings, Snapshot, sound_key};
use soundboard_remote::{RemoteApi, RemoteServer, RemoteSound, RemoteState, RemoteTab};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::sync::Arc;

pub fn color_hex([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

pub fn remote_state(snapshot: &Snapshot, playing: &[(u64, f32)]) -> RemoteState {
    let playing: HashSet<u64> = playing.iter().map(|(key, _)| *key).collect();
    let library = &snapshot.library;
    RemoteState {
        tabs: library
            .tabs
            .iter()
            .map(|tab| RemoteTab {
                id: tab.id.to_string(),
                name: tab.name.clone(),
                sounds: tab.order.iter().map(ToString::to_string).collect(),
            })
            .collect(),
        sounds: library
            .sounds
            .iter()
            .map(|sound| RemoteSound {
                id: sound.id.to_string(),
                name: sound.name.clone(),
                color: color_hex(sound.color),
                hotkey: sound.hotkey.clone(),
                playing: playing.contains(&sound_key(sound.id)),
            })
            .collect(),
    }
}

pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    if getrandom::fill(&mut bytes).is_err() {
        let fallback = format!("{}{}", uuid_like(), uuid_like());
        return fallback;
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn uuid_like() -> String {
    soundboard_core::SoundId::new().0.simple().to_string()
}

pub fn remote_url(host: &str, port: u16, token: &str) -> String {
    format!("http://{host}:{port}/?token={token}")
}

/// The address other devices on the LAN can reach us at. Connecting a UDP
/// socket sends nothing; it only asks the OS which interface it would use.
pub fn lan_ip() -> IpAddr {
    UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|socket| {
            socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9))?;
            socket.local_addr()
        })
        .map(|addr| addr.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
}

struct AppRemote {
    controller: ControllerHandle,
}

impl RemoteApi for AppRemote {
    fn state(&self) -> RemoteState {
        remote_state(
            &self.controller.snapshot(),
            &self.controller.meters().playing(),
        )
    }

    fn send(&self, cmd: Command) {
        self.controller.send(cmd);
    }
}

/// Starts and stops the HTTP remote to match the current settings.
pub struct RemoteManager {
    controller: ControllerHandle,
    running: Option<(RemoteSettings, RemoteServer)>,
    applied: Option<RemoteSettings>,
    pub error: Option<String>,
}

impl RemoteManager {
    pub fn new(controller: ControllerHandle) -> Self {
        Self {
            controller,
            running: None,
            applied: None,
            error: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    pub fn sync(&mut self, settings: &RemoteSettings) {
        if self.applied.as_ref() == Some(settings) {
            return;
        }
        self.applied = Some(settings.clone());
        self.running = None;
        self.error = None;
        if !settings.enabled || settings.token.is_empty() {
            return;
        }
        let api = Arc::new(AppRemote {
            controller: self.controller.clone(),
        });
        match RemoteServer::start(settings.port, settings.token.clone(), api) {
            Ok(server) => self.running = Some((settings.clone(), server)),
            Err(e) => {
                tracing::warn!("remote failed to start: {e}");
                self.error = Some(e);
            }
        }
    }
}

#[cfg(test)]
#[path = "remote_api_tests.rs"]
mod tests;
