// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Manager, State};

struct AppState {
    bridge_server: Arc<Mutex<Option<Child>>>,
    apn_node: Arc<Mutex<Option<Child>>>,
}

#[tauri::command]
fn get_status(state: State<AppState>) -> Result<String, String> {
    let bridge_running = state
        .bridge_server
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|child| child.id())
        .is_some();

    let node_running = state
        .apn_node
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|child| child.id())
        .is_some();

    Ok(serde_json::json!({
        "bridge_server": bridge_running,
        "apn_node": node_running,
    })
    .to_string())
}

fn start_apn_node() -> Result<Child, std::io::Error> {
    println!("Starting APN node...");

    // Check if node is already running
    let check = Command::new("pgrep")
        .arg("-f")
        .arg("apn_node")
        .output();

    if let Ok(output) = check {
        if !output.stdout.is_empty() {
            println!("APN node already running");
            // Return a dummy child process
            return Command::new("true").spawn();
        }
    }

    // Start the node
    let node_path = std::env::current_dir()
        .unwrap()
        .join("../../target/release/apn_node");

    Command::new(node_path)
        .arg("--port")
        .arg("4001")
        .arg("--relay")
        .arg("nats://nonlocal.info:4222")
        .arg("--heartbeat-interval")
        .arg("30")
        .arg("--import")
        .arg("slush index float shaft flight citizen swear chunk correct veteran eyebrow blind")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

fn start_bridge_server() -> Result<Child, std::io::Error> {
    println!("Starting bridge server...");

    // Check if already running
    let check = Command::new("lsof")
        .arg("-i")
        .arg(":8000")
        .output();

    if let Ok(output) = check {
        if !output.stdout.is_empty() {
            println!("Bridge server already running on port 8000");
            // Return a dummy child process
            return Command::new("true").spawn();
        }
    }

    let server_path = std::env::current_dir()
        .unwrap()
        .join("../../apn_bridge_server.py");

    Command::new("python3")
        .arg(server_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

fn main() {
    let app_state = AppState {
        bridge_server: Arc::new(Mutex::new(None)),
        apn_node: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(|app| {
            let handle = app.handle();
            let state: State<AppState> = handle.state();

            // Start APN node
            match start_apn_node() {
                Ok(child) => {
                    println!("APN node started successfully");
                    *state.apn_node.lock().unwrap() = Some(child);
                }
                Err(e) => {
                    eprintln!("Failed to start APN node: {}", e);
                }
            }

            // Wait a moment for node to initialize
            std::thread::sleep(Duration::from_secs(2));

            // Start bridge server
            match start_bridge_server() {
                Ok(child) => {
                    println!("Bridge server started successfully");
                    *state.bridge_server.lock().unwrap() = Some(child);
                }
                Err(e) => {
                    eprintln!("Failed to start bridge server: {}", e);
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app_handle = window.app_handle();
                let state: State<AppState> = app_handle.state();

                // Stop bridge server
                if let Ok(mut bridge) = state.bridge_server.lock() {
                    if let Some(mut child) = bridge.take() {
                        println!("Stopping bridge server...");
                        let _ = child.kill();
                    }
                }

                // Note: We don't stop the APN node as it may be used by other services
                // Users can stop it manually if needed
            }
        })
        .invoke_handler(tauri::generate_handler![get_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
