// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! The Windows trigger: a hidden top-level window and a message loop blocked in
//! `GetMessage`, waiting for `WM_DEVICECHANGE`. Also owns the tray icon, its
//! menu, the single-instance mutex, and the startup sweep of mounted drives.

// ########## THE WINDOWS TRIGGER ##########

use std::cell::RefCell;
use std::path::Path;
use std::ptr;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives, SYNCHRONIZE};
use windows_sys::Win32::System::Diagnostics::Debug::{SEM_FAILCRITICALERRORS, SetErrorMode};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{CreateMutexW, OpenMutexW};
use windows_sys::Win32::System::WindowsProgramming::{DRIVE_FIXED, DRIVE_REMOVABLE};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
    ShellExecuteW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CW_USEDEFAULT, CreatePopupMenu, CreateWindowExW, DBT_DEVICEARRIVAL,
    DBT_DEVTYP_VOLUME, DBTF_NET, DEV_BROADCAST_HDR, DEV_BROADCAST_VOLUME, DefWindowProcW,
    DestroyMenu, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW, MF_STRING,
    MSG, PostQuitMessage, RegisterClassW, SW_SHOWNORMAL, SetForegroundWindow, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WM_CONTEXTMENU, WM_DESTROY, WM_DEVICECHANGE,
    WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
};

use common::utf16::wide;

use crate::constants::{
    ID_MENU_EXIT, ID_MENU_OPEN_LOG, INSTANCE_MUTEX, LAUNCH_GRACE_MILLISECONDS, TRAY_ICON_RESOURCE,
    TRAY_ICON_UID, WINDOW_CLASS, WM_TRAYICON,
};
use crate::log::Log;
use crate::volume;
use crate::volume::Announce;

/// Everything the window procedure needs.
struct State {
    log: Log,
    /// When this listener last spawned a launcher, for the grace window below.
    launched: Option<Instant>,
}

impl State {
    /// True when a launcher this listener started is still on its way up.
    /// [`launcherIsOpen`] is the real answer; this covers its one gap, before
    /// the spawned process reaches its own `instance::acquire`.
    fn launchPending(&self) -> bool {
        let window = Duration::from_millis(LAUNCH_GRACE_MILLISECONDS);
        !window.is_zero() && matches!(self.launched, Some(at) if at.elapsed() < window)
    }

    /// Opens the grace window. Only a volume that really produced a launcher
    /// calls this, or a stranger's USB stick would hold off the cartridge
    /// plugged in behind it.
    fn launchStarted(&mut self) {
        self.launched = Some(Instant::now());
    }
}

// Only `run`'s thread ever populates or reads this.
thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

/// Takes ownership of `log` and runs until logout: sweeps the drives already
/// mounted, then pumps messages. Returns early, launching nothing, if another
/// instance already holds the mutex.
pub fn run(log: Log) {
    let Some(_instance) = acquireSingleInstance() else {
        // The `Run` entry can fire twice across a fast logoff/logon, and two
        // listeners racing on one arrival means two launchers on screen.
        log.line("another listener is already running; exiting");
        return;
    };

    // Sweeping drive letters touches removable drives, and an empty card reader
    // would otherwise pop the modal "There is no disk in the drive" box — from a
    // process with no visible window, which is unclosable-looking and inexplicable.
    unsafe { SetErrorMode(SEM_FAILCRITICALERRORS) };

    log.line("listener started");
    STATE.set(Some(State {
        log,
        launched: None,
    }));

    let Some(hwnd) = createHiddenWindow() else {
        logLine("FAILED to create the listener window; exiting");
        return;
    };

    // A courtesy, not a requirement: a cartridge is still launched without one.
    if !addTrayIcon(hwnd) {
        logLine("failed to add the tray icon; continuing without one");
    }

    // After the window exists, so an arrival mid-sweep is queued rather than
    // missed. The launcher check then stops it re-launching what the sweep caught.
    startupSweep();

    messageLoop();
    logLine("listener stopped");
}

