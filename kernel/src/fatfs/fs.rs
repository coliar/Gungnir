use core::borrow::BorrowMut;
use core::cell::{Cell, RefCell};
use core::char;
use core::cmp;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::u32;

#[cfg(all(not(feature = "std"), feature = "alloc", feature = "lfn"))]
use alloc::string::String;
#[cfg(feature = "std")]
use embedded_io_adapters::tokio_1::FromTokio;

use crate::{debug, error, println, log};

use super::boot_sector::{format_boot_sector, BiosParameterBlock, BootSector};
use super::dir::{Dir, DirRawStream};
use super::dir_entry::{DirFileEntryData, FileAttributes, SFN_PADDING, SFN_SIZE};
use super::error::Error;
use super::file::File;
use super::io::{self, IoBase, Read, ReadLeExt, Seek, SeekFrom, Write, WriteLeExt};
use super::table::{
    alloc_cluster, count_free_clusters, format_fat, read_fat_flags, ClusterIterator, RESERVED_FAT_ENTRIES,
};
use super::time::{DefaultTimeProvider, TimeProvider};


/// A type of FAT filesystem.
///
/// `FatType` values are based on the size of File Allocation Table entry.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum FatType {
    /// 12 bits per FAT entry
    Fat12,
    /// 16 bits per FAT entry
    Fat16,
    /// 32 bits per FAT entry
    Fat32,
}

impl FatType {
    const FAT16_MIN_CLUSTERS: u32 = 4085;
    const FAT32_MIN_CLUSTERS: u32 = 65525;
    const FAT32_MAX_CLUSTERS: u32 = 0x0FFF_FFF4;

    pub(super) fn from_clusters(total_clusters: u32) -> Self {
        if total_clusters < Self::FAT16_MIN_CLUSTERS {
            FatType::Fat12
        } else if total_clusters < Self::FAT32_MIN_CLUSTERS {
            FatType::Fat16
        } else {
            FatType::Fat32
        }
    }

    pub(super) fn bits_per_fat_entry(self) -> u32 {
        match self {
            FatType::Fat12 => 12,
            FatType::Fat16 => 16,
            FatType::Fat32 => 32,
        }
    }

    pub(super) fn min_clusters(self) -> u32 {
        match self {
            FatType::Fat12 => 0,
            FatType::Fat16 => Self::FAT16_MIN_CLUSTERS,
            FatType::Fat32 => Self::FAT32_MIN_CLUSTERS,
        }
    }

    pub(super) fn max_clusters(self) -> u32 {
        match self {
            FatType::Fat12 => Self::FAT16_MIN_CLUSTERS - 1,
            FatType::Fat16 => Self::FAT32_MIN_CLUSTERS - 1,
            FatType::Fat32 => Self::FAT32_MAX_CLUSTERS,
        }
    }
}

/// A FAT volume status flags retrived from the Boot Sector and the allocation table second entry.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct FsStatusFlags {
    pub(crate) dirty: bool,
    pub(crate) io_error: bool,
}

impl FsStatusFlags {
    /// Checks if the volume is marked as dirty.
    ///
    /// Dirty flag means volume has been suddenly ejected from filesystem without unmounting.
    #[must_use]
    pub(crate) fn dirty(&self) -> bool {
        self.dirty
    }

    /// Checks if the volume has the IO Error flag active.
    #[must_use]
    pub(crate) fn io_error(&self) -> bool {
        self.io_error
    }

    fn encode(self) -> u8 {
        let mut res = 0_u8;
        if self.dirty {
            res |= 1;
        }
        if self.io_error {
            res |= 2;
        }
        res
    }

    pub(super) fn decode(flags: u8) -> Self {
        Self {
            dirty: flags & 1 != 0,
            io_error: flags & 2 != 0,
        }
    }
}

/// A sum of `Read` and `Seek` traits.
pub(crate) trait ReadSeek: Read + Seek {}
impl<T: IoBase + Read + Seek> ReadSeek for T {}

/// A sum of `Read`, `Write` and `Seek` traits.
pub(crate) trait ReadWriteSeek: Read + Write + Seek {}

impl<T: IoBase + Read + Write + Seek> ReadWriteSeek for T {}

#[derive(Clone, Default, Debug)]
struct FsInfoSector {
    free_cluster_count: Option<u32>,
    next_free_cluster: Option<u32>,
    dirty: bool,
}

impl FsInfoSector {
    const LEAD_SIG: u32 = 0x4161_5252;
    const STRUC_SIG: u32 = 0x6141_7272;
    const TRAIL_SIG: u32 = 0xAA55_0000;

