use std::process::{Command, Stdio};
use std::fs;
use std::path::PathBuf;
use std::env;

// This macro compiles your UI layout directly into native Rust structures
slint::slint! {
    import { Button, VerticalBox, HorizontalBox } from "std-widgets.slint";

    export component AppWindow inherits Window {
        title: "Silvermere Server Panel";
        min-width: 650px;
        min-height: 350px;
        
        // This globally updates the base font size for all text and inherited widgets inside the window
        default-font-size: 30px;

        // Properties to dynamically track server state in the UI
        in-out property <string> status-text: "Unknown";
        in-out property <brush> status-color: #ffd700; // Yellow default

        // Callbacks to send click events back to our Rust code
        callback start-clicked();
        callback shutdown-clicked();
        callback refresh-clicked();

        VerticalBox {
            spacing: 20px;
            padding: 20px;

            // Status Box
            HorizontalBox {
                alignment: center;
                Text {
                    text: "Server Status: ";
                    font-size: 40px; // Explicitly override global size for large title headers
                }
                Text {
                    text: root.status-text;
                    font-size: 40px;
                    color: root.status-color;
                    font-weight: 700;
                }
            }

            // Action Buttons
            VerticalBox {
                spacing: 20px;
                
                Button {
                    text: "▶  Start Server";
                    height: 52px; // Adjusted bounds to support a clean padding frame
                    clicked => { root.start-clicked() }
                }
                Button {
                    text: "■  Shutdown Server";
                    height: 52px;
                    clicked => { root.shutdown-clicked() }
                }
            }

            // Invisible structural layout element that expands vertically 
            // and positions the elements above it higher up in the pane
            Rectangle { vertical-stretch: 1; }

            // Utility Actions
            HorizontalBox {
                alignment: space-between;
                Button {
                    text: "↻  Refresh Status";
                    height: 44px;
                    clicked => { root.refresh-clicked() }
                }
                Button {
                    text: "Exit Panel";
                    height: 44px;
                    clicked => { root.close() }
                }
            }
        }
    }
}


// Helper to reliably grab paths relative to the active directory
fn get_server_paths() -> (PathBuf, PathBuf, PathBuf) {
    let base_dir = env::current_dir().expect("Failed to determine current directory");
    let binary = base_dir.join("silvermere");
    let pid_file = base_dir.join("silvermere.pid");
    let log_file = base_dir.join("server.log");
    (binary, pid_file, log_file)
}

// Check the folder states and return UI formatting helpers
fn check_status() -> (&'static str, slint::Color) {
    let (_, pid_file, _) = get_server_paths();
    if pid_file.exists() {
        // Light Green for running
        ("Running", slint::Color::from_rgb_u8(46, 204, 113))
    } else {
        // Red for stopped
        ("Stopped", slint::Color::from_rgb_u8(231, 76, 60))
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    // Sync initial status on startup
    let (text, color) = check_status();
    ui.set_status_text(text.into());
    ui.set_status_color(color.into());

    // --- Wire Up GUI Callbacks to Rust Logic ---
    
    let ui_handle = ui.as_weak();
    ui.on_refresh_clicked(move || {
        if let Some(ui) = ui_handle.upgrade() {
            let (text, color) = check_status();
            ui.set_status_text(text.into());
            ui.set_status_color(color.into());
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_start_clicked(move || {
        let (binary, pid_file, log_file) = get_server_paths();

        if !binary.exists() || pid_file.exists() {
            return; // Guard against bad paths or duplicate runs
        }

        let log = fs::File::create(log_file).expect("Failed to create log file");
        if let Ok(child) = Command::new(&binary)
            .current_dir(env::current_dir().unwrap())
            .stdout(Stdio::from(log))
            .stderr(Stdio::null())
            .spawn() 
        {
            let pid = child.id();
            fs::write(pid_file, pid.to_string()).expect("Failed to write PID file");
        }

        // Force UI update
        if let Some(ui) = ui_handle.upgrade() {
            let (text, color) = check_status();
            ui.set_status_text(text.into());
            ui.set_status_color(color.into());
        }
    });

    let ui_handle = ui.as_weak();
    ui.on_shutdown_clicked(move || {
        let (_, pid_file, _) = get_server_paths();

        if !pid_file.exists() {
            return;
        }

        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                if let Ok(mut child) = Command::new("kill").arg(pid.to_string()).spawn() {
                    let _ = child.wait(); // Wait for kill command to finish
                    let _ = fs::remove_file(pid_file);
                }
            }
        }

        // Force UI update
        if let Some(ui) = ui_handle.upgrade() {
            let (text, color) = check_status();
            ui.set_status_text(text.into());
            ui.set_status_color(color.into());
        }
    });

    ui.run()
}
