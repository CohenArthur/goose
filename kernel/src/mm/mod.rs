mod page_alloc;
mod physical_memory_manager;

pub use physical_memory_manager::PhysicalMemoryManager;

use crate::arch_mem::ArchitectureMemory;
use crate::device_tree::DeviceTree;
use crate::utils;

use bitflags::bitflags;

extern "C" {
    pub static KERNEL_START: usize;
    pub static KERNEL_END: usize;
}

bitflags! {
    pub struct Permissions: u8 {
        const READ    = 0b00000001;
        const WRITE   = 0b00000010;
        const EXECUTE = 0b00000100;
    }
}

#[derive(Clone, Copy)]
pub struct VAddr {
    addr: usize,
}

impl core::convert::From<usize> for VAddr {
    fn from(val: usize) -> Self {
        Self { addr: val }
    }
}

impl core::convert::From<VAddr> for usize {
    fn from(val: VAddr) -> Self {
        val.addr
    }
}

#[derive(Clone, Copy)]
pub struct PAddr {
    addr: usize,
}

impl core::convert::From<usize> for PAddr {
    fn from(val: usize) -> Self {
        Self { addr: val }
    }
}

impl core::convert::From<PAddr> for usize {
    fn from(val: PAddr) -> Self {
        val.addr
    }
}

impl<T> core::convert::From<PAddr> for *mut T {
    fn from(val: PAddr) -> Self {
        val.addr as *mut T
    }
}

impl<T> core::convert::From<&PAddr> for *mut T {
    fn from(val: &PAddr) -> Self {
        val.addr as *mut T
    }
}

pub fn is_kernel_page(base: usize) -> bool {
    let (kernel_start, kernel_end) = unsafe {
        (
            utils::external_symbol_value(&KERNEL_START),
            utils::external_symbol_value(&KERNEL_END),
        )
    };

    base >= kernel_start && base < kernel_end
}

pub fn is_reserved_page(base: usize, device_tree: &DeviceTree) -> bool {
    let mut is_res = false;

    device_tree.for_all_reserved_memory_regions(|regions| {
        is_res = regions
            .map(|(start, size)| (start, size)) // this is a weird hack to fix a type error.
            .any(|(region_start, region_size)| {
                base >= region_start && base <= (region_start + region_size)
            })
    });

    is_res
}

fn map_kernel_rwx(mm: &mut crate::MemoryImpl, pmm: &mut PhysicalMemoryManager, page_size: usize) {
    let kernel_start = unsafe { utils::external_symbol_value(&KERNEL_START) };
    let kernel_end = unsafe { utils::external_symbol_value(&KERNEL_END) };
    let kernel_end_align = ((kernel_end + page_size - 1) / page_size) * page_size;

    for addr in (kernel_start..kernel_end_align).step_by(page_size) {
        mm.map(
            pmm,
            PAddr::from(addr),
            VAddr::from(addr),
            Permissions::READ | Permissions::WRITE | Permissions::EXECUTE,
        );
    }
}

pub fn map_address_space(
    device_tree: &DeviceTree,
    page_table: &mut crate::MemoryImpl,
    pmm: &mut PhysicalMemoryManager,
    drivers: &[&dyn drivers::Driver],
) {
    let page_size = pmm.page_size();

    map_kernel_rwx(page_table, pmm, page_size);

    let metadata_pages = pmm.metadata_pages();
    metadata_pages.for_each(|page| page_table.map(pmm, PAddr::from(page), VAddr::from(page), Permissions::READ | Permissions::WRITE));

    drivers
        .iter()
        .map(|drv| drv.get_address_range())
        .flat_map(|(base, len)| (base..(base+len)).step_by(page_size))
        .for_each(|page|
            page_table.map(pmm, PAddr::from(page), VAddr::from(page), Permissions::READ | Permissions::WRITE));

    page_table.reload();
}