    async fn deserialize<R: Read>(rdr: &mut R) -> Result<Self, Error<R::Error>> {
        let lead_sig = rdr.read_u32_le().await?;
        if lead_sig != Self::LEAD_SIG {
            error!("invalid lead_sig in FsInfo sector: {}", lead_sig);
            return Err(Error::CorruptedFileSystem);
        }
        let mut reserved = [0_u8; 480];
        rdr.read_exact(&mut reserved).await?;
        let struc_sig = rdr.read_u32_le().await?;
        if struc_sig != Self::STRUC_SIG {
            error!("invalid struc_sig in FsInfo sector: {}", struc_sig);
            return Err(Error::CorruptedFileSystem);
        }
        let free_cluster_count = match rdr.read_u32_le().await? {
            0xFFFF_FFFF => None,
            // Note: value is validated in FileSystem::new function using values from BPB
            n => Some(n),
        };
        let next_free_cluster = match rdr.read_u32_le().await? {
            0xFFFF_FFFF => None,
            0 | 1 => {
                error!("invalid next_free_cluster in FsInfo sector (values 0 and 1 are reserved)");
                None
            }
            // Note: other values are validated in FileSystem::new function using values from BPB
            n => Some(n),
        };
        let mut reserved2 = [0_u8; 12];
        rdr.read_exact(&mut reserved2).await?;
        let trail_sig = rdr.read_u32_le().await?;
        if trail_sig != Self::TRAIL_SIG {
            error!("invalid trail_sig in FsInfo sector: {}", trail_sig);
            return Err(Error::CorruptedFileSystem);
        }
        Ok(Self {
            free_cluster_count,
            next_free_cluster,
            dirty: false,
        })
    }

    async fn serialize<W: Write>(&self, wrt: &mut W) -> Result<(), Error<W::Error>> {
        wrt.write_u32_le(Self::LEAD_SIG).await?;
        let reserved = [0_u8; 480];
        wrt.write_all(&reserved).await?;
        wrt.write_u32_le(Self::STRUC_SIG).await?;
        wrt.write_u32_le(self.free_cluster_count.unwrap_or(0xFFFF_FFFF)).await?;
        wrt.write_u32_le(self.next_free_cluster.unwrap_or(0xFFFF_FFFF)).await?;
        let reserved2 = [0_u8; 12];
        wrt.write_all(&reserved2).await?;
        wrt.write_u32_le(Self::TRAIL_SIG).await?;
        wrt.flush().await?;
        Ok(())
    }

    fn validate_and_fix(&mut self, total_clusters: u32) {
        let max_valid_cluster_number = total_clusters + RESERVED_FAT_ENTRIES;
        if let Some(n) = self.free_cluster_count {
            if n > total_clusters {
                error!(
                    "invalid free_cluster_count ({}) in fs_info exceeds total cluster count ({})",
                    n, total_clusters
                );
                self.free_cluster_count = None;
            }
        }
        if let Some(n) = self.next_free_cluster {
            if n > max_valid_cluster_number {
                error!(
                    "invalid free_cluster_count ({}) in fs_info exceeds maximum cluster number ({})",
                    n, max_valid_cluster_number
                );
                self.next_free_cluster = None;
            }
        }
    }

    fn map_free_clusters(&mut self, map_fn: impl Fn(u32) -> u32) {
        if let Some(n) = self.free_cluster_count {
            self.free_cluster_count = Some(map_fn(n));
            self.dirty = true;
        }
    }

    fn set_next_free_cluster(&mut self, cluster: u32) {
        self.next_free_cluster = Some(cluster);
        self.dirty = true;
    }

    fn set_free_cluster_count(&mut self, free_cluster_count: u32) {
        self.free_cluster_count = Some(free_cluster_count);
        self.dirty = true;
    }
}

/// A FAT filesystem mount options.
///
/// Options are specified as an argument for `FileSystem::new` method.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct FsOptions<TP, OCC> {
    pub(crate) update_accessed_date: bool,
    pub(crate) oem_cp_converter: OCC,
    pub(crate) time_provider: TP,
}

impl FsOptions<DefaultTimeProvider, LossyOemCpConverter> {
    /// Creates a `FsOptions` struct with default options.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            update_accessed_date: false,
            oem_cp_converter: LossyOemCpConverter::new(),
            time_provider: DefaultTimeProvider::new(),
        }
    }
}

impl<TP: TimeProvider, OCC: OemCpConverter> FsOptions<TP, OCC> {
    /// If enabled accessed date field in directory entry is updated when reading or writing a file.
    #[must_use]
    pub(crate) fn update_accessed_date(mut self, enabled: bool) -> Self {
        self.update_accessed_date = enabled;
        self
    }

    /// Changes default OEM code page encoder-decoder.
    pub(crate) fn oem_cp_converter<OCC2: OemCpConverter>(self, oem_cp_converter: OCC2) -> FsOptions<TP, OCC2> {
        FsOptions::<TP, OCC2> {
            update_accessed_date: self.update_accessed_date,
            oem_cp_converter,
            time_provider: self.time_provider,
        }
    }

    /// Changes default time provider.
    pub(crate) fn time_provider<TP2: TimeProvider>(self, time_provider: TP2) -> FsOptions<TP2, OCC> {
        FsOptions::<TP2, OCC> {
            update_accessed_date: self.update_accessed_date,
            oem_cp_converter: self.oem_cp_converter,
            time_provider,
        }
    }
}

