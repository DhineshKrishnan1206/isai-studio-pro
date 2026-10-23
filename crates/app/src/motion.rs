//! Whether the operating system asks for less motion. Read when the app starts
//! and when its window gets focus, so turning the setting on or off is picked
//! up without a restart.

/// True when the system asks for reduced motion. Systems we can't ask say no.
pub fn os_reduces_motion() -> bool {
    #[cfg(target_os = "macos")]
    {
        // The "Reduce motion" accessibility switch.
        run(
            "defaults",
            &["read", "com.apple.universalaccess", "reduceMotion"],
        )
        .is_some_and(|out| out.trim() == "1")
    }
    #[cfg(target_os = "windows")]
    {
        windows_animations_off()
    }
    #[cfg(target_os = "linux")]
    {
        // GNOME's "Animations" switch (other desktops are not asked).
        run(
            "gsettings",
            &["get", "org.gnome.desktop.interface", "enable-animations"],
        )
        .is_some_and(|out| out.trim() == "false")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        false
    }
}

/// Output of a command that succeeded, or `None`.
#[allow(dead_code)] // Only some platforms ask the system.
fn run(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Windows' "Show animations in Windows" is bit 0x02 of the first byte of
/// the UserPreferencesMask value; when it's clear, animations are off.
#[cfg(target_os = "windows")]
fn windows_animations_off() -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let Ok(output) = std::process::Command::new("reg")
        .args([
            "query",
            r"HKCU\Control Panel\Desktop",
            "/v",
            "UserPreferencesMask",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    else {
        return false;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    // The value is printed as hex bytes after REG_BINARY.
    let first = text
        .split("REG_BINARY")
        .nth(1)
        .and_then(|hex| hex.split_whitespace().next());
    first
        .and_then(|byte| u8::from_str_radix(byte, 16).ok())
        .is_some_and(|byte| byte & 0x02 == 0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn asking_the_system_never_panics() {
        // The answer depends on the machine; the call itself must be safe.
        let _ = super::os_reduces_motion();
    }
}
