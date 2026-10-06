use examiner_runtime::{protocol::*, win32::*};
use std::os::windows::{ffi::OsStrExt, process::CommandExt};
use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

static STOP: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn stop_signal(signal: u32) -> i32 {
    if signal <= 2 {
        STOP.store(true, Ordering::Relaxed);
        1
    } else {
        0
    }
}

fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn process_path(pid: u32) -> Result<(OwnedHandle, PathBuf), String> {
    let process = OwnedHandle::new(
        unsafe { OpenProcess(0x1000 | 0x00100000, 0, pid) },
        "OpenProcess",
    )?;
    let mut buffer = vec![0u16; 32768];
    let mut size = buffer.len() as u32;
    if unsafe { QueryFullProcessImageNameW(process.0, 0, buffer.as_mut_ptr(), &mut size) } == 0 {
        return Err(error("QueryFullProcessImageNameW"));
    }
    Ok((
        process,
        PathBuf::from(String::from_utf16_lossy(&buffer[..size as usize])),
    ))
}

fn find_process(name: &str) -> Result<u32, String> {
    let snapshot = OwnedHandle::new(
        unsafe { CreateToolhelp32Snapshot(2, 0) },
        "CreateToolhelp32Snapshot",
    )?;
    let mut entry: ProcessEntry = unsafe { std::mem::zeroed() };
    entry.size = std::mem::size_of::<ProcessEntry>() as u32;
    let mut present = unsafe { Process32FirstW(snapshot.0, &mut entry) };
    let mut matches = Vec::new();
    while present != 0 {
        let end = entry
            .exe
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(entry.exe.len());
        if String::from_utf16_lossy(&entry.exe[..end]).eq_ignore_ascii_case(name) {
            matches.push(entry.pid);
        }
        present = unsafe { Process32NextW(snapshot.0, &mut entry) };
    }
    match matches.as_slice() {
        [pid] => Ok(*pid),
        [] => Err(format!(
            "{name} is not running. Start the game, then run examiner hook."
        )),
        _ => Err(format!(
            "Multiple {name} processes found; select one with --pid."
        )),
    }
}

struct Search {
    pid: u32,
    allow_hidden: bool,
    found: Vec<(Handle, u32)>,
}
unsafe extern "system" fn enum_window(window: Handle, data: isize) -> i32 {
    let search = unsafe { &mut *(data as *mut Search) };
    let mut pid = 0;
    let thread = unsafe { GetWindowThreadProcessId(window, &mut pid) };
    if pid == search.pid
        && thread != 0
        && unsafe { GetWindow(window, 4) }.is_null()
        && (search.allow_hidden || unsafe { IsWindowVisible(window) } != 0)
    {
        search.found.push((window, thread));
    }
    1
}

fn find_window(pid: u32, hidden: bool) -> Result<(Handle, u32), String> {
    let mut search = Search {
        pid,
        allow_hidden: hidden,
        found: Vec::new(),
    };
    unsafe {
        EnumWindows(enum_window, &mut search as *mut Search as isize);
    }
    match search.found.as_slice() {
        [window] => Ok(*window),
        [] => Err("The target has no suitable window yet.".into()),
        _ => Err("Multiple target windows found. Close extra game windows and retry.".into()),
    }
}

fn check_x64(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut dos = [0u8; 64];
    file.read_exact(&mut dos).map_err(|e| e.to_string())?;
    if &dos[..2] != b"MZ" {
        return Err("The target is not a Windows executable.".into());
    }
    let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap());
    if offset > 1024 * 1024 {
        return Err("Invalid PE header offset.".into());
    }
    file.seek(SeekFrom::Start(offset as u64))
        .map_err(|e| e.to_string())?;
    let mut header = [0u8; 6];
    file.read_exact(&mut header).map_err(|e| e.to_string())?;
    if &header[..4] != b"PE\0\0" || u16::from_le_bytes([header[4], header[5]]) != 0x8664 {
        return Err("The target must be an x64 executable.".into());
    }
    Ok(())
}