/// A FAT volume statistics.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct FileSystemStats {
    cluster_size: u32,
    total_clusters: u32,
    free_clusters: u32,
}

impl FileSystemStats {
    /// Cluster size in bytes
    #[must_use]
    pub(crate) fn cluster_size(&self) -> u32 {
        self.cluster_size
    }

    /// Number of total clusters in filesystem usable for file allocation
    #[must_use]
    pub(crate) fn total_clusters(&self) -> u32 {
        self.total_clusters
    }

    /// Number of free clusters
    #[must_use]
    pub(crate) fn free_clusters(&self) -> u32 {
        self.free_clusters
    }
}

/// A FAT filesystem object.
///
/// `FileSystem` struct is representing a state of a mounted FAT volume.
pub(crate) struct FileSystem<IO: Read + Write + Seek, TP, OCC> {
    pub(super) disk: RefCell<IO>,
    pub(super) options: FsOptions<TP, OCC>,
    fat_type: FatType,
    bpb: BiosParameterBlock,
    first_data_sector: u32,
    root_dir_sectors: u32,
    total_clusters: u32,
    fs_info: RefCell<FsInfoSector>,
    current_status_flags: Cell<FsStatusFlags>,
}

/// The underlying storage device
///
/// Implement this on the underlying storage device, for example, this could be a file or an in-memory buffer.
pub(crate) trait IntoStorage<T: Read + Write + Seek> {
    fn into_storage(self) -> T;
}

impl<T: ReadWriteSeek> IntoStorage<T> for T {
    fn into_storage(self) -> Self {
        self
    }
}

#[cfg(feature = "std")]
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + tokio::io::AsyncSeek + Unpin> IntoStorage<FromTokio<T>> for T {
    fn into_storage(self) -> FromTokio<Self> {
        FromTokio::new(self)
    }
}

impl<IO: ReadWriteSeek, TP, OCC> FileSystem<IO, TP, OCC> {
    pub(crate) async fn new<T: IntoStorage<IO>>(storage: T, options: FsOptions<TP, OCC>) -> Result<Self, Error<IO::Error>> {
        let mut disk = storage.into_storage();
        debug!("FileSystem::new");
        debug_assert!(disk.seek(SeekFrom::Current(0)).await? == 0);

        // read boot sector
        let bpb = {
            let boot = BootSector::deserialize(&mut disk).await?;
            boot.validate()?;
            boot.bpb
        };

        let root_dir_sectors = bpb.root_dir_sectors();
        let first_data_sector = bpb.first_data_sector();
        let total_clusters = bpb.total_clusters();
        let fat_type = FatType::from_clusters(total_clusters);

        // read FSInfo sector if this is FAT32
        let mut fs_info = if fat_type == FatType::Fat32 {
            disk.seek(SeekFrom::Start(bpb.bytes_from_sectors(bpb.fs_info_sector())))
                .await?;
            FsInfoSector::deserialize(&mut disk).await?
        } else {
            FsInfoSector::default()
        };

        // if dirty flag is set completly ignore free_cluster_count in FSInfo
        if bpb.status_flags().dirty {
            fs_info.free_cluster_count = None;
        }
        // Validate the numbers stored in the free_cluster_count and next_free_cluster are within bounds for volume
        fs_info.validate_and_fix(total_clusters);

        // return FileSystem struct
        let status_flags = bpb.status_flags();
        Ok(Self {
            disk: RefCell::new(disk),
            options,
            fat_type,
            bpb,
            first_data_sector,
            root_dir_sectors,
            total_clusters,
            fs_info: RefCell::new(fs_info),
            current_status_flags: Cell::new(status_flags),
        })
    }

    /// Returns a type of File Allocation Table (FAT) used by this filesystem.
    pub(crate) fn fat_type(&self) -> FatType {
        self.fat_type
    }

    /// Returns a volume identifier read from BPB in the Boot Sector.
    pub(crate) fn volume_id(&self) -> u32 {
        self.bpb.volume_id
    }

    pub(crate) fn volume_label_as_bytes(&self) -> &[u8] {
        let full_label_slice = &self.bpb.volume_label;
        let len = full_label_slice
            .iter()
            .rposition(|b| *b != SFN_PADDING)
            .map_or(0, |p| p + 1);
        &full_label_slice[..len]
    }

    fn offset_from_sector(&self, sector: u32) -> u64 {
        self.bpb.bytes_from_sectors(sector)
    }

    fn sector_from_cluster(&self, cluster: u32) -> u32 {
        self.first_data_sector + self.bpb.sectors_from_clusters(cluster - RESERVED_FAT_ENTRIES)
    }

    pub(crate) fn cluster_size(&self) -> u32 {
        self.bpb.cluster_size()
    }

