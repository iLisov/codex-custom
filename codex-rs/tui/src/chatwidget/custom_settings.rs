//! Local settings pages share the existing picker and show what changes token/context usage.

use super::*;
use codex_config::types::{
    CustomCheckLevel, CustomEditMessages, CustomProgressMode, NotificationMethod, Notifications,
};
use codex_protocol::config_types::Verbosity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CustomSettingsPage {
    Root,
    Answers,
    Progress,
    Edits,
    Context,
    Output,
    Compaction,
    Checks,
    Recap,
    Interface,
    Animations,
    Notifications,
    Sound,
    Status,
    Activity,
    ToolDetails,
    Command(SlashCommand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CustomSetting {
    Answers(Verbosity),
    Progress(CustomProgressMode),
    Edits(Option<CustomEditMessages>),
    Output(Option<usize>),
    Compaction(Option<i64>),
    Checks(CustomCheckLevel),
    Recap(bool),
    Animations(bool),
    Notifications(bool),
    Sound(NotificationMethod),
    Activity(bool),
    ToolDetails(bool),
}

impl CustomSetting {
    pub(crate) fn page(self) -> CustomSettingsPage {
        match self {
            Self::Answers(_) => CustomSettingsPage::Answers,
            Self::Progress(_) => CustomSettingsPage::Progress,
            Self::Edits(_) => CustomSettingsPage::Edits,
            Self::Output(_) => CustomSettingsPage::Output,
            Self::Compaction(_) => CustomSettingsPage::Compaction,
            Self::Checks(_) => CustomSettingsPage::Checks,
            Self::Recap(_) => CustomSettingsPage::Recap,
            Self::Animations(_) => CustomSettingsPage::Animations,
            Self::Notifications(_) => CustomSettingsPage::Notifications,
            Self::Sound(_) => CustomSettingsPage::Sound,
            Self::Activity(_) => CustomSettingsPage::Activity,
            Self::ToolDetails(_) => CustomSettingsPage::ToolDetails,
        }
    }
    pub(crate) fn edits(self) -> Vec<crate::legacy_core::config::edit::ConfigEdit> {
        use crate::legacy_core::config::edit::ConfigEdit;
        let clear = match self {
            Self::Edits(None) => Some(vec!["tui", "custom", "edit_messages"]),
            Self::Output(None) => Some(vec!["tool_output_token_limit"]),
            Self::Compaction(None) => Some(vec!["model_auto_compact_token_limit"]),
            _ => None,
        };
        if let Some(path) = clear {
            return vec![ConfigEdit::ClearPath {
                segments: path.into_iter().map(str::to_owned).collect(),
            }];
        }
        let (path, value) = match self {
            Self::Answers(value) => (
                vec!["model_verbosity"],
                toml_edit::value(match value {
                    Verbosity::Low => "low",
                    Verbosity::Medium => "medium",
                    Verbosity::High => "high",
                }),
            ),
            Self::Progress(value) => (
                vec!["tui", "custom", "progress"],
                toml_edit::value(match value {
                    CustomProgressMode::Off => "off",
                    CustomProgressMode::Important => "important",
                    CustomProgressMode::All => "all",
                }),
            ),
            Self::Edits(Some(value)) => (
                vec!["tui", "custom", "edit_messages"],
                toml_edit::value(match value {
                    CustomEditMessages::Brief => "brief",
                    CustomEditMessages::Diff => "diff",
                    CustomEditMessages::Off => "off",
                }),
            ),
            Self::Output(Some(value)) => (
                vec!["tool_output_token_limit"],
                toml_edit::value(value as i64),
            ),
            Self::Compaction(Some(value)) => (
                vec!["model_auto_compact_token_limit"],
                toml_edit::value(value),
            ),
            Self::Checks(value) => (
                vec!["tui", "custom", "checks"],
                toml_edit::value(match value {
                    CustomCheckLevel::Necessary => "necessary",
                    CustomCheckLevel::Normal => "normal",
                    CustomCheckLevel::Extended => "extended",
                }),
            ),
            Self::Recap(value) => (vec!["tui", "auto_recap"], toml_edit::value(value)),
            Self::Animations(value) => (vec!["tui", "animations"], toml_edit::value(value)),
            Self::Notifications(value) => (vec!["tui", "notifications"], toml_edit::value(value)),
            Self::Sound(value) => (
                vec!["tui", "notification_method"],
                toml_edit::value(value.to_string()),
            ),
            Self::Activity(value) => (
                vec!["tui", "custom", "live_activity"],
                toml_edit::value(value),
            ),
            Self::ToolDetails(value) => (
                vec!["tui", "custom", "compact_actions"],
                toml_edit::value(value),
            ),
            Self::Edits(None) | Self::Output(None) | Self::Compaction(None) => unreachable!(),
        };
        let mut edits = vec![ConfigEdit::SetPath {
            segments: path.into_iter().map(str::to_string).collect(),
            value,
        }];
        if let Self::Progress(value) = self {
            edits.push(ConfigEdit::SetPath {
                segments: vec!["tui".into(), "progress_messages".into()],
                value: toml_edit::value(value != CustomProgressMode::Off),
            });
        }
        edits
    }
    pub(crate) fn apply(self, local: &mut crate::local_settings::LocalSettings) {
        match self {
            Self::Edits(value) => {
                local
                    .tui
                    .custom
                    .get_or_insert_with(Default::default)
                    .edit_messages = value;
            }
            Self::Progress(value) => {
                local
                    .tui
                    .custom
                    .get_or_insert_with(Default::default)
                    .progress = Some(value);
                local.tui.progress_messages = Some(value != CustomProgressMode::Off);
            }
            Self::Checks(value) => {
                local.tui.custom.get_or_insert_with(Default::default).checks = Some(value)
            }
            Self::Recap(value) => local.tui.auto_recap = value,
            Self::Animations(value) => local.tui.animations = value,
            Self::Notifications(value) => {
                local.tui.notification_settings.notifications = Notifications::Enabled(value)
            }
            Self::Sound(value) => local.tui.notification_settings.method = value,
            Self::Activity(value) => {
                local
                    .tui
                    .custom
                    .get_or_insert_with(Default::default)
                    .live_activity = Some(value)
            }
            Self::ToolDetails(value) => {
                local
                    .tui
                    .custom
                    .get_or_insert_with(Default::default)
                    .compact_actions = Some(value)
            }
            _ => {}
        }
    }
}

fn link(name: &str, description: &str, page: CustomSettingsPage) -> SelectionItem {
    SelectionItem {
        name: name.into(),
        description: Some(description.into()),
        actions: vec![Box::new(move |tx| {
            tx.send(AppEvent::OpenCustomSettingsPage(page))
        })],
        dismiss_on_select: true,
        ..Default::default()
    }
}
fn choice(name: &str, description: &str, current: bool, setting: CustomSetting) -> SelectionItem {
    SelectionItem {
        name: name.into(),
        description: Some(description.into()),
        is_current: current,
        actions: vec![Box::new(move |tx| {
            tx.send(AppEvent::SaveCustomSetting(setting))
        })],
        dismiss_on_select: true,
        ..Default::default()
    }
}

impl ChatWidget {
    pub(crate) fn open_custom_settings(&mut self) {
        self.open_custom_settings_page(CustomSettingsPage::Root);
    }
    pub(crate) fn custom_live_activity_enabled(&self) -> bool {
        self.local_settings
            .tui
            .custom
            .as_ref()
            .and_then(|custom| custom.live_activity)
            .unwrap_or(true)
    }
    pub(crate) fn apply_custom_setting(&mut self, setting: CustomSetting) {
        setting.apply(&mut self.local_settings);
        match setting {
            CustomSetting::Answers(value) => self.config.model_verbosity = Some(value),
            CustomSetting::Output(value) => self.config.tool_output_token_limit = value,
            CustomSetting::Compaction(value) => self.config.model_auto_compact_token_limit = value,
            CustomSetting::Animations(value) => {
                self.bottom_pane.set_animations_enabled(value);
                self.bottom_pane.dismiss_composer_sparkle();
                self.refresh_model_display();
            }
            CustomSetting::Activity(_) => {
                self.status_state.live_activity.dismiss_header();
                self.restore_reasoning_status_header();
            }
            CustomSetting::Notifications(false) => self.pending_notification = None,
            _ => {}
        }
        self.request_redraw();
    }
    pub(crate) fn open_custom_settings_page(&mut self, page: CustomSettingsPage) {
        if let CustomSettingsPage::Command(command) = page {
            self.dispatch_command(command);
            return;
        }
        let custom = self.local_settings.tui.custom.clone().unwrap_or_default();
        let progress = custom.progress.unwrap_or(
            if self.local_settings.tui.progress_messages == Some(false) {
                CustomProgressMode::Off
            } else {
                CustomProgressMode::All
            },
        );
        let verbosity = self.config.model_verbosity.unwrap_or(Verbosity::Medium);
        let limit = self.config.tool_output_token_limit;
        let compaction = self.config.model_auto_compact_token_limit;
        let edits = custom.edit_messages;
        let compact_actions = custom.compact_actions.unwrap_or(true);
        let checks = custom.checks.unwrap_or(CustomCheckLevel::Normal);
        let notifications = match &self.local_settings.tui.notification_settings.notifications {
            Notifications::Enabled(value) => *value,
            Notifications::Custom(values) => !values.is_empty(),
        };
        let mut items = match page {
            CustomSettingsPage::Root => vec![
                link(&format!("Ответы · {}", match verbosity { Verbosity::Low => "короткие", Verbosity::Medium => "обычные", Verbosity::High => "подробные" }), "Короткие обычно требуют меньше токенов ответа.", CustomSettingsPage::Answers),
                link(&format!("Сообщения о работе · {}", match progress { CustomProgressMode::Off => "выключены", CustomProgressMode::Important => "важные этапы", CustomProgressMode::All => "все" }), "Выключение сокращает текст и его дальнейшее присутствие в контексте.", CustomSettingsPage::Progress),
                link(&format!("Сообщения о правках · {}", match edits { Some(CustomEditMessages::Brief) => "файл и цель", Some(CustomEditMessages::Diff) => "с diff", Some(CustomEditMessages::Off) => "выключены", None => "как в AGENTS.md" }), "Повторение diff в сообщениях добавляет текст в контекст.", CustomSettingsPage::Edits),
                link("Контекст", "Лимит вывода команд и порог сжатия истории.", CustomSettingsPage::Context),
                link(&format!("Проверки · {}", match checks { CustomCheckLevel::Necessary => "необходимые", CustomCheckLevel::Normal => "обычные", CustomCheckLevel::Extended => "расширенные" }), "Дополнительные проверки могут увеличить число вызовов и контекст.", CustomSettingsPage::Checks),
                link(&format!("Автосводки · {}", if self.local_settings.tui.auto_recap { "включены" } else { "выключены" }), "Каждая автоматическая сводка требует отдельного запроса модели.", CustomSettingsPage::Recap),
                link("Интерфейс", "Анимации, уведомления, звук, тема. На токены и контекст не влияют.", CustomSettingsPage::Interface),
                link("Строка статуса", "Поля и текущее действие. Формируется локально без запросов модели.", CustomSettingsPage::Status),
                link("Модель и reasoning", "Расход зависит от модели и сложности задачи; меню не тратит токены.", CustomSettingsPage::Command(SlashCommand::Model)),
                link("Permissions", "Права сами по себе не меняют токены. Влияют на доступные действия.", CustomSettingsPage::Command(SlashCommand::Permissions)),
            ],
            CustomSettingsPage::Answers => vec![
                choice("Короткие", "Обычно меньше токенов ответа и текста в последующем контексте.", verbosity == Verbosity::Low, CustomSetting::Answers(Verbosity::Low)),
                choice("Обычные", "Средний объём ответа; расход зависит от задачи.", verbosity == Verbosity::Medium, CustomSetting::Answers(Verbosity::Medium)),
                choice("Подробные", "Обычно больше токенов ответа и текста в последующем контексте.", verbosity == Verbosity::High, CustomSetting::Answers(Verbosity::High)),
            ],
            CustomSettingsPage::Progress => vec![
                choice("Выключены", "Меньше промежуточного текста и контекста. Итог, вопросы и запросы разрешений остаются.", progress == CustomProgressMode::Off, CustomSetting::Progress(CustomProgressMode::Off)),
                choice("Только важные этапы", "Меньше писанины, чем в режиме «все»; больше, чем при выключении.", progress == CustomProgressMode::Important, CustomSetting::Progress(CustomProgressMode::Important)),
                choice("Все", "Больше сообщений о работе; обычно больше токенов и контекста.", progress == CustomProgressMode::All, CustomSetting::Progress(CustomProgressMode::All)),
            ],
            CustomSettingsPage::Edits => vec![
                choice("Как в AGENTS.md", "Правила проекта определяют сообщения и diff; отдельная настройка не добавляется в контекст.", edits.is_none(), CustomSetting::Edits(None)),
                choice("Файл и цель · diff по запросу", "Короткое пояснение перед правкой. Патч инструмента не повторяется сообщением, меньше текста в контексте.", edits == Some(CustomEditMessages::Brief), CustomSetting::Edits(Some(CustomEditMessages::Brief))),
                choice("Показывать diff перед правкой", "Основные изменённые фрагменты повторяются сообщением; обычно больше текста в контексте.", edits == Some(CustomEditMessages::Diff), CustomSetting::Edits(Some(CustomEditMessages::Diff))),
                choice("Без сообщений перед правкой", "Меньше текста в контексте. Итог, проверки, вопросы и разрешения остаются.", edits == Some(CustomEditMessages::Off), CustomSetting::Edits(Some(CustomEditMessages::Off))),
            ],
            CustomSettingsPage::Context => vec![
                link(&format!("Вывод команд · {}", limit.map_or_else(|| "авто".to_owned(), |value| format!("до {value} токенов"))), "Меньший лимит оставляет меньше вывода в контексте; длинный собранный вывод доступен в файле.", CustomSettingsPage::Output),
                link(&format!("Сжатие истории · {}", compaction.map_or_else(|| "авто".to_owned(), |value| format!("{value} токенов"))), "Раннее сжатие сокращает дальнейший контекст, заменяя подробности сводкой. Само сжатие требует запроса модели.", CustomSettingsPage::Compaction),
            ],
            CustomSettingsPage::Output => {
                let mut values = vec![
                    choice("Авто", "Лимит вывода по настройкам модели; объём контекста зависит от результата команды.", limit.is_none(), CustomSetting::Output(None)),
                    choice("Краткий · до 2000 токенов", "Меньше вывода в контексте. Длинный собранный вывод сохраняется в файл для дочитывания.", limit == Some(2000), CustomSetting::Output(Some(2000))),
                    choice("Средний · до 5000 токенов", "Больше подробностей в контексте; превышение лимита сохраняется в файл.", limit == Some(5000), CustomSetting::Output(Some(5000))),
                    choice("Подробный · до 10000 токенов", "Больше вывода в контексте; превышение лимита сохраняется в файл.", limit == Some(10000), CustomSetting::Output(Some(10000))),
                ];
                if let Some(value) = limit && ![2000, 5000, 10000].contains(&value) {
                    values.push(choice(&format!("Текущий · до {value} токенов"), "Значение из config.toml; определяет объём вывода в контексте.", true, CustomSetting::Output(Some(value))));
                }
                values
            },
            CustomSettingsPage::Compaction => {
                let mut values = vec![choice("Авто", "Порог сжатия контекста выбирает модель. Удаляет пользовательский порог из config.toml.", compaction.is_none(), CustomSetting::Compaction(None))];
                for value in [100000, 200000, 400000] {
                    values.push(choice(&format!("При {value} токенов"), "При достижении порога история сжимается в сводку; дальнейший контекст становится короче.", compaction == Some(value), CustomSetting::Compaction(Some(value))));
                }
                if let Some(value) = compaction && ![100000, 200000, 400000].contains(&value) {
                    values.push(choice(&format!("Текущий · {value} токенов"), "Порог из config.toml; ограничен возможностями модели и её контекста.", true, CustomSetting::Compaction(Some(value))));
                }
                values
            },
            CustomSettingsPage::Checks => vec![
                choice("Только необходимые", "Обычно меньше вызовов инструментов и вывода в контексте. Обязательные проверки сохраняются.", checks == CustomCheckLevel::Necessary, CustomSetting::Checks(CustomCheckLevel::Necessary)),
                choice("Обычные", "Проверки по изменению и оправданные регрессии; расход зависит от задачи.", checks == CustomCheckLevel::Normal, CustomSetting::Checks(CustomCheckLevel::Normal)),
                choice("Расширенные", "Больше проверок; могут потребоваться дополнительные вызовы, токены и контекст.", checks == CustomCheckLevel::Extended, CustomSetting::Checks(CustomCheckLevel::Extended)),
            ],
            CustomSettingsPage::Recap => vec![
                choice("Выключены", "Меньше запросов и токенов: автоматические сводки не создаются. Ручная /recap остаётся.", !self.local_settings.tui.auto_recap, CustomSetting::Recap(false)),
                choice("Включены", "Дополнительные запросы и токены для сводок. Это не автоматическая очистка контекста.", self.local_settings.tui.auto_recap, CustomSetting::Recap(true)),
            ],
            CustomSettingsPage::Interface => vec![
                link(&format!("Действия инструментов · {}", if compact_actions { "компактно" } else { "подробно" }), "Меняет отображение команд и diff в CLI. На токены и контекст не влияет.", CustomSettingsPage::ToolDetails),
                link(&format!("Анимации · {}", if self.local_settings.tui.animations { "включены" } else { "выключены" }), "Токены и контекст: не влияют.", CustomSettingsPage::Animations),
                link(&format!("Уведомления · {}", if notifications { "включены" } else { "выключены" }), "Токены и контекст: не влияют.", CustomSettingsPage::Notifications),
                link("Звук и способ уведомлений", "Зависит от поддержки терминала. Токены и контекст: не влияют.", CustomSettingsPage::Sound),
                link("Тема", "Токены и контекст: не влияет.", CustomSettingsPage::Command(SlashCommand::Theme)),
            ],
            CustomSettingsPage::Sound => [
                (NotificationMethod::Auto, "Авто", "Терминал выбирает доступный способ. На токены и контекст не влияет."),
                (NotificationMethod::Bel, "Звуковой сигнал", "Если терминал поддерживает звук. Нужны включённые уведомления. На токены и контекст не влияет."),
                (NotificationMethod::Osc9, "Системные уведомления", "Без терминального сигнала; звук ОС зависит от её настроек. На токены и контекст не влияет."),
            ].into_iter().map(|(value,name,desc)| choice(name,desc,self.local_settings.tui.notification_settings.method == value, CustomSetting::Sound(value))).collect(),
            CustomSettingsPage::Status => vec![
                link(&format!("Текущее действие · {}", if self.custom_live_activity_enabled() { "включено" } else { "выключено" }), "Локальная строка. На токены и контекст не влияет.", CustomSettingsPage::Activity),
                link("Выбрать поля", "Модель, permissions, контекст и другие поля. На токены и контекст не влияет.", CustomSettingsPage::Command(SlashCommand::Statusline)),
            ],
            CustomSettingsPage::ToolDetails => vec![
                choice("Компактно", "Показывает действие и путь; команда, вывод и diff раскрываются по запросу. На контекст не влияет.", compact_actions, CustomSetting::ToolDetails(true)),
                choice("Подробно", "Команды, вывод и diff сразу раскрыты; отдельные действия можно свернуть. На контекст не влияет.", !compact_actions, CustomSetting::ToolDetails(false)),
            ],
            CustomSettingsPage::Animations | CustomSettingsPage::Notifications | CustomSettingsPage::Activity => {
                let current = match page { CustomSettingsPage::Animations => self.local_settings.tui.animations, CustomSettingsPage::Notifications => notifications, _ => custom.live_activity.unwrap_or(true) };
                [true,false].into_iter().map(|value| choice(if value { "Включены" } else { "Выключены" }, "Токены и контекст: не влияют.", current == value, match page { CustomSettingsPage::Animations => CustomSetting::Animations(value), CustomSettingsPage::Notifications => CustomSetting::Notifications(value), _ => CustomSetting::Activity(value) })).collect()
            }
            CustomSettingsPage::Command(_) => unreachable!(),
        };
        let parent = match page {
            CustomSettingsPage::Animations
            | CustomSettingsPage::Notifications
            | CustomSettingsPage::Sound => CustomSettingsPage::Interface,
            CustomSettingsPage::ToolDetails => CustomSettingsPage::Interface,
            CustomSettingsPage::Output | CustomSettingsPage::Compaction => {
                CustomSettingsPage::Context
            }
            CustomSettingsPage::Activity => CustomSettingsPage::Status,
            _ => CustomSettingsPage::Root,
        };
        if page != CustomSettingsPage::Root {
            items.push(link(
                "← Назад",
                "Открытие меню не расходует токены.",
                parent,
            ));
        }
        let title = match page {
            CustomSettingsPage::Root => "Settings",
            CustomSettingsPage::Answers => "Ответы",
            CustomSettingsPage::Progress => "Сообщения о работе",
            CustomSettingsPage::Edits => "Сообщения о правках",
            CustomSettingsPage::Context => "Контекст",
            CustomSettingsPage::Output => "Вывод команд",
            CustomSettingsPage::Compaction => "Сжатие истории",
            CustomSettingsPage::Checks => "Проверки",
            CustomSettingsPage::Recap => "Автосводки",
            CustomSettingsPage::Interface => "Интерфейс",
            CustomSettingsPage::Animations => "Анимации",
            CustomSettingsPage::Notifications => "Уведомления",
            CustomSettingsPage::Sound => "Звук и уведомления",
            CustomSettingsPage::Status => "Строка статуса",
            CustomSettingsPage::Activity => "Текущее действие",
            CustomSettingsPage::ToolDetails => "Действия инструментов",
            _ => "Settings",
        };
        self.bottom_pane.show_selection_view(SelectionViewParams {
            title: Some(title.into()), subtitle: Some("Сохраняется между запусками. Расход указан относительно других вариантов, без гарантии точного числа токенов.".into()), items, mouse_enabled: true,
            on_cancel: (page != CustomSettingsPage::Root).then(|| Box::new(move |tx: &AppEventSender| tx.send(AppEvent::OpenCustomSettingsPage(parent))) as Box<dyn Fn(&AppEventSender) + Send + Sync>),
            ..SelectionViewParams::picker()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_config::types::CustomTuiPreferences;
    #[test]
    fn custom_settings_choices_emit_typed_values_and_preserve_legacy_quiet_flag() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let tx = AppEventSender::new(tx);
        for mode in [
            CustomProgressMode::Off,
            CustomProgressMode::Important,
            CustomProgressMode::All,
        ] {
            let setting = CustomSetting::Progress(mode);
            let item = choice("test", "usage", false, setting);
            item.actions[0](&tx);
            assert!(
                matches!(rx.try_recv().unwrap(),AppEvent::SaveCustomSetting(value) if value == setting)
            );
            assert_eq!(setting.edits().len(), 2);
        }
    }

    #[tokio::test]
    async fn context_and_edit_choices_persist_reset_and_keep_unrelated_preferences() {
        use crate::legacy_core::config::edit::ConfigEditsBuilder;
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("config.toml");
        std::fs::write(
            &path,
            "model = 'test-model'\n[tui.custom]\nprogress = 'off'\nlive_activity = false\n",
        )
        .unwrap();
        let settings = [
            CustomSetting::Edits(Some(CustomEditMessages::Brief)),
            CustomSetting::Output(Some(2000)),
            CustomSetting::Compaction(Some(200000)),
            CustomSetting::ToolDetails(false),
        ];
        ConfigEditsBuilder::for_config_path(&path)
            .with_edits(settings.into_iter().flat_map(CustomSetting::edits))
            .apply()
            .await
            .unwrap();
        let config: codex_config::config_toml::ConfigToml =
            toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(config.tool_output_token_limit, Some(2000));
        assert_eq!(config.model_auto_compact_token_limit, Some(200000));
        let custom = config.tui.unwrap().custom.unwrap();
        assert_eq!(custom.edit_messages, Some(CustomEditMessages::Brief));
        assert_eq!(custom.compact_actions, Some(false));
        assert_eq!(custom.progress, Some(CustomProgressMode::Off));
        assert_eq!(custom.live_activity, Some(false));
        let resets = [
            CustomSetting::Edits(None),
            CustomSetting::Output(None),
            CustomSetting::Compaction(None),
        ];
        ConfigEditsBuilder::for_config_path(&path)
            .with_edits(resets.into_iter().flat_map(CustomSetting::edits))
            .apply()
            .await
            .unwrap();
        let reset: codex_config::config_toml::ConfigToml =
            toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(reset.tool_output_token_limit, None);
        assert_eq!(reset.model_auto_compact_token_limit, None);
        assert_eq!(reset.model.as_deref(), Some("test-model"));
        let custom = reset.tui.unwrap().custom.unwrap();
        assert_eq!(custom.edit_messages, None);
        assert_eq!(custom.progress, Some(CustomProgressMode::Off));
    }
    #[tokio::test]
    async fn custom_settings_root_and_all_pages_offer_mouse_navigation_and_cost_descriptions() {
        let (mut chat, _sender, _events, _ops) =
            super::super::tests::make_chatwidget_manual_with_sender().await;
        chat.config.tool_output_token_limit = Some(3500);
        chat.config.model_auto_compact_token_limit = Some(850000);
        for page in [
            CustomSettingsPage::Root,
            CustomSettingsPage::Answers,
            CustomSettingsPage::Progress,
            CustomSettingsPage::Edits,
            CustomSettingsPage::Context,
            CustomSettingsPage::Output,
            CustomSettingsPage::Compaction,
            CustomSettingsPage::Checks,
            CustomSettingsPage::Recap,
            CustomSettingsPage::Interface,
            CustomSettingsPage::Animations,
            CustomSettingsPage::Notifications,
            CustomSettingsPage::Sound,
            CustomSettingsPage::Status,
            CustomSettingsPage::Activity,
            CustomSettingsPage::ToolDetails,
        ] {
            chat.open_custom_settings_page(page);
            assert!(!chat.no_modal_or_popup_active());
            let area = Rect::new(0, 0, 140, 35);
            let mut buffer = ratatui::buffer::Buffer::empty(area);
            crate::render::renderable::Renderable::render(&chat.bottom_pane, area, &mut buffer);
            let screen: String = (0..area.height)
                .flat_map(|y| (0..area.width).map(move |x| (x, y)))
                .map(|(x, y)| buffer[(x, y)].symbol())
                .collect();
            assert!(
                screen.contains("токен") || screen.contains("Токен") || screen.contains("контекст"),
                "{page:?}: {screen}"
            );
            if page == CustomSettingsPage::Output {
                assert!(screen.contains("Текущий · до 3500 токенов"));
            }
            if page == CustomSettingsPage::Compaction {
                assert!(screen.contains("Текущий · 850000 токенов"));
            }
            chat.bottom_pane
                .handle_key_event(crossterm::event::KeyCode::Esc.into());
        }
        let custom = CustomTuiPreferences {
            live_activity: Some(false),
            ..Default::default()
        };
        chat.local_settings.tui.custom = Some(custom);
        assert!(!chat.custom_live_activity_enabled());
        chat.apply_custom_setting(CustomSetting::Edits(Some(CustomEditMessages::Diff)));
        chat.apply_custom_setting(CustomSetting::Compaction(Some(200000)));
        chat.apply_custom_setting(CustomSetting::Output(Some(2000)));
        chat.apply_custom_setting(CustomSetting::ToolDetails(false));
        assert_eq!(chat.config.model_auto_compact_token_limit, Some(200000));
        assert_eq!(chat.config.tool_output_token_limit, Some(2000));
        let custom = chat.local_settings.tui.custom.as_ref().unwrap();
        assert_eq!(custom.edit_messages, Some(CustomEditMessages::Diff));
        assert_eq!(custom.compact_actions, Some(false));
    }
}