struct View {
    _handle: OwnedHandle,
    pointer: *mut Shared,
}
impl View {
    fn open(pid: u32, thread: u32) -> Result<Self, String> {
        let name = wide(&mapping_name(pid, thread));
        let handle = OwnedHandle::new(
            unsafe {
                CreateFileMappingW(
                    INVALID_HANDLE,
                    std::ptr::null(),
                    4,
                    0,
                    std::mem::size_of::<Shared>() as u32,
                    name.as_ptr(),
                )
            },
            "CreateFileMappingW",
        )?;
        let existed = unsafe { GetLastError() } == 183;
        let pointer = unsafe { MapViewOfFile(handle.0, 6, 0, 0, std::mem::size_of::<Shared>()) }
            as *mut Shared;
        if pointer.is_null() {
            return Err(error("MapViewOfFile"));
        }
        let view = Self {
            _handle: handle,
            pointer,
        };
        if !existed {
            unsafe {
                pointer.write(Shared::new(pid, thread));
            }
            view.shared().magic.store(MAGIC, Ordering::Release);
        }
        let shared = view.shared();
        if shared.magic.load(Ordering::Acquire) != MAGIC
            || shared.version != VERSION
            || shared.target_pid != pid
            || shared.target_thread != thread
        {
            return Err("Shared-memory protocol mismatch.".into());
        }
        Ok(view)
    }
    fn shared(&self) -> &Shared {
        unsafe { &*self.pointer }
    }
}
impl Drop for View {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(self.pointer.cast());
        }
    }
}

struct SessionLock(OwnedHandle);
impl SessionLock {
    fn acquire(pid: u32, thread: u32) -> Result<Self, String> {
        let name = wide(&format!("Local\\EXAMINER_CONTROLLER_{pid}_{thread}"));
        let handle = OwnedHandle::new(
            unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) },
            "CreateMutexW",
        )?;
        match unsafe { WaitForSingleObject(handle.0, 0) } {
            0 | 0x80 => Ok(Self(handle)),
            _ => Err("Another EXAMINER controller is attached to this window thread.".into()),
        }
    }
}
impl Drop for SessionLock {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.0.0);
        }
    }
}

struct Library(Handle);
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.0);
        }
    }
}
struct NativeHook(Handle);
impl NativeHook {
    fn detach(&mut self) -> Result<(), String> {
        if !self.0.is_null() {
            if unsafe { UnhookWindowsHookEx(self.0) } == 0 {
                return Err(error("UnhookWindowsHookEx"));
            }
            self.0 = std::ptr::null_mut();
        }
        Ok(())
    }
}
impl Drop for NativeHook {
    fn drop(&mut self) {
        let _ = self.detach();
    }
}

struct Session {
    hooks: Vec<NativeHook>,
    _library: Library,
    view: View,
    _lock: SessionLock,
    process: OwnedHandle,
    pub window: Handle,
    pub pid: u32,
    pub thread: u32,
    id: u64,
    callbacks_at_start: u64,
    dropped_at_start: u64,
}