    pub(super) fn offset_from_cluster(&self, cluster: u32) -> u64 {
        self.offset_from_sector(self.sector_from_cluster(cluster))
    }

    pub(super) fn bytes_from_clusters(&self, clusters: u32) -> u64 {
        self.bpb.bytes_from_sectors(self.bpb.sectors_from_clusters(clusters))
    }

    pub(super) fn clusters_from_bytes(&self, bytes: u64) -> u32 {
        self.bpb.clusters_from_bytes(bytes)
    }

    fn fat_slice(&self) -> impl ReadWriteSeek<Error = Error<IO::Error>> + '_ {
        let io = FsIoAdapter { fs: self };
        fat_slice(io, &self.bpb)
    }

    pub(super) fn cluster_iter(
        &self,
        cluster: u32,
    ) -> ClusterIterator<impl ReadWriteSeek<Error = Error<IO::Error>> + '_, IO::Error> {
        let disk_slice = self.fat_slice();
        ClusterIterator::new(disk_slice, self.fat_type, cluster)
    }

    pub(super) async fn truncate_cluster_chain(&self, cluster: u32) -> Result<(), Error<IO::Error>> {
        let mut iter = self.cluster_iter(cluster);
        let num_free = iter.truncate().await?;
        let mut fs_info = self.fs_info.borrow_mut();
        fs_info.map_free_clusters(|n| n + num_free);
        Ok(())
    }

    pub(super) async fn free_cluster_chain(&self, cluster: u32) -> Result<(), Error<IO::Error>> {
        let mut iter = self.cluster_iter(cluster);
        let num_free = iter.free().await?;
        let mut fs_info = self.fs_info.borrow_mut();
        fs_info.map_free_clusters(|n| n + num_free);
        Ok(())
    }

    pub(super) async fn alloc_cluster(&self, prev_cluster: Option<u32>, zero: bool) -> Result<u32, Error<IO::Error>> {
        debug!("alloc_cluster");
        let hint = self.fs_info.borrow().next_free_cluster;
        let cluster = {
            let mut fat = self.fat_slice();
            alloc_cluster(&mut fat, self.fat_type, prev_cluster, hint, self.total_clusters).await?
        };
        if zero {
            let mut disk = self.disk.borrow_mut();
            disk.seek(SeekFrom::Start(self.offset_from_cluster(cluster))).await?;
            write_zeros(&mut *disk, u64::from(self.cluster_size())).await?;
        }
        let mut fs_info = self.fs_info.borrow_mut();
        fs_info.set_next_free_cluster(cluster + 1);
        fs_info.map_free_clusters(|n| n - 1);
        Ok(cluster)
    }

    pub(crate) async fn read_status_flags(&self) -> Result<FsStatusFlags, Error<IO::Error>> {
        let bpb_status = self.bpb.status_flags();
        let fat_status = read_fat_flags(&mut self.fat_slice(), self.fat_type).await?;
        Ok(FsStatusFlags {
            dirty: bpb_status.dirty || fat_status.dirty,
            io_error: bpb_status.io_error || fat_status.io_error,
        })
    }

    pub(crate) async fn stats(&self) -> Result<FileSystemStats, Error<IO::Error>> {
        let free_clusters_option = self.fs_info.borrow().free_cluster_count;
        let free_clusters = if let Some(n) = free_clusters_option {
            n
        } else {
            self.recalc_free_clusters().await?
        };
        Ok(FileSystemStats {
            cluster_size: self.cluster_size(),
            total_clusters: self.total_clusters,
            free_clusters,
        })
    }

    /// Forces free clusters recalculation.
    async fn recalc_free_clusters(&self) -> Result<u32, Error<IO::Error>> {
        let mut fat = self.fat_slice();
        let free_cluster_count = count_free_clusters(&mut fat, self.fat_type, self.total_clusters).await?;
        self.fs_info.borrow_mut().set_free_cluster_count(free_cluster_count);
        Ok(free_cluster_count)
    }

    pub(crate) async fn unmount(self) -> Result<(), Error<IO::Error>> {
        self.flush().await
    }

    pub(crate) async fn flush(&self) -> Result<(), Error<IO::Error>> {
        self.flush_fs_info().await?;
        self.set_dirty_flag(false).await?;
        Ok(())
    }

    async fn flush_fs_info(&self) -> Result<(), Error<IO::Error>> {
        let mut fs_info = self.fs_info.borrow_mut();
        if self.fat_type == FatType::Fat32 && fs_info.dirty {
            let mut disk = self.disk.borrow_mut();
            let fs_info_sector_offset = self.offset_from_sector(u32::from(self.bpb.fs_info_sector));
            disk.seek(SeekFrom::Start(fs_info_sector_offset)).await?;
            fs_info.serialize(&mut *disk).await?;
            fs_info.dirty = false;
        }
        Ok(())
    }

    pub(super) async fn set_dirty_flag(&self, dirty: bool) -> Result<(), IO::Error> {
        // Do not overwrite flags read from BPB on mount
        let mut flags = self.bpb.status_flags();
        flags.dirty |= dirty;
        // Check if flags has changed
        let current_flags = self.current_status_flags.get();
        if flags == current_flags {
            // Nothing to do
            return Ok(());
        }
        let encoded = flags.encode();
        // Note: only one field is written to avoid rewriting entire boot-sector which could be dangerous
        // Compute reserver_1 field offset and write new flags
        let offset = if self.fat_type() == FatType::Fat32 {
            0x041
        } else {
            0x025
        };
        let mut disk = self.disk.borrow_mut();
        disk.seek(io::SeekFrom::Start(offset)).await?;
        disk.write_u8(encoded).await?;
        disk.flush().await?;
        self.current_status_flags.set(flags);
        Ok(())
    }

    /// Returns a root directory object allowing for futher penetration of a filesystem structure.
    pub(crate) fn root_dir(&self) -> Dir<IO, TP, OCC> {
        debug!("root_dir");
        let root_rdr = {
            match self.fat_type {
                FatType::Fat12 | FatType::Fat16 => DirRawStream::Root(DiskSlice::from_sectors(
                    self.first_data_sector - self.root_dir_sectors,
                    self.root_dir_sectors,
                    1,
                    &self.bpb,
                    FsIoAdapter { fs: self },
                )),
                FatType::Fat32 => DirRawStream::File(File::new(Some(self.bpb.root_dir_first_cluster), None, self)),
            }
        };
        Dir::new(root_rdr, self)
    }
}

