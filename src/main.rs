#![windows_subsystem = "windows"]

mod i18n;
mod sysinfo;

use std::os::windows::process::CommandExt;
use std::process::{Command, Output};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText};
use i18n::{Lang, Texts};
use sysinfo::{BatteryInfo, DiskInfo, SystemInfo};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const AUTO_REFRESH: Duration = Duration::from_secs(300);
const WINDOW_WIDTH: f32 = 920.0;

/// Windows `shutdown /t` supports a maximum of 10 years (315360000 s).
const MAX_SECONDS: u64 = 315_360_000;
const MAX_HOURS: u32 = 8_760; // 365 days

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Shutdown,
    Restart,
}

impl Action {
    fn windows_flag(self) -> &'static str {
        match self {
            Self::Shutdown => "/s",
            Self::Restart => "/r",
        }
    }

    fn label(self, t: Texts) -> &'static str {
        match self {
            Self::Shutdown => t.shutdown_label,
            Self::Restart => t.restart_label,
        }
    }
}

#[derive(Clone, Copy)]
struct Schedule {
    action: Action,
    finish_at: Instant,
}

#[derive(Clone, Copy)]
enum StatusKind {
    Info,
    Success,
    Error,
}

struct RustTimeApp {
    lang: Lang,
    hours: u32,
    minutes: u32,
    action: Action,
    disabled: bool,
    schedule: Option<Schedule>,
    status: String,
    status_kind: StatusKind,
    system: SystemInfo,
    sys_loading: bool,
    sys_rx: Option<Receiver<SystemInfo>>,
    last_sys_refresh: Option<Instant>,
}

impl Default for RustTimeApp {
    fn default() -> Self {
        let lang = Lang::detect();
        Self {
            lang,
            hours: 0,
            minutes: 30,
            action: Action::Shutdown,
            disabled: false,
            schedule: None,
            status: lang.texts().initial_status.to_owned(),
            status_kind: StatusKind::Info,
            system: SystemInfo::default(),
            sys_loading: false,
            sys_rx: None,
            last_sys_refresh: None,
        }
    }
}

impl RustTimeApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.refresh_system();
        app
    }

    fn total_seconds(&self) -> u64 {
        self.hours as u64 * 3_600 + self.minutes as u64 * 60
    }

    fn set_status(&mut self, text: impl Into<String>, kind: StatusKind) {
        self.status = text.into();
        self.status_kind = kind;
    }

    fn refresh_system(&mut self) {
        if self.sys_loading {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.sys_rx = Some(receiver);
        self.sys_loading = true;
        self.last_sys_refresh = Some(Instant::now());
        let t = self.lang.texts();
        thread::spawn(move || {
            let _ = sender.send(sysinfo::collect(t));
        });
    }

    fn poll_system(&mut self) {
        let received = self
            .sys_rx
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(info) = received {
            self.system = info;
            self.sys_loading = false;
            self.sys_rx = None;
        }
        if self
            .last_sys_refresh
            .is_none_or(|moment| moment.elapsed() >= AUTO_REFRESH)
        {
            self.refresh_system();
        }
    }

    fn schedule_action(&mut self) {
        let t = self.lang.texts();
        if self.disabled {
            self.set_status(t.auto_disabled_error, StatusKind::Error);
            return;
        }

        let seconds = self.total_seconds();
        if seconds == 0 {
            self.set_status(t.need_one_minute, StatusKind::Error);
            return;
        }
        if seconds > MAX_SECONDS {
            self.set_status(t.max_ten_years, StatusKind::Error);
            return;
        }

        let seconds_text = seconds.to_string();
        let flag = self.action.windows_flag();
        let result = Command::new("shutdown")
            .args([flag, "/t", seconds_text.as_str()])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        match result {
            Ok(output) if output.status.success() => {
                self.schedule = Some(Schedule {
                    action: self.action,
                    finish_at: Instant::now() + Duration::from_secs(seconds),
                });
                self.set_status(
                    format!(
                        "{} {} {}.",
                        self.action.label(t),
                        t.scheduled_mid,
                        human_duration(seconds)
                    ),
                    StatusKind::Success,
                );
            }
            Ok(output) => {
                self.set_status(
                    format!(
                        "{}{}",
                        t.windows_failed_prefix,
                        command_message(&output, t)
                    ),
                    StatusKind::Error,
                );
            }
            Err(error) => {
                self.set_status(
                    format!("{}{error}", t.shutdown_failed_prefix),
                    StatusKind::Error,
                );
            }
        }
    }

    /// Cancels the pending shutdown/restart (`shutdown /a`).
    fn cancel_action(&mut self) {
        let t = self.lang.texts();
        match Command::new("shutdown")
            .arg("/a")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(output) if output.status.success() => {
                self.schedule = None;
                self.set_status(t.cancel_ok, StatusKind::Success);
            }
            Ok(output) => {
                self.schedule = None;
                self.set_status(
                    format!(
                        "{}{}",
                        t.cancel_failed_prefix,
                        command_message(&output, t)
                    ),
                    StatusKind::Info,
                );
            }
            Err(error) => {
                self.set_status(
                    format!("{}{error}", t.cancel_exec_failed_prefix),
                    StatusKind::Error,
                );
            }
        }
    }

    fn restart_elevated(&mut self) {
        let t = self.lang.texts();
        let executable = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                self.set_status(
                    format!("{}{error}", t.exe_not_found_prefix),
                    StatusKind::Error,
                );
                return;
            }
        };
        let escaped = executable.to_string_lossy().replace('\'', "''");
        let script = format!(
            "$ErrorActionPreference='Stop'; try {{ Start-Process -FilePath '{escaped}' -Verb RunAs -ErrorAction Stop | Out-Null; exit 0 }} catch {{ exit 1 }}"
        );
        let result = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        match result {
            Ok(output) if output.status.success() => std::process::exit(0),
            _ => self.set_status(t.elevate_failed, StatusKind::Error),
        }
    }
}

