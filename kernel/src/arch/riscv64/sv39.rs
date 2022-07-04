use crate::arch;
use crate::arch_mem::{ArchitectureMemory, Error};
use crate::mm;
use core::arch::asm;
use core::convert::TryInto;
use modular_bitfield::{bitfield, prelude::*};

#[repr(C)]
pub struct VAddr {
    addr: u64,
}

impl From<mm::VAddr> for VAddr {
    fn from(vaddr: mm::VAddr) -> Self {
        Self::from_u64(usize::from(vaddr).try_into().unwrap())
    }
}

impl VAddr {
    pub fn from_u64(addr: u64) -> VAddr {
        let mut vaddr = Self { addr };

        let bit38 = (vaddr.vpn(2) >> 8) != 0;

        vaddr.extend_38th_bit(bit38);

        vaddr
    }

    pub fn vpn(&self, nb: usize) -> u16 {
        let vpn = self.addr >> 12;

        ((vpn >> (nb * 9)) & 0x1ff) as u16
    }

    fn extend_38th_bit(&mut self, bit: bool) {
        let mask = ((!1u64) >> 38) << 38;

        if bit {
            self.addr |= mask; // set all the bits above the 38th
        } else {
            self.addr &= !mask; // clear all the bits above the 38th
        }
    }
}

#[repr(C)]
pub struct PAddr {
    addr: u64,
}

impl From<mm::PAddr> for PAddr {
    fn from(paddr: mm::PAddr) -> Self {
        Self::from_u64(usize::from(paddr).try_into().unwrap())
    }
}

impl PAddr {
    pub fn from_u64(addr: u64) -> Self {
        let mut paddr = Self { addr };

        let bit55 = (paddr.ppn(2) >> 8) != 0;

        paddr.extend_55th_bit(bit55);

        paddr
    }

    fn ppn(&self, nb: usize) -> u64 {
        let ppn = self.addr >> 12;

        if nb == 2 {
            (ppn >> (nb * 9)) & 0x3fffff
        } else {
            (ppn >> (nb * 9)) & 0x1ff
        }
    }

    fn extend_55th_bit(&mut self, bit: bool) {
        let mask = ((!0u64) >> 55) << 55;

        if bit {
            self.addr |= mask; // set all the bits above the 55th
        } else {
            self.addr &= !mask; // clear all the bits above the 55th
        }
    }
}

#[repr(u64)]
#[bitfield]
struct PageTableEntry {
    v: B1,
    r: B1,
    w: B1,
    x: B1,
    #[skip]
    u: B1,
    #[skip]
    g: B1,
    #[skip]
    a: B1,
    #[skip]
    d: B1,
    #[skip]
    rsw: B2,
    ppn0: B9,
    ppn1: B9,
    ppn2: B26,
    #[skip]
    reserved: B10,
}

impl PageTableEntry {
    fn clear(&mut self) {
        *self = PageTableEntry::from_bytes([0u8; 8])
    }

    fn is_valid(&self) -> bool {
        self.v() == 1
    }

    fn is_leaf(&self) -> bool {
        self.r() != 0 || self.w() != 0 || self.x() != 0
    }

    fn set_valid(&mut self) {
        self.set_v(1)
    }

    fn set_paddr(&mut self, paddr: &PAddr) {
        self.set_ppn2(paddr.ppn(2) as u32);
        self.set_ppn1(paddr.ppn(1) as u16);
        self.set_ppn0(paddr.ppn(0) as u16);
    }

    fn set_target(&mut self, pt: *mut PageTable) {
        let addr = pt as u64;
        self.set_paddr(&PAddr::from_u64(addr))
    }

    fn set_perms(&mut self, perms: mm::Permissions) {
        self.set_r(perms.contains(mm::Permissions::READ) as u8);
        self.set_w(perms.contains(mm::Permissions::WRITE) as u8);
        self.set_x(perms.contains(mm::Permissions::EXECUTE) as u8);
    }