impl<IO: ReadWriteSeek, TP, OCC: OemCpConverter> FileSystem<IO, TP, OCC> {
    #[cfg(feature = "alloc")]
    pub(crate) fn volume_label(&self) -> String {
        // Decode volume label from OEM codepage
        let volume_label_iter = self.volume_label_as_bytes().iter().copied();
        let char_iter = volume_label_iter.map(|c| self.options.oem_cp_converter.decode(c));
        // Build string from character iterator
        char_iter.collect()
    }
}

impl<IO: ReadWriteSeek, TP: TimeProvider, OCC: OemCpConverter> FileSystem<IO, TP, OCC> {
    #[cfg(feature = "alloc")]
    pub(crate) async fn read_volume_label_from_root_dir(&self) -> Result<Option<String>, Error<IO::Error>> {
        // Note: DirEntry::file_short_name() cannot be used because it interprets name as 8.3
        // (adds dot before an extension)
        let volume_label_opt = self.read_volume_label_from_root_dir_as_bytes().await?;
        volume_label_opt.map_or(Ok(None), |volume_label| {
            // Strip label padding
            let len = volume_label
                .iter()
                .rposition(|b| *b != SFN_PADDING)
                .map_or(0, |p| p + 1);
            let label_slice = &volume_label[..len];
            // Decode volume label from OEM codepage
            let volume_label_iter = label_slice.iter().copied();
            let char_iter = volume_label_iter.map(|c| self.options.oem_cp_converter.decode(c));
            // Build string from character iterator
            Ok(Some(char_iter.collect::<String>()))
        })
    }

    pub(crate) async fn read_volume_label_from_root_dir_as_bytes(&self) -> Result<Option<[u8; SFN_SIZE]>, Error<IO::Error>> {
        let entry_opt = self.root_dir().find_volume_entry().await?;
        Ok(entry_opt.map(|e| *e.raw_short_name()))
    }
}

/// `Drop` implementation tries to unmount the filesystem when dropping.
impl<IO: Read + Write + Seek, TP, OCC> Drop for FileSystem<IO, TP, OCC> {
    fn drop(&mut self) {
        if self.current_status_flags.get().dirty {
            debug!("Dropping FileSytem without unmount");
        }
    }
}

pub(super) struct FsIoAdapter<'a, IO: ReadWriteSeek, TP, OCC> {
    fs: &'a FileSystem<IO, TP, OCC>,
}

impl<IO: ReadWriteSeek, TP, OCC> IoBase for FsIoAdapter<'_, IO, TP, OCC> {
    type Error = IO::Error;
}

impl<IO: ReadWriteSeek, TP, OCC> Read for FsIoAdapter<'_, IO, TP, OCC> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.fs.disk.borrow_mut().read(buf).await
    }
}

impl<IO: ReadWriteSeek, TP, OCC> Write for FsIoAdapter<'_, IO, TP, OCC> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let size = self.fs.disk.borrow_mut().write(buf).await?;
        if size > 0 {
            self.fs.set_dirty_flag(true).await?;
        }
        Ok(size)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.fs.disk.borrow_mut().flush().await
    }
}

impl<IO: ReadWriteSeek, TP, OCC> Seek for FsIoAdapter<'_, IO, TP, OCC> {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        self.fs.disk.borrow_mut().seek(pos).await
    }
}

