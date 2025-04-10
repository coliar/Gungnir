use core::cmp;

use crate::{debug, error, println, log};

use super::dir_entry::DirEntryEditor;
use super::error::Error;
use super::fs::{FileSystem, ReadWriteSeek};
use super::io::{IoBase, Read, Seek, SeekFrom, Write};
use super::time::{Date, DateTime, TimeProvider};

const MAX_FILE_SIZE: u32 = core::u32::MAX;

/// A FAT filesystem file object used for reading and writing data.
///
/// This struct is created by the `open_file` or `create_file` methods on `Dir`.
pub(crate) struct File<'a, IO: ReadWriteSeek, TP, OCC> {
    context: FileContext,
    // file-system reference
    fs: &'a FileSystem<IO, TP, OCC>,
}

/// A context of an existing [`File`].
#[derive(Clone)]
pub(crate) struct FileContext {
    // Note first_cluster is None if file is empty
    pub(super) first_cluster: Option<u32>,
    // Note: if offset points between clusters current_cluster is the previous cluster
    pub(super) current_cluster: Option<u32>,
    // current position in this file
    pub(super) offset: u32,
    // file dir entry editor - None for root dir
    pub(super) entry: Option<DirEntryEditor>,
}

/// An extent containing a file's data on disk.
#[derive(Clone, Debug)]
pub(crate) struct Extent {
    pub offset: u64,
    pub size: u32,
}

impl<'a, IO: ReadWriteSeek, TP, OCC> File<'a, IO, TP, OCC> {
    pub(super) fn new(
        first_cluster: Option<u32>,
        entry: Option<DirEntryEditor>,
        fs: &'a FileSystem<IO, TP, OCC>,
    ) -> Self {
        File {
            context: FileContext {
                first_cluster,
                entry,
                current_cluster: None, // cluster before first one
                offset: 0,
            },
            fs,
        }
    }

    /// Create a file from a prexisting [`FileContext`] & [`FileSystem`].
    pub(super) fn new_from_context(context: FileContext, fs: &'a FileSystem<IO, TP, OCC>) -> Self {
        File { context, fs }
    }

    /// Truncate file in current position.
    pub(crate) async fn truncate(&mut self) -> Result<(), Error<IO::Error>> {
        debug!("File::truncate");
        if let Some(ref mut e) = self.context.entry {
            e.set_size(self.context.offset);
            if self.context.offset == 0 {
                e.set_first_cluster(None, self.fs.fat_type());
            }
        } else {
            // Note: we cannot handle this case because there is no size field
            panic!("Trying to truncate a file without an entry");
        }
        if let Some(current_cluster) = self.context.current_cluster {
            // current cluster is none only if offset is 0
            debug_assert!(self.context.offset > 0);
            self.fs.truncate_cluster_chain(current_cluster).await
        } else {
            debug_assert!(self.context.offset == 0);
            if let Some(n) = self.context.first_cluster {
                self.fs.free_cluster_chain(n).await?;
                self.context.first_cluster = None;
            }
            Ok(())
        }
    }

    /// Get the extents of a file on disk.
    ///
    /// This returns an iterator over the byte ranges on-disk occupied by
    /// this file.
    // pub fn extents(&mut self) -> impl Iterator<Item = Result<Extent, Error<IO::Error>>> + 'a {
    // let fs = self.fs;
    // let cluster_size = fs.cluster_size();
    // let mut bytes_left = match self.size() {
    //     Some(s) => s,
    //     None => return None.into_iter().flatten(),
    // };
    // let first = match self.context.first_cluster {
    //     Some(f) => f,
    //     None => return None.into_iter().flatten(),
    // };

    // Some(
    //     core::iter::once(Ok(first))
    //         .chain(fs.cluster_iter(first))
    //         .map(move |cluster_err| match cluster_err {
    //             Ok(cluster) => {
    //                 let size = cluster_size.min(bytes_left);
    //                 bytes_left -= size;
    //                 Ok(Extent {
    //                     offset: fs.offset_from_cluster(cluster),
    //                     size,
    //                 })
    //             }
    //             Err(e) => Err(e),
    //         }),
    // )
    // .into_iter()
    // .flatten()
    // todo!("extents needs to be implemented using AsyncIterator");
    // }

