#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod storage;
mod timer;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use storage::Config;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use timer::{Mode, Session, Timer, View};

struct Inner {
    timer: Timer,
    config: Config,
    config_path: PathBuf,
    /// Set once the final session is written, so a second quit cannot append it again.
    saved_for_exit: bool,
}

impl Inner {
    fn threshold(&self) -> u64 {
        match self.timer.mode() {
            Mode::Work => self.config.work_threshold_s,
            Mode::Break => self.config.break_threshold_s,
        }
    }

    /// Append the session unless it is shorter than the configured minimum.
    fn save(&self, session: &Session) -> Result<(), String> {
        if session.length_s < self.config.min_session_s {
            return Ok(());
        }
        storage::append_session(&self.config.data_path, session).map_err(err)
    }

    fn view(&self) -> View {
        self.timer.view(Instant::now(), self.threshold())
    }
}

type AppState = Mutex<Inner>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Desktop notification over D-Bus, sent off-thread so the caller never blocks.
fn notify(body: &str) {
    let body = body.to_owned();
    thread::spawn(move || {
        if let Err(e) = send_notification(&body) {
            eprintln!("notification failed: {e}");
        }
    });
}

/// Uses one session-bus connection for the app's lifetime: GNOME Shell drops a
/// notification as soon as its sender disconnects, unless it can map the
/// sender to an installed .desktop app.
fn send_notification(body: &str) -> zbus::Result<()> {
    static CONN: Mutex<Option<zbus::blocking::Connection>> = Mutex::new(None);
    let mut conn = CONN.lock().unwrap();
    if conn.is_none() {
        *conn = Some(zbus::blocking::Connection::session()?);
    }
    let hints: HashMap<&str, zbus::zvariant::Value> =
        HashMap::from([("desktop-entry", "Repoti".into())]);
    conn.as_ref().unwrap().call_method(
        Some("org.freedesktop.Notifications"),
        "/org/freedesktop/Notifications",
        Some("org.freedesktop.Notifications"),
        "Notify",
        // app_name, replaces_id, icon, summary, body, actions, hints, timeout
        &("Repoti", 0u32, "com.addoh.repoti", "Repoti", body, Vec::<&str>::new(), hints, -1i32),
    )?;
    Ok(())
}

#[tauri::command]
fn get_state(state: State<AppState>) -> View {
    state.lock().unwrap().view()
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Config {
    state.lock().unwrap().config.clone()
}

#[tauri::command]
fn set_config(state: State<AppState>, cfg: Config) -> Result<Config, String> {
    if cfg.work_threshold_s == 0 || cfg.break_threshold_s == 0 {
        return Err("thresholds must be greater than zero".into());
    }
    if cfg.data_path.as_os_str().is_empty() {
        return Err("data file path is empty".into());
    }
    let mut s = state.lock().unwrap();
    storage::move_data_file(&s.config.data_path, &cfg.data_path).map_err(err)?;
    storage::save_config(&s.config_path, &cfg).map_err(err)?;
    s.config = cfg;
    Ok(s.config.clone())
}

/// "need a break" / "back to work": save the running session, start the other mode.
#[tauri::command]
fn switch_mode(state: State<AppState>) -> Result<View, String> {
    let mut s = state.lock().unwrap();
    let now = Instant::now();
    let session = s.timer.session(now, s.threshold());
    s.save(&session)?;
    s.timer.switch(now);
    Ok(s.view())
}

#[tauri::command]
fn reset(state: State<AppState>) -> View {
    let mut s = state.lock().unwrap();
    s.timer.reset(Instant::now());
    s.view()
}

#[tauri::command]
fn toggle_pause(state: State<AppState>) -> View {
    let mut s = state.lock().unwrap();
    s.timer.toggle_pause(Instant::now());
    s.view()
}

/// "that's it for now": stop the clock, persist the session, exit.
/// On a write error the app stays open so the user can retry.
#[tauri::command]
fn quit(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    if !s.saved_for_exit {
        let now = Instant::now();
        s.timer.pause(now);
        let session = s.timer.session(now, s.threshold());
        s.save(&session)?;
        s.saved_for_exit = true;
    }
    app.exit(0);
    Ok(())
}

fn spawn_ticker(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(250));
        let state = app.state::<AppState>();
        let (view, crossed) = {
            let mut s = state.lock().unwrap();
            if s.saved_for_exit {
                return;
            }
            let threshold = s.threshold();
            let crossed = s.timer.check_threshold(Instant::now(), threshold);
            (s.view(), crossed)
        };
        let _ = app.emit("tick", &view);
        if crossed {
            let body = match view.mode {
                Mode::Work => "Well done, you can rest now",
                Mode::Break => "That was needed, you may continue with work",
            };
            notify(body);
            let _ = app.emit("threshold-reached", ());
        }
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let paths = app.path();
            let config_path = paths.config_dir()?.join("repoti/config.json");
            let default_data = paths.data_dir()?.join("repoti/sessions.csv");
            let config = storage::load_config(&config_path, default_data);
            app.manage::<AppState>(Mutex::new(Inner {
                timer: Timer::new(Mode::Work, Instant::now()),
                config,
                config_path,
                saved_for_exit: false,
            }));
            spawn_ticker(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Route the window's close button through the same save-then-exit flow.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.emit("close-requested", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            get_config,
            set_config,
            switch_mode,
            reset,
            toggle_pause,
            quit
        ])
        .run(tauri::generate_context!())
        .expect("error while running Repoti");
}
