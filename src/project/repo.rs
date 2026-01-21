use git2::Repository;

pub fn clone_repo(src: &String, dest: &String, branch: &String) -> Repository {
    let mut repo = match Repository::open(dest) {
        Ok(repo) => repo,
        Err(_) => {
            tracing::info!("Cloning `{}` to `{:?}`", src, dest);
            Repository::clone(src, dest).unwrap()
        }
    };

    checkout_branch(&mut repo, branch, dest);

    fetch_latest(&repo, branch, dest);

    repo
}

fn checkout_branch(repo: &mut Repository, branch: &String, dest: &String) {
    let repo_ref = repo
        .find_reference(&format!("refs/remotes/origin/{}", branch))
        .or_else(|_| repo.find_reference(&format!("refs/heads/{}", branch)))
        .unwrap_or_else(|_| {
            panic!(
                "Branch `{}` not found in cloned repository at `{}`",
                branch, dest
            )
        });

    let object = repo_ref
        .peel(git2::ObjectType::Commit)
        .expect("Could not peel branch to commit");

    repo.checkout_tree(&object, None)
        .expect("Failed to checkout tree");

    repo.set_head(&format!("refs/heads/{}", branch))
        .expect("Failed to set HEAD to branch");

    tracing::info!("Checked out branch `{}`", branch);
}

fn fetch_latest(repo: &Repository, branch: &String, dest: &String) {
    let mut remote = repo.find_remote("origin").unwrap_or_else(|_| {
        // Try to find the first remote as fallback
        let remotes = repo.remotes().expect("Could not list remotes");
        if let Some(name) = remotes.get(0) {
            repo.find_remote(name).expect("Could not get remote")
        } else {
            panic!("No remotes found in repository at `{}`", dest);
        }
    });

    let mut fetch_opts = git2::FetchOptions::new();
    // Do not specify credentials callback; uses default since this repo expects https/public
    let refspecs = [&format!(
        "refs/heads/{}:refs/remotes/origin/{}",
        branch, branch
    )];

    tracing::info!(
        "Fetching latest changes from remote `origin` for branch `{}` in `{}`",
        branch,
        dest
    );

    remote
        .fetch(&refspecs, Some(&mut fetch_opts), None)
        .map_err(|e| {
            tracing::error!("Failed to fetch from remote: {}", e);
            e
        })
        .ok();
}
