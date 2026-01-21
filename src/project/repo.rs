use git2::Repository;

pub fn clone(src: &String, name: &String) {
    let dest = format!("/tmp/{}", name);

    tracing::info!("Cloning `{}` to `{}`", src, dest);
    let repo: Repository = match Repository::clone(src, dest) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };

    tracing::info!("Cloned repository to `{}`", repo.path().display());
}