    pub(super) fn abs_pos(&self) -> Option<u64> {
        // Returns current position relative to filesystem start
        // Note: when between clusters it returns position after previous cluster
        match self.context.current_cluster {
            Some(n) => {
                let cluster_size = self.fs.cluster_size();
                let offset_mod_cluster_size = self.context.offset % cluster_size;
                let offset_in_cluster = if offset_mod_cluster_size == 0 {
                    // position points between clusters - we are returning previous cluster so
                    // offset must be set to the cluster size
                    cluster_size
                } else {
                    offset_mod_cluster_size
                };
                let offset_in_fs = self.fs.offset_from_cluster(n) + u64::from(offset_in_cluster);
                Some(offset_in_fs)
            }
            None => None,
        }
    }

    async fn flush_dir_entry(&mut self) -> Result<(), Error<IO::Error>> {
        if let Some(ref mut e) = self.context.entry {
            e.flush(self.fs).await?;
        }
        Ok(())
    }

    /// Sets date and time of creation for this file.
    ///
    /// Note: it is set to a value from the `TimeProvider` when creating a file.
    /// Deprecated: if needed implement a custom `TimeProvider`.
    #[deprecated]
    pub(crate) fn set_created(&mut self, date_time: DateTime) {
        if let Some(ref mut e) = self.context.entry {
            e.set_created(date_time);
        }
    }

    /// Sets date of last access for this file.
    ///
    /// Note: it is overwritten by a value from the `TimeProvider` on every file read operation.
    /// Deprecated: if needed implement a custom `TimeProvider`.
    #[deprecated]
    pub(crate) fn set_accessed(&mut self, date: Date) {
        if let Some(ref mut e) = self.context.entry {
            e.set_accessed(date);
        }
    }

    /// Sets date and time of last modification for this file.
    ///
    /// Note: it is overwritten by a value from the `TimeProvider` on every file write operation.
    /// Deprecated: if needed implement a custom `TimeProvider`.
    #[deprecated]
    pub(crate) fn set_modified(&mut self, date_time: DateTime) {
        if let Some(ref mut e) = self.context.entry {
            e.set_modified(date_time);
        }
    }

    fn size(&self) -> Option<u32> {
        match self.context.entry {
            Some(ref e) => e.inner().size(),
            None => None,
        }
    }

    fn is_dir(&self) -> bool {
        match self.context.entry {
            Some(ref e) => e.inner().is_dir(),
            None => false,
        }
    }

    fn bytes_left_in_file(&self) -> Option<usize> {
        // Note: seeking beyond end of file is not allowed so overflow is impossible
        self.size().map(|s| (s - self.context.offset) as usize)
    }

    fn set_first_cluster(&mut self, cluster: u32) {
        self.context.first_cluster = Some(cluster);
        if let Some(ref mut e) = self.context.entry {
            e.set_first_cluster(self.context.first_cluster, self.fs.fat_type());
        }
    }

    pub(super) fn first_cluster(&self) -> Option<u32> {
        self.context.first_cluster
    }

    async fn flush(&mut self) -> Result<(), Error<IO::Error>> {
        self.flush_dir_entry().await?;
        let mut disk = self.fs.disk.borrow_mut();
        disk.flush().await?;
        Ok(())
    }
}

impl<IO: ReadWriteSeek, TP: TimeProvider, OCC> File<'_, IO, TP, OCC> {
    fn update_dir_entry_after_write(&mut self) {
        let offset = self.context.offset;
        if let Some(ref mut e) = self.context.entry {
            let now = self.fs.options.time_provider.get_current_date_time();
            e.set_modified(now);
            if e.inner().size().map_or(false, |s| offset > s) {
                e.set_size(offset);
            }
        }
    }

    /// Manually close the file
    ///
    /// A [`FileContext`] is returned, which can be used in conjunction with the
    /// `to_file_with_context` API.
    pub(crate) async fn close(self) -> Result<FileContext, Error<IO::Error>> {
        Ok(FileContext {
            first_cluster: self.context.first_cluster,
            current_cluster: self.context.current_cluster,
            offset: self.context.offset,
            entry: self.context.entry.clone(),
        })
    }
}

