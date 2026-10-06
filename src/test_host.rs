use examiner_runtime::win32::*;

fn packed(x: i16, y: i16) -> isize {
    ((y as u16 as u32) << 16 | x as u16 as u32) as isize
}

unsafe extern "system" fn window_proc(
    window: Handle,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match message {
        WM_TEST => {
            let sequence = [
                (0x0100, 0x11, 0),
                (0x0101, 0x11, 0),
                (0x0100, 0x75, 0),
                (0x0101, 0x75, 0),
                (0x0201, 1, packed(-100, 120)),
                (0x0200, 1, packed(-70, 140)),
                (0x0200, 1, packed(-60, 145)),
                (0x0202, 0, packed(-60, 145)),
                (0x0204, 2, packed(10, 20)),
                (0x0205, 0, packed(10, 20)),
                (0x0100, 0x10, 0),
                (0x0101, 0x10, 0),
            ];
            for (id, wp, lp) in sequence {
                unsafe {
                    PostMessageW(window, id, wp, lp);
                }
            }
            unsafe {
                PostMessageW(window, WM_TEST + 2, 0, 0);
            }
            0
        }
        id if id == WM_TEST + 2 => {
            unsafe {
                SendMessageW(window, 0x001f, 0, 0);
            }
            0
        }
        0x0010 => {
            unsafe {
                DestroyWindow(window);
            }
            0
        }
        0x0002 => {
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

fn main() {
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let name = wide("EXAMINER_NATIVE_TEST_HOST");
        let mut class: WindowClass = std::mem::zeroed();
        class.size = std::mem::size_of::<WindowClass>() as u32;
        class.proc = Some(window_proc);
        class.instance = instance;
        class.class = name.as_ptr();
        if RegisterClassExW(&class) == 0 {
            std::process::exit(2);
        }
        let window = CreateWindowExW(
            0,
            name.as_ptr(),
            name.as_ptr(),
            0,
            0,
            0,
            1,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null_mut(),
        );
        if window.is_null() {
            std::process::exit(3);
        }
        let mut message = Msg::default();
        loop {
            let result = GetMessageW(&mut message, std::ptr::null_mut(), 0, 0);
            if result <= 0 {
                break;
            }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