// Note: derive cannot be used because of invalid bounds. See: https://github.com/rust-lang/rust/issues/26925
impl<IO: ReadWriteSeek, TP, OCC> Clone for FsIoAdapter<'_, IO, TP, OCC> {
    fn clone(&self) -> Self {
        FsIoAdapter { fs: self.fs }
    }
}

fn fat_slice<S: ReadWriteSeek, B: BorrowMut<S>>(
    io: B,
    bpb: &BiosParameterBlock,
) -> impl ReadWriteSeek<Error = Error<S::Error>> {
    let sectors_per_fat = bpb.sectors_per_fat();
    let mirroring_enabled = bpb.mirroring_enabled();
    let (fat_first_sector, mirrors) = if mirroring_enabled {
        (bpb.reserved_sectors(), bpb.fats)
    } else {
        let active_fat = u32::from(bpb.active_fat());
        let fat_first_sector = (bpb.reserved_sectors()) + active_fat * sectors_per_fat;
        (fat_first_sector, 1)
    };
    DiskSlice::from_sectors(fat_first_sector, sectors_per_fat, mirrors, bpb, io)
}

pub(super) struct DiskSlice<B, S = B> {
    begin: u64,
    size: u64,
    offset: u64,
    mirrors: u8,
    inner: B,
    phantom: PhantomData<S>,
}

impl<B: BorrowMut<S>, S: ReadWriteSeek> DiskSlice<B, S> {
    pub(super) fn new(begin: u64, size: u64, mirrors: u8, inner: B) -> Self {
        Self {
            begin,
            size,
            mirrors,
            inner,
            offset: 0,
            phantom: PhantomData,
        }
    }

    fn from_sectors(first_sector: u32, sector_count: u32, mirrors: u8, bpb: &BiosParameterBlock, inner: B) -> Self {
        Self::new(
            bpb.bytes_from_sectors(first_sector),
            bpb.bytes_from_sectors(sector_count),
            mirrors,
            inner,
        )
    }

    pub(super) fn abs_pos(&self) -> u64 {
        self.begin + self.offset
    }
}

impl<B: Clone, S> Clone for DiskSlice<B, S> {
    fn clone(&self) -> Self {
        Self {
            begin: self.begin,
            size: self.size,
            offset: self.offset,
            mirrors: self.mirrors,
            inner: self.inner.clone(),
            // phantom is needed to add type bounds on the storage type
            phantom: PhantomData,
        }
    }
}

impl<B, S: IoBase> IoBase for DiskSlice<B, S> {
    type Error = Error<S::Error>;
}

impl<B: BorrowMut<S>, S: Read + Seek> Read for DiskSlice<B, S> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let offset = self.begin + self.offset;
        let read_size = cmp::min(self.size - self.offset, buf.len() as u64) as usize;
        self.inner.borrow_mut().seek(SeekFrom::Start(offset)).await?;
        let size = self.inner.borrow_mut().read(&mut buf[..read_size]).await?;
        self.offset += size as u64;
        Ok(size)
    }
}

impl<B: BorrowMut<S>, S: Write + Seek> Write for DiskSlice<B, S> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let offset = self.begin + self.offset;
        let write_size = cmp::min(self.size - self.offset, buf.len() as u64) as usize;
        if write_size == 0 {
            return Ok(0);
        }
        // Write data
        let storage = self.inner.borrow_mut();
        for i in 0..self.mirrors {
            storage.seek(SeekFrom::Start(offset + u64::from(i) * self.size)).await?;
            storage.write_all(&buf[..write_size]).await?;
        }
        self.offset += write_size as u64;
        Ok(write_size)
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(self.inner.borrow_mut().flush().await?)
    }
}

impl<B, S: IoBase> Seek for DiskSlice<B, S> {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        let new_offset_opt: Option<u64> = match pos {
            SeekFrom::Current(x) => i64::try_from(self.offset)
                .ok()
                .and_then(|n| n.checked_add(x))
                .and_then(|n| u64::try_from(n).ok()),
            SeekFrom::Start(x) => Some(x),
            SeekFrom::End(o) => i64::try_from(self.size)
                .ok()
                .and_then(|size| size.checked_add(o))
                .and_then(|n| u64::try_from(n).ok()),
        };
        if let Some(new_offset) = new_offset_opt {
            if new_offset > self.size {
                error!("Seek beyond the end of the file");
                Err(Error::InvalidInput)
            } else {
                self.offset = new_offset;
                Ok(self.offset)
            }
        } else {
            error!("Invalid seek offset");
            Err(Error::InvalidInput)
        }
    }
}

pub(crate) trait OemCpConverter: Debug {
    fn decode(&self, oem_char: u8) -> char;
    fn encode(&self, uni_char: char) -> Option<u8>;
}

impl<T: OemCpConverter + ?Sized> OemCpConverter for &T {
    fn decode(&self, oem_char: u8) -> char {
        (*self).decode(oem_char)
    }

    fn encode(&self, uni_char: char) -> Option<u8> {
        (*self).encode(uni_char)
    }
}

