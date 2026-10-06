#[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
compile_error!("EXAMINER requires 64-bit Windows.");

pub mod protocol;
pub mod win32;

use protocol::{Event, MAGIC, Shared, VERSION, mapping_name, records_input};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use win32::*;

static VIEW: AtomicUsize = AtomicUsize::new(0);
static RECORDING: AtomicBool = AtomicBool::new(false);

unsafe fn shared_view() -> Option<&'static Shared> {
    let mut address = VIEW.load(Ordering::Acquire);
    if address == 0 {
        let name = wide(&mapping_name(unsafe { GetCurrentProcessId() }, unsafe {
            GetCurrentThreadId()
        }));
        let handle = unsafe { OpenFileMappingW(6, 0, name.as_ptr()) };
        if handle.is_null() {
            return None;
        }
        let view = unsafe { MapViewOfFile(handle, 6, 0, 0, std::mem::size_of::<Shared>()) };
        unsafe {
            CloseHandle(handle);
        }
        if view.is_null() {
            return None;
        }
        address = match VIEW.compare_exchange(0, view as usize, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => view as usize,
            Err(existing) => {
                unsafe {
                    UnmapViewOfFile(view);
                }
                existing
            }
        };
    }
    let shared = unsafe { &*(address as *const Shared) };
    if shared.magic.load(Ordering::Acquire) != MAGIC
        || shared.version != VERSION
        || shared.target_pid != unsafe { GetCurrentProcessId() }
        || shared.target_thread != unsafe { GetCurrentThreadId() }
    {
        return None;
    }
    Some(shared)
}

unsafe fn record(message: u32, wparam: usize, lparam: isize, source: u32) {
    if RECORDING.swap(true, Ordering::Acquire) {
        return;
    }
    // Release the reentrancy guard even if initialization fails.
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            RECORDING.store(false, Ordering::Release);
        }
    }
    let _guard = Guard;
    let Some(shared) = (unsafe { shared_view() }) else {
        return;
    };
    shared.writers.fetch_add(1, Ordering::AcqRel);
    if shared.enabled.load(Ordering::Acquire) != 0 {
        shared
            .runtime_pid
            .store(unsafe { GetCurrentProcessId() }, Ordering::Release);
        shared
            .runtime_thread
            .store(unsafe { GetCurrentThreadId() }, Ordering::Release);
        shared.callbacks.fetch_add(1, Ordering::Relaxed);
        if records_input(message, wparam as u64)
            || matches!(message, 0x0008 | 0x001f | 0x0215 | WM_PING)
        {
            let event = Event {
                session: shared.session.load(Ordering::Acquire),
                milliseconds: unsafe { GetTickCount64() },
                message,
                source,
                wparam: wparam as u64,
                lparam: lparam as i64,
            };
            unsafe {
                shared.push(event);
            }
        }
    }
    shared.writers.fetch_sub(1, Ordering::Release);
}

/// Windows invokes this callback inside the selected Exanima window thread.
/// The message is observed and passed through unchanged.
///
/// # Safety
/// Must be invoked by Windows as a WH_GETMESSAGE callback. When code is zero,
/// lparam must refer to the MSG supplied by Windows for this callback.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn examiner_get_message(
    code: i32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if code == 0 && wparam == 1 && lparam != 0 {
        let msg = unsafe { &*(lparam as *const Msg) };
        unsafe {
            record(msg.message, msg.wparam, msg.lparam, 1);
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

/// Capture sent focus/cancellation messages, which bypass the message queue.
///
/// # Safety
/// Must be invoked by Windows as a WH_CALLWNDPROC callback. When code is zero,
/// lparam must refer to the CWPSTRUCT supplied by Windows for this callback.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn examiner_call_window(
    code: i32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if code == 0 && lparam != 0 {
        let msg = unsafe { &*(lparam as *const CwpStruct) };
        if matches!(msg.message, 0x0008 | 0x001f | 0x0215) {
            unsafe {
                record(msg.message, msg.wparam, msg.lparam, 2);
            }
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_struct_layout_matches_windows_x64() {
        assert_eq!(std::mem::size_of::<Msg>(), 48);
        assert_eq!(std::mem::size_of::<CwpStruct>(), 32);
        assert_eq!(std::mem::size_of::<ProcessEntry>(), 568);
        assert_eq!(std::mem::size_of::<WindowClass>(), 80);
        assert_eq!(std::mem::offset_of!(Msg, wparam), 16);
        assert_eq!(std::mem::offset_of!(CwpStruct, hwnd), 24);
    }
}