    fn get_target(&mut self) -> &mut PageTable {
        let addr =
            ((self.ppn2() as u64) << 18 | (self.ppn1() as u64) << 9 | self.ppn0() as u64) * 4096u64;
        unsafe { (addr as *mut PageTable).as_mut().unwrap() }
    }
}

pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    fn map_inner(
        &mut self,
        mut allocator: Option<&mut mm::PhysicalMemoryManager>,
        paddr: PAddr,
        vaddr: VAddr,
        perms: mm::Permissions,
        level: usize,
    ) -> Result<&mut PageTableEntry, Error> {
        let vpn = vaddr.vpn(level);

        let pte = &mut self.entries[vpn as usize];

        if level == 0 {
            pte.set_paddr(&paddr);
            pte.set_perms(perms);
            pte.set_valid();
            return Ok(pte);
        }

        if !pte.is_valid() {
            assert!(!pte.is_leaf());

            if let Some(alloc) = allocator.as_mut() {
                let new_page_table = PageTable::new(alloc);
                pte.set_target(new_page_table as *mut PageTable);
                pte.set_valid();
            } else {
                return Err(Error::CannotMapNoAlloc);
            }
        }

        pte.get_target()
            .map_inner(allocator, paddr, vaddr, perms, level - 1)
    }
}

impl ArchitectureMemory for PageTable {
    fn new<'alloc>(allocator: &mut mm::PhysicalMemoryManager) -> &'alloc mut Self {
        // FIXME: No unwrap here
        let page = allocator.alloc_pages(1).unwrap();
        let page_table: *mut PageTable = page.into();
        // FIXME: Do not unwrap either
        let page_table = unsafe { page_table.as_mut().unwrap() };

        page_table.entries.iter_mut().for_each(|pte| pte.clear());

        page_table
    }

    fn get_page_size() -> usize {
        4096
    }

    fn map(
        &mut self,
        allocator: &mut mm::PhysicalMemoryManager,
        pa: mm::PAddr,
        va: mm::VAddr,
        perms: mm::Permissions,
    ) -> Result<(), Error> {
        self.map_inner(Some(allocator), pa.into(), va.into(), perms, 2)?;

        Ok(())
    }

    fn map_noalloc(
        &mut self,
        pa: mm::PAddr,
        va: mm::VAddr,
        perms: mm::Permissions,
    ) -> Result<(), Error> {
        self.map_inner(None, pa.into(), va.into(), perms, 2)?;

        Ok(())
    }

    fn add_invalid_entry(
        &mut self,
        allocator: &mut mm::PhysicalMemoryManager,
        vaddr: mm::VAddr,
    ) -> Result<(), Error> {
        let pte = self.map_inner(
            Some(allocator),
            PAddr::from_u64(0x0A0A_0A0A_0A0A_0A0A),
            vaddr.into(),
            mm::Permissions::READ,
            2,
        )?;

        pte.set_v(0);

        Ok(())
    }

    fn reload(&mut self) {
        load_pt(self)
    }

    fn disable(&mut self) {
        let satp = Satp::new()
            .with_ppn(0)
            .with_asid(0)
            .with_mode(SatpMode::Bare as u8);

        unsafe {
            asm!("csrw satp, {}", in(reg)u64::from(satp));
            asm!("sfence.vma");
        }
    }
}

#[repr(u8)]
pub enum SatpMode {
    Bare = 0,
    Sv39 = 8,
    _Sv48 = 9,
    _Sv57 = 10,
    _Sv64 = 11,
}

#[repr(u64)]
#[bitfield]
pub struct Satp {
    #[skip(getters)]
    ppn: B44,
    #[skip(getters)]
    asid: B16,
    #[skip(getters)]
    mode: B4,
}

pub fn load_pt(pt: &PageTable) {
    let pt_addr = pt as *const PageTable as usize;
    let ppn = pt_addr >> 12;

    let satp = Satp::new()
        .with_ppn(ppn as u64)
        .with_asid(0)
        .with_mode(SatpMode::Sv39 as u8);

    unsafe {
        asm!("csrw satp, {}", in(reg)u64::from(satp));
        asm!("sfence.vma");
    }
}
