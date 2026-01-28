use std::process::{Command, Output};

const BUILDER_NAME: &str = "builder";
// BuildKit daemon endpoint (deployed separately via k8s.yaml)
const BUILDKITD_ENDPOINT: &str = "tcp://buildkitd.build.svc.cluster.local:1234";

pub fn initialize() -> Result<(), String> {
    tracing::info!(
        "Initializing buildx builder: {} with remote endpoint: {}",
        BUILDER_NAME,
        BUILDKITD_ENDPOINT
    );

    // Ensure Docker config directory exists (if not already created by volume mount)
    // Ignore errors as the directory may already exist or be created by volume mounts
    let _ = std::fs::create_dir_all("/root/.docker");

    // Check if builder already exists
    let builder_exists = check_builder_exists()?;

    if builder_exists {
        tracing::info!(
            "Builder {} already exists, using existing builder",
            BUILDER_NAME
        );
        use_builder()?;
    } else {
        tracing::info!("Creating new buildx builder: {}", BUILDER_NAME);
        create_builder()?;
        bootstrap_builder()?;
    }

    tracing::info!("Buildx builder ready");
    Ok(())
}

fn run_command_output(command: &mut Command, description: &str) -> Result<Output, String> {
    let output = command
        .output()
        .map_err(|e| format!("Failed to run {}: {}", description, e))?;

    if !output.status.success() && !output.stderr.is_empty() {
        tracing::warn!(
            "{} stderr: {}",
            description,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(output)
}

fn check_builder_exists() -> Result<bool, String> {
    let output = run_command_output(
        Command::new("docker").args(["buildx", "ls"]),
        "docker buildx ls",
    )?;

    if !output.status.success() {
        return Err(format!(
            "Failed to list builders: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains(BUILDER_NAME))
}

fn use_builder() -> Result<(), String> {
    let output = run_command_output(
        Command::new("docker").args(["buildx", "use", BUILDER_NAME]),
        "docker buildx use",
    )?;

    if !output.status.success() {
        return Err("Failed to use builder".to_string());
    }

    Ok(())
}

fn create_builder() -> Result<(), String> {
    // Use the remote driver to connect to buildkitd via TCP.
    // This avoids the cgroup v2 exec issues with the kubernetes driver.
    let output = run_command_output(
        Command::new("docker").args([
            "buildx",
            "create",
            "--driver",
            "remote",
            "--name",
            BUILDER_NAME,
            BUILDKITD_ENDPOINT,
            "--use",
        ]),
        "docker buildx create",
    )?;

    if !output.status.success() {
        return Err("Failed to create builder".to_string());
    }

    Ok(())
}

fn bootstrap_builder() -> Result<(), String> {
    let output = run_command_output(
        Command::new("docker").args(["buildx", "inspect", "--bootstrap"]),
        "docker buildx inspect --bootstrap",
    )?;

    if !output.status.success() {
        return Err("Failed to bootstrap builder".to_string());
    }

    Ok(())
}
