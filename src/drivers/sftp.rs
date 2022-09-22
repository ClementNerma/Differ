use std::{
    collections::HashSet,
    convert::TryInto,
    net::TcpStream,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use anyhow::{bail, Context, Result};
use ssh2::{Session, Sftp};

use super::{Driver, DriverFileMetadata, DriverItem, DriverItemMetadata, OnItemHandler};

pub struct SftpDriver {
    sftp: Arc<Sftp>,
}

impl SftpDriver {
    pub fn connect(
        address: &str,
        username: &str,
        pub_key_file: &Path,
        priv_key_file: &Path,
    ) -> Result<Self> {
        let tcp = TcpStream::connect(address)?;

        let mut session = Session::new().unwrap();
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();

        session.userauth_pubkey_file(username, Some(pub_key_file), priv_key_file, None)?;

        if !session.authenticated() {
            bail!("Session is not authenticated!");
        }

        let sftp = session.sftp().unwrap();

        Ok(Self {
            sftp: Arc::new(sftp),
        })
    }
}

impl Driver for SftpDriver {
    fn find_all(
        &self,
        root: &str,
        ignore: &HashSet<&str>,
        stop_request: Arc<AtomicBool>,
        on_item: Option<OnItemHandler>,
    ) -> Result<Vec<DriverItem>> {
        let root = Path::new(root);
        let dirs_contents = Arc::new(Mutex::new(vec![]));
        let remaining = Arc::new(AtomicU32::new(1));

        let state = ReadDirState {
            sftp: Arc::clone(&self.sftp),
            ignore: Arc::new(ignore.iter().map(|val| val.to_string()).collect()),
            root: Arc::new(root.to_path_buf()),
            stop_request,
            on_item: Arc::new(on_item),
            dirs_contents: Arc::clone(&dirs_contents),
            remaining: Arc::clone(&remaining),
        };

        let root_bis = root.to_path_buf();

        stateful_read_dir_spawn(root_bis, state);

        while remaining.load(Ordering::Acquire) > 0 {
            std::thread::sleep(Duration::from_millis(100));
        }

        // TODO: handle these .unwrap() as errors
        Ok(Arc::try_unwrap(dirs_contents)
            .unwrap()
            .into_inner()
            .unwrap())
    }
}

fn get_relative_utf8_path<'a>(path: &'a Path, source: &Path) -> Result<&'a str> {
    path.strip_prefix(source)
        .context("Internal error: failed to strip prefix")?
        .to_str()
        .with_context(|| {
            format!(
                "Item path contains invalid UTF-8 characters: {}",
                path.display()
            )
        })
}

fn get_filename(path: &Path) -> Result<&str> {
    path.file_name()
        .with_context(|| format!("Filename is missing on path: {}", path.display()))?
        .to_str()
        .with_context(|| {
            format!(
                "Item path contains invalid UTF-8 characters: {}",
                path.display()
            )
        })
}

#[derive(Clone)]
struct ReadDirState {
    sftp: Arc<Sftp>,
    ignore: Arc<HashSet<String>>,
    root: Arc<PathBuf>,
    stop_request: Arc<AtomicBool>,
    on_item: Arc<Option<OnItemHandler>>,
    dirs_contents: Arc<Mutex<Vec<DriverItem>>>,
    remaining: Arc<AtomicU32>,
}

fn stateful_read_dir(dir: PathBuf, state: ReadDirState) -> Result<()> {
    let mut items = vec![];

    for (item_path, stat) in state.sftp.readdir(&dir)? {
        if state.stop_request.load(Ordering::Relaxed) {
            bail!("Process was requested to stop.");
        }

        let metadata: DriverItemMetadata;

        if state.ignore.contains(get_filename(&item_path)?) {
            continue;
        }

        let path = get_relative_utf8_path(&item_path, &state.root)?.to_string();

        if stat.is_dir() {
            metadata = DriverItemMetadata::Directory;
        } else if stat.is_file() {
            metadata = DriverItemMetadata::File(DriverFileMetadata {
                modification_date: stat
                    .mtime
                    .with_context(|| {
                        format!("Missing modification time on item: {}", item_path.display())
                    })?
                    .try_into()
                    .with_context(|| {
                        format!(
                            "Invalid modification time found for item: {}",
                            item_path.display()
                        )
                    })?,
                size: stat
                    .size
                    .with_context(|| format!("Missing size on item: {}", item_path.display()))?,
            })
        } else {
            bail!("Unknown item type at: {}", item_path.display());
        }

        let item = DriverItem { path, metadata };

        if let Some(handler) = state.on_item.as_deref() {
            handler(&item);
        }

        items.push(item);

        if metadata.is_dir() {
            state.remaining.store(
                state.remaining.load(Ordering::Acquire) + 1,
                Ordering::Release,
            );

            stateful_read_dir_spawn(item_path, state.clone());
        }
    }

    state.dirs_contents.lock().unwrap().extend(items);

    state.remaining.store(
        state.remaining.load(Ordering::Acquire) - 1,
        Ordering::Release,
    );

    Ok(())
}

fn stateful_read_dir_spawn(dir: PathBuf, state: ReadDirState) {
    rayon::spawn(move || {
        // TODO: handle erros
        stateful_read_dir(dir, state).unwrap()
    });
}
