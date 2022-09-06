use std::{collections::HashSet, convert::TryInto, net::TcpStream, path::Path};

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
    fn find_all(&self, root: &str, ignore: &HashSet<&str>) -> Result<Vec<DriverItem>> {
        let root = Path::new(root);

        let read_sub_dir = |dir: &Path| -> Result<Vec<DriverItem>> {
            let mut items = vec![];

            for (item, stat) in self.sftp.readdir(Path::new(dir))? {
                let metadata: DriverItemMetadata;

                if ignore.contains(get_filename(&item)?) {
                    continue;
                }

                let path = get_relative_utf8_path(root, &item)?.to_string();

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
            }
            todo!()
        };

        read_sub_dir(root)
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
