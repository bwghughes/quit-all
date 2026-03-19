use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};
use objc2_foundation::NSArray;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

const SAFELIST: &[&str] = &[
    "com.apple.finder",
    "com.apple.systempreferences",
];

#[derive(Debug, Deserialize, Default, PartialEq)]
struct Config {
    #[serde(default)]
    whitelist: Vec<String>,
}

fn default_config_path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join(".config").join("quit-all.json")
}

fn load_config(path: &Path) -> Config {
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
            eprintln!("Warning: failed to parse {}: {}", path.display(), e);
            Config::default()
        }),
        Err(_) => Config::default(),
    }
}

fn parse_args(args: &[String]) -> (bool, bool) {
    let force = args.contains(&"--force".to_string());
    let dry_run = args.contains(&"--dry-run".to_string());
    (force, dry_run)
}

#[derive(Debug, Clone, PartialEq)]
enum AppAction {
    SkipSelf,
    SkipSafelisted,
    SkipWhitelisted,
    SkipBackground,
    Quit,
}

fn decide_action(
    bundle_id: &str,
    pid: i32,
    my_pid: i32,
    is_regular_policy: bool,
    whitelist: &[String],
) -> AppAction {
    if pid == my_pid {
        return AppAction::SkipSelf;
    }
    if SAFELIST.contains(&bundle_id) {
        return AppAction::SkipSafelisted;
    }
    if whitelist.iter().any(|w| w == bundle_id) {
        return AppAction::SkipWhitelisted;
    }
    if !is_regular_policy {
        return AppAction::SkipBackground;
    }
    AppAction::Quit
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (force, dry_run) = parse_args(&args);
    let config = load_config(&default_config_path());
    quit_all_apps(force, dry_run, &config);
}

fn quit_all_apps(force: bool, dry_run: bool, config: &Config) {
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

        let policy = unsafe { app.activationPolicy() };
        let is_regular = policy == NSApplicationActivationPolicy::Regular;

        match decide_action(&bundle_id, pid, my_pid, is_regular, &config.whitelist) {
            AppAction::SkipSelf | AppAction::SkipBackground => continue,
            AppAction::SkipSafelisted | AppAction::SkipWhitelisted => {
                println!("  skipping: {}", app_name);
                continue;
            }
            AppAction::Quit => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_args_no_flags() {
        let args = vec!["quit-all".to_string()];
        assert_eq!(parse_args(&args), (false, false));
    }

    #[test]
    fn parse_args_force() {
        let args = vec!["quit-all".to_string(), "--force".to_string()];
        assert_eq!(parse_args(&args), (true, false));
    }

    #[test]
    fn parse_args_dry_run() {
        let args = vec!["quit-all".to_string(), "--dry-run".to_string()];
        assert_eq!(parse_args(&args), (false, true));
    }

    #[test]
    fn parse_args_both_flags() {
        let args = vec![
            "quit-all".to_string(),
            "--force".to_string(),
            "--dry-run".to_string(),
        ];
        assert_eq!(parse_args(&args), (true, true));
    }

    #[test]
    fn skip_own_process() {
        assert_eq!(
            decide_action("com.example.app", 100, 100, true, &[]),
            AppAction::SkipSelf
        );
    }

    #[test]
    fn skip_finder() {
        assert_eq!(
            decide_action("com.apple.finder", 200, 100, true, &[]),
            AppAction::SkipSafelisted
        );
    }

    #[test]
    fn skip_system_preferences() {
        assert_eq!(
            decide_action("com.apple.systempreferences", 300, 100, true, &[]),
            AppAction::SkipSafelisted
        );
    }

    #[test]
    fn skip_background_app() {
        assert_eq!(
            decide_action("com.example.daemon", 400, 100, false, &[]),
            AppAction::SkipBackground
        );
    }

    #[test]
    fn quit_regular_app() {
        assert_eq!(
            decide_action("com.example.app", 500, 100, true, &[]),
            AppAction::Quit
        );
    }

    #[test]
    fn quit_app_with_empty_bundle_id() {
        assert_eq!(
            decide_action("", 600, 100, true, &[]),
            AppAction::Quit
        );
    }

    #[test]
    fn safelist_checked_before_policy() {
        assert_eq!(
            decide_action("com.apple.finder", 200, 100, false, &[]),
            AppAction::SkipSafelisted
        );
    }

    #[test]
    fn self_check_takes_priority() {
        assert_eq!(
            decide_action("com.apple.finder", 100, 100, true, &[]),
            AppAction::SkipSelf
        );
    }

    #[test]
    fn safelist_contains_expected_entries() {
        assert!(SAFELIST.contains(&"com.apple.finder"));
        assert!(SAFELIST.contains(&"com.apple.systempreferences"));
        assert!(!SAFELIST.contains(&"com.apple.safari"));
    }

    // Config / whitelist tests

    #[test]
    fn whitelist_skips_app() {
        let whitelist = vec!["com.example.keep".to_string()];
        assert_eq!(
            decide_action("com.example.keep", 500, 100, true, &whitelist),
            AppAction::SkipWhitelisted
        );
    }

    #[test]
    fn whitelist_does_not_affect_other_apps() {
        let whitelist = vec!["com.example.keep".to_string()];
        assert_eq!(
            decide_action("com.example.other", 500, 100, true, &whitelist),
            AppAction::Quit
        );
    }

    #[test]
    fn safelist_takes_priority_over_whitelist() {
        let whitelist = vec!["com.apple.finder".to_string()];
        assert_eq!(
            decide_action("com.apple.finder", 200, 100, true, &whitelist),
            AppAction::SkipSafelisted
        );
    }

    #[test]
    fn self_takes_priority_over_whitelist() {
        let whitelist = vec!["com.example.app".to_string()];
        assert_eq!(
            decide_action("com.example.app", 100, 100, true, &whitelist),
            AppAction::SkipSelf
        );
    }

    #[test]
    fn whitelist_checked_before_policy() {
        let whitelist = vec!["com.example.bg".to_string()];
        assert_eq!(
            decide_action("com.example.bg", 500, 100, false, &whitelist),
            AppAction::SkipWhitelisted
        );
    }

    #[test]
    fn load_config_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"whitelist": ["com.example.keep", "org.foo.bar"]}}"#).unwrap();

        let config = load_config(&path);
        assert_eq!(config.whitelist, vec!["com.example.keep", "org.foo.bar"]);
    }

    #[test]
    fn load_config_missing_file_returns_default() {
        let config = load_config(Path::new("/nonexistent/path/config.json"));
        assert_eq!(config, Config::default());
        assert!(config.whitelist.is_empty());
    }

    #[test]
    fn load_config_invalid_json_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "not json {{{").unwrap();

        let config = load_config(&path);
        assert_eq!(config, Config::default());
    }

    #[test]
    fn load_config_empty_whitelist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{}"#).unwrap();

        let config = load_config(&path);
        assert!(config.whitelist.is_empty());
    }

    #[test]
    fn load_config_empty_array() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"whitelist": []}"#).unwrap();

        let config = load_config(&path);
        assert!(config.whitelist.is_empty());
    }
}
