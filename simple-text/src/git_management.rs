use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::env;
use chrono::{Local};

use git2::{Cred, PushOptions, Remote, RemoteCallbacks, Repository, Signature, Tree};

use crate::config::{Config, get_config, DFT_CONF_PATH};

// Error management to concatenate many possible error sources into a single handleable error
// Using this type loses original error origin. Should fix
type Result<T> = std::result::Result<T, git2::Error>;

fn decrypt(file: &Path) -> Result<()> {
    let full_path = match file.canonicalize() {
        Ok(p) => p,
        Err(e) => return Err(git2::Error::from_str(format!("Failed to canonicalise path: {:?} with error: {}", file, e.to_string()).as_str())),
    };
    match Command::new("unscramble")
    .args([full_path.as_os_str()])
    .env("PATH", env::var("PATH").unwrap_or("".to_string()))
    .output() {
        Ok(_) => Ok(()),
        Err(e) => Err(git2::Error::from_str(format!("Failed to decrypt buffer with error: {}.", e.to_string()).as_str()))
    }
}

fn encrypt(file: &Path) -> Result<()> {
    let full_path = match file.canonicalize() {
        Ok(p) => p,
        Err(e) => return Err(git2::Error::from_str(format!("Failed to canonicalise path: {:?} with error: {}", file, e.to_string()).as_str())),
    };
    match Command::new("scramble")
    .args([full_path.as_os_str()])
    .env("PATH", env::var("PATH").unwrap_or("".to_string()))
    .output() {
        Ok(_) => Ok(()),
        Err(e) => Err(git2::Error::from_str(format!("Failed to encrypt buffer with error: {}.", e.to_string()).as_str()))
    }
}

pub fn get_dft_conf() -> String {
    format!("{}/.simple-text/{}", env::var("HOME").unwrap(), DFT_CONF_PATH)
}

pub fn get_now() -> String {
    let local_time = Local::now();
    format!("{}", local_time.format("%d_%b_%y-%H_%M"))
}

pub fn push_to_repo(repo: &Repository) -> Result<()> {
    let conf: Config = get_config(Path::new(&get_dft_conf()))?;
    
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
    let mut remote: Remote = repo.find_remote("origin")?;

    // Assume the branch name is the same on the host and the remote
    let refspec = format!("refs/heads/{}:refs/heads/{}", conf.branch, conf.branch);

    remote.push(&[refspec], Some(push_options))?;

    Ok(())
}

fn clone_repo() -> Result<Repository> {
    // Largely based on example
    // https://docs.rs/git2/latest/git2/build/struct.RepoBuilder.html

    let conf: Config = get_config(Path::new(&get_dft_conf()))?;

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
        Err(e) => Err(e),
    }
}

pub fn open_repo() -> Result<Repository> {
    let conf: Config = get_config(Path::new(&get_dft_conf()))?;

    let repo_dir: &Path = Path::new(&conf.local_dir);

    // Check firstly if we already have the repo cloned
    let repo = match Repository::open(repo_dir) {
        Ok(repo) => repo,
        Err(_) => clone_repo()?,
    };


    {
        // Check current head
        let curr_head = repo.head()?;
        let curr_head_name = match curr_head.name() {
            Some(n) => n,
            None => return Err(git2::Error::from_str("Could not get name of current HEAD.")),
        };

        if curr_head_name != format!("refs/heads/{}", &conf.branch).as_str() {
            // Checkout
            let reference = repo.find_reference(format!("refs/remotes/origin/{}", conf.branch).as_str())?;
            let commit = repo.reference_to_annotated_commit(&reference)?;

            let branch = repo.branch_from_annotated_commit(&conf.branch, &commit, false)?;
            
            let branch_name = match branch.get().name() {
                Some(n) => n,
                None => return Err(git2::Error::from_str("Could not get branch name from reference.")),
            };

            let obj = repo.revparse_single(branch_name)?;

            // Change head and checkout tree
            repo.checkout_tree(&obj, None)?;
            repo.set_head(branch_name)?;
        }
    }
    Ok(repo)
}

