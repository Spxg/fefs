use core::cmp::min;
use alloc::sync::Arc;
use alloc:: vec::Vec;
use super::BLOCK_SIZE;
use super::device::BlockDevice;
use super::sblock::SuperBlock;
use super::cache::get_block_cache;
use super::fat::{
    alloc_clusters, 
    dealloc_clusters
};
use super::{
    iter_sector,
    iter_sector_mut
};

#[derive(Debug)]
pub enum FileError {
    BufTooSmall,
}

pub enum WriteType {
    OverWritten,
    Append,
}

#[repr(C)]
struct Data {
    inner: [u8; BLOCK_SIZE]
}

impl Data {
    fn empty() -> Self {
        Data {
            inner: [0; BLOCK_SIZE]
        }
    }

    fn copy_from_slice(buf: &[u8]) -> Self {
        let mut data = Data::empty();
        data.inner[0..buf.len()].copy_from_slice(buf);
        data
    }
}

pub struct FileEntry {
    pub(crate) device: Arc<dyn BlockDevice>,
    pub(crate) clusters: Vec<usize>,
    pub(crate) size: usize,
    pub(crate) sblock: SuperBlock,
}

impl FileEntry {
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, FileError> {
        if self.size > buf.len() { return Err(FileError::BufTooSmall) }
        let mut idx = 0;
        let size = self.size;
        iter_sector!(self, |data: &Data| {
            let start = idx * BLOCK_SIZE;
            let end = min((idx + 1) * BLOCK_SIZE, size);
            buf[start..end].copy_from_slice(&data.inner[0..end - start]);
            idx += 1;
            end == size
        });
        Ok(self.size)
    }

    pub fn write(&mut self, buf: &[u8], write_type: WriteType) -> Result<(), FileError> {
        let mut idx = 0;
        let len = buf.len();
        match write_type {
            WriteType::OverWritten => {
                self.clean_data();
                dealloc_clusters(self.clusters[0]);
                self.clusters = alloc_clusters(buf.len());
                iter_sector_mut!(self, |data: &mut Data| {
                    let start = idx * BLOCK_SIZE;
                    let end = min((idx + 1) * BLOCK_SIZE, len);
                    *data = Data::copy_from_slice(&buf[start..end]);
                    idx += 1;
                    end == len
                });
            }
            WriteType::Append => {}
        }
        self.size = buf.len();
        Ok(())
    }

    pub(crate) fn clean_data(&mut self) {
        iter_sector_mut!(self, |data: &mut Data| {
            *data = Data::empty();
            false
        });
    }
}
