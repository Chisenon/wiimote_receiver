use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use enigo::{
    Coordinate,
    Direction::{Press, Release},
    Enigo, Key, Keyboard, Mouse, Settings,
};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tauri::Manager;
use tauri::State;
use wiimote_rs::input::{ButtonData, InputReport};
use wiimote_rs::output::{DataReporingMode, OutputReport, PlayerLedFlags};
use wiimote_rs::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WiimoteData {
    pub buttons: Vec<String>,
    pub acc_x: f32,
    pub acc_y: f32,
    pub acc_z: f32,
    pub gyro_yaw: f32,
    pub gyro_roll: f32,
    pub gyro_pitch: f32,
    pub quat_w: f32,
    pub quat_x: f32,
    pub quat_y: f32,
    pub quat_z: f32,
    pub quat_valid: bool,
    pub motion_plus_active: bool,
}

const CONFIG_FILE_NAME: &str = "wiimote_receiver.config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct MouseLookConfig {
    roll_deadzone_deg: f32,
    roll_speed_pixels_per_deg: f32,
    roll_max_pixels_per_frame: i32,
    roll_offset_smoothing_alpha: f32,
    roll_speed_exponent: f32,
    roll_invert_x: bool,
}

impl Default for MouseLookConfig {
    fn default() -> Self {
        Self {
            roll_deadzone_deg: 10.0,
            roll_speed_pixels_per_deg: 0.1,
            roll_max_pixels_per_frame: 10,
            roll_offset_smoothing_alpha: 0.25,
            roll_speed_exponent: 1.2,
            roll_invert_x: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct AppConfig {
    mouse_look: MouseLookConfig,
    button_bindings: HashMap<String, String>,
}

struct AppState {
    config: Arc<Mutex<AppConfig>>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut button_bindings = HashMap::new();
        button_bindings.insert("A".to_string(), "Control".to_string());
        button_bindings.insert("B".to_string(), "Space".to_string());
        button_bindings.insert("1".to_string(), "S".to_string());
        button_bindings.insert("2".to_string(), "W".to_string());
        button_bindings.insert("+".to_string(), "Return".to_string());
        button_bindings.insert("-".to_string(), "Backspace".to_string());
        button_bindings.insert("HOME".to_string(), "Escape".to_string());
        button_bindings.insert("UP".to_string(), "LeftArrow".to_string());
        button_bindings.insert("DOWN".to_string(), "RightArrow".to_string());
        button_bindings.insert("LEFT".to_string(), "DownArrow".to_string());
        button_bindings.insert("RIGHT".to_string(), "UpArrow".to_string());

        Self {
            mouse_look: MouseLookConfig::default(),
            button_bindings,
        }
    }
}

fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var("WIIMOTE_RECEIVER_CONFIG") {
        return PathBuf::from(path);
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(CONFIG_FILE_NAME)
}

fn load_config() -> AppConfig {
    let path = config_path();

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
            Ok(config) => {
                println!("Loaded config: {}", path.display());
                config
            }
            Err(err) => {
                eprintln!(
                    "Failed to parse config {}: {:?}. Falling back to defaults.",
                    path.display(),
                    err
                );
                AppConfig::default()
            }
        },
        Err(_) => {
            let config = AppConfig::default();
            match serde_json::to_string_pretty(&config) {
                Ok(json) => {
                    if let Err(err) = fs::write(&path, json) {
                        eprintln!(
                            "Failed to write default config {}: {:?}",
                            path.display(),
                            err
                        );
                    } else {
                        println!("Created default config: {}", path.display());
                    }
                }
                Err(err) => {
                    eprintln!("Failed to serialize default config: {:?}", err);
                }
            }
            config
        }
    }
}

fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let json = serde_json::to_string_pretty(config)
        .map_err(|err| format!("Failed to serialize config: {err:?}"))?;
    fs::write(&path, json).map_err(|err| format!("Failed to write {}: {err:?}", path.display()))
}

struct OrientationFilter {
    q: [f32; 4],
}

impl OrientationFilter {
    fn new() -> Self {
        Self {
            q: [1.0, 0.0, 0.0, 0.0],
        }
    }

