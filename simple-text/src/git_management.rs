use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use chrono::{Local};

use git2::{BranchType, Cred, PushOptions, Remote, RemoteCallbacks, Repository, Signature, Tree};

fn decrypt(file: &Path) {
    Command::new("unscramble ")
    .args([file.as_os_str()])
    .output()
    .expect("Could not unscramble");
}

fn encrypt(file: &Path) {
    Command::new("scramble ")
    .args([file.as_os_str()])
    .output()
    .expect("Could not unscramble");
}

fn get_now() -> String {
    let local_time = Local::now();
    format!("{}", local_time.format("%d-%b-%y:%H-%M"))
}

pub fn push_to_repo(repo: &Repository) {
    // Prepare callbacks
    let ssh_key_str: &String = &format!("{}/.ssh/id_edSOMETHING", env::var("HOME").unwrap());
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            Path::new(ssh_key_str),
            None,
        )
    });

    // Pedantics taken from example
    // https://docs.rs/git2/latest/git2/build/struct.RepoBuilder.html
    let mut binding = PushOptions::new();
    let push_options = binding.remote_callbacks(callbacks);
    let mut remote: Remote = repo.find_remote("mono").expect("oopsies");
    let refspec = "refs/heads/api/simple-text:refs/heads/api/simple-text";

    remote.push(&[refspec], Some(push_options)).unwrap();
}

fn clone_repo(repo_dir: &Path) -> Repository {
    // Largely based on example
    // https://docs.rs/git2/latest/git2/build/struct.RepoBuilder.html

    // "Prepare callbacks"
    let ssh_key_str: &String = &format!("{}/.ssh/id_edSOMETHING", env::var("HOME").unwrap());
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            Path::new(ssh_key_str),
            None,
        )
    });

    // Pedantics taken from example
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    builder.clone("git@github.com:mono", repo_dir).unwrap()
}

pub fn open_repo() -> Repository {
    let repo_dir_str = &format!("{}./mono_simple_text", env::var("HOME").unwrap());
    let repo_dir: &Path = Path::new(repo_dir_str);

    // Check firstly if we already have the repo cloned
    let repo = match Repository::open(repo_dir) {
        Ok(repo) => repo,
        Err(_) => {
            // If error is relevant (?) clone dir
            clone_repo(repo_dir)
        }
    };

    // Ensure branch is correct
    let branch_name = "api/simple-text";
    match repo.find_branch(branch_name, BranchType::Remote) {
        Ok(branch) => branch,
        Err(_) => {
            // Create new branch from Main
            // This is seeming a bit tricky. For now just complain
            panic!("Please create a branch api/simple-text to use.")
        }
    };

    // Change head before this
    repo.set_head(branch_name).unwrap();
    repo.checkout_head(None).unwrap();

    repo
}

pub fn modify_buffer(buf_name: &String, buf_text: &String) {
    let buf_path_str = format!(
        "{}./mono_simple_text/mono/text/buffer/{}",
        env::var("HOME").unwrap(),
        buf_name
    );
    let buf_path = Path::new(&buf_path_str);

    decrypt(buf_path);

    let mut file = OpenOptions::new()
        .append(true)
        .open(buf_path)
        .expect("File failed to open.");

    file.write(&format!("\n{}\n", buf_text).as_bytes())
        .expect("Write failed.");

    encrypt(buf_path);
}

pub fn add_buffer(buf_name: &String, repo: &Repository) {
    let mut index = repo.index().unwrap();

    let buf_path_str = format!(
        "{}./mono_simple_text/mono/text/buffer/{}",
        env::var("HOME").unwrap(),
        buf_name
    );
    let buf_path = Path::new(&buf_path_str);
    index.add_path(buf_path).unwrap();
}

pub fn commit_buffer(repo: &Repository) {
    // Inspired by https://github.com/rust-lang/git2-rs/issues/561
    let oid = repo.index().unwrap().write_tree().unwrap();
    let curr_tree: Tree = repo.find_tree(oid).unwrap();
    let author: Signature = repo.signature().unwrap();
    let message: &str = &get_now();
    let parent_ref = repo.head().expect("bad.");
    let parent_commit = parent_ref.peel_to_commit().expect("Failure.");

    repo.commit(
        Some("HEAD"),
        &author,
        &author,
        message,
        &curr_tree,
        &[&parent_commit],
    )
    .unwrap();
}

#[cfg(test)]
mod tests {
    // These will not be good automatic tests
    // They will require a lot of manual intervention and checking
    // Can be confident in basic git operation however with these tests
    // I.e. they will all always pass because they cannot actually check the git operations
    // Checking is best done with git log and looking at the repo itself (or checking its there)

    use crate::git_management::{clone_repo, open_repo, modify_buffer, add_buffer, commit_buffer};
    use std::path::Path;

    #[test]
    fn test_clone_repo() {
        let repo_dir = Path::new("/tmp/test_mono");
        clone_repo(repo_dir);
    } 

    #[test]
    fn find_local_repo() {
        let repo = open_repo();
        assert!(!repo.is_empty().unwrap());
    }

    #[test]
    fn test_modify_buffer() {
        assert!(true);
    }

    #[test]
    fn test_add() {
        assert!(true);
    }

    #[test]
    fn test_commit() {
        assert!(true);
    }
}
