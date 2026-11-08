use std::process::Command;

/// RAII Guard for managing OS-level system proxy settings.
/// Automatically restores system proxy to disabled state upon drop/exit.
pub struct SystemProxyGuard {
    enabled: bool,
}

impl SystemProxyGuard {
    pub fn new() -> Self {
        Self { enabled: false }
    }

    pub fn enable(&mut self, port: u16) -> bool {
        #[cfg(target_os = "windows")]
        {
            let addr = format!("127.0.0.1:{}", port);
            let _ = Command::new("reg")
                .args([
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                    "/v",
                    "ProxyServer",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &addr,
                    "/f",
                ])
                .output();

            let res = Command::new("reg")
                .args([
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                    "/v",
                    "ProxyEnable",
                    "/t",
                    "REG_DWORD",
                    "/d",
                    "1",
                    "/f",
                ])
                .output();

            self.enabled = res.map(|o| o.status.success()).unwrap_or(false);
            self.enabled
        }

        #[cfg(target_os = "macos")]
        {
            let res = Command::new("networksetup")
                .args(["-setwebproxy", "Wi-Fi", "127.0.0.1", &port.to_string()])
                .output();
            self.enabled = res.map(|o| o.status.success()).unwrap_or(false);
            self.enabled
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            false
        }
    }

    pub fn disable(&mut self) {
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("reg")
                .args([
                    "add",
                    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                    "/v",
                    "ProxyEnable",
                    "/t",
                    "REG_DWORD",
                    "/d",
                    "0",
                    "/f",
                ])
                .output();
            self.enabled = false;
        }

        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("networksetup")
                .args(["-setwebproxystate", "Wi-Fi", "off"])
                .output();
            self.enabled = false;
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            self.enabled = false;
        }
    }
}

impl Drop for SystemProxyGuard {
    fn drop(&mut self) {
        if self.enabled {
            self.disable();
        }
    }
}