impl Session {
    fn attach(pid: u32, fixture: bool) -> Result<Self, String> {
        let (process, path) = process_path(pid)?;
        let expected = if fixture {
            "examiner_test_host.exe"
        } else {
            "Exanima.exe"
        };
        if !path
            .file_name()
            .unwrap_or(OsStr::new(""))
            .to_string_lossy()
            .eq_ignore_ascii_case(expected)
        {
            return Err(format!("EXAMINER only attaches to {expected}."));
        }
        check_x64(&path)?;
        let (window, thread) = find_window(pid, fixture)?;
        let lock = SessionLock::acquire(pid, thread)?;
        let view = View::open(pid, thread)?;
        view.shared().enabled.store(0, Ordering::Release);
        wait_writers(view.shared())?;
        while unsafe { view.shared().pop() }.is_some() {}
        let dll = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .with_file_name("examiner_runtime.dll");
        let encoded: Vec<u16> = dll.as_os_str().encode_wide().chain(Some(0)).collect();
        let module = unsafe { LoadLibraryW(encoded.as_ptr()) };
        if module.is_null() {
            return Err(error("LoadLibraryW(examiner_runtime.dll)"));
        }
        let library = Library(module);
        let callbacks_at_start = view.shared().callbacks.load(Ordering::Acquire);
        let dropped_at_start = view.shared().dropped.load(Ordering::Acquire);
        let mut session = Self {
            hooks: Vec::new(),
            _library: library,
            view,
            _lock: lock,
            process,
            window,
            pid,
            thread,
            id: timestamp(),
            callbacks_at_start,
            dropped_at_start,
        };
        session
            .view
            .shared()
            .session
            .store(session.id, Ordering::Release);
        session.view.shared().enabled.store(1, Ordering::Release);
        for (kind, export) in [(3, c"examiner_get_message"), (4, c"examiner_call_window")] {
            let address = unsafe { GetProcAddress(module, export.as_ptr().cast()) }
                .ok_or_else(|| error("GetProcAddress"))?;
            let callback: HookProc = unsafe { std::mem::transmute(address) };
            let hook = unsafe { SetWindowsHookExW(kind, callback, module, thread) };
            if hook.is_null() {
                return Err(error("SetWindowsHookExW"));
            }
            session.hooks.push(NativeHook(hook));
        }
        if unsafe { PostMessageW(window, WM_PING, 0, 0) } == 0 {
            return Err(error("PostMessageW"));
        }
        Ok(session)
    }