/// Default implementation of `OemCpConverter` that changes all non-ASCII characters to the replacement character (U+FFFD).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LossyOemCpConverter {
    _dummy: (),
}

impl LossyOemCpConverter {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self { _dummy: () }
    }
}

impl OemCpConverter for LossyOemCpConverter {
    fn decode(&self, oem_char: u8) -> char {
        if oem_char <= 0x7F {
            char::from(oem_char)
        } else {
            '\u{FFFD}'
        }
    }
    fn encode(&self, uni_char: char) -> Option<u8> {
        if uni_char <= '\x7F' {
            Some(uni_char as u8) // safe cast: value is in range [0, 0x7F]
        } else {
            None
        }
    }
}

async fn write_zeros<IO: ReadWriteSeek>(disk: &mut IO, mut len: u64) -> Result<(), IO::Error> {
    const ZEROS: [u8; 512] = [0_u8; 512];
    while len > 0 {
        let write_size = cmp::min(len, ZEROS.len() as u64) as usize;
        disk.write_all(&ZEROS[..write_size]).await?;
        len -= write_size as u64;
    }
    Ok(())
}

async fn write_zeros_until_end_of_sector<IO: ReadWriteSeek>(
    disk: &mut IO,
    bytes_per_sector: u16,
) -> Result<(), IO::Error> {
    let pos = disk.seek(SeekFrom::Current(0)).await?;
    let total_bytes_to_write = u64::from(bytes_per_sector) - (pos % u64::from(bytes_per_sector));
    if total_bytes_to_write != u64::from(bytes_per_sector) {
        write_zeros(disk, total_bytes_to_write).await?;
    }
    Ok(())
}

#[derive(Default, Debug, Clone)]
pub(crate) struct FormatVolumeOptions {
    pub(super) bytes_per_sector: Option<u16>,
    pub(super) total_sectors: Option<u32>,
    pub(super) bytes_per_cluster: Option<u32>,
    pub(super) fat_type: Option<FatType>,
    pub(super) max_root_dir_entries: Option<u16>,
    pub(super) fats: Option<u8>,
    pub(super) media: Option<u8>,
    pub(super) sectors_per_track: Option<u16>,
    pub(super) heads: Option<u16>,
    pub(super) drive_num: Option<u8>,
    pub(super) volume_id: Option<u32>,
    pub(super) volume_label: Option<[u8; SFN_SIZE]>,
}

impl FormatVolumeOptions {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub(crate) fn bytes_per_cluster(mut self, bytes_per_cluster: u32) -> Self {
        assert!(
            bytes_per_cluster.count_ones() == 1 && bytes_per_cluster >= 512,
            "Invalid bytes_per_cluster"
        );
        self.bytes_per_cluster = Some(bytes_per_cluster);
        self
    }

    #[must_use]
    pub(crate) fn fat_type(mut self, fat_type: FatType) -> Self {
        self.fat_type = Some(fat_type);
        self
    }

    #[must_use]
    pub(crate) fn bytes_per_sector(mut self, bytes_per_sector: u16) -> Self {
        assert!(
            bytes_per_sector.count_ones() == 1 && bytes_per_sector >= 512,
            "Invalid bytes_per_sector"
        );
        self.bytes_per_sector = Some(bytes_per_sector);
        self
    }

    #[must_use]
    pub(crate) fn total_sectors(mut self, total_sectors: u32) -> Self {
        self.total_sectors = Some(total_sectors);
        self
    }

    #[must_use]
    pub(crate) fn max_root_dir_entries(mut self, max_root_dir_entries: u16) -> Self {
        self.max_root_dir_entries = Some(max_root_dir_entries);
        self
    }

    #[must_use]
    pub(crate) fn fats(mut self, fats: u8) -> Self {
        assert!((1..=2).contains(&fats), "Invalid number of FATs");
        self.fats = Some(fats);
        self
    }

    #[must_use]
    pub(crate) fn media(mut self, media: u8) -> Self {
        self.media = Some(media);
        self
    }

    #[must_use]
    pub(crate) fn sectors_per_track(mut self, sectors_per_track: u16) -> Self {
        self.sectors_per_track = Some(sectors_per_track);
        self
    }

    #[must_use]
    pub(crate) fn heads(mut self, heads: u16) -> Self {
        self.heads = Some(heads);
        self
    }

    #[must_use]
    pub(crate) fn drive_num(mut self, drive_num: u8) -> Self {
        self.drive_num = Some(drive_num);
        self
    }

    #[must_use]
    pub(crate) fn volume_id(mut self, volume_id: u32) -> Self {
        self.volume_id = Some(volume_id);
        self
    }


    #[must_use]
    pub(crate) fn volume_label(mut self, volume_label: [u8; SFN_SIZE]) -> Self {
        self.volume_label = Some(volume_label);
        self
    }
}

