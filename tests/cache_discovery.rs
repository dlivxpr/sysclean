use std::collections::HashMap;
use std::path::PathBuf;

use sysclean::cache_cleaner::{
    CacheTargetKind, CommandOutput, CommandRunner, discover_cache_target,
};

#[derive(Default)]
struct StubRunner {
    commands: HashMap<String, Result<CommandOutput, String>>,
}

impl StubRunner {
    fn with_command(mut self, command: &str, output: Result<CommandOutput, String>) -> Self {
        self.commands.insert(command.to_string(), output);
        self
    }
}

impl CommandRunner for StubRunner {
    fn run(&self, program: &str, args: &[&str]) -> anyhow::Result<CommandOutput> {
        let key = if args.is_empty() {
            program.to_string()
        } else {
            format!("{program} {}", args.join(" "))
        };

        self.commands
            .get(&key)
            .cloned()
            .unwrap_or_else(|| Err(format!("missing stub for {key}")))
            .map_err(anyhow::Error::msg)
    }
}

#[test]
fn cargo_discovery_uses_default_user_profile_paths() {
    let discovered = discover_cache_target(
        CacheTargetKind::Cargo,
        &StubRunner::default(),
        Some(PathBuf::from(r"C:\Users\demo")),
        Some(PathBuf::from(r"C:\Users\demo\AppData\Local")),
    )
    .expect("cargo discovery should succeed");

    let paths: Vec<String> = discovered
        .paths
        .iter()
        .map(|path| path.display().to_string())
        .collect();

    assert!(paths.contains(&r"C:\Users\demo\.cargo\registry".to_string()));
    assert!(paths.contains(&r"C:\Users\demo\.cargo\git".to_string()));
}

#[test]
fn uv_discovery_prefers_command_result_before_default_path() {
    let discovered = discover_cache_target(
        CacheTargetKind::Uv,
        &StubRunner::default().with_command(
            "uv cache dir",
            Ok(CommandOutput::success(r"C:\custom\uv-cache")),
        ),
        Some(PathBuf::from(r"C:\Users\demo")),
        Some(PathBuf::from(r"C:\Users\demo\AppData\Local")),
    )
    .expect("uv discovery should succeed");

    assert_eq!(discovered.paths, vec![PathBuf::from(r"C:\custom\uv-cache")]);
}

#[test]
fn npm_discovery_falls_back_to_local_app_data_when_command_is_missing() {
    let discovered = discover_cache_target(
        CacheTargetKind::Npm,
        &StubRunner::default(),
        Some(PathBuf::from(r"C:\Users\demo")),
        Some(PathBuf::from(r"C:\Users\demo\AppData\Local")),
    )
    .expect("npm discovery should succeed");

    assert_eq!(
        discovered.paths,
        vec![PathBuf::from(r"C:\Users\demo\AppData\Local\npm-cache")]
    );
}

#[test]
fn docker_discovery_marks_target_available_when_cli_responds() {
    let discovered = discover_cache_target(
        CacheTargetKind::Docker,
        &StubRunner::default().with_command(
            "docker system df --format json",
            Ok(CommandOutput::success(
                r#"{"Type":"Build Cache","Size":"1.5GB","Reclaimable":"1.2GB (80%)"}"#,
            )),
        ),
        Some(PathBuf::from(r"C:\Users\demo")),
        Some(PathBuf::from(r"C:\Users\demo\AppData\Local")),
    )
    .expect("docker discovery should succeed");

    assert!(discovered.available);
    assert_eq!(discovered.reclaimable_bytes, Some(1_200_000_000));
}