/// Runs `f` against the thread-local state. `None` before `run` has populated
/// it, which is any callback that could somehow fire before startup finished.
///
/// `borrow_mut` panics on a nested call, so nothing inside `f` may reach back
/// into `withState` — every caller here is a leaf.
fn withState<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with_borrow_mut(|state| state.as_mut().map(f))
}

/// One log line, dropped if the state does not exist yet.
fn logLine(text: &str) {
    withState(|state| state.log.line(text));
}

// ========== Single Instance ==========

/// Holds the process-wide mutex; dropping it (or exiting) frees the name.
struct InstanceGuard(HANDLE);

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// `Some` if this is the first instance, `None` if one is already running.
/// Same named-mutex pattern as `../../../launcher/src/main.rs`.
fn acquireSingleInstance() -> Option<InstanceGuard> {
    let name = wide(INSTANCE_MUTEX);
    unsafe {
        // A named mutex, not a lock file: the kernel frees the name when the
        // process dies, however it dies, so a crash cannot leave a stale claim.
        let handle = CreateMutexW(ptr::null(), 0, name.as_ptr());
        if handle.is_null() {
            // Never let the guard itself become a reason not to run.
            return Some(InstanceGuard(handle));
        }
        // CreateMutexW succeeds either way; only the error code separates
        // "created it" from "opened someone else's".
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(handle);
            return None;
        }
        Some(InstanceGuard(handle))
    }
}

// ========== Window And Message Loop ==========

/// Registers the class and creates the (never shown) top-level window.
///
/// Top-level rather than `HWND_MESSAGE`: broadcast `WM_DEVICECHANGE` reaches
/// nothing else, and the failure is silent. It is never passed to `ShowWindow`,
/// so it stays invisible with no taskbar button.
fn createHiddenWindow() -> Option<HWND> {
    unsafe {
        let instance = GetModuleHandleW(ptr::null());
        let class_name = wide(WINDOW_CLASS);

        let mut class: WNDCLASSW = std::mem::zeroed();
        class.lpfnWndProc = Some(wndProc);
        class.hInstance = instance;
        class.lpszClassName = class_name.as_ptr();
        if RegisterClassW(&class) == 0 {
            return None;
        }

        let title = wide("Romzeta Listener");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            instance,
            ptr::null(),
        );
        (!hwnd.is_null()).then_some(hwnd)
    }
}

