use std::{
    collections::HashSet,
    convert::TryInto,
    net::TcpStream,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::{bail, Context, Result};
use ssh2::{Session, Sftp};

use super::{Driver, DriverFileMetadata, DriverItem, DriverItemMetadata};

pub struct SftpDriver {
    sftp: Sftp,
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

        Ok(Self { sftp })
    }
}

impl Driver for SftpDriver {
    fn find_all(
        &self,
        root: &str,
        ignore: &HashSet<&str>,
        stop_request: Arc<AtomicBool>,
    ) -> Result<Vec<DriverItem>> {
        let root = Path::new(root);

        fn read_sub_dir(
            dir: &Path,
            sftp: &Sftp,
            ignore: &HashSet<&str>,
            root: &Path,
            stop_request: Arc<AtomicBool>,
        ) -> Result<Vec<DriverItem>> {
            let mut items = vec![];

            for (item, stat) in sftp.readdir(Path::new(dir))? {
                if stop_request.load(Ordering::Relaxed) {
                    bail!("Process was requested to stop.");
                }

                let metadata: DriverItemMetadata;

                if ignore.contains(get_filename(&item)?) {
                    continue;
                }

                let path = get_relative_utf8_path(&item, root)?.to_string();

                if stat.is_dir() {
                    metadata = DriverItemMetadata::Directory;
                } else if stat.is_file() {
                    metadata = DriverItemMetadata::File(DriverFileMetadata {
                        modification_date: stat
                            .mtime
                            .with_context(|| {
                                format!("Missing modification time on item: {}", item.display())
                            })?
                            .try_into()
                            .with_context(|| {
                                format!(
                                    "Invalid modification time found for item: {}",
                                    item.display()
                                )
                            })?,
                        size: stat
                            .size
                            .with_context(|| format!("Missing size on item: {}", item.display()))?,
                    })
                } else {
                    bail!("Unknown item type at: {}", item.display());
                }

                items.push(DriverItem { path, metadata });

                if metadata.is_dir() {
                    let sub_items =
                        read_sub_dir(&item, sftp, ignore, root, Arc::clone(&stop_request))?;
                    items.extend(sub_items);
                }
            }

            Ok(items)
        }

        read_sub_dir(root, &self.sftp, ignore, root, stop_request)
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