fn command_message(output: &Output, t: Texts) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let detail = match (stderr.is_empty(), stdout.is_empty()) {
        (false, false) => format!("{stderr} ({stdout})"),
        (false, true) => stderr,
        (true, false) => stdout,
        (true, true) => format!("{}{:?}", t.exit_code_prefix, output.status.code()),
    };

    if detail.chars().count() > 160 {
        format!("{}…", detail.chars().take(160).collect::<String>())
    } else {
        detail
    }
}

fn human_duration(seconds: u64) -> String {
    let hours = seconds / 3_600;
    let minutes = (seconds % 3_600) / 60;
    match (hours, minutes) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

fn clock(duration: Duration) -> String {
    let total = duration.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        total / 3_600,
        (total % 3_600) / 60,
        total % 60
    )
}

/// Fixed-width row that the centered layout places in the middle of the window.
fn centered_row(ui: &mut egui::Ui, width: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 0.0),
        egui::Layout::left_to_right(egui::Align::Center),
        add_contents,
    );
}

fn system_heading(ui: &mut egui::Ui, title: &str) {
    ui.heading(title);
}

fn computer_section(ui: &mut egui::Ui, info: &SystemInfo, t: Texts) {
    if let Some(manufacturer) = &info.manufacturer {
        if !manufacturer.is_empty() {
            ui.label(format!("{}{}", t.manufacturer_prefix, manufacturer));
        }
    }
    if let Some(model) = &info.model {
        if !model.is_empty() {
            ui.label(format!("{}{}", t.model_prefix, model));
        }
    }
    if let Some(date) = &info.bios_date {
        ui.label(format!("{}{}", t.bios_date_prefix, date));
    }
    if let Some(date) = &info.baseboard_date {
        ui.label(format!("{}{}", t.baseboard_date_prefix, date));
    }
}