/// Blocks in `GetMessage` until the window is destroyed or the session ends.
/// This call, and not a timer, is why the idle CPU cost is zero.
fn messageLoop() {
    let mut message: MSG = unsafe { std::mem::zeroed() };
    loop {
        // 0 = WM_QUIT, -1 = error. Either way there is nothing left to pump.
        let result = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
        if result <= 0 {
            return;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

/// Handles device arrivals, tray clicks and destruction, and hands everything
/// else to the default handler. The return value's meaning is per-message.
///
/// # Safety
///
/// Called only by Windows, with the arguments it documents for each message.
unsafe extern "system" fn wndProc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_DEVICECHANGE if wparam as u32 == DBT_DEVICEARRIVAL => {
            unsafe { onDeviceArrival(lparam) };
            // TRUE: the message was handled. Device-change broadcasts are
            // documented to be answered this way.
            1
        }
        WM_TRAYICON => {
            // With no `NIM_SETVERSION` call, the shell reports mouse activity
            // the old way: `lparam` is the mouse message itself, the same
            // value a click on a real window would have arrived as.
            if matches!(lparam as u32, WM_RBUTTONUP | WM_CONTEXTMENU) {
                unsafe { showTrayMenu(hwnd) };
            }
            0
        }
        WM_DESTROY => {
            removeTrayIcon(hwnd);
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

// ========== Tray Icon ==========

/// Adds the tray icon. `false` if the shell refused it, which is not fatal —
/// see the call site in `run`.
fn addTrayIcon(hwnd: HWND) -> bool {
    unsafe {
        let instance = GetModuleHandleW(ptr::null());
        let icon = LoadIconW(instance, TRAY_ICON_RESOURCE);

        let mut data: NOTIFYICONDATAW = std::mem::zeroed();
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = hwnd;
        data.uID = TRAY_ICON_UID;
        data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        data.uCallbackMessage = WM_TRAYICON;
        data.hIcon = icon;
        setTip(&mut data.szTip, "Romzeta Listener");

        Shell_NotifyIconW(NIM_ADD, &data) != 0
    }
}

/// Removes the tray icon on the way out, so it doesn't linger as a stale
/// entry in the hidden-icons flyout after the process has already exited.
fn removeTrayIcon(hwnd: HWND) {
    unsafe {
        let mut data: NOTIFYICONDATAW = std::mem::zeroed();
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = hwnd;
        data.uID = TRAY_ICON_UID;
        Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

/// Copies `text` (truncated to fit) into a `NOTIFYICONDATAW` fixed-size
/// UTF-16 field, NUL-terminated.
fn setTip(field: &mut [u16], text: &str) {
    let wide = wide(text);
    let n = wide.len().min(field.len());
    field[..n].copy_from_slice(&wide[..n]);
    // `wide()` already NUL-terminates, so this only bites when truncation above
    // cut that NUL off the end.
    field[n - 1] = 0;
}

/// Builds and shows the right-click menu at the cursor, then acts on
/// whichever item (if any) was picked.
unsafe fn showTrayMenu(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let open_log = wide("Open log");
        let exit = wide("Exit");
        AppendMenuW(menu, MF_STRING, ID_MENU_OPEN_LOG as usize, open_log.as_ptr());
        AppendMenuW(menu, MF_STRING, ID_MENU_EXIT as usize, exit.as_ptr());

        let mut point: POINT = std::mem::zeroed();
        GetCursorPos(&mut point);

        // A window that isn't the foreground window never gets the message that
        // dismisses a popup menu when the user clicks elsewhere — the classic
        // "menu that won't go away" bug. Nothing else ever activates this one.
        SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            hwnd,
            ptr::null(),
        );
        DestroyMenu(menu);

        match cmd as u32 {
            ID_MENU_OPEN_LOG => openLogFile(),
            ID_MENU_EXIT => {
                DestroyWindow(hwnd);
            }
            _ => {}
        }
    }
}

/// Opens the log with whatever the user has associated with `.log` files.
fn openLogFile() {
    withState(|state| {
        let Some(path) = state.log.path() else {
            return;
        };
        let verb = wide("open");
        let file = wide(&path.to_string_lossy());
        unsafe {
            ShellExecuteW(
                ptr::null_mut(),
                verb.as_ptr(),
                file.as_ptr(),
                ptr::null(),
                ptr::null(),
                SW_SHOWNORMAL,
            );
        }
    });
}

// ========== Device Arrival ==========

/// Decodes one `DBT_DEVICEARRIVAL` and runs the shared core over every drive
/// letter it names.
///
/// # Safety
///
/// `lparam` must be the pointer Windows passed with the message: either null,
/// or a valid `DEV_BROADCAST_HDR` whose `dbch_devicetype` says what the rest of
/// the allocation actually is.
unsafe fn onDeviceArrival(lparam: LPARAM) {
    if lparam == 0 {
        return;
    }
    // Ports, interfaces and handles use this message too, with differently
    // shaped payloads. The device type is the only field safe to read first.
    let header = lparam as *const DEV_BROADCAST_HDR;
    if unsafe { (*header).dbch_devicetype } != DBT_DEVTYP_VOLUME {
        return;
    }

    let volume = lparam as *const DEV_BROADCAST_VOLUME;
    let (unitmask, flags) = unsafe { ((*volume).dbcv_unitmask, (*volume).dbcv_flags) };

    // A mapped network share arrives as a volume like any other. It cannot be a
    // cartridge, and probing one can block on an unreachable server.
    if flags & DBTF_NET != 0 {
        logLine("ignored a network drive arrival");
        return;
    }

    // A bitmask, not a single letter: one event can carry several.
    for letter in lettersFromMask(unitmask) {
        handleLetter(letter, "arrival", Announce::Detached);
    }
}

/// Runs the shared core over one drive letter, subject to the drive-type filter
/// and the debounce. `reason` names what brought us here, for the log line.
fn handleLetter(letter: char, reason: &str, announce: Announce) {
    withState(|state| {
        let drive_type = driveType(letter);
        if !isCandidateDrive(drive_type) {
            state.log.line(&format!(
                "{letter}: ignored on {reason}: drive type {drive_type} is not local storage"
            ));
            return;
        }
        // Asked before the volume is touched at all: a second launcher would
        // find the mutex held and exit without ever showing anything.
        if launcherIsOpen() {
            state.log.line(&format!(
                "{letter}: ignored on {reason}: a launcher is already open"
            ));
            return;
        }
        if state.launchPending() {
            state.log.line(&format!(
                "{letter}: ignored on {reason}: a launcher is already starting"
            ));
            return;
        }
        if let volume::Outcome::Launched =
            volume::handleVolume(Path::new(&driveRoot(letter)), &state.log, announce)
        {
            state.launchStarted();
        }
    });
}

/// Whether a launcher already holds its single-instance mutex.
///
/// Opened, never created: creating the name here would leave the *listener*
/// holding it, and every launcher after would exit believing one was open.
fn launcherIsOpen() -> bool {
    let name = wide(common::constants::LAUNCHER_INSTANCE_MUTEX);
    let handle = unsafe { OpenMutexW(SYNCHRONIZE, 0, name.as_ptr()) };
    if handle.is_null() {
        return false;
    }
    unsafe { CloseHandle(handle) };
    true
}

// ========== Volume Enumeration ==========

/// Runs the core over every drive already mounted.
///
/// Without this, a cartridge already connected at boot produces no arrival
/// event, and the failure reads as flakiness rather than as a missing feature.
fn startupSweep() {
    for letter in mountedDriveLetters() {
        handleLetter(letter, "startup sweep", Announce::Never);
    }
}

/// Every drive letter currently mounted, A–Z.
fn mountedDriveLetters() -> impl Iterator<Item = char> {
    // Same bitmask shape as an arrival event, so one expander serves both.
    lettersFromMask(unsafe { GetLogicalDrives() })
}

/// Expands a 26-bit drive-letter bitmask (bit 0 = `A`) into the letters it sets.
fn lettersFromMask(mask: u32) -> impl Iterator<Item = char> {
    (0..26u32)
        .filter(move |bit| mask & (1 << bit) != 0)
        // Arithmetic on the ASCII byte, then back to a char — every drive
        // letter is A–Z, so there is no multi-byte case to worry about.
        .map(|bit| (b'A' + bit as u8) as char)
}

/// A drive letter as a root path, e.g. `E:\`.
fn driveRoot(letter: char) -> String {
    format!("{letter}:\\")
}

/// What kind of drive is mounted at `letter` — one of the Win32 `DRIVE_*`
/// values. `DRIVE_NO_ROOT_DIR` for a letter nothing is mounted at.
fn driveType(letter: char) -> u32 {
    let root = wide(&driveRoot(letter));
    unsafe { GetDriveTypeW(root.as_ptr()) }
}

/// Whether a drive is the kind of thing a cartridge can be: any mounted *local*
/// storage volume — NVMe, SSD, HDD and USB stick alike, never a specific USB id.
///
/// Network shares, optical drives and RAM disks are excluded here rather than
/// left to the missing launcher to reject, because reaching for a file on a
/// stale network mount can block for a long time on the message thread.
fn isCandidateDrive(drive_type: u32) -> bool {
    matches!(drive_type, DRIVE_FIXED | DRIVE_REMOVABLE)
}
