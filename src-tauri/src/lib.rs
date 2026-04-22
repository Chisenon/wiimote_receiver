use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use wiimote_rs::prelude::*;
use wiimote_rs::input::{InputReport, ButtonData};
use wiimote_rs::output::{OutputReport, DataReporingMode, PlayerLedFlags};
use serde::{Serialize, Deserialize};
use tauri::Emitter;

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
        // en:Calculate tilt angles from accelerometer
        let roll = acc_y.atan2(acc_z);
        let pitch = (-acc_x).atan2((acc_y * acc_y + acc_z * acc_z).sqrt());
        
        // en:Create target quaternion from calculated angles
        let target_q = euler_xyz_to_quaternion(-roll, 0.0, pitch);
        
        // SLERP interpolation: blend smoothly between current and target
        let alpha = 0.1; // 10% new orientation each frame for smooth transition
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
        // Very close, use linear interpolation
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

fn set_reporting_mode(device: &Arc<Mutex<WiimoteDevice>>, mode: u8) -> WiimoteResult<()> {
    let wiimote = device.lock().unwrap();
    wiimote.write(&OutputReport::DataReportingMode(DataReporingMode {
        continuous: true,
        mode,
    }))?;
    Ok(())
}

#[tauri::command]
fn start_wiimote_reader(app_handle: tauri::AppHandle) -> String {
    thread::spawn(move || {
        if let Err(e) = run_wiimote_reader(&app_handle) {
            eprintln!("Wiimote reader error: {:?}", e);
        }
    });
    "Wiimote reader started".to_string()
}

fn run_wiimote_reader(app_handle: &tauri::AppHandle) -> WiimoteResult<()> {
    let manager = WiimoteManager::get_instance();
    let new_devices = manager.lock().unwrap().new_devices_receiver();

    new_devices.iter().try_for_each(|device| -> WiimoteResult<()> {
        let app_handle = app_handle.clone();
        thread::spawn(move || {
            if let Err(e) = handle_wiimote(&device, &app_handle) {
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
) -> WiimoteResult<()> {
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
        wiimote.write(&OutputReport::PlayerLed(PlayerLedFlags::LED_1 | PlayerLedFlags::LED_3))?;
    }

    let mut orientation_filter = OrientationFilter::new();

    loop {
        let input_report = device.lock().unwrap().read_timeout(20);

        match input_report {
            Ok(InputReport::StatusInformation(_)) => {
                let _ = set_reporting_mode(device, report_mode);
            }
            Ok(InputReport::DataReport(0x31, wiimote_data)) => {
                let buttons = wiimote_data.buttons();
                let accelerometer_data = AccelerometerData::from_normal_reporting(&wiimote_data.data);
                let (acc_x, acc_y, acc_z) = acc_calibration.get_acceleration(&accelerometer_data);

                // Update orientation from accelerometer data only
                orientation_filter.update_from_accel(acc_x as f32, acc_y as f32, acc_z as f32);
                let q = orientation_filter.quaternion();

                let wiimote_data = WiimoteData {
                    buttons: get_pressed_buttons(&buttons),
                    acc_x: acc_x as f32,
                    acc_y: acc_y as f32,
                    acc_z: acc_z as f32,
                    gyro_yaw: 0.0,
                    gyro_roll: 0.0,
                    gyro_pitch: 0.0,
                    quat_w: q[0],
                    quat_x: q[1],
                    quat_y: q[2],
                    quat_z: q[3],
                    quat_valid: true,
                    motion_plus_active: false,
                };

                let _ = app_handle.emit("wiimote-data", wiimote_data);
            }
            Ok(InputReport::DataReport(0x35, wiimote_data)) => {
                // 0x35 with gyro not implemented for now, fall back to accel
                let buttons = wiimote_data.buttons();
                let accelerometer_data = AccelerometerData::from_normal_reporting(&wiimote_data.data);
                let (acc_x, acc_y, acc_z) = acc_calibration.get_acceleration(&accelerometer_data);

                orientation_filter.update_from_accel(acc_x as f32, acc_y as f32, acc_z as f32);
                let q = orientation_filter.quaternion();

                let wiimote_data = WiimoteData {
                    buttons: get_pressed_buttons(&buttons),
                    acc_x: acc_x as f32,
                    acc_y: acc_y as f32,
                    acc_z: acc_z as f32,
                    gyro_yaw: 0.0,
                    gyro_roll: 0.0,
                    gyro_pitch: 0.0,
                    quat_w: q[0],
                    quat_x: q[1],
                    quat_y: q[2],
                    quat_z: q[3],
                    quat_valid: true,
                    motion_plus_active: false,
                };

                let _ = app_handle.emit("wiimote-data", wiimote_data);
            }
            Err(_) => {}
            _ => {}
        }
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
        (ButtonData::UP, "↑"),
        (ButtonData::DOWN, "↓"),
        (ButtonData::LEFT, "←"),
        (ButtonData::RIGHT, "→"),
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
            thread::spawn(move || {
                if let Err(e) = run_wiimote_reader(&app_handle) {
                    eprintln!("Wiimote reader error: {:?}", e);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![start_wiimote_reader])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
