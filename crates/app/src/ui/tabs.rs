use egui::{Key, Ui};
use soundboard_core::{Command, Library, SoundId, TabId};

pub enum TabAction {
    Select(Option<TabId>),
    Command(Command),
}

#[derive(Default)]
pub struct TabBar {
    renaming: Option<(TabId, String)>,
}

impl TabBar {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        library: &Library,
        active: Option<TabId>,
    ) -> Vec<TabAction> {
        let mut actions = Vec::new();
        if ui.selectable_label(active.is_none(), "All").clicked() {
            actions.push(TabAction::Select(None));
        }
        let can_delete = library.tabs.len() > 1;
        for tab in &library.tabs {
            if let Some((id, name)) = &mut self.renaming
                && *id == tab.id
            {
                let edit = ui.add(egui::TextEdit::singleline(name).desired_width(110.0));
                edit.request_focus();
                if ui.input(|i| i.key_pressed(Key::Enter)) || edit.lost_focus() {
                    let name = name.trim().to_string();
                    if !name.is_empty() && name != tab.name {
                        actions.push(TabAction::Command(Command::RenameTab(tab.id, name)));
                    }
                    self.renaming = None;
                } else if ui.input(|i| i.key_pressed(Key::Escape)) {
                    self.renaming = None;
                }
                continue;
            }
            let response = ui.selectable_label(active == Some(tab.id), &tab.name);
            if response.clicked() {
                actions.push(TabAction::Select(Some(tab.id)));
            }
            if response.double_clicked() {
                self.renaming = Some((tab.id, tab.name.clone()));
            }
            if let Some(sound) = response.dnd_release_payload::<SoundId>() {
                actions.push(TabAction::Command(Command::MoveSound {
                    sound: *sound,
                    tab: tab.id,
                    index: usize::MAX,
                }));
            }
            response.context_menu(|ui| {
                if ui.button("Rename").clicked() {
                    self.renaming = Some((tab.id, tab.name.clone()));
                    ui.close();
                }
                let delete = ui
                    .add_enabled(can_delete, egui::Button::new("Delete tab"))
                    .on_disabled_hover_text("The last tab cannot be deleted.")
                    .on_hover_text("Its sounds move to the first tab.");
                if delete.clicked() {
                    actions.push(TabAction::Command(Command::DeleteTab(tab.id)));
                    ui.close();
                }
            });
        }
        if ui.button("+").on_hover_text("Add a tab").clicked() {
            actions.push(TabAction::Command(Command::AddTab(format!(
                "Tab {}",
                library.tabs.len() + 1
            ))));
        }
        actions
    }
}
