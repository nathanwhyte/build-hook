use git2::build::RepoBuilder;
use git2::{Cred, FetchOptions, RemoteCallbacks};
use std::fs;
use std::path::Path;

pub fn clone_repo(github_token: &str, src: &String, dest: &String, branch: &str) {
    let dest_path = Path::new(dest);
    if dest_path.exists() {
        tracing::info!("Removing existing repo at `{}`", dest);
        if dest_path.is_dir() {
            fs::remove_dir_all(dest_path).unwrap_or_else(|err| {
                panic!("Failed to remove repository directory `{}`: {}", dest, err)
            });
        } else {
            fs::remove_file(dest_path).unwrap_or_else(|err| {
                panic!("Failed to remove repository file `{}`: {}", dest, err)
            });
        }
    }

    tracing::info!("Cloning `{}` to `{:?}`", src, dest);
    let mut builder = RepoBuilder::new();
    builder.fetch_options(get_fetch_options(github_token));
    builder.branch(branch);
    let _ = builder.clone(src, dest_path).unwrap();
}

fn get_fetch_options(token: &str) -> FetchOptions<'_> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |_, username_from_url, _| {
        if !token.is_empty() {
            let username = username_from_url.unwrap_or("x-access-token");
            return Cred::userpass_plaintext(username, token);
        }

        Cred::default()
    });

    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);
    fetch_opts
}
