use crate::{PentaKuru, integrations, system, ui_theme as theme};
use eframe::egui;

impl PentaKuru {
    pub(super) fn integrations_settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("DeepTutor · Music · Clip",|ui|{
            ui.checkbox(&mut self.prefs.integrations.deeptutor,"Bật DeepTutor · HKUDS");
            ui.checkbox(&mut self.prefs.integrations.shared_ai,"DeepTutor dùng API key/model AI của PentaKuru");
            theme::caption(ui,"Bấm Cài đặt / mở: tự chuẩn bị Python/Node còn thiếu và DeepTutor. Dùng tài liệu để hỏi đáp, học và tạo bài tập; RAG cần cấu hình embedding trong DeepTutor.");
            ui.horizontal(|ui|{
                if ui.add_enabled(self.integrations.job.is_none(),egui::Button::new(if self.integrations.child.is_some(){"Mở DeepTutor"}else{"Cài đặt / mở DeepTutor"})).clicked(){
                    self.prefs.integrations.deeptutor=true;
                    self.integrations.seen_tutor=true;
                    self.open_tutor(ui.ctx());
                }
                if ui.add_enabled(self.prefs.integrations.deeptutor&&self.integrations.job.is_none(),egui::Button::new("Kết nối lại / đồng bộ AI")).clicked(){self.restart_tutor(ui.ctx());}
            });
            ui.separator();
            ui.checkbox(&mut self.prefs.integrations.music,"Bật Penta Music · khung nhạc nhỏ");
            theme::caption(ui,"Đăng nhập, playlist và tìm nhạc trên chính trang Penta Music. Khung web tối ưu cho màn hình nhỏ.");
            if ui.add_enabled(self.prefs.integrations.music,egui::Button::new("Mở Music")).clicked() && let Err(e)=integrations::music_open(){self.integrations.status=e;}
            ui.checkbox(&mut self.prefs.integrations.clip,"Bật Penta Clip · video");
            if ui.add_enabled(self.prefs.integrations.clip,egui::Button::new("Mở Clip")).clicked()&& let Err(e)=integrations::clip_open(){self.integrations.status=e;}
            if self.integrations.job.is_some(){ui.spinner();let stage=self.integrations.progress.lock().map(|p|p.clone()).unwrap_or_default();theme::caption(ui,&stage);if ui.button("Dừng cài đặt / khởi động").clicked(){self.prefs.integrations.deeptutor=false;self.integrations.cancel.store(true,std::sync::atomic::Ordering::Relaxed);}}
            theme::caption(ui,"Chương trình chạy trên máy này. Cần Internet cho lần cài đầu và khi dùng API AI trực tuyến.");
            if !self.integrations.status.is_empty(){theme::caption(ui,&self.integrations.status);}
        });
    }
    pub(super) fn open_tutor(&mut self, ctx: &egui::Context) {
        self.prefs.integrations.deeptutor = true;
        self.integrations.seen_tutor = true;
        self.integrations.tutor_status_open = true;
        if let Some(child) = &mut self.integrations.child {
            if child.0.try_wait().ok().flatten().is_none() {
                match integrations::tutor_open(child.2) {
                    Ok(()) => self.integrations.tutor_status_open = false,
                    Err(error) => self.integrations.status = error,
                }
                return;
            }
            self.integrations.child = None;
        }
        self.restart_tutor(ctx);
    }
    pub(super) fn tutor_status_window(&mut self, ctx: &egui::Context) {
        if !self.integrations.tutor_status_open {
            return;
        }
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("deeptutor-status"),
            egui::ViewportBuilder::default().with_title("PentaKuru · DeepTutor")
                .with_inner_size([380.0, 180.0]).with_min_inner_size([320.0, 160.0]),
            |ctx, _| {
                if ctx.input(|i| i.viewport().close_requested()) { self.integrations.tutor_status_open = false; }
                egui::CentralPanel::default().frame(egui::Frame::new().fill(theme::PANEL).inner_margin(12)).show(ctx, |ui| {
                    if self.integrations.job.is_some() {
                        ui.horizontal(|ui| { ui.spinner(); ui.label("Đang chuẩn bị DeepTutor…"); });
                        let stage = self.integrations.progress.lock().map(|p| p.clone()).unwrap_or_default();
                        ui.label(stage);
                        theme::caption(ui, "Lần cài đầu cần tải các thư viện. Anh có thể tiếp tục dùng chức năng khác.");
                        if ui.button("Dừng").clicked() {
                            self.prefs.integrations.deeptutor = false;
                            self.integrations.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    } else {
                        ui.label(&self.integrations.status);
                        if ui.button("Thử mở lại").clicked() { self.open_tutor(ui.ctx()); }
                    }
                });
            },
        );
    }
    pub(super) fn restart_tutor(&mut self, ctx: &egui::Context) {
        self.integrations.tutor_status_open = true;
        if self.integrations.job.is_some() {
            return;
        }
        if let Some(mut child) = self.integrations.child.take() {
            integrations::stop_child(&mut child.0);
        }
        let ai = self.prefs.ai.clone();
        let shared = self.prefs.integrations.shared_ai;
        let cancel = self.integrations.cancel.clone();
        let progress = self.integrations.progress.clone();
        cancel.store(false, std::sync::atomic::Ordering::Relaxed);
        self.integrations.status = "Đang tải/khởi động DeepTutor…".into();
        self.integrations.job = Some(system::Job::spawn(ctx, move || {
            integrations::start_tutor(&ai, shared, &cancel, &progress)
        }));
    }
    pub(super) fn poll_integrations(&mut self, ctx: &egui::Context) {
        if self.smoke_test {
            return;
        }
        let settings = &self.prefs.integrations;
        if settings.music != self.integrations.seen_music {
            self.integrations.seen_music = settings.music;
            let result = if settings.music {
                Ok(())
            } else {
                crate::sally::launcher::close_web("penta-music")
            };
            if let Err(error) = result {
                self.integrations.status = error;
            }
        }
        if settings.clip != self.integrations.seen_clip {
            self.integrations.seen_clip = settings.clip;
            let result = if settings.clip {
                Ok(())
            } else {
                crate::sally::launcher::close_web("penta-clip")
            };
            if let Err(error) = result {
                self.integrations.status = error;
            }
        }
        if settings.deeptutor != self.integrations.seen_tutor {
            self.integrations.seen_tutor = settings.deeptutor;
            if !settings.deeptutor {
                self.integrations
                    .cancel
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                let _ = crate::sally::launcher::close_web("penta-tutor");
                if let Some(mut child) = self.integrations.child.take() {
                    integrations::stop_child(&mut child.0);
                }
            }
        }
        if let Some(result) = system::take(&mut self.integrations.job) {
            match result {
                Ok(mut child) => {
                    if self.prefs.integrations.deeptutor {
                        let synced = child.1;
                        let opened = integrations::tutor_open(child.2);
                        self.integrations.child = Some(child);
                        self.integrations.tutor_status_open = opened.is_err();
                        self.integrations.status = opened.err().unwrap_or_else(|| {
                            if synced {
                                "DeepTutor sẵn sàng · đã đồng bộ AI PentaKuru.".into()
                            } else {
                                "DeepTutor đã mở. Chọn API/model trong Settings nếu cần.".into()
                            }
                        });
                    } else {
                        integrations::stop_child(&mut child.0);
                    }
                }
                Err(error) => {
                    self.integrations.status = error;
                    self.integrations.tutor_status_open = true;
                }
            }
        }
        if self.integrations.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }
}
