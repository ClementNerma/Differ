use std::{convert::TryInto, net::TcpStream, path::Path};

use anyhow::{bail, Context, Result};
use ssh2::{Session, Sftp};

use super::{Driver, DriverFileMetadata, DriverItem, DriverItemMetadata};

pub struct SftpDriver {
    sftp: Sftp,
}

impl SftpDriver {
    pub fn connect(address: &str, username: &str) -> Result<Self> {
        let tcp = TcpStream::connect(address)?;

        let mut session = Session::new().unwrap();
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();

        session.userauth_agent(username)?;

        if !session.authenticated() {
            bail!("Session is not authenticated!");
        }

        let sftp = session.sftp().unwrap();

        Ok(Self { sftp })
    }
}

impl Driver for SftpDriver {
    fn id(&self) -> String {
        "sftp".to_string()
    }

    fn canonicalize(&self, path: &str) -> Result<String> {
        todo!()
    }

    fn find_all(&self, dir: &str) -> Result<Vec<DriverItem>> {
        let read_sub_dir = |dir: &str| -> Result<Vec<DriverItem>> {
            let mut items = vec![];

            for (path, stat) in self.sftp.readdir(Path::new(dir))? {
                let metadata: DriverItemMetadata;

                if stat.is_dir() {
                    metadata = DriverItemMetadata::Directory;
                } else if stat.is_file() {
                    metadata = DriverItemMetadata::File(DriverFileMetadata {
                        modification_date: stat
                            .mtime
                            .with_context(|| {
                                format!("Missing modification time on item: {}", path.display())
                            })?
                            .try_into()
                            .with_context(|| {
                                format!(
                                    "Invalid modification time found for item: {}",
                                    path.display()
                                )
                            })?,
                        size: stat
                            .size
                            .with_context(|| format!("Missing size on item: {}", path.display()))?,
                    })
                } else {
                    bail!("Unknown item type at: {}", path.display());
                }

                items.push(DriverItem {
                    path: path
                        .to_str()
                        .with_context(|| {
                            format!("File contains non-UTF-8 characters: {}", path.display())
                        })?
                        .to_string(),
                    metadata,
                });
            }
            todo!()
        };

        read_sub_dir(dir)
    }
}