pub fn modify_buffer(buf_name: &String, buf_text: &String) -> Result<()> {
    let conf: Config = get_config(Path::new(&get_dft_conf()))?;

    let buf_path_str = format!(
        "{}/{}/{}",
        conf.local_dir,
        conf.buffer_dir_rel,
        buf_name,
    );
    let buf_path = Path::new(&buf_path_str);

    if match buf_path.try_exists() {
        Ok(b) => b,
        Err(e) => return Err(git2::Error::from_str(format!("Buffer lookup failed with text: {}.", e.to_string()).as_str())),
    } {
        decrypt(buf_path)?;
    } else {
        let parent = match buf_path.parent() {
            Some(p) => p,
            None => return Err(git2::Error::from_str("Could not get parents of buffer file.")),
        };
        if match parent.try_exists() {
            Ok(b) => b,
            Err(e) => return Err(git2::Error::from_str(format!("Buffer parent lookup failed with text: {}.", e.to_string()).as_str())),
        } {
            // Create parent dir
            match create_dir_all(parent) {
                Ok(_) => (),
                Err(e) => return Err(git2::Error::from_str(format!("Could not create parent dir. Error text: {}.", e.to_string()).as_str())),
            }
        }
    }


    let mut file = match OpenOptions::new()
        .append(true)
        .create(true)
        .open(buf_path) {
            Ok(f) => f,
            Err(e) => return Err(git2::Error::from_str(format!("Failed to open buffer ({:?}) with error text: {}.", buf_path, e.to_string()).as_str())),
        };

    match file.write(&format!("\n{}\n", buf_text).as_bytes()) {
        Ok(_) => (),
        Err(e) => return Err(git2::Error::from_str(format!("Failed to write to buffer ({:?}) with error text: {}.", buf_path,  e.to_string()).as_str())),
    }

    encrypt(buf_path)?;

    Ok(())
}

pub fn add_buffer(buf_name: &String, repo: &Repository) -> Result<()> {
    let conf: Config = get_config(Path::new(&get_dft_conf()))?;

    let mut index = repo.index()?;

    let buf_path_str = format!(
        "{}/{}",
        conf.buffer_dir_rel,
        buf_name,
    );

    let buf_path = Path::new(&buf_path_str);
    index.add_path(buf_path)?;
    index.write()?;

    Ok(())
}


pub fn commit_buffer(repo: &Repository) -> Result<()> {
    // Inspired by https://github.com/rust-lang/git2-rs/issues/561
    // Should look at replacing match statemnts with some sort of macro
    let mut index = repo.index()?;
    let oid = index.write_tree()?;
    let curr_tree: Tree = repo.find_tree(oid)?;
    let author: Signature = repo.signature()?;
    let message: &str = &get_now();
    let parent_ref = repo.head()?;
    let parent_commit = parent_ref.peel_to_commit()?;

    match repo.commit(
        Some("HEAD"),
        &author,
        &author,
        message,
        &curr_tree,
        &[&parent_commit],
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
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

    use super::push_to_repo;

    fn write_test_config() {
        let test_config_toml = format!(r#"
            url = 'git@github.com:boomerlen/text-upload.git'
            local_dir = '/tmp/test-simple-text'
            branch = 'test-branch'
            buffer_dir_rel = 'test-buffers'
            ssh_file = '{}/.ssh/id_rsa'
        "#, env::var("HOME").unwrap());

        let text = get_dft_conf();
        let config_path = Path::new(&text);
        let mut file: File = File::create(&config_path).unwrap();

        file.write_all(test_config_toml.as_bytes()).unwrap();
    }

    #[test]
    fn test_clone_repo() {
        write_test_config();
        clone_repo().unwrap();
    } 

    #[test]
    fn find_local_repo() {
        write_test_config();
        let repo = open_repo().unwrap();
        assert!(!repo.is_empty().unwrap());
    }

    #[test]
    fn test_modify_buffer_new() {
        write_test_config();
        // Note that buffer parsing is handled by api.rs
        // Tests for that should go there
        let buf_name = String::from("sample_new_buffer.md");
        let buf_text = String::from("new added text!");
        modify_buffer(&buf_name, &buf_text).unwrap()
    }

    #[test]
    fn test_modify_buffer_existing() {
        write_test_config();
        let buf_name = String::from("places.md");
        let buf_text = String::from("extra text!");
        modify_buffer(&buf_name, &buf_text).unwrap();
    }

    #[test]
    fn test_add() {
        // Only still broken operation
        write_test_config();
        let buf_name = String::from("places.md");
        let repo = open_repo().unwrap();
        add_buffer(&buf_name, &repo).unwrap();
    }

    #[test]
    fn test_commit() {
        write_test_config();
        let repo = open_repo().unwrap();
        commit_buffer(&repo).unwrap();
    }

    #[test]
    fn test_push() {
        write_test_config();
        let repo = open_repo().unwrap();
        push_to_repo(&repo).unwrap();
    }
}
