use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};
use objc2_foundation::NSArray;
use std::thread;
use std::time::Duration;

const SAFELIST: &[&str] = &[
    "com.apple.finder",
    "com.apple.systempreferences",
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let force = args.contains(&"--force".to_string());
    let dry_run = args.contains(&"--dry-run".to_string());
    quit_all_apps(force, dry_run);
}

fn quit_all_apps(force: bool, dry_run: bool) {
    let workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let running_apps: objc2::rc::Retained<NSArray<NSRunningApplication>> =
        unsafe { workspace.runningApplications() };

    let my_pid = unsafe { libc::getpid() };
    let mut quit_count = 0;

    let count = running_apps.len();
    for i in 0..count {
        let app = unsafe { running_apps.objectAtIndex(i) };

        let pid: libc::pid_t = unsafe { objc2::msg_send![&*app, processIdentifier] };

        let bundle_id = unsafe { app.bundleIdentifier() }
            .map(|s| s.to_string())
            .unwrap_or_default();

        let app_name = unsafe { app.localizedName() }
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("PID:{}", pid));

        if pid == my_pid {
            continue;
        }

        if SAFELIST.contains(&bundle_id.as_str()) {
            println!("  skipping: {}", app_name);
            continue;
        }

        let policy = unsafe { app.activationPolicy() };
        if policy != NSApplicationActivationPolicy::Regular {
            continue;
        }

        if dry_run {
            println!("  would quit: {}", app_name);
            continue;
        }

        println!("  quitting: {}", app_name);
        let success = if force {
            unsafe { app.forceTerminate() }
        } else {
            unsafe { app.terminate() }
        };

        if success {
            quit_count += 1;
        } else {
            eprintln!("  failed to quit: {}", app_name);
        }
    }

    if !dry_run {
        if !force {
            println!("\nWaiting for apps to save...");
            thread::sleep(Duration::from_secs(2));
        }
        println!("\nQuit {} apps.", quit_count);
    }
}