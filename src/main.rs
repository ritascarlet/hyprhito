use std::path::Path;
use tokio::process::Command as TokioCommand;
use tokio::io::AsyncWriteExt;
use log::{info, error, warn};
use evdev::{Device, InputEventKind};

const DISPLAY_CONTROL_PATH: &str = "/sys/class/drm/card1-eDP-1/status";
const DP1_STATUS_PATH: &str = "/sys/class/drm/card1-DP-1/status";
const DP2_STATUS_PATH: &str = "/sys/class/drm/card1-DP-2/status";

#[derive(Debug, PartialEq)]
enum LidEvent {
    Opened,
    Closed,
}



async fn check_external_displays() -> Result<bool, Box<dyn std::error::Error>> {
    use tokio::fs;
    
    if let Ok(status) = fs::read_to_string(DP1_STATUS_PATH).await {
        let status = status.trim();
        if status == "connected" {
            info!("External display detected on DP-1: {}", status);
            return Ok(true);
        }
    }
    
    if let Ok(status) = fs::read_to_string(DP2_STATUS_PATH).await {
        let status = status.trim();
        if status == "connected" {
            info!("External display detected on DP-2: {}", status);
            return Ok(true);
        }
    }
    
    info!("No external displays detected");
    Ok(false)
}

async fn control_display(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    let command_value = if enable { "on" } else { "off" };
    
    info!("Setting display state to: {}", command_value);
    
    let mut cmd = TokioCommand::new("tee")
        .arg(DISPLAY_CONTROL_PATH)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    
    if let Some(mut stdin) = cmd.stdin.take() {
        stdin.write_all(command_value.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        drop(stdin);
    }
    
    let output = cmd.wait_with_output().await?;
    
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Command execution failed: {}", error_msg).into());
    }
    
    info!("Display state changed to: {}", command_value);
    Ok(())
}

async fn suspend_system() -> Result<(), Box<dyn std::error::Error>> {
    info!("Suspending system");
    
    let output = TokioCommand::new("systemctl")
        .args(&["suspend"])
        .output()
        .await?;
    
    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Suspend failed: {}", error_msg).into());
    }
    
    info!("System suspended successfully");
    Ok(())
}

async fn monitor_lid_events() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting low-level lid monitoring via /dev/input/event0");
    
    let device_path = "/dev/input/event0";
    let mut device = match Device::open(device_path) {
        Ok(dev) => {
            info!("Device {} opened: {}", device_path, dev.name().unwrap_or("Unknown"));
            dev
        }
        Err(e) => {
            error!("Failed to open {}: {}", device_path, e);
            warn!("Fallback: searching for lid device automatically");
            find_lid_device().await?
        }
    };
    
    if device.supported_switches().is_none() {
        error!("Device does not support switch events");
        return Err("Device does not support lid switch".into());
    }
    
    info!("Supported switches: {:?}", device.supported_switches());
    info!("Lid event monitoring started, waiting for events");
    loop {
        match device.fetch_events() {
            Ok(events) => {
                for event in events {
                    if let InputEventKind::Switch(switch_type) = event.kind() {
                        info!("Raw switch event: {:?} = {}", switch_type, event.value());
                        
                        if switch_type.0 == 0x00 {
                            let lid_event = if event.value() == 1 { 
                                LidEvent::Closed 
                            } else { 
                                LidEvent::Opened 
                            };
                            
                            info!("Processing lid event: {:?}", lid_event);
                            
                                                         match lid_event {
                                 LidEvent::Closed => {
                                     info!("Lid closed, checking external displays");
                                     
                                     match check_external_displays().await {
                                         Ok(has_external) => {
                                             if has_external {
                                                 info!("External display detected, disabling eDP only");
                                                 if let Err(e) = control_display(false).await {
                                                     error!("Failed to disable display: {}", e);
                                                 }
                                             } else {
                                                 info!("No external display, suspending system");
                                                 if let Err(e) = control_display(false).await {
                                                     error!("Failed to disable display: {}", e);
                                                 }
                                                 tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                                                 if let Err(e) = suspend_system().await {
                                                     error!("Failed to suspend system: {}", e);
                                                 }
                                             }
                                         }
                                         Err(e) => {
                                             error!("Failed to check external displays: {}", e);
                                             info!("Fallback: disabling display only");
                                             if let Err(e) = control_display(false).await {
                                                 error!("Failed to disable display: {}", e);
                                             }
                                         }
                                     }
                                 }
                                 LidEvent::Opened => {
                                     info!("Lid opened, enabling display");
                                     if let Err(e) = control_display(true).await {
                                         error!("Failed to enable display: {}", e);
                                     }
                                     info!("System ready");
                                 }
                             }
                        }
                    }
                }
            }
                         Err(e) => {
                 if e.kind() == std::io::ErrorKind::WouldBlock {
                     tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                 } else {
                     error!("Failed to read events: {}", e);
                     return Err(e.into());
                 }
             }
        }
    }
}

async fn find_lid_device() -> Result<Device, Box<dyn std::error::Error>> {
    info!("Searching for lid device among input devices");
    
    for entry in std::fs::read_dir("/dev/input/")? {
        let entry = entry?;
        let path = entry.path();
        
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("event") {
                if let Ok(device) = Device::open(&path) {
                    if let Some(device_name) = device.name() {
                        info!("Checking device: {} ({})", path.display(), device_name);
                        
                        if device_name.to_lowercase().contains("gpio") || 
                           device_name.to_lowercase().contains("key") {
                            if device.supported_switches().is_some() {
                                info!("Found lid device: {} ({})", path.display(), device_name);
                                return Ok(device);
                            }
                        }
                    }
                }
            }
        }
    }
    
    Err("No suitable lid device found".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    info!("Starting hyprhito - laptop lid display manager");
    
    if !Path::new(DISPLAY_CONTROL_PATH).exists() {
        error!("Display control file not accessible: {}", DISPLAY_CONTROL_PATH);
        return Err("Display control file not accessible".into());
    }
    
    if !Path::new("/dev/input/").exists() {
        error!("/dev/input/ not accessible - input subsystem required");
        return Err("Input subsystem not available".into());
    }
    
    info!("Input subsystem available");
    info!("System components ready");
    
    monitor_lid_events().await?;
    
    Ok(())
}
