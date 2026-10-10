//! A persisted communication preference, emitted only when model-visible state changes.

use super::PreviousSectionState;
use super::SectionTransition;
use super::WorldStateSection;
use super::WorldStateUpdate;
use crate::context::ContextualUserFragment;
use codex_protocol::models::ContentItemKind;

#[derive(Clone, Debug)]
pub(crate) struct ProgressMessagesState(Option<bool>, codex_config::types::CustomTuiPreferences);

impl ProgressMessagesState {
    pub(crate) fn with_custom(mut self, custom: codex_config::types::CustomTuiPreferences) -> Self {
        use codex_config::types::CustomProgressMode;
        if let Some(mode) = custom.progress {
            self.0 = Some(mode != CustomProgressMode::Off);
        }
        self.1 = custom;
        self
    }

    pub(crate) fn new(enabled: Option<bool>) -> Self {
        Self(enabled, Default::default())
    }
}

struct ProgressMessagesInstruction(Option<bool>, codex_config::types::CustomTuiPreferences);

impl ContextualUserFragment for ProgressMessagesInstruction {
    fn role(&self) -> &'static str {
        "developer"
    }
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("custom_settings.progress_messages".into())
    }
    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }
    fn type_markers() -> (&'static str, &'static str) {
        (
            "<progress_messages_preference>",
            "</progress_messages_preference>",
        )
    }
    fn body(&self) -> String {
        let instruction = match self.0 {
            Some(false) => {
                "The user has disabled routine progress narration. Skip preambles, acknowledgements, plans in commentary, and periodic work updates, including before tool calls. This replaces earlier instructions requiring those updates. Work autonomously and provide a concise final result. Still ask necessary questions or approvals and report blockers when user input is required."
            }
            Some(true) => {
                "The user has enabled routine progress messages again. The earlier quiet-mode preference no longer applies; provide concise useful progress updates when appropriate."
            }
            None => {
                "The earlier progress-message preference no longer applies. Follow the normal communication instructions."
            }
        };
        use codex_config::types::{CustomCheckLevel, CustomEditMessages, CustomProgressMode};
        let mut text = instruction.to_string();
        if self.1.progress == Some(CustomProgressMode::Important) {
            text = "The user wants only major progress milestones. Skip routine preambles and tool-by-tool narration. Briefly report a significant finding, a change of approach, or a blocker; retain necessary questions and approvals. This replaces earlier requirements for frequent updates.".into();
        }
        if let Some(level) = self.1.checks {
            text.push_str("\nValidation preference: ");
            text.push_str(match level {
                CustomCheckLevel::Necessary => "Run only checks needed to verify the change and required project checks. Avoid broad optional audits and repeating successful checks without new evidence.",
                CustomCheckLevel::Normal => "Run relevant targeted tests and required project checks. Add regression checks when the change justifies them.",
                CustomCheckLevel::Extended => "Run relevant targeted tests, required project checks, and broader regression checks appropriate to the change. Do not invent unrelated work.",
            });
            text.push_str(
                " Always respect explicit user constraints, including rules about who runs builds.",
            );
        } else {
            text.push_str("\nUse the normal project validation instructions; earlier custom validation preferences no longer apply.");
        }
        text.push_str("\nEdit-message preference: ");
        text.push_str(match self.1.edit_messages {
            Some(CustomEditMessages::Brief) => "Before editing, briefly name the file and the purpose. Show a prose diff only when explicitly requested; avoid repeating the tool patch in messages.",
            Some(CustomEditMessages::Diff) => "Before editing, name the file and purpose and show the principal changed fragments in a Markdown diff. For large changes, summarize the remaining edits; do not repeat the entire patch.",
            Some(CustomEditMessages::Off) => "Skip routine pre-edit announcements and prose diffs unless explicitly requested. Keep the final result, validation details, necessary questions, approvals, and blockers.",
            None => "The earlier custom edit-message preference no longer applies. Follow the current user and project instructions for edit announcements and diffs.",
        });
        if self.1.edit_messages.is_some() {
            text.push_str(" This is the user's selected communication style and replaces earlier standing requirements about edit announcements and prose diffs, including AGENTS.md. It applies to edit announcements independently of routine progress narration. Respect explicit requests in the current task. This preference does not hide tool activity or change approval requirements.");
        }
        format!("\n{text}\n")
    }
}

impl WorldStateSection for ProgressMessagesState {
    const ID: &'static str = "custom_progress_messages";
    type Snapshot = String;

