use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use wiimote_rs::prelude::*;
use wiimote_rs::input::{InputReport, ButtonData};
use wiimote_rs::output::{OutputReport, DataReporingMode};
use serde::{Serialize, Deserialize};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WiimoteData {
    pub buttons: Vec<String>,
    pub acc_x: f32,
    pub acc_y: f32,
    pub acc_z: f32,
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
    {
        let wiimote = device.lock().unwrap();
        wiimote.write(&OutputReport::DataReportingMode(DataReporingMode {
            continuous: false,
            mode: 0x31,
        }))?;
    }

    let acc_calibration = device
        .lock()
        .unwrap()
        .accelerometer_calibration()
        .expect("Accelerometer calibration required")
        .clone();

    loop {
        let input_report = device.lock().unwrap().read_timeout(200);

        match input_report {
            Ok(InputReport::DataReport(0x31, wiimote_data)) => {
                let buttons = wiimote_data.buttons();
                let accelerometer_data = AccelerometerData::from_normal_reporting(&wiimote_data.data);
                let (acc_x, acc_y, acc_z) = acc_calibration.get_acceleration(&accelerometer_data);

                let wiimote_data = WiimoteData {
                    buttons: get_pressed_buttons(&buttons),
                    acc_x: acc_x as f32,
                    acc_y: acc_y as f32,
                    acc_z: acc_z as f32,
                };

                let _ = app_handle.emit("wiimote-data", wiimote_data);
            }
            Err(_) | _ => {}
        }

        thread::sleep(Duration::from_millis(10));
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