    fn quaternion(&self) -> [f32; 4] {
        self.q
    }

    fn update_from_accel(&mut self, acc_x: f32, acc_y: f32, acc_z: f32) {
        let roll = acc_y.atan2(acc_z);
        let pitch = (-acc_x).atan2((acc_y * acc_y + acc_z * acc_z).sqrt());

        let target_q = euler_xyz_to_quaternion(-roll, 0.0, pitch);

        let alpha = 0.1;
        self.q = slerp(&self.q, &target_q, alpha);
        normalize_quaternion(&mut self.q);
    }
}

fn slerp(q1: &[f32; 4], q2: &[f32; 4], t: f32) -> [f32; 4] {
    let [w1, x1, y1, z1] = q1;
    let [w2, x2, y2, z2] = q2;

    let mut dot = w1 * w2 + x1 * x2 + y1 * y2 + z1 * z2;
    let (w2, x2, y2, z2) = if dot < 0.0 {
        dot = -dot;
        (-w2, -x2, -y2, -z2)
    } else {
        (*w2, *x2, *y2, *z2)
    };

    if dot > 0.9995 {
        let t = t.min(1.0);
        let w = w1 + t * (w2 - w1);
        let x = x1 + t * (x2 - x1);
        let y = y1 + t * (y2 - y1);
        let z = z1 + t * (z2 - z1);
        let mut q = [w, x, y, z];
        normalize_quaternion(&mut q);
        return q;
    }

    let theta_0 = dot.acos();
    let theta = theta_0 * t;

    let w3 = w2 - w1 * dot;
    let x3 = x2 - x1 * dot;
    let y3 = y2 - y1 * dot;
    let z3 = z2 - z1 * dot;

    let len_sq = w3 * w3 + x3 * x3 + y3 * y3 + z3 * z3;
    let len = len_sq.sqrt();

    let w = w1 * theta.cos() + (w3 / len) * theta.sin();
    let x = x1 * theta.cos() + (x3 / len) * theta.sin();
    let y = y1 * theta.cos() + (y3 / len) * theta.sin();
    let z = z1 * theta.cos() + (z3 / len) * theta.sin();

    [w, x, y, z]
}

fn euler_xyz_to_quaternion(x: f32, y: f32, z: f32) -> [f32; 4] {
    let (cx, sx) = ((x * 0.5).cos(), (x * 0.5).sin());
    let (cy, sy) = ((y * 0.5).cos(), (y * 0.5).sin());
    let (cz, sz) = ((z * 0.5).cos(), (z * 0.5).sin());
    [
        cx * cy * cz + sx * sy * sz,
        sx * cy * cz - cx * sy * sz,
        cx * sy * cz + sx * cy * sz,
        cx * cy * sz - sx * sy * cz,
    ]
}

fn normalize_quaternion(q: &mut [f32; 4]) {
    let norm = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if norm > 1e-8 {
        q[0] /= norm;
        q[1] /= norm;
        q[2] /= norm;
        q[3] /= norm;
    } else {
        *q = [1.0, 0.0, 0.0, 0.0];
    }
}

fn quaternion_roll_x_deg(q: [f32; 4]) -> f32 {
    let [w, x, y, z] = q;
    let sinr_cosp = 2.0 * (w * x + y * z);
    let cosr_cosp = 1.0 - 2.0 * (x * x + y * y);
    sinr_cosp.atan2(cosr_cosp).to_degrees()
}

fn set_reporting_mode(device: &Arc<Mutex<WiimoteDevice>>, mode: u8) -> WiimoteResult<()> {
    let wiimote = device.lock().unwrap();
    wiimote.write(&OutputReport::DataReportingMode(DataReporingMode {
        continuous: true,
        mode,
    }))?;
    Ok(())
}

#[tauri::command]
fn start_wiimote_reader(app_handle: tauri::AppHandle, state: State<AppState>) -> String {
    let config = Arc::clone(&state.config);
    thread::spawn(move || {
        if let Err(e) = run_wiimote_reader(&app_handle, config) {
            eprintln!("Wiimote reader error: {:?}", e);
        }
    });
    "Wiimote reader started".to_string()
}

