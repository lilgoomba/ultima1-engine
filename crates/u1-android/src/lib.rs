//! The engine on Android: SDL2's Java activity loads this library
//! (libmain.so) and calls `SDL_main` with the arguments our activity gives
//! (`--game-dir <app files>/ULTIMA1`). The program itself is the desktop
//! one (crates/u1/src/main.rs, compiled in here), with the touch controls
//! on. See docs/ANDROID.md.
//!
//! Also exported: `ImportActivity.missingFiles`, the same check of the game
//! folder the desktop program makes, for the first-run import.

#[allow(dead_code)]
#[path = "../../u1/src/main.rs"]
mod app;

use std::ffi::{c_char, c_int, CStr, CString};
use std::path::Path;

/// Called by SDL on its own thread once the activity is up.
///
/// # Safety
/// `argv` holds `argc` C strings (SDL's contract).
#[no_mangle]
pub unsafe extern "C" fn SDL_main(argc: c_int, argv: *const *const c_char) -> c_int {
    let args: Vec<String> = (0..argc.max(0) as usize).map(|i| CStr::from_ptr(*argv.add(i)).to_string_lossy().into_owned()).collect();
    // The app's private folder: the game files' parent. The config and
    // the saves go beside them (~/.config and ~/.local/share there).
    let files = args.windows(2).find(|w| w[0] == "--game-dir").and_then(|w| Path::new(&w[1]).parent().map(Path::to_path_buf));
    if let Some(f) = &files {
        std::env::set_var("HOME", f);
        std::env::set_var("XDG_CONFIG_HOME", f.join("config"));
        std::env::set_var("XDG_DATA_HOME", f.join("data"));
        // Messages for stderr go to a log file there (Android has no
        // terminal).
        if let Ok(log) = std::fs::OpenOptions::new().create(true).append(true).open(f.join("u1.log")) {
            use std::os::fd::AsRawFd;
            libc::dup2(log.as_raw_fd(), 2);
        }
    }
    std::env::set_var("U1_TOUCH", "1");
    let code = app::run(args);
    if code == std::process::ExitCode::SUCCESS {
        0
    } else {
        1
    }
}

/// `String ImportActivity.missingFiles(String dir)`: "" when the folder
/// holds everything the game needs, else what is missing.
///
/// # Safety
/// Called by the JVM with a valid environment and string.
#[no_mangle]
pub unsafe extern "system" fn Java_org_u1engine_ImportActivity_missingFiles(env: *mut jni_sys::JNIEnv, _cls: jni_sys::jclass, dir: jni_sys::jstring) -> jni_sys::jstring {
    let f = &**env;
    let chars = (f.GetStringUTFChars.unwrap())(env, dir, std::ptr::null_mut());
    let d = CStr::from_ptr(chars).to_string_lossy().into_owned();
    (f.ReleaseStringUTFChars.unwrap())(env, dir, chars);
    let msg = match u1_assets::gamedir::GameDir::new(&d) {
        Ok(g) => g.missing(true).join(", "),
        Err(e) => e.to_string(),
    };
    let c = CString::new(msg).unwrap_or_default();
    (f.NewStringUTF.unwrap())(env, c.as_ptr())
}
