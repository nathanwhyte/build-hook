use std::process::{Command, Output};

pub fn rollout_restart(namespace: &str, resources: &[String]) -> Result<(), String> {
    for resource in resources {
        tracing::info!(
            "Restarting resource `{}` in namespace `{}`",
            resource,
            namespace
        );
        let output = run_command_output(
            Command::new("kubectl").args([
                "rollout",
                "restart",
                "-n",
                namespace,
                resource,
            ]),
            "kubectl rollout restart",
        )?;

        if !output.status.success() {
            return Err(format!(
                "Failed to restart `{}` in namespace `{}`: {}",
                resource,
                namespace,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    Ok(())
}

fn run_command_output(command: &mut Command, description: &str) -> Result<Output, String> {
    let output = command
        .output()
        .map_err(|err| format!("Failed to run {}: {}", description, err))?;

    if !output.status.success() && !output.stderr.is_empty() {
        tracing::warn!(
            "{} stderr: {}",
            description,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(output)
}
