use crate::App;

impl App {
    pub(crate) fn poll_surge_start_delay(&mut self, ctx: &eframe::egui::Context) {
        if let Ok(config) = &self.config_path {
            if let Some(error) = self.surge_start_delay.poll(config, ctx) {
                self.log_error(&error);
            }
        }
        if self.surge_start_delay.take_changed() {
            self.save_session();
        }
    }
}
