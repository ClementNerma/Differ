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

use crate::drivers::OnItemHandlerRef;

use super::{Driver, DriverFileMetadata, DriverItem, DriverItemMetadata, OnItemHandler};

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
        on_item: Option<OnItemHandler>,
    ) -> Result<Vec<DriverItem>> {
        let root = Path::new(root);

        fn read_sub_dir(
            dir: &Path,
            sftp: &Sftp,
            ignore: &HashSet<&str>,
            root: &Path,
            stop_request: Arc<AtomicBool>,
            on_item: Option<OnItemHandlerRef>,
        ) -> Result<Vec<DriverItem>> {
            let mut items = vec![];

            for (item_path, stat) in sftp.readdir(Path::new(dir))? {
                if stop_request.load(Ordering::Relaxed) {
                    bail!("Process was requested to stop.");
                }

                let metadata: DriverItemMetadata;

                if ignore.contains(get_filename(&item_path)?) {
                    continue;
                }

                let path = get_relative_utf8_path(&item_path, root)?.to_string();

                if stat.is_dir() {
                    metadata = DriverItemMetadata::Directory;
                } else if stat.is_file() {
                    metadata = DriverItemMetadata::File(DriverFileMetadata {
                        modification_date: stat
                            .mtime
                            .with_context(|| {
                                format!(
                                    "Missing modification time on item: {}",
                                    item_path.display()
                                )
                            })?
                            .try_into()
                            .with_context(|| {
                                format!(
                                    "Invalid modification time found for item: {}",
                                    item_path.display()
                                )
                            })?,
                        size: stat.size.with_context(|| {
                            format!("Missing size on item: {}", item_path.display())
                        })?,
                    })
                } else {
                    bail!("Unknown item type at: {}", item_path.display());
                }

                let item = DriverItem { path, metadata };

                if let Some(handler) = &on_item {
                    handler(&item);
                }

                items.push(item);

                if metadata.is_dir() {
                    let sub_items = read_sub_dir(
                        &item_path,
                        sftp,
                        ignore,
                        root,
                        Arc::clone(&stop_request),
                        on_item,
                    )?;
                    items.extend(sub_items);
                }
            }

            Ok(items)
        }

        read_sub_dir(
            root,
            &self.sftp,
            ignore,
            root,
            stop_request,
            on_item.as_deref(),
        )
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