impl<IO: ReadWriteSeek, TP, OCC> Drop for File<'_, IO, TP, OCC> {
    fn drop(&mut self) {
        if let Some(e) = &self.context.entry {
            if e.dirty() {
                debug!("Dropping dirty file before flushing");
                #[cfg(feature = "dirty-file-panic")]
                {
                    panic!("Dropping unflushed file");
                }
            }
        }
    }
}

// Note: derive cannot be used because of invalid bounds. See: https://github.com/rust-lang/rust/issues/26925
impl<IO: ReadWriteSeek, TP, OCC> Clone for File<'_, IO, TP, OCC> {
    fn clone(&self) -> Self {
        File {
            context: self.context.clone(),
            fs: self.fs,
        }
    }
}

impl<IO: ReadWriteSeek, TP, OCC> IoBase for File<'_, IO, TP, OCC> {
    type Error = Error<IO::Error>;
}

impl<IO: ReadWriteSeek, TP: TimeProvider, OCC> Read for File<'_, IO, TP, OCC> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        debug!("File::read");
        let cluster_size = self.fs.cluster_size();
        let current_cluster_opt = if self.context.offset % cluster_size == 0 {
            // next cluster
            match self.context.current_cluster {
                None => self.context.first_cluster,
                Some(n) => {
                    let r = self.fs.cluster_iter(n).next().await;
                    match r {
                        Some(Err(err)) => return Err(err),
                        Some(Ok(n)) => Some(n),
                        None => None,
                    }
                }
            }
        } else {
            self.context.current_cluster
        };
        let current_cluster = match current_cluster_opt {
            Some(n) => n,
            None => return Ok(0),
        };
        let offset_in_cluster = self.context.offset % cluster_size;
        let bytes_left_in_cluster = (cluster_size - offset_in_cluster) as usize;
        let bytes_left_in_file = self.bytes_left_in_file().unwrap_or(bytes_left_in_cluster);
        let read_size = cmp::min(cmp::min(buf.len(), bytes_left_in_cluster), bytes_left_in_file);
        if read_size == 0 {
            return Ok(0);
        }
        debug!("read {} bytes in cluster {}", read_size, current_cluster);
        let offset_in_fs = self.fs.offset_from_cluster(current_cluster) + u64::from(offset_in_cluster);
        let read_bytes = {
            let mut disk = self.fs.disk.borrow_mut();
            disk.seek(SeekFrom::Start(offset_in_fs)).await?;
            disk.read(&mut buf[..read_size]).await?
        };
        if read_bytes == 0 {
            return Ok(0);
        }
        self.context.offset += read_bytes as u32;
        self.context.current_cluster = Some(current_cluster);

        if let Some(ref mut e) = self.context.entry {
            if self.fs.options.update_accessed_date {
                let now = self.fs.options.time_provider.get_current_date();
                e.set_accessed(now);
            }
        }
        Ok(read_bytes)
    }
}

