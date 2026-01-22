use git2::build::RepoBuilder;
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
use std::env;
use std::path::Path;

pub fn clone_repo(src: &String, dest: &String, branch: &String) {
    let token = env::var("GITHUB_TOKEN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let mut repo = match Repository::open(dest) {
        Ok(repo) => repo,
        Err(_) => {
            tracing::info!("Cloning `{}` to `{:?}`", src, dest);
            let mut builder = RepoBuilder::new();
            builder.fetch_options(fetch_options(token.as_deref()));
            builder.clone(src, Path::new(dest)).unwrap()
        }
    };

    checkout_branch(&mut repo, branch, dest);

    fetch_latest(&repo, branch, dest, token.as_deref());
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

fn fetch_latest(repo: &Repository, branch: &String, dest: &String, token: Option<&str>) {
    let mut remote = repo.find_remote("origin").unwrap_or_else(|_| {
        // Try to find the first remote as fallback
        let remotes = repo.remotes().expect("Could not list remotes");
        if let Some(name) = remotes.get(0) {
            repo.find_remote(name).expect("Could not get remote")
        } else {
            panic!("No remotes found in repository at `{}`", dest);
        }
    });

    let mut fetch_opts = fetch_options(token);
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

fn fetch_options(token: Option<&str>) -> FetchOptions<'_> {
    let mut callbacks = RemoteCallbacks::new();
    let token = token.map(str::to_owned);
    callbacks.credentials(move |_, username_from_url, _| {
        if let Some(token) = token.as_ref() {
            let username = username_from_url.unwrap_or("x-access-token");
            return Cred::userpass_plaintext(username, token);
        }

        Cred::default()
    });

    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);
    fetch_opts
}
