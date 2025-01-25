// copied from https://github.com/rust-osdev/linked-list-allocator

#![allow(dead_code)]

use core::ptr::NonNull;
use core::mem::{align_of, size_of};
use core::alloc::{Layout, LayoutError};
use core::mem;
use core::mem::MaybeUninit;
use spin::mutex::Mutex;
use core::alloc::GlobalAlloc;
use core::ops::Deref;
use crate::println;

struct Hole {
    size: usize,
    next: Option<NonNull<Hole>>,
}

#[derive(Debug, Clone, Copy)]
struct HoleInfo {
    addr: *mut u8,
    size: usize,
}

struct HoleList {
    first: Hole,
    bottom: *mut u8,
    top: *mut u8,
    pending_extend: u8,
}

impl HoleList {
    const fn empty() -> HoleList {
        HoleList {
            first: Hole {
                size: 0,
                next: None,
            },
            bottom: core::ptr::null_mut(),
            top: core::ptr::null_mut(),
            pending_extend: 0,
        }
    }

    fn cursor(&mut self) -> Option<Cursor> {
        if let Some(hole) = self.first.next {
            Some(Cursor {
                hole,
                prev: NonNull::new(&mut self.first)?,
                top: self.top,
            })
        } else {
            None
        }
    }

    fn debug(&mut self) {
        if let Some(cursor) = self.cursor() {
            let mut cursor = cursor;
            loop {
                println!(
                    "prev: {:?}[{}], hole: {:?}[{}]",
                    cursor.previous() as *const Hole,
                    cursor.previous().size,
                    cursor.current() as *const Hole,
                    cursor.current().size,
                );
                if let Some(c) = cursor.next() {
                    cursor = c;
                } else {
                    println!("Done!");
                    return;
                }
            }
        } else {
            println!("No holes");
        }
    }

    unsafe fn new(hole_addr: *mut u8, hole_size: usize) -> HoleList {
        assert_eq!(size_of::<Hole>(), Self::min_size());
        assert!(hole_size >= size_of::<Hole>());

        let aligned_hole_addr = align_up(hole_addr, align_of::<Hole>());
        let requested_hole_size = hole_size - ((aligned_hole_addr as usize) - (hole_addr as usize));
        let aligned_hole_size = align_down_size(requested_hole_size, align_of::<Hole>());
        assert!(aligned_hole_size >= size_of::<Hole>());

        let ptr = aligned_hole_addr as *mut Hole;
        ptr.write(Hole {
            size: aligned_hole_size,
            next: None,
        });

        assert_eq!(
            hole_addr.wrapping_add(hole_size),
            aligned_hole_addr.wrapping_add(requested_hole_size)
        );

        HoleList {
            first: Hole {
                size: 0,
                next: Some(NonNull::new_unchecked(ptr)),
            },
            bottom: aligned_hole_addr,
            top: aligned_hole_addr.wrapping_add(aligned_hole_size),
            pending_extend: (requested_hole_size - aligned_hole_size) as u8,
        }
    }

    fn align_layout(layout: Layout) -> Result<Layout, LayoutError> {
        let mut size = layout.size();
        if size < Self::min_size() {
            size = Self::min_size();
        }
        let size = align_up_size(size, mem::align_of::<Hole>());
        Layout::from_size_align(size, layout.align())
    }

    fn allocate_first_fit(&mut self, layout: Layout) -> Result<(NonNull<u8>, Layout), ()> {
        let aligned_layout = Self::align_layout(layout).map_err(|_| ())?;
        let mut cursor = self.cursor().ok_or(())?;

        loop {
            match cursor.split_current(aligned_layout) {
                Ok((ptr, _len)) => {
                    return Ok((NonNull::new(ptr).ok_or(())?, aligned_layout));
                }
                Err(curs) => {
                    cursor = curs.next().ok_or(())?;
                }
            }
        }
    }

    unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) -> Layout {
        let aligned_layout = Self::align_layout(layout).unwrap();
        deallocate(self, ptr.as_ptr(), aligned_layout.size());
        aligned_layout
    }

    fn min_size() -> usize {
        size_of::<usize>() * 2
    }

    #[cfg(test)]
    fn first_hole(&self) -> Option<(*const u8, usize)> {
        self.first.next.as_ref().map(|hole| {
            (hole.as_ptr() as *mut u8 as *const u8, unsafe {
                hole.as_ref().size
            })
        })
    }

    unsafe fn extend(&mut self, by: usize) {
        assert!(!self.top.is_null(), "tried to extend an empty heap");

        let top = self.top;

        let dead_space = top.align_offset(align_of::<Hole>());
        debug_assert_eq!(
            0, dead_space,
            "dead space detected during extend: {} bytes. This means top was unaligned",
            dead_space
        );

        debug_assert!(
            (self.pending_extend as usize) < Self::min_size(),
            "pending extend was larger than expected"
        );

        let extend_by = self.pending_extend as usize + by;

        let minimum_extend = Self::min_size();
        if extend_by < minimum_extend {
            self.pending_extend = extend_by as u8;
            return;
        }

        let new_hole_size = align_down_size(extend_by, align_of::<Hole>());
        let layout = Layout::from_size_align(new_hole_size, 1).unwrap();

        self.deallocate(NonNull::new_unchecked(top as *mut u8), layout);
        self.top = top.add(new_hole_size);

        self.pending_extend = (extend_by - new_hole_size) as u8;
    }
}

