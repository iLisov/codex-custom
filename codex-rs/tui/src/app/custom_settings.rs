use super::*;
use crate::chatwidget::custom_settings::CustomSetting;

impl App {
    pub(super) async fn save_custom_setting(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
        setting: CustomSetting,
    ) {
        match ConfigEditsBuilder::for_config_path(self.local_settings.user_config_path.as_path())
            .with_edits(setting.edits())
            .apply()
            .await
        {
            Ok(()) => {
                setting.apply(&mut self.local_settings);
                match setting {
                    CustomSetting::Answers(value) => self.config.model_verbosity = Some(value),
                    CustomSetting::Output(value) => {
                        self.config.tool_output_token_limit = Some(value)
                    }
                    _ => {}
                }
                self.chat_widget.apply_custom_setting(setting);
                if matches!(setting, CustomSetting::Recap(false)) {
                    self.recap.stop_automatic_requests();
                }
                tui.set_notification_settings(
                    self.local_settings.tui.notification_settings.method,
                    self.local_settings.tui.notification_settings.condition,
                );
                if let Err(err) = app_server.reload_user_config().await {
                    self.chat_widget.add_error_message(format!("Сохранено, но обновить сессию не удалось: {err}. Перезапустите Codex Custom."));
                }
                self.chat_widget.open_custom_settings_page(setting.page());
            }
            Err(err) => self
                .chat_widget
                .add_error_message(format!("Не удалось сохранить настройку: {err}")),
        }
        tui.frame_requester().schedule_frame();
    }
}
