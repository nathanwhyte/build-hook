use std::path::Path;
use std::process::Command;

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
    Command::new("kubectl")
        .args(&[
            "config",
            "set-cluster",
            "k8s",
            "--server",
            &format!("https://{}", server),
        ])
        .args(&["--certificate-authority", ca_cert])
        .env("KUBECONFIG", kubeconfig_path)
        .output()
        .map_err(|e| format!("Failed to set cluster: {}", e))?;

    // Set credentials
    Command::new("kubectl")
        .args(&["config", "set-credentials", "k8s", "--token", &token])
        .env("KUBECONFIG", kubeconfig_path)
        .output()
        .map_err(|e| format!("Failed to set credentials: {}", e))?;

    // Set context
    Command::new("kubectl")
        .args(&[
            "config",
            "set-context",
            "k8s",
            "--cluster",
            "k8s",
            "--user",
            "k8s",
        ])
        .env("KUBECONFIG", kubeconfig_path)
        .output()
        .map_err(|e| format!("Failed to set context: {}", e))?;

    // Use context
    Command::new("kubectl")
        .args(&["config", "use-context", "k8s"])
        .env("KUBECONFIG", kubeconfig_path)
        .output()
        .map_err(|e| format!("Failed to use context: {}", e))?;

    Ok(())
}

fn check_builder_exists() -> Result<bool, String> {
    let output = Command::new("docker")
        .args(&["buildx", "ls"])
        .output()
        .map_err(|e| format!("Failed to list buildx builders: {}", e))?;

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
    let output = Command::new("docker")
        .args(&["buildx", "use", BUILDER_NAME])
        .output()
        .map_err(|e| format!("Failed to use builder: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to use builder: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

fn create_builder() -> Result<(), String> {
    let output = Command::new("docker")
        .args(&[
            "buildx",
            "create",
            "--driver",
            "kubernetes",
            "--name",
            BUILDER_NAME,
        ])
        .args(&["--driver-opt", &format!("namespace={}", NAMESPACE)])
        .args(&["--use"])
        .output()
        .map_err(|e| format!("Failed to create buildx builder: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create builder: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

fn bootstrap_builder() -> Result<(), String> {
    let output = Command::new("docker")
        .args(&["buildx", "inspect", "--bootstrap"])
        .output()
        .map_err(|e| format!("Failed to bootstrap builder: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to bootstrap builder: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}