fn disk_section(
    ui: &mut egui::Ui,
    disks: &[DiskInfo],
    info: &SystemInfo,
    app: &mut RustTimeApp,
    t: Texts,
) {
    ui.label(RichText::new(t.power_hours_caption).small().weak());

    if app.sys_loading && !info.loaded {
        ui.add_space(6.0);
        ui.spinner();
        ui.label(t.querying);
    } else if disks.is_empty() {
        ui.label(t.no_disks);
    } else {
        for disk in disks {
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(&disk.name).strong());
                    ui.label(
                        RichText::new(format!(
                            "{} · {:.0} GB",
                            disk.media, disk.size_gb
                        ))
                        .small()
                        .weak(),
                    );
                    match disk.power_on_hours {
                        Some(hours) => {
                            ui.label(format!(
                                "{}{}",
                                format_number(hours, t),
                                t.power_hours_suffix
                            ));
                        }
                        None => {
                            ui.label(
                                RichText::new(t.hours_unavailable)
                                    .small()
                                    .color(Color32::from_rgb(255, 190, 110)),
                            );
                        }
                    }
                });
            });
        }
    }

    if let Some(error) = &info.disk_error {
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("{}{}", t.disks_error_prefix, error))
                .small()
                .weak(),
        );
    }

    if disks.iter().any(|disk| disk.power_on_hours.is_none()) && !info.elevated {
        ui.add_space(6.0);
        ui.label(RichText::new(t.admin_hint).small().weak());
        if ui.small_button(t.restart_admin).clicked() {
            app.restart_elevated();
        }
    }
}

fn battery_section(ui: &mut egui::Ui, batteries: &[BatteryInfo], info: &SystemInfo, t: Texts) {
    if batteries.is_empty() {
        ui.label(t.no_battery);
    } else {
        for battery in batteries {
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(&battery.name).strong());
                    match battery.cycles {
                        Some(cycles) => {
                            ui.label(format!(
                                "{}{}",
                                format_number(cycles as u64, t),
                                t.cycles_suffix
                            ));
                        }
                        None => {
                            ui.label(RichText::new(t.cycles_unavailable).small().weak());
                        }
                    }
                    if let Some(charge) = battery.charge_percent {
                        ui.label(format!("{}{charge} %", t.charge_prefix));
                    }
                    if let (Some(full), Some(design)) =
                        (battery.full_capacity, battery.design_capacity)
                    {
                        if design > 0 {
                            let health = full as f64 / design as f64 * 100.0;
                            ui.label(
                                RichText::new(format!(
                                    "{}{health:.0} % ({} / {} mWh)",
                                    t.health_prefix,
                                    format_number(full, t),
                                    format_number(design, t)
                                ))
                                .small()
                                .weak(),
                            );
                        }
                    }
                });
            });
        }
    }

    if let Some(error) = &info.battery_error {
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("{}{}", t.battery_error_prefix, error))
                .small()
                .weak(),
        );
    }
}

fn format_number(value: u64, t: Texts) -> String {
    let text = value.to_string();
    let mut result = String::new();
    for (index, character) in text.chars().enumerate() {
        if index > 0 && (text.len() - index) % 3 == 0 {
            result.push(t.thousands_separator);
        }
        result.push(character);
    }
    result
}

