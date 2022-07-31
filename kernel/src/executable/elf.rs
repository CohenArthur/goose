use core::iter::Iterator;

use crate::mm;
use crate::paging::PagingImpl;

use goblin;
use goblin::elf::header::header64::Header;
use goblin::elf::program_header::program_header64::ProgramHeader;
use goblin::elf::program_header::*;

pub struct Elf<'a> {
    load_addr: usize,
    data: &'a [u8],
}

impl<'a> Elf<'a> {
    /// Create a new Elf struct from a byte slice
    pub fn from_bytes(data: &'a [u8]) -> Self {
        Self {
            load_addr: data.as_ptr() as usize,
            data,
        }
    }

    /// Get the header struct of an ELF file
    fn header(&self) -> &Header {
        let header_slice = self.data[..64].try_into().unwrap();

        Header::from_bytes(header_slice)
    }

    /// Get an iterator over all the segment of an ELF file
    fn segments(&self) -> impl Iterator<Item = &ProgramHeader> + '_ {
        let header = self.header();

        (0..header.e_phnum)
            .map(|n| {
                self.load_addr
                    + header.e_phoff as usize
                    + (n as usize * header.e_phentsize as usize)
            })
            .map(|addr| unsafe { &(*(addr as *const ProgramHeader)) })
    }

    pub fn get_entry_point(&self) -> usize {
        self.header().e_entry as usize
    }

    fn pages_needed(
        segment: &goblin::elf64::program_header::ProgramHeader,
        page_size: usize,
    ) -> usize {
        let p_memsz = segment.p_memsz as usize;

        if p_memsz < page_size {
            1
        } else {
            p_memsz / page_size
        }
    }

    pub fn load(&self, pagetable: &mut mm::UserPageTable, mm: &mut mm::MemoryManager) {
        let page_size = mm.page_size();

        for segment in self.segments() {
            if segment.p_type != PT_LOAD {
                continue;
            }

            let p_offset = segment.p_offset as usize;
            let p_filesz = segment.p_filesz as usize;
            let p_memsz = segment.p_memsz as usize;

            let pages_needed = Self::pages_needed(segment, page_size);
            let physical_pages = mm.alloc_pages(pages_needed).unwrap();
            let virtual_pages = segment.p_paddr as *mut u8;

            let segment_data_src_addr = (self.load_addr + p_offset) as *const u8;
            let segment_data_dst_addr = (usize::from(physical_pages) + p_offset) as *mut u8;

            let segment_data_src: &[u8] =
                unsafe { core::slice::from_raw_parts(segment_data_src_addr, p_filesz) };
            let segment_data_dst: &mut [u8] = {
                let dst =
                    unsafe { core::slice::from_raw_parts_mut(segment_data_dst_addr, p_memsz) };

                // Zeroing uninitialized data
                for i in p_filesz..p_memsz {
                    dst[i as usize] = 0u8;
                }

                dst
            };

            segment_data_dst[0..p_filesz].clone_from_slice(segment_data_src);

            let perms = elf_to_mm_permissions(segment.p_flags);

            for i in 0..pages_needed {
                let page_offset = i * page_size;
                // FIXME: No unwrap
                pagetable
                    .map(
                        mm,
                        usize::from(physical_pages) + page_offset,
                        crate::PagingImpl::align_down(virtual_pages as usize) + page_offset,
                        perms,
                    )
                    .unwrap();
            }
        }
    }
}

/// Convert ELF p_flags permissions to mm::Permissions
fn elf_to_mm_permissions(elf_permsission: u32) -> mm::Permissions {
    // An elf file can only be mapped in user space
    let mut perms = mm::Permissions::USER;

    if elf_permsission & PF_R != 0 {
        perms |= mm::Permissions::READ;
    }

    if elf_permsission & PF_W != 0 {
        perms |= mm::Permissions::WRITE;
    }

    if elf_permsission & PF_X != 0 {
        perms |= mm::Permissions::EXECUTE;
    }

    perms
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_tests::*;

    #[test_case]
    fn elf_load(ctx: &mut TestContext) {
        ctx.reset();

        let elf_bytes = core::include_bytes!("../../fixtures/small");
        let elf = Elf::from_bytes(elf_bytes);

        let mut user_pagetable = mm::UserPageTable::new(&mut ctx.mm).unwrap();

        elf.load(&mut user_pagetable, &mut ctx.mm);
    }
}