impl<IO: ReadWriteSeek, TP: TimeProvider, OCC> Write for File<'_, IO, TP, OCC> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        debug!("File::write");
        let cluster_size = self.fs.cluster_size();
        let offset_in_cluster = self.context.offset % cluster_size;
        let bytes_left_in_cluster = (cluster_size - offset_in_cluster) as usize;
        let bytes_left_until_max_file_size = (MAX_FILE_SIZE - self.context.offset) as usize;
        let write_size = cmp::min(buf.len(), bytes_left_in_cluster);
        let write_size = cmp::min(write_size, bytes_left_until_max_file_size);
        // Exit early if we are going to write no data
        if write_size == 0 {
            return Ok(0);
        }
        // Mark the volume 'dirty'
        self.fs.set_dirty_flag(true).await?;
        // Get cluster for write possibly allocating new one
        let current_cluster = if self.context.offset % cluster_size == 0 {
            // next cluster
            let next_cluster = match self.context.current_cluster {
                None => self.context.first_cluster,
                Some(n) => {
                    let r = self.fs.cluster_iter(n).next().await;
                    match r {
                        Some(Err(err)) => return Err(err),
                        Some(Ok(n)) => Some(n),
                        None => None,
                    }
                }
            };
            if let Some(n) = next_cluster {
                n
            } else {
                // end of chain reached - allocate new cluster
                let new_cluster = self
                    .fs
                    .alloc_cluster(self.context.current_cluster, self.is_dir())
                    .await?;
                debug!("allocated cluster {}", new_cluster);
                if self.context.first_cluster.is_none() {
                    self.set_first_cluster(new_cluster);
                }
                new_cluster
            }
        } else {
            // self.context.current_cluster should be a valid cluster
            match self.context.current_cluster {
                Some(n) => n,
                None => panic!("Offset inside cluster but no cluster allocated"),
            }
        };
        debug!("write {} bytes in cluster {}", write_size, current_cluster);
        let offset_in_fs = self.fs.offset_from_cluster(current_cluster) + u64::from(offset_in_cluster);
        let written_bytes = {
            let mut disk = self.fs.disk.borrow_mut();
            disk.seek(SeekFrom::Start(offset_in_fs)).await?;
            disk.write(&buf[..write_size]).await?
        };
        if written_bytes == 0 {
            return Ok(0);
        }
        // some bytes were writter - update position and optionally size
        self.context.offset += written_bytes as u32;
        self.context.current_cluster = Some(current_cluster);
        self.update_dir_entry_after_write();
        Ok(written_bytes)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Self::flush(self).await
    }
}

impl<IO: ReadWriteSeek, TP, OCC> Seek for File<'_, IO, TP, OCC> {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        debug!("File::seek");
        let size_opt = self.size();
        let new_offset_opt: Option<u32> = match pos {
            SeekFrom::Current(x) => i64::from(self.context.offset)
                .checked_add(x)
                .and_then(|n| u32::try_from(n).ok()),
            SeekFrom::Start(x) => u32::try_from(x).ok(),
            SeekFrom::End(o) => size_opt
                .and_then(|s| i64::from(s).checked_add(o))
                .and_then(|n| u32::try_from(n).ok()),
        };
        let mut new_offset = if let Some(new_offset) = new_offset_opt {
            new_offset
        } else {
            error!("Invalid seek offset");
            return Err(Error::InvalidInput);
        };
        if let Some(size) = size_opt {
            if new_offset > size {
                debug!("Seek beyond the end of the file");
                new_offset = size;
            }
        }
        debug!(
            "file seek {} -> {} - entry {:?}",
            self.context.offset,
            new_offset,
            self.context.entry
        );
        if new_offset == self.context.offset {
            // position is the same - nothing to do
            return Ok(u64::from(self.context.offset));
        }
        let new_offset_in_clusters = self.fs.clusters_from_bytes(u64::from(new_offset));
        let old_offset_in_clusters = self.fs.clusters_from_bytes(u64::from(self.context.offset));
        let new_cluster = if new_offset == 0 {
            None
        } else if new_offset_in_clusters == old_offset_in_clusters {
            self.context.current_cluster
        } else if let Some(first_cluster) = self.context.first_cluster {
            // calculate number of clusters to skip
            // return the previous cluster if the offset points to the cluster boundary
            // Note: new_offset_in_clusters cannot be 0 here because new_offset is not 0
            debug_assert!(new_offset_in_clusters > 0);
            let clusters_to_skip = new_offset_in_clusters - 1;
            let mut cluster = first_cluster;
            let mut iter = self.fs.cluster_iter(first_cluster);
            for i in 0..clusters_to_skip {
                cluster = if let Some(r) = iter.next().await {
                    r?
                } else {
                    // cluster chain ends before the new position - seek to the end of the last cluster
                    new_offset = self.fs.bytes_from_clusters(i + 1) as u32;
                    break;
                };
            }
            Some(cluster)
        } else {
            // empty file - always seek to 0
            new_offset = 0;
            None
        };
        self.context.offset = new_offset;
        self.context.current_cluster = new_cluster;
        Ok(u64::from(self.context.offset))
    }
}
