#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(windows)]
    {
        use std::ptr;

        #[link(name = "kernel32")]
        extern "system" {
            fn CreateMutexW(
                lp_mutex_attributes: *const std::ffi::c_void,
                b_initial_owner: i32,
                lp_name: *const u16,
            ) -> *mut std::ffi::c_void;
            fn GetLastError() -> u32;
        }

        #[link(name = "user32")]
        extern "system" {
            fn MessageBoxW(
                h_wnd: *mut std::ffi::c_void,
                lp_text: *const u16,
                lp_caption: *const u16,
                u_type: u32,
            ) -> i32;
        }

        const ERROR_ALREADY_EXISTS: u32 = 183;
        const MB_OK: u32 = 0x0000_0000;
        const MB_ICONINFORMATION: u32 = 0x0000_0040;

        let name: Vec<u16> = "PortManager_SingleInstance\0".encode_utf16().collect();

        unsafe {
            let handle = CreateMutexW(ptr::null(), 0, name.as_ptr());

            if handle.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
                let title: Vec<u16> = "Port Manager\0".encode_utf16().collect();
                let msg: Vec<u16> = "Port Manager is already running.\nCheck the system tray.\0"
                    .encode_utf16()
                    .collect();
                MessageBoxW(
                    ptr::null_mut(),
                    msg.as_ptr(),
                    title.as_ptr(),
                    MB_OK | MB_ICONINFORMATION,
                );
                std::process::exit(0);
            }
            // Mutex stays alive for the process lifetime — intentionally not closed.
        }
    }

    port_manager_lib::run()
}