#[tauri::command]
fn ensure_config_file(state: State<AppState>) -> Result<String, String> {
    let path = config_path();
    if path.exists() {
        return Ok(path.display().to_string());
    }

    let config = state.config.lock().map(|c| c.clone()).unwrap_or_default();
    save_config(&config)?;
    Ok(path.display().to_string())
}

#[tauri::command]
fn import_config_from_json(json: String, state: State<AppState>) -> Result<String, String> {
    let config = serde_json::from_str::<AppConfig>(&json)
        .map_err(|err| format!("Invalid config JSON: {err:?}"))?;
    save_config(&config)?;

    if let Ok(mut current) = state.config.lock() {
        *current = config;
    }
    Ok("Config loaded and applied".to_string())
}

#[tauri::command]
fn get_config_path() -> String {
    config_path().display().to_string()
}

fn run_wiimote_reader(
    app_handle: &tauri::AppHandle,
    config: Arc<Mutex<AppConfig>>,
) -> WiimoteResult<()> {
    let manager = WiimoteManager::get_instance();
    let new_devices = manager.lock().unwrap().new_devices_receiver();

    new_devices
        .iter()
        .try_for_each(|device| -> WiimoteResult<()> {
            let app_handle = app_handle.clone();
            let config = Arc::clone(&config);
            thread::spawn(move || {
                if let Err(e) = handle_wiimote(&device, &app_handle, config) {
                    eprintln!("Wiimote error: {:?}", e);
                }
            });
            Ok(())
        })?;

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

fn handle_wiimote(
    device: &Arc<Mutex<WiimoteDevice>>,
    app_handle: &tauri::AppHandle,
    config: Arc<Mutex<AppConfig>>,
) -> WiimoteResult<()> {
    let mut enigo = Enigo::new(&Settings::default()).ok();
    if enigo.is_none() {
        eprintln!("Keyboard injector is unavailable. Button-to-key mapping is disabled.");
    }
    let mut prev_pressed_buttons: HashSet<String> = HashSet::new();

    let (acc_calibration, motion_plus_calibration) = {
        let wiimote = device.lock().unwrap();
        let acc = wiimote
            .accelerometer_calibration()
            .expect("Accelerometer calibration required")
            .clone();

        let mut mp_calibration = None;
        if let Some(motion_plus) = wiimote.motion_plus() {
            let mp_type = motion_plus.motion_plus_type();
            println!("MotionPlus detected: {:?}", mp_type);

            match motion_plus.initialize(&wiimote) {
                Ok(_) => {
                    if let Err(err) = motion_plus.change_mode(&wiimote, MotionPlusMode::Active) {
                        eprintln!("MotionPlus change_mode error: {:?}", err);
                    } else {
                        println!("MotionPlus active mode enabled");
                        mp_calibration = Some(motion_plus.calibration());
                    }
                }
                Err(err) => {
                    eprintln!("MotionPlus initialize error: {:?}", err);
                }
            }
        } else {
            println!("MotionPlus not detected. Running accelerometer-only mode.");
        }

        (acc, mp_calibration)
    };

    let report_mode = if motion_plus_calibration.is_some() { 0x35 } else { 0x31 };
    set_reporting_mode(device, report_mode)?;

    {
        let wiimote = device.lock().unwrap();
        wiimote.write(&OutputReport::PlayerLed(
            PlayerLedFlags::LED_1 | PlayerLedFlags::LED_3,
        ))?;
    }

    let mut orientation_filter = OrientationFilter::new();
    let mut neutral_roll_deg: Option<f32> = None;
    let mut filtered_roll_offset_deg = 0.0f32;

    loop {
        let input_report = device.lock().unwrap().read_timeout(20);

        match input_report {
            Ok(InputReport::StatusInformation(_)) => {
                let _ = set_reporting_mode(device, report_mode);
            }
            Ok(InputReport::DataReport(0x31, wiimote_data)) => {
                let buttons = wiimote_data.buttons();
                let accelerometer_data =
                    AccelerometerData::from_normal_reporting(&wiimote_data.data);
                let (acc_x, acc_y, acc_z) =
                    acc_calibration.get_acceleration(&accelerometer_data);

                orientation_filter.update_from_accel(acc_x as f32, acc_y as f32, acc_z as f32);
                let q = orientation_filter.quaternion();
                let roll_x_deg = quaternion_roll_x_deg(q);

                let wiimote_data = WiimoteData {
                    buttons: get_pressed_buttons(&buttons),
                    acc_x: acc_x as f32,
                    acc_y: acc_y as f32,
                    acc_z: acc_z as f32,
                    gyro_yaw: 0.0,
                    gyro_roll: roll_x_deg,
                    gyro_pitch: 0.0,
                    quat_w: q[0],
                    quat_x: q[1],
                    quat_y: q[2],
                    quat_z: q[3],
                    quat_valid: true,
                    motion_plus_active: false,
                };

                sync_keyboard_bindings(
                    &wiimote_data.buttons,
                    &mut prev_pressed_buttons,
                    &config
                        .lock()
                        .map(|c| c.button_bindings.clone())
                        .unwrap_or_default(),
                    enigo.as_mut(),
                );
                sync_mouse_look_roll_from_tilt(
                    wiimote_data.gyro_roll,
                    &mut neutral_roll_deg,
                    &mut filtered_roll_offset_deg,
                    &config
                        .lock()
                        .map(|c| c.mouse_look.clone())
                        .unwrap_or_default(),
                    enigo.as_mut(),
                );
                let _ = app_handle.emit("wiimote-data", wiimote_data);
            }
            Ok(InputReport::DataReport(0x35, wiimote_data)) => {
                let buttons = wiimote_data.buttons();
                let accelerometer_data =
                    AccelerometerData::from_normal_reporting(&wiimote_data.data);
                let (acc_x, acc_y, acc_z) =
                    acc_calibration.get_acceleration(&accelerometer_data);

                orientation_filter.update_from_accel(acc_x as f32, acc_y as f32, acc_z as f32);
                let q = orientation_filter.quaternion();
                let roll_x_deg = quaternion_roll_x_deg(q);

                let wiimote_data = WiimoteData {
                    buttons: get_pressed_buttons(&buttons),
                    acc_x: acc_x as f32,
                    acc_y: acc_y as f32,
                    acc_z: acc_z as f32,
                    gyro_yaw: 0.0,
                    gyro_roll: roll_x_deg,
                    gyro_pitch: 0.0,
                    quat_w: q[0],
                    quat_x: q[1],
                    quat_y: q[2],
                    quat_z: q[3],
                    quat_valid: true,
                    motion_plus_active: false,
                };

                sync_keyboard_bindings(
                    &wiimote_data.buttons,
                    &mut prev_pressed_buttons,
                    &config
                        .lock()
                        .map(|c| c.button_bindings.clone())
                        .unwrap_or_default(),
                    enigo.as_mut(),
                );
                sync_mouse_look_roll_from_tilt(
                    wiimote_data.gyro_roll,
                    &mut neutral_roll_deg,
                    &mut filtered_roll_offset_deg,
                    &config
                        .lock()
                        .map(|c| c.mouse_look.clone())
                        .unwrap_or_default(),
                    enigo.as_mut(),
                );
                let _ = app_handle.emit("wiimote-data", wiimote_data);
            }
            Err(_) => {}
            _ => {}
        }
    }
}

fn parse_key_binding(value: &str) -> Option<Key> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "space" => Some(Key::Space),
        "return" | "enter" => Some(Key::Return),
        "escape" | "esc" => Some(Key::Escape),
        "backspace" => Some(Key::Backspace),
        "control" | "ctrl" => Some(Key::Control),
        "leftarrow" | "left" => Some(Key::LeftArrow),
        "rightarrow" | "right" => Some(Key::RightArrow),
        "uparrow" | "up" => Some(Key::UpArrow),
        "downarrow" | "down" => Some(Key::DownArrow),
        _ => {
            let mut chars = normalized.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) if ch.is_ascii_alphanumeric() => Some(Key::Unicode(ch)),
                _ => None,
            }
        }
    }
}

