#[cfg(target_os = "windows")]
mod windows {
    use std::env;
    use std::io;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "NTMTracker";

    fn run_key() -> Result<RegKey, String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey(RUN_KEY)
            .map_err(|err| format!("Unable to open Run registry key: {err}"))?;
        Ok(key)
    }

    fn desired_value() -> Result<String, String> {
        let exe = env::current_exe()
            .map_err(|err| format!("Unable to resolve executable path: {err}"))?;
        Ok(format!("\"{}\" --minimized", exe.display()))
    }

    pub(super) fn set_enabled(enabled: bool) -> Result<(), String> {
        let key = run_key()?;
        if enabled {
            let value = desired_value()?;
            key.set_value(VALUE_NAME, &value)
                .map_err(|err| format!("Unable to set autostart value: {err}"))?;
            return Ok(());
        }

        match key.delete_value(VALUE_NAME) {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(format!("Unable to remove autostart value: {err}")),
        }
    }

    #[allow(dead_code)]
    pub(super) fn is_enabled() -> Result<bool, String> {
        let key = run_key()?;
        match key.get_value::<String, _>(VALUE_NAME) {
            Ok(value) => Ok(value == desired_value()?),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(format!("Unable to read autostart value: {err}")),
        }
    }
}

#[cfg(target_os = "windows")]
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    windows::set_enabled(enabled)
}

#[cfg(not(target_os = "windows"))]
pub fn set_enabled(_enabled: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub fn is_enabled() -> Result<bool, String> {
    windows::is_enabled()
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn is_enabled() -> Result<bool, String> {
    Ok(false)
}
