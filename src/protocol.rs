use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub const MAGIC: u32 = 0x45584132;
pub const VERSION: u32 = 1;
pub const CAPACITY: usize = 512;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Event {
    pub session: u64,
    pub milliseconds: u64,
    pub message: u32,
    pub source: u32,
    pub wparam: u64,
    pub lparam: i64,
}

// One producer (the target window thread), one consumer (the controller).
#[repr(C)]
pub struct Shared {
    pub magic: AtomicU32,
    pub version: u32,
    pub target_pid: u32,
    pub target_thread: u32,
    pub enabled: AtomicU32,
    pub writers: AtomicU32,
    pub runtime_pid: AtomicU32,
    pub runtime_thread: AtomicU32,
    pub callbacks: AtomicU64,
    pub dropped: AtomicU64,
    pub session: AtomicU64,
    write: AtomicU64,
    read: AtomicU64,
    events: [UnsafeCell<Event>; CAPACITY],
}

impl Shared {
    pub fn new(pid: u32, thread: u32) -> Self {
        Self {
            magic: AtomicU32::new(0),
            version: VERSION,
            target_pid: pid,
            target_thread: thread,
            enabled: AtomicU32::new(0),
            writers: AtomicU32::new(0),
            runtime_pid: AtomicU32::new(0),
            runtime_thread: AtomicU32::new(0),
            callbacks: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            session: AtomicU64::new(0),
            write: AtomicU64::new(0),
            read: AtomicU64::new(0),
            events: [const {
                UnsafeCell::new(Event {
                    session: 0,
                    milliseconds: 0,
                    message: 0,
                    source: 0,
                    wparam: 0,
                    lparam: 0,
                })
            }; CAPACITY],
        }
    }

    /// # Safety
    /// Caller must be the sole producer, with no reentrant calls. The mapping
    /// must remain valid until both producer and consumer finish accessing it.
    pub unsafe fn push(&self, event: Event) -> bool {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Acquire);
        if write.wrapping_sub(read) >= CAPACITY as u64 {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        unsafe {
            self.events[write as usize % CAPACITY].get().write(event);
        }
        self.write.store(write.wrapping_add(1), Ordering::Release);
        true
    }

    /// # Safety
    /// Caller must be the sole consumer. The mapping must remain valid and its
    /// producer must obey the single-producer publication protocol.
    pub unsafe fn pop(&self) -> Option<Event> {
        let read = self.read.load(Ordering::Relaxed);
        if read == self.write.load(Ordering::Acquire) {
            return None;
        }
        let event = unsafe { self.events[read as usize % CAPACITY].get().read() };
        self.read.store(read.wrapping_add(1), Ordering::Release);
        Some(event)
    }
}

pub fn mapping_name(pid: u32, thread: u32) -> String {
    format!("Local\\EXAMINER_INPUT_V{VERSION}_{pid}_{thread}")
}

pub fn coordinates(lparam: i64) -> (i32, i32) {
    (
        lparam as u16 as i16 as i32,
        (lparam >> 16) as u16 as i16 as i32,
    )
}

pub fn records_input(message: u32, wparam: u64) -> bool {
    match message {
        0x0200..=0x020a | 0x00ff => true,
        0x0100 | 0x0101 | 0x0104 | 0x0105 => matches!(wparam, 0x10..=0x12 | 0x71 | 0x75),
        _ => false,
    }
}

#[derive(Default, Debug)]
pub struct DragTracker {
    pub started: u64,
    pub ended: u64,
    pub cancelled: u64,
    pub distance: u64,
    pub dragging: bool,
    last: (i32, i32),
}

impl DragTracker {
    pub fn observe(&mut self, event: &Event) -> &'static str {
        match event.message {
            0x0201 | 0x0203 if !self.dragging => {
                self.dragging = true;
                self.started += 1;
                self.distance = 0;
                self.last = coordinates(event.lparam);
                "start"
            }
            0x0200 if self.dragging => {
                let next = coordinates(event.lparam);
                self.distance += (i64::from(next.0) - i64::from(self.last.0)).unsigned_abs()
                    + (i64::from(next.1) - i64::from(self.last.1)).unsigned_abs();
                self.last = next;
                "move"
            }
            0x0202 if self.dragging => {
                self.dragging = false;
                self.ended += 1;
                "end"
            }
            0x0008 | 0x001f | 0x0215 if self.dragging => {
                self.dragging = false;
                self.cancelled += 1;
                "cancel"
            }
            _ => "input",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(message: u32, x: i16, y: i16) -> Event {
        Event {
            message,
            lparam: ((y as u16 as u32) << 16 | x as u16 as u32) as i64,
            ..Event::default()
        }
    }

    #[test]
    fn ring_preserves_order_across_wrap_and_reports_overflow() {
        let shared = Shared::new(1, 2);
        for cycle in 0..3 {
            for i in 0..CAPACITY {
                assert!(unsafe {
                    shared.push(Event {
                        wparam: (cycle * CAPACITY + i) as u64,
                        ..Event::default()
                    })
                });
            }
            assert!(!unsafe { shared.push(Event::default()) });
            for i in 0..CAPACITY {
                assert_eq!(
                    unsafe { shared.pop() }.unwrap().wparam,
                    (cycle * CAPACITY + i) as u64
                );
            }
            assert!(unsafe { shared.pop() }.is_none());
        }
        assert_eq!(shared.dropped.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn drag_starts_at_click_and_cancels_on_lost_capture() {
        let mut drag = DragTracker::default();
        assert_eq!(drag.observe(&event(0x0201, -100, 120)), "start");
        drag.observe(&event(0x0201, -100, 120));
        drag.observe(&event(0x0200, -70, 140));
        drag.observe(&event(0x0200, -60, 145));
        assert_eq!(drag.distance, 65);
        assert_eq!(drag.observe(&event(0x0215, 0, 0)), "cancel");
        drag.observe(&event(0x0202, 0, 0));
        assert_eq!((drag.started, drag.ended, drag.cancelled), (1, 0, 1));
    }

    #[test]
    fn ordinary_typing_is_not_recorded() {
        assert!(!records_input(0x0100, 0x41));
        assert!(!records_input(0x0102, 0x41));
        assert!(records_input(0x0100, 0x11));
        assert!(records_input(0x0101, 0x75));
    }
}
