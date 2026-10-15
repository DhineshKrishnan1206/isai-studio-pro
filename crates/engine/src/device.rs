use cpal::traits::{DeviceTrait as _, HostTrait as _};

/// An audio output the user can pick in the setup screen and settings.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputDevice {
    /// Stable id saved in settings, so the same device is picked next launch.
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn list_output_devices() -> Vec<OutputDevice> {
    let host = cpal::default_host();
    let default_id = host
        .default_output_device()
        .and_then(|d| d.id().ok())
        .map(|id| id.to_string());

    let Ok(devices) = host.output_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|device| {
            let id = device.id().ok()?.to_string();
            let name = device
                .description()
                .map(|d| d.name().to_owned())
                .unwrap_or_else(|_| id.clone());
            let is_default = default_id.as_deref() == Some(id.as_str());
            Some(OutputDevice { id, name, is_default })
        })
        .collect()
}
