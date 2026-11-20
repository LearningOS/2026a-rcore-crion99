use easy_fs::{BlockDevice, EasyFileSystem, BLOCK_SZ};
use std::sync::{Arc, Mutex};

struct MemoryBlockDevice(Mutex<Vec<[u8; BLOCK_SZ]>>);

impl MemoryBlockDevice {
    fn new(blocks: usize) -> Self {
        Self(Mutex::new(vec![[0; BLOCK_SZ]; blocks]))
    }
}

impl BlockDevice for MemoryBlockDevice {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        buf.copy_from_slice(&self.0.lock().unwrap()[block_id]);
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0.lock().unwrap()[block_id].copy_from_slice(buf);
    }
}

#[test]
fn hard_link_lifecycle() {
    let device: Arc<dyn BlockDevice> = Arc::new(MemoryBlockDevice::new(4096));
    let fs = EasyFileSystem::create(device, 4096, 1);
    let root = EasyFileSystem::root_inode(&fs);
    let file = root.create("old").unwrap();
    assert_eq!(file.write_at(0, b"hello"), 5);
    assert_eq!(file.stat().2, 1);
    assert!(root.link_at("old", "new"));
    assert_eq!(file.stat().2, 2);
    assert!(root.unlink_at("old"));
    assert!(root.find("old").is_none());
    let linked = root.find("new").unwrap();
    assert_eq!(linked.stat().2, 1);
    let mut data = [0; 5];
    assert_eq!(linked.read_at(0, &mut data), 5);
    assert_eq!(&data, b"hello");
    assert!(root.unlink_at("new"));
    assert!(root.find("new").is_none());
    assert!(root.create("reused").is_some());
}
