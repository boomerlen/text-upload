use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::env;
use chrono::{Local};
use std::fmt;

use git2::{BranchType, Cred, PushOptions, Remote, RemoteCallbacks, Repository, Signature, Tree};

use crate::config::{Config, get_config, DFT_CONF_PATH};

// Error management to concatenate many possible error sources into a single handleable error
// Using this type loses original error origin. Should fix
type Result<T> = std::result::Result<T, GitActionError>;

#[derive(Debug, Clone)]
pub struct GitActionError;

impl fmt::Display for GitActionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "git action failed")
    }
}

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

pub fn get_dft_conf() -> String {
    format!("{}/.simple-text/{}", env::var("HOME").unwrap(), DFT_CONF_PATH)
}

pub fn get_now() -> String {
    let local_time = Local::now();
    format!("{}", local_time.format("%d_%b_%y-%H_%M"))
}

pub fn push_to_repo(repo: &Repository) -> Result<()> {
    let conf: Config = match get_config(Path::new(&get_dft_conf())) {
        Ok(c) => c,
        Err(_) => return Err(GitActionError),
    };
    
    // Prepare callbacks
    let ssh_key_str: &String = &conf.ssh_file;
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
    let mut remote: Remote = match repo.find_remote("origin") {
        Ok(r) => r,
        Err(_) => return Err(GitActionError),
    };

    // Assume the branch name is the same on the host and the remote
    let refspec = format!("refs/heads/{}:refs/heads/{}", conf.branch, conf.branch);

    match remote.push(&[refspec], Some(push_options)) {
        Ok(_) => Ok(()),
        Err(_) => Err(GitActionError)
    }
}

fn clone_repo() -> Result<Repository> {
    // Largely based on example
    // https://docs.rs/git2/latest/git2/build/struct.RepoBuilder.html

    let conf: Config = match get_config(Path::new(&get_dft_conf())) {
        Ok(c) => c,
        Err(_) => return Err(GitActionError),
    };

    // "Prepare callbacks"
    let ssh_key_str: &String = &conf.ssh_file;
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

    match builder.clone(&conf.url, Path::new(&conf.local_dir)) {
        Ok(r) => Ok(r),
        Err(_) => Err(GitActionError),
    }
}

pub fn open_repo() -> Result<Repository> {
    let conf: Config = match get_config(Path::new(&get_dft_conf())) {
        Ok(c) => c,
        Err(_) => return Err(GitActionError),
    };

    let repo_dir: &Path = Path::new(&conf.local_dir);

    // Check firstly if we already have the repo cloned
    let repo = match Repository::open(repo_dir) {
        Ok(repo) => repo,
        Err(_) => {
            // If error is relevant (?) clone dir
            match clone_repo() {
                Ok(r) => r,
                Err(_) => return Err(GitActionError),
            }
        }
    };

    // Ensure branch is correct
    match repo.find_branch(&conf.branch, BranchType::Remote) {
        Ok(branch) => branch,
        Err(_) => return Err(GitActionError),
    };

    // Change head before this
    match repo.set_head(&conf.branch) {
        Ok(_) => (),
        Err(_) => return Err(GitActionError),
    };
    match repo.checkout_head(None) {
        Ok(_) => (),
        Err(_) => return Err(GitActionError),
    };

    Ok(repo)
}

pub fn modify_buffer(buf_name: &String, buf_text: &String) -> Result<()> {
    let conf: Config = match get_config(Path::new(&get_dft_conf())) {
        Ok(c) => c,
        Err(_) => return Err(GitActionError),
    };

    let buf_path_str = format!(
        "{}/{}/{}",
        conf.local_dir,
        conf.buffer_dir_rel,
        buf_name,
    );
    let buf_path = Path::new(&buf_path_str);

    decrypt(buf_path);

    let mut file = match OpenOptions::new()
        .append(true)
        .open(buf_path) {
            Ok(f) => f,
            Err(_) => return Err(GitActionError),
        };

    match file.write(&format!("\n{}\n", buf_text).as_bytes()) {
        Ok(_) => (),
        Err(_) => return Err(GitActionError),
    }

    encrypt(buf_path);

    Ok(())
}