    fn matches_legacy_fragment(role: &str, text: &str) -> bool {
        role == "developer" && ProgressMessagesInstruction::matches_text(text)
    }
    fn has_retained_fragment_matcher() -> bool {
        true
    }
    fn matches_retained_fragment(role: &str, text: &str) -> bool {
        Self::matches_legacy_fragment(role, text)
    }
    fn render_diff(
        &self,
        previous: PreviousSectionState<'_, Self::Snapshot>,
    ) -> SectionTransition<Self::Snapshot> {
        // Null snapshots are removed by WorldState; keep an explicit default state.
        let snapshot = match self.0 {
            Some(true) => "on",
            Some(false) => "off",
            None => "default",
        }
        .to_string();
        let snapshot = if self.1.progress.is_some()
            || self.1.checks.is_some()
            || self.1.edit_messages.is_some()
        {
            format!("{snapshot}:{:?}:{:?}", self.1.progress, self.1.checks)
        } else {
            snapshot
        };
        let snapshot = if let Some(mode) = self.1.edit_messages {
            format!("{snapshot}:edits:{mode:?}")
        } else {
            snapshot
        };
        if matches!(previous, PreviousSectionState::Known(value) if *value == snapshot) {
            return (None, Vec::new());
        }
        if self.0.is_none()
            && self.1.checks.is_none()
            && self.1.edit_messages.is_none()
            && matches!(previous, PreviousSectionState::Absent)
        {
            return (Some(snapshot), Vec::new());
        }
        (
            Some(snapshot),
            vec![WorldStateUpdate::fragment(ProgressMessagesInstruction(
                self.0,
                self.1.clone(),
            ))],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::world_state::WorldState;
    use crate::context::world_state::WorldStateSnapshot;

    #[test]
    fn progress_messages_preference_survives_resume_without_repeated_prompt_text() {
        let mut old = WorldState::default();
        old.add_section(ProgressMessagesState::new(None));
        let mut history = Vec::new();
        let mut snapshot = old.render_full().0;
        for value in [Some(false), Some(false), Some(true), Some(true), None, None] {
            let previous: WorldStateSnapshot =
                serde_json::from_value(serde_json::to_value(&snapshot).unwrap()).unwrap();
            let mut state = WorldState::default();
            state.add_section(ProgressMessagesState::new(value));
            let (next, fragments) = state.render_history_fragment_diff(Some(&previous), &history);
            let repeated = previous == next;
            assert_eq!(fragments.is_empty(), repeated);
            if value == Some(false) && !repeated {
                let text = fragments[0].render();
                assert!(text.contains("Skip preambles"));
                assert!(text.contains("necessary questions or approvals"));
            }
            history.extend(
                fragments
                    .into_iter()
                    .map(ContextualUserFragment::into_boxed_response_item),
            );
            snapshot = next;
        }
    }

    #[test]
    fn progress_messages_default_adds_no_instruction() {
        let mut state = WorldState::default();
        state.add_section(ProgressMessagesState::new(None));
        assert!(state.render_history_fragment_diff(None, &[]).1.is_empty());
    }

    #[test]
    fn custom_settings_important_progress_and_checks_replace_quiet_mode_once() {
        use codex_config::types::{CustomCheckLevel, CustomProgressMode, CustomTuiPreferences};
        let pref = CustomTuiPreferences {
            progress: Some(CustomProgressMode::Important),
            checks: Some(CustomCheckLevel::Necessary),
            live_activity: Some(false),
            ..Default::default()
        };
        let mut state = WorldState::default();
        state.add_section(ProgressMessagesState::new(Some(false)).with_custom(pref));
        let (snapshot, updates) = state.render_history_fragment_diff(None, &[]);
        let text = updates[0].render();
        assert!(text.contains("only major progress milestones"));
        assert!(text.contains("required project checks"));
        assert!(text.contains("who runs builds"));
        let history: Vec<_> = updates
            .into_iter()
            .map(ContextualUserFragment::into_boxed_response_item)
            .collect();
        assert!(
            state
                .render_history_fragment_diff(Some(&snapshot), &history)
                .1
                .is_empty()
        );
    }

    #[test]
    fn progress_messages_recovers_preference_from_legacy_history() {
        let mut state = WorldState::default();
        state.add_section(ProgressMessagesState::new(Some(true)));
        let history = vec![ContextualUserFragment::into(ProgressMessagesInstruction(
            Some(false),
            Default::default(),
        ))];
        let (_, fragments) = state.render_history_fragment_diff(None, &history);
        assert_eq!(fragments.len(), 1);
        assert!(
            fragments[0]
                .render()
                .contains("enabled routine progress messages again")
        );
    }

    #[test]
    fn edit_messages_switch_and_reset_without_repeating_preferences() {
        use codex_config::types::{CustomEditMessages, CustomTuiPreferences};
        let mut snapshot = None;
        let mut history = Vec::new();
        let mut last = None;
        for mode in [
            Some(CustomEditMessages::Brief),
            Some(CustomEditMessages::Brief),
            Some(CustomEditMessages::Diff),
            Some(CustomEditMessages::Off),
            None,
            None,
        ] {
            let mut state = WorldState::default();
            state.add_section(ProgressMessagesState::new(Some(false)).with_custom(
                CustomTuiPreferences {
                    edit_messages: mode,
                    ..Default::default()
                },
            ));
            let (next, updates) = state.render_history_fragment_diff(snapshot.as_ref(), &history);
            assert_eq!(updates.is_empty(), snapshot.is_some() && mode == last);
            if let Some(update) = updates.first() {
                let text = update.render();
                assert!(text.contains(match mode {
                    Some(CustomEditMessages::Brief) =>
                        "Show a prose diff only when explicitly requested",
                    Some(CustomEditMessages::Diff) => "Markdown diff",
                    Some(CustomEditMessages::Off) => "Skip routine pre-edit announcements",
                    None => "earlier custom edit-message preference no longer applies",
                }));
            }
            history.extend(
                updates
                    .into_iter()
                    .map(ContextualUserFragment::into_boxed_response_item),
            );
            snapshot = Some(serde_json::from_value(serde_json::to_value(next).unwrap()).unwrap());
            last = mode;
        }
    }

    #[test]
    fn visual_action_preferences_do_not_add_model_context() {
        use codex_config::types::CustomTuiPreferences;
        let mut snapshot = None;
        for compact in [true, false, true] {
            let mut state = WorldState::default();
            state.add_section(
                ProgressMessagesState::new(None).with_custom(CustomTuiPreferences {
                    compact_actions: Some(compact),
                    ..Default::default()
                }),
            );
            let (next, updates) = state.render_history_fragment_diff(snapshot.as_ref(), &[]);
            assert!(updates.is_empty());
            if let Some(previous) = &snapshot {
                assert_eq!(*previous, next);
            }
            snapshot = Some(next);
        }
    }
}
