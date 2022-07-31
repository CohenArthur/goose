use super::mm;
use core::num::TryFromIntError;

#[derive(Debug)]
pub enum Error {
    CannotMapNoAlloc, // TODO: put in the mm::{PAddr, VAddr}
    InvalidConversion(TryFromIntError),
}

impl From<TryFromIntError> for Error {
    fn from(tfie: TryFromIntError) -> Self {
        Self::InvalidConversion(tfie)
    }
}

pub trait PagingImpl {
    fn new<'alloc>(mm: &mut mm::MemoryManager) -> &'alloc mut Self;

    fn get_page_size() -> usize;
    fn get_uppermost_address() -> usize;

    fn align_down(addr: usize) -> usize {
        let page_size = Self::get_page_size();
        let page_mask = !(page_size - 1);

        addr & page_mask
    }

    fn align_up(addr: usize) -> usize {
        let page_size = Self::get_page_size();
        ((addr + page_size - 1) / page_size) * page_size
    }

    fn map(
        &mut self,
        mm: &mut mm::MemoryManager,
        pa: mm::PAddr,
        va: mm::VAddr,
        perms: mm::Permissions,
    ) -> Result<(), Error>;

    fn map_noalloc(
        &mut self,
        pa: mm::PAddr,
        va: mm::VAddr,
        perms: mm::Permissions,
    ) -> Result<(), Error>;

    fn add_invalid_entry(
        &mut self,
        mm: &mut mm::MemoryManager,
        vaddr: mm::VAddr,
    ) -> Result<(), Error>;

    fn reload(&mut self);
    fn disable(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_tests::TestContext;

    struct PagingImplDummy {}
    impl PagingImpl for PagingImplDummy {
        fn new<'alloc>(_mm: &mut mm::MemoryManager) -> &'alloc mut Self {
            unreachable!("We will never use this, we just need the compiler to be happy");
        }

        fn get_page_size() -> usize {
            4096
        }

        fn get_uppermost_address() -> usize {
            unreachable!("We will never use this, we just need the compiler to be happy");
        }

        fn map(
            &mut self,
            _mm: &mut mm::MemoryManager,
            _pa: mm::PAddr,
            _va: mm::VAddr,
            _perms: mm::Permissions,
        ) -> Result<(), Error> {
            unreachable!("We will never use this, we just need the compiler to be happy");
        }

        fn map_noalloc(
            &mut self,
            _pa: mm::PAddr,
            _va: mm::VAddr,
            _perms: mm::Permissions,
        ) -> Result<(), Error> {
            unreachable!("We will never use this, we just need the compiler to be happy");
        }

        fn add_invalid_entry(
            &mut self,
            _mm: &mut mm::MemoryManager,
            _vaddr: mm::VAddr,
        ) -> Result<(), Error> {
            unreachable!("We will never use this, we just need the compiler to be happy");
        }

        fn reload(&mut self) {}
        fn disable(&mut self) {}
    }

    #[test_case]
    fn align_down(_ctx: &mut TestContext) {
        assert!(PagingImplDummy::align_down(0x1042) == 0x1000);
    }

    #[test_case]
    fn align_up(_ctx: &mut TestContext) {
        assert!(PagingImplDummy::align_up(0x1042) == 0x2000);
    }
}