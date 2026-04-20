// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use wiimote_rs::prelude::*;
use wiimote_rs::input::{InputReport, ButtonData};
use wiimote_rs::output::{OutputReport, PlayerLedFlags, DataReporingMode};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn start_wiimote_reader() -> String {
    thread::spawn(|| {
        if let Err(e) = run_wiimote_reader() {
            eprintln!("Wiimote reader error: {:?}", e);
        }
    });
    "Wiimote reader started in background".to_string()
}

fn run_wiimote_reader() -> WiimoteResult<()> {
    println!("=== Wiimote Sensor Reader ===\n");

    let manager = WiimoteManager::get_instance();
    let new_devices = {
        let manager = manager.lock().unwrap();
        manager.new_devices_receiver()
    };

    new_devices.iter().try_for_each(|device| -> WiimoteResult<()> {
        let device_id = {
            let wiimote = device.lock().unwrap();
            wiimote.identifier().to_string()
        };

        println!("✓ Wiimote Connected: {}\n", device_id);

        thread::spawn(move || {
            if let Err(e) = handle_wiimote(&device, &device_id) {
                eprintln!("Error: {:?}", e);
            }
        });

        Ok(())
    })?;

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}

fn handle_wiimote(device: &Arc<Mutex<WiimoteDevice>>, device_id: &str) -> WiimoteResult<()> {
    // LED sequence animation: 1→2→3→4→1→2→3→4
    let leds = [
        PlayerLedFlags::LED_1,
        PlayerLedFlags::LED_2,
        PlayerLedFlags::LED_3,
        PlayerLedFlags::LED_4,
    ];

    for (i, &led) in leds.iter().cycle().take(8).enumerate() {
        {
            let wiimote = device.lock().unwrap();
            let led_report = OutputReport::PlayerLed(led);
            wiimote.write(&led_report)?;
        }
        print!("[{}] ✓ LED {} ON\r", device_id, (i % 4) + 1);
        thread::sleep(Duration::from_millis(200));
    }
    println!();

    // Set reporting mode (Core Buttons + Accelerometer)
    {
        let wiimote = device.lock().unwrap();
        let reporting_mode = OutputReport::DataReportingMode(DataReporingMode {
            continuous: false,
            mode: 0x31,
        });
        wiimote.write(&reporting_mode)?;
    }

    // Get accelerometer calibration
    let acc_calibration = {
        let wiimote = device.lock().unwrap();
        wiimote
            .accelerometer_calibration()
            .expect("Wiimote should have accelerometer calibration")
            .clone()
    };

    let mut prev_buttons = 0u16;
    let mut frame_count = 0u64;
    let mut last_status_frame = 0u64;
    const STATUS_INTERVAL: u64 = 100;

    println!("Ready! Press buttons and move the Wiimote...\n");

    // Main sensor data loop
    loop {
        let input_report = {
            let wiimote = device.lock().unwrap();
            wiimote.read_timeout(200)
        };

        match input_report {
            Ok(InputReport::DataReport(0x31, wiimote_data)) => {
                frame_count += 1;

                let buttons = wiimote_data.buttons();
                let buttons_raw = buttons.bits();
                let accelerometer_data = AccelerometerData::from_normal_reporting(&wiimote_data.data);
                let (acc_x, acc_y, acc_z) = acc_calibration.get_acceleration(&accelerometer_data);

                // Display button changes
                if buttons_raw != prev_buttons {
                    print_button_state(&buttons);
                    prev_buttons = buttons_raw;
                }

                print!(
                    "\r[{}] Frame:{:6} | Acc: X={:6.2}g Y={:6.2}g Z={:6.2}g",
                    device_id, frame_count, acc_x, acc_y, acc_z
                );
            }
            Ok(InputReport::StatusInformation(status)) => {
                // Display battery status every 100 frames
                if frame_count - last_status_frame >= STATUS_INTERVAL {
                    let battery_percent = (status.battery_level() as f32 / 200.0 * 100.0) as u8;
                    println!(
                        "\n[{}] Status: battery={}%",
                        device_id, battery_percent
                    );
                    last_status_frame = frame_count;
                }
            }
            Err(_) => {} // Ignore timeouts
            _ => {}       // Ignore other report types
        }

        thread::sleep(Duration::from_millis(1));
    }
}

fn print_button_state(buttons: &ButtonData) {
    let mut pressed = Vec::new();

    macro_rules! check_button {
        ($button:expr, $label:expr) => {
            if buttons.contains($button) {
                pressed.push($label);
            }
        };
    }

    check_button!(ButtonData::A, "A");
    check_button!(ButtonData::B, "B");
    check_button!(ButtonData::ONE, "1");
    check_button!(ButtonData::TWO, "2");
    check_button!(ButtonData::PLUS, "+");
    check_button!(ButtonData::MINUS, "-");
    check_button!(ButtonData::HOME, "HOME");
    check_button!(ButtonData::UP, "↑");
    check_button!(ButtonData::DOWN, "↓");
    check_button!(ButtonData::LEFT, "←");
    check_button!(ButtonData::RIGHT, "→");

    if !pressed.is_empty() {
        println!("\nButtons: {}", pressed.join(", "));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            // Start wiimote reader in background when app starts
            thread::spawn(|| {
                if let Err(e) = run_wiimote_reader() {
                    eprintln!("Wiimote reader error: {:?}", e);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, start_wiimote_reader])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
