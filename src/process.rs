use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
}

#[cfg(target_os = "windows")]
mod platform {
    use super::ProcessInfo;
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::path::Path;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    const AF_INET: u32 = 2;
    const TCP_TABLE_OWNER_PID_ALL: u32 = 5;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    #[repr(C)]
    struct MibTcprowOwnerPid {
        dw_state: u32,
        dw_local_addr: u32,
        dw_local_port: u32,
        dw_remote_addr: u32,
        dw_remote_port: u32,
        dw_owning_pid: u32,
    }

    #[link(name = "iphlpapi")]
    extern "system" {
        fn GetExtendedTcpTable(
            pTcpTable: *mut std::ffi::c_void,
            pdwSize: *mut u32,
            bOrder: i32,
            ulAf: u32,
            TableClass: u32,
            Reserved: u32,
        ) -> u32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(dwDesiredAccess: u32, bInheritHandle: i32, dwProcessId: u32) -> *mut std::ffi::c_void;
        fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
        fn QueryFullProcessImageNameW(
            hProcess: *mut std::ffi::c_void,
            dwFlags: u32,
            lpExeName: *mut u16,
            lpdwSize: *mut u32,
        ) -> i32;
    }

    // 2 秒缓存，避免代理高频并发时反复全表扫描
    static PORT_CACHE: Mutex<Option<HashMap<u16, (ProcessInfo, Instant)>>> = Mutex::new(None);

    pub fn get_process_name_by_pid(pid: u32) -> String {
        if pid == 0 {
            return "System Idle".to_string();
        }
        if pid == 4 {
            return "System".to_string();
        }

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return format!("PID:{}", pid);
            }

            let mut buf = [0u16; 1024];
            let mut size = buf.len() as u32;
            let success = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
            CloseHandle(handle);

            if success != 0 && size > 0 {
                let os_str = OsString::from_wide(&buf[..size as usize]);
                if let Some(s) = os_str.to_str() {
                    if let Some(file_name) = Path::new(s).file_name().and_then(|f| f.to_str()) {
                        return file_name.to_string();
                    }
                }
            }
        }
        format!("PID:{}", pid)
    }

    pub fn get_process_by_port(port: u16) -> Option<ProcessInfo> {
        let now = Instant::now();
        // 尝试从缓存中查找
        if let Ok(mut lock) = PORT_CACHE.lock() {
            let cache = lock.get_or_insert_with(HashMap::new);
            if let Some((info, timestamp)) = cache.get(&port) {
                if now.duration_since(*timestamp) < Duration::from_secs(2) {
                    return Some(info.clone());
                }
            }
        }

        // 调用 Windows API 查询当前所有活动 TCP 连接
        let mut size: u32 = 0;
        unsafe {
            GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                AF_INET,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if size == 0 {
                return None;
            }

            let mut buf = vec![0u8; size as usize];
            let ret = GetExtendedTcpTable(
                buf.as_mut_ptr() as *mut _,
                &mut size,
                0,
                AF_INET,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if ret != 0 {
                return None;
            }

            let num_entries = *(buf.as_ptr() as *const u32);
            let rows_ptr = buf.as_ptr().add(4) as *const MibTcprowOwnerPid;

            for i in 0..num_entries {
                let row = &*rows_ptr.add(i as usize);
                let local_port = u16::from_be((row.dw_local_port & 0xFFFF) as u16);
                if local_port == port {
                    let pid = row.dw_owning_pid;
                    let name = get_process_name_by_pid(pid);
                    let info = ProcessInfo { pid, name };

                    // 写入缓存
                    if let Ok(mut lock) = PORT_CACHE.lock() {
                        let cache = lock.get_or_insert_with(HashMap::new);
                        // 清理过期缓存
                        if cache.len() > 1000 {
                            cache.retain(|_, (_, t)| now.duration_since(*t) < Duration::from_secs(5));
                        }
                        cache.insert(port, (info.clone(), now));
                    }

                    return Some(info);
                }
            }
        }
        None
    }

    pub fn list_active_network_processes() -> Vec<ProcessInfo> {
        let mut map: HashMap<u32, (String, usize)> = HashMap::new();
        let mut size: u32 = 0;
        unsafe {
            GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                AF_INET,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if size > 0 {
                let mut buf = vec![0u8; size as usize];
                if GetExtendedTcpTable(
                    buf.as_mut_ptr() as *mut _,
                    &mut size,
                    0,
                    AF_INET,
                    TCP_TABLE_OWNER_PID_ALL,
                    0,
                ) == 0 {
                    let num_entries = *(buf.as_ptr() as *const u32);
                    let rows_ptr = buf.as_ptr().add(4) as *const MibTcprowOwnerPid;
                    for i in 0..num_entries {
                        let row = &*rows_ptr.add(i as usize);
                        let pid = row.dw_owning_pid;
                        let entry = map.entry(pid).or_insert_with(|| (get_process_name_by_pid(pid), 0));
                        entry.1 += 1;
                    }
                }
            }
        }

        let mut list: Vec<_> = map.into_iter()
            .map(|(pid, (name, count))| (pid, name, count))
            .collect();
        // 按照连接活跃数降序排列
        list.sort_by(|a, b| b.2.cmp(&a.2));

        list.into_iter().map(|(pid, name, _)| ProcessInfo { pid, name }).collect()
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::ProcessInfo;

    pub fn get_process_by_port(_port: u16) -> Option<ProcessInfo> {
        None
    }

    pub fn list_active_network_processes() -> Vec<ProcessInfo> {
        Vec::new()
    }
}

pub fn get_process_by_port(port: u16) -> Option<ProcessInfo> {
    platform::get_process_by_port(port)
}

pub fn list_active_network_processes() -> Vec<ProcessInfo> {
    platform::list_active_network_processes()
}