impl eframe::App for RustTimeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.schedule.is_some() {
            ui.ctx().request_repaint_after(Duration::from_secs(1));
        }
        self.poll_system();

        let info = self.system.clone();
        let status = self.status.clone();
        let status_kind = self.status_kind;
        let schedule = self.schedule;
        let t = self.lang.texts();

        let mut content_height = None;
        let panel_frame = egui::Frame::central_panel(ui.style())
            .inner_margin(egui::Margin::symmetric(18, 4));

        egui::CentralPanel::default().frame(panel_frame).show(ui, |ui| {
            let start_y = ui.cursor().top();
                    ui.vertical_centered(|ui| {
                    ui.add_space(6.0);
                    ui.heading(RichText::new("RustTime").size(24.0).strong());
                    ui.label(t.app_subtitle);
                    ui.add_space(10.0);
                });

                ui.horizontal_top(|ui| {
                    let right_width = (ui.available_width() - 390.0).max(260.0);

                    // Left column: shutdown and restart.
                    ui.allocate_ui_with_layout(
                        egui::vec2(370.0, 0.0),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.heading(t.shutdown_restart);
                            ui.add_space(6.0);

                            centered_row(ui, 220.0, |ui| {
                                ui.label(t.hours_label);
                                ui.add(egui::DragValue::new(&mut self.hours).range(0..=MAX_HOURS));
                                ui.add_space(10.0);
                                ui.label(t.minutes_label);
                                ui.add(egui::DragValue::new(&mut self.minutes).range(0..=59));
                            });

                            ui.add_space(6.0);
                            centered_row(ui, 290.0, |ui| {
                                ui.radio_value(
                                    &mut self.action,
                                    Action::Shutdown,
                                    t.shutdown_option,
                                );
                                ui.radio_value(
                                    &mut self.action,
                                    Action::Restart,
                                    t.restart_option,
                                );
                            });

                            ui.add_space(6.0);
                            ui.label(format!(
                                "{}{}",
                                t.duration_prefix,
                                human_duration(self.total_seconds())
                            ));

                            ui.add_space(6.0);
                            let mut disabled = self.disabled;
                            if ui
                                .checkbox(&mut disabled, t.disable_auto)
                                .changed()
                            {
                                self.disabled = disabled;
                                if disabled {
                                    self.cancel_action();
                                } else {
                                    self.set_status(t.auto_enabled_again, StatusKind::Info);
                                }
                            }

                            ui.add_space(6.0);
                            centered_row(ui, 280.0, |ui| {
                                if ui
                                    .add_enabled(!self.disabled, egui::Button::new(t.schedule_button))
                                    .clicked()
                                {
                                    self.schedule_action();
                                }
                                if ui.button(t.cancel_button).clicked() {
                                    self.cancel_action();
                                }
                            });

                            ui.add_space(8.0);
                            let color = match status_kind {
                                StatusKind::Info => Color32::from_gray(180),
                                StatusKind::Success => Color32::from_rgb(120, 220, 140),
                                StatusKind::Error => Color32::from_rgb(255, 130, 130),
                            };
                            ui.label(RichText::new(&status).color(color));

                            if let Some(schedule) = schedule {
                                let remaining =
                                    schedule.finish_at.saturating_duration_since(Instant::now());
                                if !remaining.is_zero() {
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new(format!(
                                            "{}{}{}",
                                            schedule.action.label(t),
                                            t.pending_mid,
                                            clock(remaining)
                                        ))
                                        .strong()
                                        .color(Color32::from_rgb(255, 200, 90)),
                                    );
                                }
                            }
                                });
                            });

                            ui.add_space(6.0);
                            ui.label(RichText::new(t.manufacturing_note).small().weak());
                        },
                    );

                    ui.add_space(16.0);

                    // Right column: computer information.
                    ui.allocate_ui_with_layout(
                        egui::vec2(right_width, 0.0),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    system_heading(ui, t.computer);
                                    computer_section(ui, &info, t);
                                });
                            });

                            ui.add_space(10.0);
                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    system_heading(ui, t.hard_drives);
                                    disk_section(ui, &info.disks, &info, self, t);
                                });
                            });

                            ui.add_space(10.0);
                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.heading(t.battery);
                                    battery_section(ui, &info.batteries, &info, t);
                                });
                            });
                        },
                    );
                });

                ui.vertical_centered(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(t.footer)
                        .small()
                        .weak(),
                    );
                    ui.add_space(4.0);
                    ui.label(RichText::new(t.credits).small().weak());
                    ui.add_space(6.0);
                });

            content_height = Some(ui.cursor().top() - start_y);
        });

        // Dynamic height: the window fits the actual content (with a small margin).
        if let Some(height) = content_height {
            let target = (height + 6.0).clamp(140.0, 900.0);
            if (ui.ctx().viewport_rect().height() - target).abs() > 1.0 {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    WINDOW_WIDTH,
                    target,
                )));
            }
        }
    }
}

/// Window icon, taken from `design/Icon.png` and embedded in the executable.
fn window_icon() -> Arc<egui::IconData> {
    let bytes = include_bytes!("../design/Icon.png");
    let decoded = image::load_from_memory(bytes).expect("Icon.png is not a valid image");
    let icon = decoded
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    Arc::new(egui::IconData {
        rgba: icon.into_raw(),
        width: 256,
        height: 256,
    })
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, 470.0])
            .with_min_inner_size([WINDOW_WIDTH, 140.0])
            .with_maximized(false)
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(window_icon()),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "RustTime",
        options,
        Box::new(|cc| Ok(Box::new(RustTimeApp::new(cc)))),
    )
}