    fn detach(&mut self) -> Result<(), String> {
        self.view.shared().enabled.store(0, Ordering::Release);
        let mut failure = None;
        for hook in &mut self.hooks {
            if let Err(e) = hook.detach() {
                failure = Some(e);
            }
        }
        wait_writers(self.view.shared())?;
        if let Some(error) = failure {
            Err(error)
        } else {
            Ok(())
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.detach();
    }
}

fn wait_writers(shared: &Shared) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(1);
    while shared.writers.load(Ordering::Acquire) != 0 {
        if Instant::now() >= deadline {
            return Err("The runtime callback did not finish within one second.".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

fn pump_messages() {
    let mut message = Msg::default();
    while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, 1) } != 0 {
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

struct Summary {
    events: u64,
    input_events: u64,
    sent_events: u64,
    callbacks: u64,
    dropped: u64,
    runtime_pid: u32,
    runtime_thread: u32,
    drag: DragTracker,
}

fn observe(
    mut session: Session,
    seconds: u64,
    path: &Path,
    fixture: bool,
) -> Result<Summary, String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut log = File::create(path).map_err(|e| e.to_string())?;
    writeln!(
        log,
        "{{\"kind\":\"attach\",\"session\":{},\"pid\":{},\"thread\":{},\"protocol\":{VERSION}}}",
        session.id, session.pid, session.thread
    )
    .map_err(|e| e.to_string())?;
    let shared = session.view.shared();
    let mut summary = Summary {
        events: 0,
        input_events: 0,
        sent_events: 0,
        callbacks: 0,
        dropped: 0,
        runtime_pid: 0,
        runtime_thread: 0,
        drag: DragTracker::default(),
    };
    let start = Instant::now();
    let mut test_sent = false;
    let mut confirmed = false;
    let mut failure = None;
    println!(
        "EXAMINER: native hooks attached to PID {} / thread {}.",
        session.pid, session.thread
    );
    while start.elapsed() < Duration::from_secs(seconds) && !STOP.load(Ordering::Relaxed) {
        pump_messages();
        if unsafe { WaitForSingleObject(session.process.0, 0) } == 0 {
            break;
        }
        if !confirmed
            && shared.runtime_pid.load(Ordering::Acquire) == session.pid
            && shared.runtime_thread.load(Ordering::Acquire) == session.thread
            && shared.callbacks.load(Ordering::Acquire) > session.callbacks_at_start
        {
            confirmed = true;
            println!("Confirmed: our DLL callback is executing inside the target process.");
            if fixture {
                if unsafe { PostMessageW(session.window, WM_TEST, 0, 0) } == 0 {
                    failure = Some(error("PostMessageW(test)"));
                    break;
                }
                test_sent = true;
            }
        }
        if !confirmed && start.elapsed() >= Duration::from_secs(5) {
            failure = Some(
                "No in-process callback was confirmed. The target may not be pumping messages."
                    .into(),
            );
            break;
        }
        drain(shared, session.id, &mut log, &mut summary)?;
        thread::sleep(Duration::from_millis(10));
    }
    session.detach()?;
    drain(session.view.shared(), session.id, &mut log, &mut summary)?;
    summary.callbacks = session
        .view
        .shared()
        .callbacks
        .load(Ordering::Acquire)
        .saturating_sub(session.callbacks_at_start);
    summary.dropped = session
        .view
        .shared()
        .dropped
        .load(Ordering::Acquire)
        .saturating_sub(session.dropped_at_start);
    summary.runtime_pid = session.view.shared().runtime_pid.load(Ordering::Acquire);
    summary.runtime_thread = session.view.shared().runtime_thread.load(Ordering::Acquire);
    writeln!(log, "{{\"kind\":\"detach\",\"events\":{},\"input_events\":{},\"callbacks\":{},\"dropped\":{},\"drag_starts\":{},\"drag_ends\":{},\"drag_cancels\":{},\"distance_px\":{}}}",
        summary.events, summary.input_events, summary.callbacks, summary.dropped,
        summary.drag.started, summary.drag.ended, summary.drag.cancelled, summary.drag.distance).map_err(|e| e.to_string())?;
    log.flush().map_err(|e| e.to_string())?;
    println!(
        "Detached. Events: {}, input events: {}, dropped: {}. Log: {}",
        summary.events,
        summary.input_events,
        summary.dropped,
        path.display()
    );
    if let Some(error) = failure {
        return Err(error);
    }
    if !confirmed {
        return Err("The runtime was not confirmed in the target process.".into());
    }
    if fixture
        && (!test_sent
            || summary.input_events != 12
            || summary.sent_events == 0
            || summary.drag.started != 1
            || summary.drag.ended != 1
            || summary.drag.distance != 65
            || summary.dropped != 0)
    {
        return Err(format!(
            "Native integration test failed: input={}, starts={}, ends={}, distance={}, dropped={}",
            summary.input_events,
            summary.drag.started,
            summary.drag.ended,
            summary.drag.distance,
            summary.dropped
        ));
    }
    Ok(summary)
}

fn drain(shared: &Shared, id: u64, log: &mut File, summary: &mut Summary) -> Result<(), String> {
    while let Some(event) = unsafe { shared.pop() } {
        if event.session != id {
            continue;
        }
        summary.events += 1;
        if event.source == 2 {
            summary.sent_events += 1;
        }
        if records_input(event.message, event.wparam) {
            summary.input_events += 1;
        }
        let phase = summary.drag.observe(&event);
        let (x, y) = coordinates(event.lparam);
        writeln!(log, "{{\"kind\":\"message\",\"ms\":{},\"message\":{},\"source\":{},\"wparam\":{},\"lparam\":{},\"x\":{},\"y\":{},\"phase\":\"{}\",\"drag\":{},\"distance_px\":{}}}",
            event.milliseconds, event.message, event.source, event.wparam, event.lparam,
            x, y, phase, summary.drag.started, summary.drag.distance).map_err(|e| e.to_string())?;
    }
    Ok(())
}

struct Fixture(Child);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn self_test(path: &Path) -> Result<(), String> {
    let host = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name("examiner_test_host.exe");
    let child = Command::new(host)
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut fixture = Fixture(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    while find_window(fixture.0.id(), true).is_err() {
        if fixture.0.try_wait().map_err(|e| e.to_string())?.is_some() {
            return Err("Test host exited before creating its window.".into());
        }
        if Instant::now() >= deadline {
            return Err("Test host window did not appear.".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
    for cycle in 1..=2 {
        let session = Session::attach(fixture.0.id(), true)?;
        let target_thread = session.thread;
        let cycle_path = if cycle == 1 {
            path.to_path_buf()
        } else {
            path.with_extension("reattach.jsonl")
        };
        let summary = observe(session, 2, &cycle_path, true)?;
        let view = View::open(fixture.0.id(), target_thread)?;
        let before = view.shared().callbacks.load(Ordering::Acquire);
        unsafe {
            PostMessageW(find_window(fixture.0.id(), true)?.0, WM_TEST, 0, 0);
        }
        thread::sleep(Duration::from_millis(200));
        if view.shared().callbacks.load(Ordering::Acquire) != before {
            return Err("Callbacks continued after detaching the native hooks.".into());
        }
        println!(
            "PASS {cycle}: both native hooks, 12 input events, one complete drag, 65 px, zero dropped events, clean detach. Runtime PID {} / thread {}.",
            summary.runtime_pid, summary.runtime_thread
        );
    }
    Ok(())
}

fn help() {
    println!(
        "EXAMINER 0.2.0 - independent x64 Windows hook\n\nexaminer inspect [--pid PID]\nexaminer hook [--pid PID] [--seconds N] [--log PATH]\nexaminer self-test [--log PATH]\n\nStart Exanima before attaching. Ctrl+C detaches. Default duration: 120 seconds.\nOnly mouse input, Ctrl/Shift/Alt, F2/F6, raw-input notifications and cancellation are recorded.\nThis build observes input; it does not modify object selection or physics."
    );
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        help();
        return Ok(());
    };
    if matches!(command.as_str(), "--help" | "-h") {
        help();
        return Ok(());
    }
    let mut pid = None;
    let mut seconds = 120u64;
    let mut log = PathBuf::from(format!("logs/session-{}.jsonl", timestamp()));
    while let Some(option) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("Missing value for {option}"))?;
        match option.as_str() {
            "--pid" => pid = Some(value.parse::<u32>().map_err(|_| "Invalid PID")?),
            "--seconds" => {
                seconds = value.parse().map_err(|_| "Invalid duration")?;
                if seconds == 0 {
                    return Err("Duration must be greater than zero.".into());
                }
            }
            "--log" => log = PathBuf::from(value),
            _ => return Err(format!("Unknown option: {option}")),
        }
    }
    match command.as_str() {
        "self-test" => self_test(&log),
        "inspect" | "hook" => {
            let pid = match pid {
                Some(pid) => pid,
                None => find_process("Exanima.exe")?,
            };
            if command == "inspect" {
                let (_, path) = process_path(pid)?;
                if !path
                    .file_name()
                    .unwrap_or(OsStr::new(""))
                    .to_string_lossy()
                    .eq_ignore_ascii_case("Exanima.exe")
                {
                    return Err("The selected process is not Exanima.exe.".into());
                }
                check_x64(&path)?;
                let (_, thread) = find_window(pid, false)?;
                println!(
                    "Exanima x64: PID {pid}, window thread {thread}, executable {}",
                    path.display()
                );
                Ok(())
            } else {
                STOP.store(false, Ordering::Relaxed);
                if unsafe { SetConsoleCtrlHandler(Some(stop_signal), 1) } == 0 {
                    return Err(error("SetConsoleCtrlHandler"));
                }
                let result = Session::attach(pid, false)
                    .and_then(|s| observe(s, seconds, &log, false).map(|_| ()));
                unsafe {
                    SetConsoleCtrlHandler(Some(stop_signal), 0);
                }
                result
            }
        }
        _ => Err(format!("Unknown command: {command}")),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("EXAMINER: {error}");
        std::process::exit(1);
    }
}