fn sync_keyboard_bindings(
    current_buttons: &[String],
    prev_buttons: &mut HashSet<String>,
    button_bindings: &HashMap<String, String>,
    enigo: Option<&mut Enigo>,
) {
    let Some(enigo) = enigo else {
        return;
    };

    let current_set: HashSet<String> = current_buttons.iter().cloned().collect();

    for button in current_set.difference(prev_buttons) {
        if let Some(mapping) = button_bindings.get(button).and_then(|value| parse_key_binding(value))
        {
            let key = mapping;
            if let Err(err) = enigo.key(key, Press) {
                eprintln!("Failed to press key for button {}: {:?}", button, err);
            }
        }
    }

    for button in prev_buttons.difference(&current_set) {
        if let Some(mapping) = button_bindings.get(button).and_then(|value| parse_key_binding(value))
        {
            let key = mapping;
            if let Err(err) = enigo.key(key, Release) {
                eprintln!("Failed to release key for button {}: {:?}", button, err);
            }
        }
    }

    *prev_buttons = current_set;
}

fn normalize_delta_angle_deg(delta: f32) -> f32 {
    if delta > 180.0 {
        delta - 360.0
    } else if delta < -180.0 {
        delta + 360.0
    } else {
        delta
    }
}

fn sync_mouse_look_roll_from_tilt(
    roll_deg: f32,
    neutral_roll_deg: &mut Option<f32>,
    filtered_roll_offset_deg: &mut f32,
    mouse_look: &MouseLookConfig,
    enigo: Option<&mut Enigo>,
) {
    let Some(enigo) = enigo else {
        return;
    };

    let Some(neutral_roll) = *neutral_roll_deg else {
        *neutral_roll_deg = Some(roll_deg);
        return;
    };

    let mut roll_offset = normalize_delta_angle_deg(roll_deg - neutral_roll);
    *filtered_roll_offset_deg = *filtered_roll_offset_deg
        + (roll_offset - *filtered_roll_offset_deg) * mouse_look.roll_offset_smoothing_alpha;
    roll_offset = *filtered_roll_offset_deg;

    if roll_offset.abs() <= mouse_look.roll_deadzone_deg {
        return;
    }

    let signed_offset = if mouse_look.roll_invert_x {
        -roll_offset
    } else {
        roll_offset
    };
    let offset_without_deadzone = signed_offset.abs() - mouse_look.roll_deadzone_deg;
    let speed =
        offset_without_deadzone.powf(mouse_look.roll_speed_exponent) * mouse_look.roll_speed_pixels_per_deg;
    let dx = speed
        .clamp(0.0, mouse_look.roll_max_pixels_per_frame as f32)
        .round() as i32
        * if signed_offset.is_sign_negative() { -1 } else { 1 };
    if dx == 0 {
        return;
    }

    if let Err(err) = enigo.move_mouse(dx, 0, Coordinate::Rel) {
        eprintln!("Failed to move mouse from roll input: {:?}", err);
    }
}

fn get_pressed_buttons(buttons: &ButtonData) -> Vec<String> {
    let mut pressed = Vec::new();

    let button_map = [
        (ButtonData::A, "A"),
        (ButtonData::B, "B"),
        (ButtonData::ONE, "1"),
        (ButtonData::TWO, "2"),
        (ButtonData::PLUS, "+"),
        (ButtonData::MINUS, "-"),
        (ButtonData::HOME, "HOME"),
        (ButtonData::UP, "UP"),
        (ButtonData::DOWN, "DOWN"),
        (ButtonData::LEFT, "LEFT"),
        (ButtonData::RIGHT, "RIGHT"),
    ];

    for (button, label) in &button_map {
        if buttons.contains(*button) {
            pressed.push(label.to_string());
        }
    }

    pressed
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let config = Arc::new(Mutex::new(load_config()));
            app.manage(AppState {
                config: Arc::clone(&config),
            });
            thread::spawn(move || {
                if let Err(e) = run_wiimote_reader(&app_handle, config) {
                    eprintln!("Wiimote reader error: {:?}", e);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_wiimote_reader,
            ensure_config_file,
            import_config_from_json,
            get_config_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


