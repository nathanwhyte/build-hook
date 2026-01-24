use std::path::Path;
use std::process::{Command, Output};

const BUILDER_NAME: &str = "builder";
const NAMESPACE: &str = "build";

pub fn initialize() -> Result<(), String> {
    tracing::info!(
        "Initializing buildx builder: {} in namespace: {}",
        BUILDER_NAME,
        NAMESPACE
    );

    // Set up kubeconfig if running in Kubernetes
    setup_kubeconfig()?;

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

fn setup_kubeconfig() -> Result<(), String> {
    let token_path = "/var/run/secrets/kubernetes.io/serviceaccount/token";
    if !Path::new(token_path).exists() {
        tracing::warn!("Service account token not found, skipping kubeconfig setup");
        return Ok(());
    }

    let server_host = std::env::var("KUBERNETES_SERVICE_HOST")
        .map_err(|_| "KUBERNETES_SERVICE_HOST not set".to_string())?;
    let server_port = std::env::var("KUBERNETES_SERVICE_PORT")
        .map_err(|_| "KUBERNETES_SERVICE_PORT not set".to_string())?;
    let server = format!("{}:{}", server_host, server_port);
    let ca_cert = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt";
    let token = std::fs::read_to_string(token_path)
        .map_err(|e| format!("Failed to read service account token: {}", e))?;

    let kubeconfig_path = "/tmp/kubeconfig";

    // Set cluster
    let output = run_command_output(
        Command::new("kubectl")
            .args([
                "config",
                "set-cluster",
                "k8s",
                "--server",
                &format!("https://{}", server),
            ])
            .args(["--certificate-authority", ca_cert])
            .env("KUBECONFIG", kubeconfig_path),
        "kubectl set-cluster",
    )?;
    if !output.status.success() {
        return Err("Failed to set cluster".to_string());
    }

    // Set credentials
    let output = run_command_output(
        Command::new("kubectl")
            .args(["config", "set-credentials", "k8s", "--token", &token])
            .env("KUBECONFIG", kubeconfig_path),
        "kubectl set-credentials",
    )?;
    if !output.status.success() {
        return Err("Failed to set credentials".to_string());
    }

    // Set context
    let output = run_command_output(
        Command::new("kubectl")
            .args([
                "config",
                "set-context",
                "k8s",
                "--cluster",
                "k8s",
                "--user",
                "k8s",
            ])
            .env("KUBECONFIG", kubeconfig_path),
        "kubectl set-context",
    )?;
    if !output.status.success() {
        return Err("Failed to set context".to_string());
    }

    // Use context
    let output = run_command_output(
        Command::new("kubectl")
            .args(["config", "use-context", "k8s"])
            .env("KUBECONFIG", kubeconfig_path),
        "kubectl use-context",
    )?;
    if !output.status.success() {
        return Err("Failed to use context".to_string());
    }

    Ok(())
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
    let mut command = Command::new("docker");
    command
        .args([
            "buildx",
            "create",
            "--driver",
            "kubernetes",
            "--name",
            BUILDER_NAME,
        ])
        .args(["--driver-opt", &format!("namespace={}", "build")])
        .args(["--driver-opt", &format!("replicas={}", 1)])
        .args(["--driver-opt", &format!("requests.cpu={}", "2")])
        .args(["--driver-opt", &format!("requests.memory={}", "2Gi")])
        .args(["--driver-opt", &format!("limits.cpu={}", "4")])
        .args(["--driver-opt", &format!("limits.memory={}", "4Gi")]);

    command.args(["--use"]);

    let output = run_command_output(&mut command, "docker buildx create")?;

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
