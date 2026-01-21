use std::path::Path;

use git2::Repository;

pub fn clone(src: &String, dest: &String) -> Repository {
    if Path::new(dest).exists() {
        tracing::info!("Destination `{:?}` already exists, skipping clone", dest);
        return Repository::open(dest).unwrap();
    }

    tracing::info!("Cloning `{}` to `{:?}`", src, dest);

    let repo: Repository = match Repository::clone(src, dest) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };

    tracing::info!("Cloned repository to `{}`", repo.path().display());

    repo
}
