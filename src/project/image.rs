use std::path::Path;

pub struct BuildImage {
    pub tag: String,
    pub dockerfile_path: String,
    pub context_dir: String,
}

pub fn build_images(mut image_builds: Vec<BuildImage>, repo_dest: String) -> Result<(), String> {
    for build in &image_builds {
        if !Path::new(&build.dockerfile_path).is_file() {
            return Err(format!(
                "Dockerfile for {} not found at {}",
                build.tag, build.dockerfile_path
            ));
        }
    }

    let first_build = image_builds
        .drain(0..1)
        .next()
        .ok_or_else(|| "project.image must have at least one entry!".to_string())?;

    tracing::info!(
        "building {} using {}",
        first_build.tag,
        first_build.dockerfile_path
    );

    let mut child = spawn_build(
        &first_build.context_dir,
        &first_build.tag,
        &first_build.dockerfile_path,
    )?;
    verify_build_started(&mut child)?;

    handle_build_completion(child, &first_build.tag)?;

    for build in image_builds {
        tracing::info!("building {} using {}", build.tag, build.dockerfile_path);
        let mut next_child = spawn_build(&build.context_dir, &build.tag, &build.dockerfile_path)?;
        verify_build_started(&mut next_child)
            .map_err(|e| format!("Build process for {} exited immediately: {}", build.tag, e))?;
        handle_build_completion(next_child, &build.tag)?;
    }

    if let Err(e) = std::fs::remove_dir_all(&repo_dest) {
        tracing::warn!(
            "Failed to remove temporary repository directory {}: {}",
            repo_dest,
            e
        );
    }

    Ok(())
}

fn spawn_build(
    context_dir: &str,
    image_tag: &str,
    dockerfile_path: &str,
) -> Result<std::process::Child, String> {
    std::process::Command::new("docker")
        .args([
            "buildx",
            "build",
            "--builder",
            "builder",
            "--no-cache",
            "--push",
            "-t",
            image_tag,
            "--file",
            dockerfile_path,
            context_dir,
        ])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to execute docker buildx: {}", e))
}

fn verify_build_started(child: &mut std::process::Child) -> Result<(), String> {
    match child.try_wait() {
        Ok(Some(status)) => {
            if !status.success() {
                return Err(format!(
                    "Build process exited immediately with code: {:?}",
                    status.code()
                ));
            }
        }
        Ok(None) => {}
        Err(e) => {
            return Err(format!("Failed to check build process status: {}", e));
        }
    }

    Ok(())
}

fn handle_build_completion(child: std::process::Child, image_tag: &str) -> Result<(), String> {
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(e) => {
            return Err(format!("Failed to wait for build process: {}", e));
        }
    };

    if !output.stdout.is_empty() {
        tracing::debug!(
            "Build stdout for {}: {}",
            image_tag,
            String::from_utf8_lossy(&output.stdout)
        );
    }
    if !output.status.success() && !output.stderr.is_empty() {
        tracing::warn!(
            "Build stderr for {}: {}",
            image_tag,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if !output.status.success() {
        return Err(format!(
            "Build failed for {} with exit code: {:?}",
            image_tag,
            output.status.code()
        ));
    } else {
        tracing::info!("Successfully built and pushed image: {}", image_tag);
    }

    Ok(())
}
