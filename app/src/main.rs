mod cli {
    use clap::Parser;
    use std::path::PathBuf;

    #[derive(Parser, Debug)]
    #[command(version, about, long_about = None)]

    pub struct Args {
        /// Show all entries
        #[arg(short = 'a', long = "verbose")]
        pub verbose: bool,

        /// Path to the ssh private key to use for authentication. Defaults to ~/.ssh/id_rsa
        #[arg(short = 'i', long = "ssh-private-key")]
        pub ssh_private_key: Option<PathBuf>,

        /// The directory where the repositories are stored. Defaults to the current working directory.
        pub repos_directory: Option<PathBuf>,
    }

    pub fn get_args() -> Args {
        Args::parse()
    }
}

use anyhow::{Context, Error, Result, ensure};
use std::{fmt::format, fs, path};

struct Printer {
    verbose: bool,
    messages: Vec<String>,
}

const UNEXPECTED_GENERAL_ENTRY_ERROR: &str = "Something unexpectedly failed for the current entry";

impl Printer {
    fn flush(&mut self) {
        for message in self.messages.iter() {
            println!("{}", message);
        }
        self.messages.clear();
    }
    fn new(verbose: bool) -> Self {
        Self { verbose, messages: Vec::new() }
    }
    fn msg_symlink(path: &std::path::Path) -> String {
        format!("⚠️ Found symlink: {}. Ignoring this entry, as at the time of making this tool, I have never made symlinks in there, so I don't know what it means semantically.", path.display())
    }
    fn log_symlink(&mut self, path: &std::path::Path) {
        self.messages.push(Self::msg_symlink(path));
    }
    fn msg_file(path: &std::path::Path) -> String {
        format!("❗ Found file: {}. Files are unlikely to be git-pushed; move them somewhere safe if necessary.", path.display())
    }
    fn log_file(&mut self, path: &std::path::Path) {
        self.messages.push(Self::msg_file(path));
    }
    fn msg_nongit_dir(path: &std::path::Path, msg: &str) -> String {
        format!("❗ {}: {}. This is not a git repository.", msg, path.display())
    }
    fn log_nongit_dir(&mut self, path: &std::path::Path, msg: &str) {
        self.messages.push(Self::msg_nongit_dir(path, msg));
    }
    fn msg_local_only_branch(entry: &fs::DirEntry, local_branch: git2::Branch) -> String {
        format!("💥 {}: Local branch {} has no upstream (tracking remote branch)", entry.path().display(), local_branch.name().unwrap().unwrap())
    }
    fn log_local_only_branch(&mut self, entry: &fs::DirEntry, local_branch: git2::Branch) {
        self.messages.push(Self::msg_local_only_branch(entry, local_branch));
    }
    fn msg_general_entry_error(error: Error) -> String {
        format!("🚨 {}: {}", UNEXPECTED_GENERAL_ENTRY_ERROR, error)
    }
    fn log_general_entry_error(&mut self, error: Error) {
        self.messages.push(Self::msg_general_entry_error(error));
    }
    fn msg_general_entry_error_for_entry(entry: &fs::DirEntry, error: Error) -> String {
        format!("🚨 Failed for the entry {}: {}", entry.path().display(), error)
    }
    fn log_general_entry_error_for_entry(&mut self, entry: &fs::DirEntry, error: Error) {
        self.messages.push(Self::msg_general_entry_error_for_entry(entry, error));
    }
    fn msg_remote_not_found(entry: &fs::DirEntry, remote: &str) -> String {
        format!("🚨 {}: Remote {} not found", entry.path().display(), remote)
    }
    fn log_remote_not_found(&mut self, entry: &fs::DirEntry, remote_name: &str) {
        self.messages.push(Self::msg_remote_not_found(entry, remote_name));
    }
    fn msg_unqualified_remote(entry: &fs::DirEntry, remote_name: &str) -> String {
        format!("⚠️ {}: Remote {} is not a qualifying remote", entry.path().display(), remote_name)
    }
    fn log_unqualified_remote(&mut self, entry: &fs::DirEntry, remote_name: &str) {
        self.messages.push(Self::msg_unqualified_remote(entry, remote_name));
    }
    fn msg_remote_fetch_failed(entry: &fs::DirEntry, remote_name: &str, error: git2::Error) -> String {
        format!("🚨 {}: Failed to fetch remote {}: {}", entry.path().display(), remote_name, error)
    }
    fn log_remote_fetch_failed(&mut self, entry: &fs::DirEntry, remote_name: &str, error: git2::Error) {
        self.messages.push(Self::msg_remote_fetch_failed(entry, remote_name, error));
    }
    fn msg_remote_bad_name(entry: &fs::DirEntry, remote_name_bytes: &[u8]) -> String {
        format!("🚨 {}: Remote {} skipped due to invalid utf8", entry.path().display(), String::from_utf8_lossy(remote_name_bytes))
    }
    fn log_remote_bad_name(&mut self, entry: &fs::DirEntry, remote_name_bytes: &[u8]) {
        self.messages.push(Self::msg_remote_bad_name(entry, remote_name_bytes));
    }
    fn msg_remote_no_name(entry: &fs::DirEntry) -> String {
        format!("🚨 {}: A remote was skipped because it was not named", entry.path().display())
    }
    fn log_remote_no_name(&mut self, entry: &fs::DirEntry) {
        self.messages.push(Self::msg_remote_no_name(entry));
    }
    fn msg_remote_bad_url(entry: &fs::DirEntry, remote_name: &str, url: &[u8]) -> String {
        format!("🚨 {}: Remote {} has a bad url: {}", entry.path().display(), remote_name, String::from_utf8_lossy(url))
    }
    fn log_remote_bad_url(&mut self, entry: &fs::DirEntry, remote_name: &str, url: &[u8]) {
        self.messages.push(Self::msg_remote_bad_url(entry, remote_name, url));
    }
    fn msg_branch_name_error(entry: &fs::DirEntry, error: Error) -> String {
        format!("🚨 {}: Failed to get the name of a branch: {}", entry.path().display(), error)
    }
    fn log_branch_name_error(&mut self, entry: &fs::DirEntry, error: Error) {
        self.messages.push(Self::msg_branch_name_error(entry, error));
    }
    fn msg_local_branch_has_no_remote_tracking_branch(entry: &fs::DirEntry, branch_name: &str, error: Error) -> String {
        format!("💥 {}: Local branch {} has no remote tracking branch: {}", entry.path().display(), branch_name, error)
    }
    fn log_local_branch_has_no_remote_tracking_branch(&mut self, entry: &fs::DirEntry, branch_name: &str, error: Error) {
        self.messages.push(Self::msg_local_branch_has_no_remote_tracking_branch(entry, branch_name, error));
    }
    fn msg_branch_bad_name(entry: &fs::DirEntry, branch_name_bytes: &[u8]) -> String {
        format!("🚨 {}: Branch {} has invalid utf8", entry.path().display(), String::from_utf8_lossy(branch_name_bytes))
    }
    fn log_branch_bad_name(&mut self, entry: &fs::DirEntry, branch_name_bytes: &[u8]) {
        self.messages.push(Self::msg_branch_bad_name(entry, branch_name_bytes));
    }
    fn msg_general_branch_error(entry: &fs::DirEntry, branch_name: &str, error: Error) -> String {
        format!("🚨 {}: An operation on branch {} failed: {}", entry.path().display(), branch_name, error)
    }
    fn log_general_branch_error(&mut self, entry: &fs::DirEntry, branch_name: &str, error: Error) {
        self.messages.push(Self::msg_general_branch_error(entry, branch_name, error));
    }
    fn msg_local_branch_ahead_of_upstream(entry: &fs::DirEntry, branch_name: &str) -> String {
        format!("🚨 {}: Local branch {} is ahead of the upstream", entry.path().display(), branch_name)
    }
    fn log_local_branch_ahead_of_upstream(&mut self, entry: &fs::DirEntry, branch_name: &str) {
        self.messages.push(Self::msg_local_branch_ahead_of_upstream(entry, branch_name));
    }
    fn msg_local_branch_not_found_in_remote_ancestor(entry: &fs::DirEntry, branch_name: &str) -> String {
        format!("🚨 {}: Local branch {} is not in the ancestor of the upstream", entry.path().display(), branch_name)
    }
    fn log_local_branch_not_found_in_remote_ancestor(&mut self, entry: &fs::DirEntry, branch_name: &str) {
        self.messages.push(Self::msg_local_branch_not_found_in_remote_ancestor(entry, branch_name));
    }
    fn msg_branch_is_synced(entry: &fs::DirEntry, branch_name: &str) -> String {
        format!("✅ {}: Local branch {} is synced with the remote", entry.path().display(), branch_name)
    }
    fn log_branch_is_synced(&mut self, entry: &fs::DirEntry, branch_name: &str) {
        self.messages.push(Self::msg_branch_is_synced(entry, branch_name));
    }
    fn msg_entry(entry: &fs::DirEntry) -> String {
        format!("📝 Looking at the entry {}", entry.path().display())
    }
    fn log_entry(&mut self, entry: &fs::DirEntry) {
        if !self.verbose { return; }
        self.messages.push(Self::msg_entry(entry));
    }
    fn msg_entry_is_a_git_repo(entry: &fs::DirEntry) -> String {
        format!("📝 {}: This is a git repo ✔︎", entry.path().display())
    }
    fn log_entry_is_a_git_repo(&mut self, entry: &fs::DirEntry) {
        if !self.verbose { return; }
        self.messages.push(Self::msg_entry_is_a_git_repo(entry));
    }
    fn msg_remote_fetch_succeeded(entry: &fs::DirEntry, remote_name: &str) -> String {
        format!("📝 {}: Synced remote {}", entry.path().display(), remote_name)
    }
    fn log_remote_fetch_succeeded(&mut self, entry: &fs::DirEntry, remote_name: &str) {
        if !self.verbose { return; }
        self.messages.push(Self::msg_remote_fetch_succeeded(entry, remote_name));
    }
    fn msg_branch_name(entry: &fs::DirEntry, branch_name: &str) -> String {
        format!("📝 {}: Looking at branch {}", entry.path().display(), branch_name)
    }
    fn log_branch_name(&mut self, entry: &fs::DirEntry, branch_name: &str) {
        if !self.verbose { return; }
        self.messages.push(Self::msg_branch_name(entry, branch_name));
    }
    fn msg_branch_upstream_name(entry: &fs::DirEntry, branch_name: &str, upstream_name: &str) -> String {
        format!("📝 {}: Branch {} has upstream {}", entry.path().display(), branch_name, upstream_name)
    }
    fn log_branch_upstream_name(&mut self, entry: &fs::DirEntry, branch_name: &str, upstream_name: &str) {
        if !self.verbose { return; }
        self.messages.push(Self::msg_branch_upstream_name(entry, branch_name, upstream_name));
    }
    fn msg_branch_upstream_remote_name(entry: &fs::DirEntry, branch_name: &str, remote_name: &str) -> String {
        format!("📝 {}: Branch {} has upstream remote {}", entry.path().display(), branch_name, remote_name)
    }
    fn log_branch_upstream_remote_name(&mut self, entry: &fs::DirEntry, branch_name: &str, remote_name: &str) {
        if !self.verbose { return; }
        self.messages.push(Self::msg_branch_upstream_remote_name(entry, branch_name, remote_name));
    }
    fn msg_branch_remote_not_fetched(entry: &fs::DirEntry, branch_name: &str, remote_name: &str) -> String {
        format!("🚨 {}: Branch {} has non-fetched remote {}", entry.path().display(), branch_name, remote_name)
    }
    fn log_branch_remote_not_fetched(&mut self, entry: &fs::DirEntry, branch_name: &str, remote_name: &str) {
        self.messages.push(Self::msg_branch_remote_not_fetched(entry, branch_name, remote_name));
    }
    fn simple_log(&mut self, message: &str) {
        self.messages.push(message.to_string());
    }
}