pub fn add_buffer(buf_name: &String, repo: &Repository) -> Result<()> {
    let conf: Config = match get_config(Path::new(&get_dft_conf())) {
        Ok(c) => c,
        Err(_) => return Err(GitActionError),
    };

    let mut index = match repo.index() {
        Ok(i) => i,
        Err(_) => return Err(GitActionError),
    };

    let buf_path_str = format!(
        "{}/{}/{}",
        conf.local_dir,
        conf.buffer_dir_rel,
        buf_name,
    );

    let buf_path = Path::new(&buf_path_str);
    match index.add_path(buf_path) {
        Ok(_) => Ok(()),
        Err(_) => Err(GitActionError),
    }
}


pub fn commit_buffer(repo: &Repository) -> Result<()> {
    // Inspired by https://github.com/rust-lang/git2-rs/issues/561
    // Should look at replacing match statemnts with some sort of macro
    let mut index = match repo.index() { 
        Ok(i) => i,
        Err(_) => return Err(GitActionError),
    };
    let oid = match index.write_tree() {
        Ok(oid) => oid,
        Err(_) => return Err(GitActionError),
    };
    let curr_tree: Tree = match repo.find_tree(oid) {
        Ok(t) => t,
        Err(_) => return Err(GitActionError),
    };
    let author: Signature = match repo.signature() {
        Ok(sig) => sig,
        Err(_) => return Err(GitActionError),
    };
    let message: &str = &get_now();
    let parent_ref = match repo.head() {
        Ok(h) => h,
        Err(_) => return Err(GitActionError),
    };
    let parent_commit = match parent_ref.peel_to_commit() {
        Ok(c) => c,
        Err(_) => return Err(GitActionError),
    };

    match repo.commit(
        Some("HEAD"),
        &author,
        &author,
        message,
        &curr_tree,
        &[&parent_commit],
    ) {
        Ok(_) => Ok(()),
        Err(_) => Err(GitActionError),
    }
}

#[cfg(test)]
mod tests {
    // These will not be good automatic tests
    // They will require a lot of manual intervention and checking
    // Can be confident in basic git operation however with these tests
    // I.e. they will all always pass because they cannot actually check the git operations
    // Checking is best done with git log and looking at the repo itself (or checking its there)

    use crate::git_management::{clone_repo, open_repo, modify_buffer, add_buffer, commit_buffer, get_dft_conf};
    use std::path::Path;
    use std::fs::File;
    use std::io::Write;
    use std::env;

    fn write_test_config() {
        let test_config_toml = format!(r#"
            url = 'git@github.com:boomerlen/text-upload.git'
            local_dir = '/tmp/test-simple-text'
            branch = 'test-branch'
            buffer_dir_rel = 'test-buffers'
            ssh_file = '{}/.ssh/id_rsa.pub'
        "#, env::var("HOME").unwrap());

        let text = get_dft_conf();
        let config_path = Path::new(&text);
        let mut file: File = File::create(&config_path).unwrap();

        file.write_all(test_config_toml.as_bytes()).unwrap();
    }

    #[test]
    fn test_clone_repo() {
        write_test_config();
        clone_repo();
    } 

    #[test]
    fn find_local_repo() {
        write_test_config();
        let repo = open_repo().unwrap();
        assert!(!repo.is_empty().unwrap());
    }

    #[test]
    fn test_modify_buffer() {
        write_test_config();
        assert!(true);
    }

    #[test]
    fn test_add() {
        write_test_config();
        assert!(true);
    }

    #[test]
    fn test_commit() {
        write_test_config();
        assert!(true);
    }
}