/// Create FAT filesystem on a disk or partition (format a volume)
#[allow(clippy::needless_pass_by_value)]
pub(crate) async fn format_volume<S: ReadWriteSeek>(
    storage: &mut S,
    options: FormatVolumeOptions,
) -> Result<(), Error<S::Error>> {
    debug!("format_volume");
    debug_assert!(storage.seek(SeekFrom::Current(0)).await? == 0);

    let bytes_per_sector = options.bytes_per_sector.unwrap_or(512);
    let total_sectors = if let Some(total_sectors) = options.total_sectors {
        total_sectors
    } else {
        let total_bytes: u64 = storage.seek(SeekFrom::End(0)).await?;
        let total_sectors_64 = total_bytes / u64::from(bytes_per_sector);
        storage.seek(SeekFrom::Start(0)).await?;
        if total_sectors_64 > u64::from(u32::MAX) {
            error!("Volume has too many sectors: {}", total_sectors_64);
            return Err(Error::InvalidInput);
        }
        total_sectors_64 as u32 // safe case: possible overflow is handled above
    };

    // Create boot sector, validate and write to storage device
    let (boot, fat_type) = format_boot_sector(&options, total_sectors, bytes_per_sector)?;
    if boot.validate::<S::Error>().is_err() {
        return Err(Error::InvalidInput);
    }
    boot.serialize(storage).await?;
    // Make sure entire logical sector is updated (serialize method always writes 512 bytes)
    let bytes_per_sector = boot.bpb.bytes_per_sector;
    write_zeros_until_end_of_sector(storage, bytes_per_sector).await?;

    let bpb = &boot.bpb;
    if bpb.is_fat32() {
        // FSInfo sector
        let fs_info_sector = FsInfoSector {
            free_cluster_count: None,
            next_free_cluster: None,
            dirty: false,
        };
        storage
            .seek(SeekFrom::Start(bpb.bytes_from_sectors(bpb.fs_info_sector())))
            .await?;
        fs_info_sector.serialize(storage).await?;
        write_zeros_until_end_of_sector(storage, bytes_per_sector).await?;

        // backup boot sector
        storage
            .seek(SeekFrom::Start(bpb.bytes_from_sectors(bpb.backup_boot_sector())))
            .await?;
        boot.serialize(storage).await?;
        write_zeros_until_end_of_sector(storage, bytes_per_sector).await?;
    }

    // format File Allocation Table
    let reserved_sectors = bpb.reserved_sectors();
    let fat_pos = bpb.bytes_from_sectors(reserved_sectors);
    let sectors_per_all_fats = bpb.sectors_per_all_fats();
    storage.seek(SeekFrom::Start(fat_pos)).await?;
    write_zeros(storage, bpb.bytes_from_sectors(sectors_per_all_fats)).await?;
    {
        let mut fat_slice = fat_slice::<S, &mut S>(storage, bpb);
        let sectors_per_fat = bpb.sectors_per_fat();
        let bytes_per_fat = bpb.bytes_from_sectors(sectors_per_fat);
        format_fat(&mut fat_slice, fat_type, bpb.media, bytes_per_fat, bpb.total_clusters()).await?;
    }

    // init root directory - zero root directory region for FAT12/16 and alloc first root directory cluster for FAT32
    let root_dir_first_sector = reserved_sectors + sectors_per_all_fats;
    let root_dir_sectors = bpb.root_dir_sectors();
    let root_dir_pos = bpb.bytes_from_sectors(root_dir_first_sector);
    storage.seek(SeekFrom::Start(root_dir_pos)).await?;
    write_zeros(storage, bpb.bytes_from_sectors(root_dir_sectors)).await?;
    if fat_type == FatType::Fat32 {
        let root_dir_first_cluster = {
            let mut fat_slice = fat_slice::<S, &mut S>(storage, bpb);
            alloc_cluster(&mut fat_slice, fat_type, None, None, 1).await?
        };
        assert!(root_dir_first_cluster == bpb.root_dir_first_cluster);
        let first_data_sector = reserved_sectors + sectors_per_all_fats + root_dir_sectors;
        let data_sectors_before_root_dir = bpb.sectors_from_clusters(root_dir_first_cluster - RESERVED_FAT_ENTRIES);
        let fat32_root_dir_first_sector = first_data_sector + data_sectors_before_root_dir;
        let fat32_root_dir_pos = bpb.bytes_from_sectors(fat32_root_dir_first_sector);
        storage.seek(SeekFrom::Start(fat32_root_dir_pos)).await?;
        write_zeros(storage, u64::from(bpb.cluster_size())).await?;
    }

    // Create volume label directory entry if volume label is specified in options
    if let Some(volume_label) = options.volume_label {
        storage.seek(SeekFrom::Start(root_dir_pos)).await?;
        let volume_entry = DirFileEntryData::new(volume_label, FileAttributes::VOLUME_ID);
        volume_entry.serialize(storage).await?;
    }

    storage.flush().await?;
    storage.seek(SeekFrom::Start(0)).await?;
    debug!("format_volume end");
    Ok(())
}