impl Drop for Printer {
    fn drop(&mut self) {
        self.flush();
    }
}

fn main() -> Result<()> {
    let args = cli::get_args();
    println!("{:?}", args);
    let repos_directory = if let Some(repos_directory) = args.repos_directory {
        repos_directory
    } else {
        std::env::current_dir().context("Failed to get current directory")?
    };
    let ssh_private_key = if let Some(ssh_private_key) = args.ssh_private_key {
        ssh_private_key
    } else {
        let home_dir = dirs::home_dir().context("Failed to get home directory")?;
        home_dir.join(".ssh/id_rsa")
    };
    {
        let ssh_private_key_metadata = fs::metadata(&ssh_private_key).context(format!("Failed to get metadata for ssh private key: {}", ssh_private_key.display()))?;
        ensure!(ssh_private_key_metadata.is_file(), "The ssh private key path is not a file: {}", ssh_private_key.display());
    }

    for entry in fs::read_dir(&repos_directory)
        .with_context(|| format!("Failed to read projects directory: {}", repos_directory.display()))?
    {
        let mut printer = Printer::new(args.verbose);

        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                printer.log_general_entry_error(error.into());
                continue;
            },
        };

        printer.log_entry(&entry);

        // Each entry is handled in a closure to catch errors and print them
        // Most errors should be handled gracefully and printed by the Printer,
        // but some errors are propagated up from git2, and those should be printed
        // by the general entry error printer. For the first case, we return Ok(()) after
        // the Printer prints a message. For the second case, we return the error.
        // INFO: the return type is NOT a ControlFlow, because we always want to continue the loop
        let mut handle_entry = |entry: fs::DirEntry| -> Result<()> {
            // Only unknown errors should be returned.
            // "Errors" that can be handled should print a nice UX message and continue
            let path = entry.path();
            let symlink_metadata = path.metadata()?; // This doesn't follow symlinks
            if symlink_metadata.is_symlink() {
                printer.log_symlink(&path);
                return Ok(());
            } else if path.is_file() {
                printer.log_file(&path);
                return Ok(());
            }

            // Current entry is a directory
            use git2::{Repository, Remote};
            let repo = match Repository::open(&path) {
                Ok(repo) => repo,
                Err(error) => {
                    printer.log_nongit_dir(&path, error.message());
                    return Ok(())
                },
            };
            // Current entry is a git repository
            printer.log_entry_is_a_git_repo(&entry);

            // TODO: check for uncommitted changes, both unstaged and staged

            // Find all remotes
            let remote_names = repo.remotes()?;
            let mut qualifying_remotes: Vec<Remote> = Vec::new();
            for (remote_name, remote_name_bytes) in std::iter::zip(remote_names.iter(), remote_names.iter_bytes()) {
                let remote_name = match remote_name {
                    Some(remote) => remote,
                    None => {
                        printer.log_remote_bad_name(&entry, remote_name_bytes);
                        continue;
                    },
                };
                let remote = match repo.find_remote(&remote_name) {
                    Ok(remote) => remote,
                    Err(error) => {
                        printer.log_remote_not_found(&entry, remote_name);
                        continue;
                    },
                };
                let url = match remote.url() {
                    Some(url) => url,
                    None => {
                        printer.log_remote_bad_url(&entry, remote_name, remote.url_bytes());
                        continue;
                    },
                };
                // If the url begins with "https://github.com/", then it is a qualifying remote
                // TODO: support more urls / make them configurable
                if url.starts_with("https://github.com/") || url.starts_with("git@github.com:") {
                    qualifying_remotes.push(remote);
                } else {
                    printer.log_unqualified_remote(&entry, remote_name);
                }
            }
            
            let synced_remotes = {
                // Fetch all qualifying remotes
                let synced_remotes: Vec<_> = qualifying_remotes.iter_mut().filter_map(|remote| {
                    let remote_cb = {
                        let mut remote_cb_builder = git2::RemoteCallbacks::new();
                        remote_cb_builder.credentials(|user, user_from_url, cred| {
                            // See https://github.com/rust-lang/git2-rs/issues/329#issuecomment-403318088
                            let user = user_from_url.unwrap_or(user);
                            if cred.is_username() {
                                // TODO: since `cred` is a bitset, figure out if we need to check for other flags
                                return git2::Cred::username(user);
                            }
                            git2::Cred::ssh_key(user, None, &ssh_private_key, None)
                        });
                        remote_cb_builder
                    };
                    let mut fetch_opts = git2::FetchOptions::new();
                    fetch_opts.remote_callbacks(remote_cb);

                    match remote.fetch(&[] as &[&str], Some(&mut fetch_opts), None) {
                        Ok(_) => {
                            printer.log_remote_fetch_succeeded(&entry, remote.name().unwrap());
                            Some(remote)
                        },
                        Err(error) => {
                            printer.log_remote_fetch_failed(&entry, remote.name().unwrap(), error);
                            None
                        },
                    }
                }).collect(); // Must be eagerly iterated, because `printer` is borrowed mutably
                synced_remotes
            };

            // Get all local branches (i.e. not remote-tracking branches) and check
            // 1. that they have a corresponding remote-tracking branch
            // 2. that they're not ahead of the remote-tracking branch
            let branches = repo.branches(Some(git2::BranchType::Local))?;
            for branch in branches {
                let (branch, _) = branch?;
                // Convert a Result<Option<&str, Error> to a Result<String, Error>
                let branch_name = branch.name().and_then(|maybe_branch_name| {
                    maybe_branch_name.map_or_else(|| {
                        branch.name_bytes().map(|slice| String::from_utf8_lossy(slice).to_string())
                    }, |branch_name| {
                        Ok(branch_name.to_owned())
                    })
                });
                let branch_name = match branch_name {
                    Ok(branch_name) => {
                        printer.log_branch_name(&entry, &branch_name);
                        branch_name
                    },
                    Err(error) => {
                        printer.log_branch_name_error(&entry, error.into());
                        continue;
                    }
                };
                let remote_tracking_branch = match branch.upstream() {
                    Ok(remote_tracking_branch) => remote_tracking_branch,
                    Err(error) => {
                        printer.log_local_branch_has_no_remote_tracking_branch(&entry, &branch_name, error.into());
                        continue;
                    }
                };
                
                // Check upstream tracks a synced remote
                let remote_tracking_branch_fqrefname = match remote_tracking_branch.name() {
                    Ok(Some(remote_tracking_branch_name)) => {
                        printer.log_branch_upstream_name(&entry, &branch_name, remote_tracking_branch_name);
                        // The `repo.branch_remote_name` function expects a fully qualified refname
                        format!("refs/remotes/{}", remote_tracking_branch_name)
                    },
                    Ok(None) => {
                        // TODO: refactor to handle Err from name_bytes()
                        printer.log_branch_bad_name(&entry, remote_tracking_branch.name_bytes().unwrap());
                        continue;
                    }
                    Err(error) => {
                        printer.log_branch_name_error(&entry, error.into());
                        continue;
                    }
                };
                let remote_name = match repo.branch_remote_name(&remote_tracking_branch_fqrefname) {
                    Ok(buf) => match buf.as_str() {
                        Some(remote_name) => {
                            printer.log_branch_upstream_remote_name(&entry, &branch_name, remote_name);
                            remote_name.to_owned()
                        },
                        None => {
                            printer.log_remote_bad_name(&entry, &[]);
                            continue;
                        },
                    },
                    Err(error) => {
                        printer.log_general_branch_error(&entry, &remote_tracking_branch_fqrefname, error.into());
                        continue;
                    },
                };
                let has_synced_remote = synced_remotes.iter().any(|remote| {
                    remote.name().unwrap() == remote_name
                });
                if !has_synced_remote {
                    printer.log_branch_remote_not_fetched(&entry, &branch_name, &remote_name);
                    continue;
                }

                // Check that the local branch is not ahead of the remote-tracking branch
                let branch_direct_ref = match branch.get().resolve() {
                    Ok(direct_ref) => direct_ref,
                    Err(error) => {
                        printer.log_general_branch_error(&entry, &branch_name, error.into());
                        continue;
                    }
                };
                let branch_oid = branch_direct_ref.target().unwrap();
                let upstream_direct_ref = match remote_tracking_branch.get().resolve() {
                    Ok(direct_ref) => direct_ref,
                    Err(error) => {
                        printer.log_general_branch_error(&entry, &branch_name, error.into());
                        continue;
                    }
                };
                let upstream_oid = upstream_direct_ref.target().unwrap();

                let mut revwalk = repo.revwalk()?;
                revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
                revwalk.push(upstream_oid)?;
                let local_oid_is_ancestor_of_upstream = revwalk.any(|oid| {
                    match oid {
                        Ok(oid) => oid == branch_oid,
                        Err(error) => {
                            printer.log_general_branch_error(&entry, &branch_name, error.into());
                            false
                        }
                    }
                });
                if !local_oid_is_ancestor_of_upstream {
                    // Either the local branch is ahead of the upstream, or it diverged
                    let mut revwalk = repo.revwalk()?;
                    revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
                    revwalk.push(branch_oid)?;
                    let upstream_oid_is_ancestor_of_local = revwalk.any(|oid| {
                        match oid {
                            Ok(oid) => oid == upstream_oid,
                            Err(error) => {
                                printer.log_general_branch_error(&entry, &branch_name, error.into());
                                false
                            }
                        }
                    });
                    if upstream_oid_is_ancestor_of_local {
                        printer.log_local_branch_ahead_of_upstream(&entry, &branch_name);
                    } else {
                        printer.log_local_branch_not_found_in_remote_ancestor(&entry, &branch_name);
                    }
                    continue;
                }

                // Local branch is in the ancestor of upstream
                printer.log_branch_is_synced(&entry, &branch_name);
            }

            Ok(())
        };
        if let Err(error) = handle_entry(entry) {
            // TODO: add current entry as context
            printer.log_general_entry_error(error);
        }
    }
    Ok(())
}