struct Cursor {
    prev: core::ptr::NonNull<Hole>,
    hole: core::ptr::NonNull<Hole>,
    top: *mut u8,
}

impl Cursor {
    fn next(mut self) -> Option<Self> {
        unsafe {
            self.hole.as_mut().next.map(|nhole| Cursor {
                prev: self.hole,
                hole: nhole,
                top: self.top,
            })
        }
    }

    fn current(&self) -> &Hole {
        unsafe { self.hole.as_ref() }
    }

    fn previous(&self) -> &Hole {
        unsafe { self.prev.as_ref() }
    }

    fn split_current(self, required_layout: Layout) -> Result<(*mut u8, usize), Self> {
        let front_padding;
        let alloc_ptr;
        let alloc_size;
        let back_padding;

        {
            let hole_size = self.current().size;
            let hole_addr_u8 = self.hole.as_ptr().cast::<u8>();
            let required_size = required_layout.size();
            let required_align = required_layout.align();
            if hole_size < required_size {
                return Err(self);
            }

            let aligned_addr = if hole_addr_u8 == align_up(hole_addr_u8, required_align) {
                front_padding = None;
                hole_addr_u8
            } else {
                let new_start = hole_addr_u8.wrapping_add(HoleList::min_size());

                let aligned_addr = align_up(new_start, required_align);
                front_padding = Some(HoleInfo {
                    addr: hole_addr_u8,
                    size: (aligned_addr as usize) - (hole_addr_u8 as usize),
                });
                aligned_addr
            };

            let allocation_end = aligned_addr.wrapping_add(required_size);
            let hole_end = hole_addr_u8.wrapping_add(hole_size);

            if allocation_end > hole_end {
                return Err(self);
            }

            alloc_ptr = aligned_addr;
            alloc_size = required_size;

            let back_padding_size = hole_end as usize - allocation_end as usize;
            back_padding = if back_padding_size == 0 {
                None
            } else {
                let hole_layout = Layout::new::<Hole>();
                let back_padding_start = align_up(allocation_end, hole_layout.align());
                let back_padding_end = back_padding_start.wrapping_add(hole_layout.size());

                if back_padding_end <= hole_end {
                    Some(HoleInfo {
                        addr: back_padding_start,
                        size: back_padding_size,
                    })
                } else {
                    return Err(self);
                }
            };
        }

        let Cursor {
            mut prev, mut hole, ..
        } = self;
        unsafe {
            prev.as_mut().next = None;
        }
        let maybe_next_addr: Option<NonNull<Hole>> = unsafe { hole.as_mut().next.take() };

        match (front_padding, back_padding) {
            (None, None) => {
                unsafe {
                    prev.as_mut().next = maybe_next_addr;
                }
            }
            (None, Some(singlepad)) | (Some(singlepad), None) => unsafe {
                let singlepad_ptr = singlepad.addr.cast::<Hole>();
                singlepad_ptr.write(Hole {
                    size: singlepad.size,
                    next: maybe_next_addr,
                });

                prev.as_mut().next = Some(NonNull::new_unchecked(singlepad_ptr));
            },
            (Some(frontpad), Some(backpad)) => unsafe {
                let backpad_ptr = backpad.addr.cast::<Hole>();
                backpad_ptr.write(Hole {
                    size: backpad.size,
                    next: maybe_next_addr,
                });

                let frontpad_ptr = frontpad.addr.cast::<Hole>();
                frontpad_ptr.write(Hole {
                    size: frontpad.size,
                    next: Some(NonNull::new_unchecked(backpad_ptr)),
                });

                prev.as_mut().next = Some(NonNull::new_unchecked(frontpad_ptr));
            },
        }

        Ok((alloc_ptr, alloc_size))
    }
}

