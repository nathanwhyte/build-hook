use std::fs;
use std::path::Path;
use std::process::{Command, Output};

pub fn clone_repo(
    github_token: &str,
    src: &String,
    dest: &String,
    branch: &str,
) -> Result<(), String> {
    let dest_path = Path::new(dest);
    if dest_path.exists() {
        tracing::info!("Removing existing repo at `{}`", dest);
        if dest_path.is_dir() {
            fs::remove_dir_all(dest_path).map_err(|err| {
                format!("Failed to remove repository directory `{}`: {}", dest, err)
            })?;
        } else {
            fs::remove_file(dest_path)
                .map_err(|err| format!("Failed to remove repository file `{}`: {}", dest, err))?;
        }
    }

    tracing::info!("Cloning `{}` to `{:?}`", src, dest);

    let clone_url = with_github_credentials(src, github_token)?;
    let output = run_command_output(
        Command::new("git")
            .args(["clone", "--branch", branch, "--single-branch"])
            .arg(clone_url)
            .arg(dest)
            .env("GIT_TERMINAL_PROMPT", "0"),
        "git clone",
    )?;

    if !output.status.success() {
        return Err("Failed to clone repository".to_string());
    }

    Ok(())
}

fn run_command_output(command: &mut Command, description: &str) -> Result<Output, String> {
    let output = command
        .output()
        .map_err(|err| format!("Failed to run {}: {}", description, err))?;

    if !output.status.success() {
        tracing::warn!("{} failed", description);
    }

    Ok(output)
}

fn with_github_credentials(src: &str, github_token: &str) -> Result<String, String> {
    if github_token.is_empty() {
        return Ok(src.to_string());
    }

    if let Some((scheme, rest)) = src.split_once("://") {
        return Ok(format!(
            "{}://x-access-token:{}@{}",
            scheme, github_token, rest
        ));
    }

    Err(format!("Unsupported repository URL format: {}", src))
}
