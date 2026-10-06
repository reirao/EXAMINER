#![allow(non_snake_case)]

use std::ffi::c_void;

pub type Handle = *mut c_void;
pub type HookProc = unsafe extern "system" fn(i32, usize, isize) -> isize;
pub const INVALID_HANDLE: Handle = -1isize as Handle;
pub const WM_TEST: u32 = 0x8400;
pub const WM_PING: u32 = 0x8401;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Msg {
    pub hwnd: Handle,
    pub message: u32,
    pub wparam: usize,
    pub lparam: isize,
    pub time: u32,
    pub point: Point,
    pub private: u32,
}

#[repr(C)]
pub struct CwpStruct {
    pub lparam: isize,
    pub wparam: usize,
    pub message: u32,
    pub hwnd: Handle,
}

#[repr(C)]
pub struct ProcessEntry {
    pub size: u32,
    pub usage: u32,
    pub pid: u32,
    pub heap: usize,
    pub module: u32,
    pub threads: u32,
    pub parent: u32,
    pub priority: i32,
    pub flags: u32,
    pub exe: [u16; 260],
}

#[repr(C)]
pub struct WindowClass {
    pub size: u32,
    pub style: u32,
    pub proc: Option<unsafe extern "system" fn(Handle, u32, usize, isize) -> isize>,
    pub extra_class: i32,
    pub extra_window: i32,
    pub instance: Handle,
    pub icon: Handle,
    pub cursor: Handle,
    pub background: Handle,
    pub menu: *const u16,
    pub class: *const u16,
    pub small_icon: Handle,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetLastError() -> u32;
    pub fn GetCurrentProcessId() -> u32;
    pub fn GetCurrentThreadId() -> u32;
    pub fn GetTickCount64() -> u64;
    pub fn CloseHandle(handle: Handle) -> i32;
    pub fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> Handle;
    pub fn Process32FirstW(snapshot: Handle, entry: *mut ProcessEntry) -> i32;
    pub fn Process32NextW(snapshot: Handle, entry: *mut ProcessEntry) -> i32;
    pub fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
    pub fn QueryFullProcessImageNameW(
        process: Handle,
        flags: u32,
        name: *mut u16,
        size: *mut u32,
    ) -> i32;
    pub fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
    pub fn CreateMutexW(attributes: *const c_void, owner: i32, name: *const u16) -> Handle;
    pub fn ReleaseMutex(handle: Handle) -> i32;
    pub fn LoadLibraryW(path: *const u16) -> Handle;
    pub fn FreeLibrary(module: Handle) -> i32;
    pub fn GetProcAddress(
        module: Handle,
        name: *const u8,
    ) -> Option<unsafe extern "system" fn() -> isize>;
    pub fn GetModuleHandleW(name: *const u16) -> Handle;
    pub fn CreateFileMappingW(
        file: Handle,
        attributes: *const c_void,
        protect: u32,
        high: u32,
        low: u32,
        name: *const u16,
    ) -> Handle;
    pub fn OpenFileMappingW(access: u32, inherit: i32, name: *const u16) -> Handle;
    pub fn MapViewOfFile(
        mapping: Handle,
        access: u32,
        high: u32,
        low: u32,
        bytes: usize,
    ) -> *mut c_void;
    pub fn UnmapViewOfFile(address: *const c_void) -> i32;
    pub fn SetConsoleCtrlHandler(
        handler: Option<unsafe extern "system" fn(u32) -> i32>,
        add: i32,
    ) -> i32;
}

#[link(name = "user32")]
unsafe extern "system" {
    pub fn EnumWindows(
        callback: unsafe extern "system" fn(Handle, isize) -> i32,
        data: isize,
    ) -> i32;
    pub fn GetWindowThreadProcessId(window: Handle, pid: *mut u32) -> u32;
    pub fn IsWindowVisible(window: Handle) -> i32;
    pub fn GetWindow(window: Handle, command: u32) -> Handle;
    pub fn SetWindowsHookExW(kind: i32, callback: HookProc, module: Handle, thread: u32) -> Handle;
    pub fn UnhookWindowsHookEx(hook: Handle) -> i32;
    pub fn CallNextHookEx(hook: Handle, code: i32, wparam: usize, lparam: isize) -> isize;
    pub fn PostMessageW(window: Handle, message: u32, wparam: usize, lparam: isize) -> i32;
    pub fn SendMessageW(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize;
    pub fn PeekMessageW(message: *mut Msg, window: Handle, min: u32, max: u32, remove: u32) -> i32;
    pub fn GetMessageW(message: *mut Msg, window: Handle, min: u32, max: u32) -> i32;
    pub fn TranslateMessage(message: *const Msg) -> i32;
    pub fn DispatchMessageW(message: *const Msg) -> isize;
    pub fn RegisterClassExW(class: *const WindowClass) -> u16;
    pub fn CreateWindowExW(
        style_ex: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Handle,
        menu: Handle,
        instance: Handle,
        param: *mut c_void,
    ) -> Handle;
    pub fn DefWindowProcW(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize;
    pub fn DestroyWindow(window: Handle) -> i32;
    pub fn PostQuitMessage(code: i32);
}

pub fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

pub fn error(operation: &str) -> String {
    format!("{operation}: Windows error {}", unsafe { GetLastError() })
}

pub struct OwnedHandle(pub Handle);
impl OwnedHandle {
    pub fn new(value: Handle, operation: &str) -> Result<Self, String> {
        if value.is_null() || value == INVALID_HANDLE {
            Err(error(operation))
        } else {
            Ok(Self(value))
        }
    }
}
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