impl Cursor {
    fn try_insert_back(self, node: NonNull<Hole>, bottom: *mut u8) -> Result<Self, Self> {
        if node < self.hole {
            let node_u8 = node.as_ptr().cast::<u8>();
            let node_size = unsafe { node.as_ref().size };
            let hole_u8 = self.hole.as_ptr().cast::<u8>();

            assert!(
                node_u8.wrapping_add(node_size) <= hole_u8,
                "Freed node aliases existing hole! Bad free?",
            );
            debug_assert_eq!(self.previous().size, 0);

            let Cursor {
                mut prev,
                hole,
                top,
            } = self;
            unsafe {
                let mut node = check_merge_bottom(node, bottom);
                prev.as_mut().next = Some(node);
                node.as_mut().next = Some(hole);
            }
            Ok(Cursor {
                prev,
                hole: node,
                top,
            })
        } else {
            Err(self)
        }
    }

    fn try_insert_after(&mut self, mut node: NonNull<Hole>) -> Result<(), ()> {
        let node_u8 = node.as_ptr().cast::<u8>();
        let node_size = unsafe { node.as_ref().size };

        if let Some(next) = self.current().next.as_ref() {
            if node < *next {
                let node_u8 = node_u8 as *const u8;
                assert!(
                    node_u8.wrapping_add(node_size) <= next.as_ptr().cast::<u8>(),
                    "Freed node aliases existing hole! Bad free?",
                );
            } else {
                return Err(());
            }
        }

        debug_assert!(self.hole < node, "Hole list out of order?");

        let hole_u8 = self.hole.as_ptr().cast::<u8>();
        let hole_size = self.current().size;

        assert!(
            hole_u8.wrapping_add(hole_size) <= node_u8,
            "Freed node ({:?}) aliases existing hole ({:?}[{}])! Bad free?",
            node_u8,
            hole_u8,
            hole_size,
        );

        unsafe {
            let maybe_next = self.hole.as_mut().next.replace(node);
            node.as_mut().next = maybe_next;
        }

        Ok(())
    }

    fn try_merge_next_n(self, max: usize) {
        let Cursor {
            prev: _,
            mut hole,
            top,
            ..
        } = self;

        for _ in 0..max {
            // Is there a next node?
            let mut next = if let Some(next) = unsafe { hole.as_mut() }.next.as_ref() {
                *next
            } else {
                check_merge_top(hole, top);
                return;
            };

            let hole_u8 = hole.as_ptr().cast::<u8>();
            let hole_sz = unsafe { hole.as_ref().size };
            let next_u8 = next.as_ptr().cast::<u8>();
            let end = hole_u8.wrapping_add(hole_sz);

            let touching = end == next_u8;

            if touching {
                let next_sz;
                let next_next;
                unsafe {
                    let next_mut = next.as_mut();
                    next_sz = next_mut.size;
                    next_next = next_mut.next.take();
                }
                unsafe {
                    let hole_mut = hole.as_mut();
                    hole_mut.next = next_next;
                    hole_mut.size += next_sz;
                }
            } else {
                hole = next;
            }
        }
    }
}



fn deallocate(list: &mut HoleList, addr: *mut u8, size: usize) {
    let hole = unsafe { make_hole(addr, size) };

    let cursor = if let Some(cursor) = list.cursor() {
        cursor
    } else {
        let hole = check_merge_bottom(hole, list.bottom);
        check_merge_top(hole, list.top);
        list.first.next = Some(hole);
        return;
    };

    let (cursor, n) = match cursor.try_insert_back(hole, list.bottom) {
        Ok(cursor) => {
            (cursor, 1)
        }
        Err(mut cursor) => {
            while let Err(()) = cursor.try_insert_after(hole) {
                cursor = cursor
                    .next()
                    .expect("Reached end of holes without finding deallocation hole!");
            }
            (cursor, 2)
        }
    };

    cursor.try_merge_next_n(n);
}


unsafe fn make_hole(addr: *mut u8, size: usize) -> NonNull<Hole> {
    let hole_addr = addr.cast::<Hole>();
    debug_assert_eq!(
        addr as usize % align_of::<Hole>(),
        0,
        "Hole address not aligned!",
    );
    hole_addr.write(Hole { size, next: None });
    NonNull::new_unchecked(hole_addr)
}

fn align_down_size(size: usize, align: usize) -> usize {
    if align.is_power_of_two() {
        size & !(align - 1)
    } else if align == 0 {
        size
    } else {
        panic!("`align` must be a power of 2");
    }
}

fn align_up_size(size: usize, align: usize) -> usize {
    align_down_size(size + align - 1, align)
}

fn align_up(addr: *mut u8, align: usize) -> *mut u8 {
    let offset = addr.align_offset(align);
    addr.wrapping_add(offset)
}


fn check_merge_bottom(node: NonNull<Hole>, bottom: *mut u8) -> NonNull<Hole> {
    debug_assert_eq!(bottom as usize % align_of::<Hole>(), 0);

    if bottom.wrapping_add(core::mem::size_of::<Hole>()) > node.as_ptr().cast::<u8>() {
        let offset = (node.as_ptr() as usize) - (bottom as usize);
        let size = unsafe { node.as_ref() }.size + offset;
        unsafe { make_hole(bottom, size) }
    } else {
        node
    }
}

fn check_merge_top(mut node: NonNull<Hole>, top: *mut u8) {
    let node_u8 = node.as_ptr().cast::<u8>();
    let node_sz = unsafe { node.as_ref().size };

    // If this is the last node, we need to see if we need to merge to the end
    let end = node_u8.wrapping_add(node_sz);
    let hole_layout = Layout::new::<Hole>();
    if end < top {
        let next_hole_end = align_up(end, hole_layout.align()).wrapping_add(hole_layout.size());

        if next_hole_end > top {
            let offset = (top as usize) - (end as usize);
            unsafe {
                node.as_mut().size += offset;
            }
        }
    }
}

pub(crate) struct Heap {
    used: usize,
    holes: HoleList,
}

impl Heap {
    pub(crate) fn debug(&mut self) {
        println!(
            "bottom: {:?}, top: {:?}, size: {}, pending: {}",
            self.bottom(),
            self.top(),
            self.size(),
            self.holes.first.size,
        );
        self.holes.debug();
    }
}

unsafe impl Send for Heap {}

impl Heap {
    pub(crate) const fn empty() -> Heap {
        Heap {
            used: 0,
            holes: HoleList::empty(),
        }
    }

    pub(crate) unsafe fn init(&mut self, heap_bottom: *mut u8, heap_size: usize) {
        self.used = 0;
        self.holes = HoleList::new(heap_bottom, heap_size);
    }

    pub(crate) fn init_from_slice(&mut self, mem: &'static mut [MaybeUninit<u8>]) {
        assert!(
            self.bottom().is_null(),
            "The heap has already been initialized."
        );
        let size = mem.len();
        let address = mem.as_mut_ptr().cast();
        unsafe { self.init(address, size) }
    }

    pub(crate) unsafe fn new(heap_bottom: *mut u8, heap_size: usize) -> Heap {
        Heap {
            used: 0,
            holes: HoleList::new(heap_bottom, heap_size),
        }
    }

    pub(crate) fn from_slice(mem: &'static mut [MaybeUninit<u8>]) -> Heap {
        let size = mem.len();
        let address = mem.as_mut_ptr().cast();
        unsafe { Self::new(address, size) }
    }

    #[allow(clippy::result_unit_err)]
    pub(crate) fn allocate_first_fit(&mut self, layout: Layout) -> Result<NonNull<u8>, ()> {
        match self.holes.allocate_first_fit(layout) {
            Ok((ptr, aligned_layout)) => {
                self.used += aligned_layout.size();
                Ok(ptr)
            }
            Err(err) => Err(err),
        }
    }

    pub(crate) unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        self.used -= self.holes.deallocate(ptr, layout).size();
    }

    pub(crate) fn bottom(&self) -> *mut u8 {
        self.holes.bottom
    }

    pub(crate) fn size(&self) -> usize {
        unsafe { self.holes.top.offset_from(self.holes.bottom) as usize }
    }

    pub(crate) fn top(&self) -> *mut u8 {
        unsafe { self.holes.top.add(self.holes.pending_extend as usize) }
    }

    pub(crate) fn used(&self) -> usize {
        self.used
    }

    pub(crate) fn free(&self) -> usize {
        self.size() - self.used
    }

    pub(crate) unsafe fn extend(&mut self, by: usize) {
        self.holes.extend(by);
    }
}

pub(crate) struct LockedHeap(Mutex<Heap>);

impl LockedHeap {
    pub(crate) const fn empty() -> LockedHeap {
        LockedHeap(Mutex::new(Heap::empty()))
    }

    pub(crate) unsafe fn new(heap_bottom: *mut u8, heap_size: usize) -> LockedHeap {
        LockedHeap(Mutex::new(Heap {
            used: 0,
            holes: HoleList::new(heap_bottom, heap_size),
        }))
    }
}

impl Deref for LockedHeap {
    type Target = Mutex<Heap>;

    fn deref(&self) -> &Mutex<Heap> {
        &self.0
    }
}

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0
            .lock()
            .allocate_first_fit(layout)
            .ok()
            .map_or(core::ptr::null_mut(), |allocation| allocation.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0
            .lock()
            .deallocate(NonNull::new_unchecked(ptr), layout)
    }
}
