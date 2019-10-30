use context_allocator::adaptors::*;
use context_allocator::allocators::global::*;
use context_allocator::allocators::*;
use context_allocator::memory_sources::arena_memory_source::ArenaMemorySource;
use context_allocator::memory_sources::mmap::MemoryMapSource;
use context_allocator::MemoryAddress;
use std::alloc::{Alloc, AllocErr, CannotReallocInPlace, Excess, GlobalAlloc, Layout, System};
use std::mem::replace;
use std::num::NonZeroUsize;

/// //////////////////////////////////////////////////
/// /////////// GLOBAL ALLOCATOR
/// //////////////////////////////////////////////////
#[global_allocator]
static GLOBAL: GlobalThreadAndCoroutineSwitchableAllocatorInstance =
    GlobalThreadAndCoroutineSwitchableAllocatorInstance {
        global_allocator: GlobalAllocToAllocatorAdaptor(System),
    };

/// Effectively this is a field of `GlobalThreadAndCoroutineSwitchableAllocatorInstance` with a different value for each thread.
///
/// It is this piece of logic that necessitates this macro definition.
#[thread_local]
static mut PER_THREAD_STATE: PerThreadState<
    BumpAllocator<ArenaMemorySource<MemoryMapSource>>,
    MultipleBinarySearchTreeAllocator<MemoryMapSource>,
> = PerThreadState::empty();

pub(crate) struct GlobalThreadAndCoroutineSwitchableAllocatorInstance {
    pub(crate) global_allocator: GlobalAllocToAllocatorAdaptor<System>,
}

#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for GlobalThreadAndCoroutineSwitchableAllocatorInstance {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            GlobalThreadAndCoroutineSwitchableAllocatorInstance {
                global_allocator: ref __self_0_0,
            } => {
                let mut debug_trait_builder =
                    f.debug_struct("GlobalThreadAndCoroutineSwitchableAllocatorInstance");
                let _ = debug_trait_builder.field("global_allocator", &&(*__self_0_0));
                debug_trait_builder.finish()
            }
        }
    }
}

unsafe impl Sync for GlobalThreadAndCoroutineSwitchableAllocatorInstance {}

unsafe impl GlobalAlloc for GlobalThreadAndCoroutineSwitchableAllocatorInstance {
    #[inline(always)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.GlobalAlloc_alloc(layout)
    }
    #[inline(always)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.GlobalAlloc_dealloc(ptr, layout)
    }
    #[inline(always)]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.GlobalAlloc_alloc_zeroed(layout)
    }
    #[inline(always)]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        self.GlobalAlloc_realloc(ptr, layout, new_size)
    }
}

unsafe impl Alloc for GlobalThreadAndCoroutineSwitchableAllocatorInstance {
    #[inline(always)]
    unsafe fn alloc(&mut self, layout: Layout) -> Result<MemoryAddress, AllocErr> {
        self.Alloc_alloc(layout)
    }
    #[inline(always)]
    unsafe fn dealloc(&mut self, ptr: MemoryAddress, layout: Layout) {
        self.Alloc_dealloc(ptr, layout)
    }
    #[inline(always)]
    unsafe fn realloc(
        &mut self,
        ptr: MemoryAddress,
        layout: Layout,
        new_size: usize,
    ) -> Result<MemoryAddress, AllocErr> {
        self.Alloc_realloc(ptr, layout, new_size)
    }
    #[inline(always)]
    unsafe fn alloc_zeroed(&mut self, layout: Layout) -> Result<MemoryAddress, AllocErr> {
        self.Alloc_alloc_zeroed(layout)
    }
    #[inline(always)]
    unsafe fn alloc_excess(&mut self, layout: Layout) -> Result<Excess, AllocErr> {
        self.Alloc_alloc_excess(layout)
    }
    #[inline(always)]
    unsafe fn realloc_excess(
        &mut self,
        ptr: MemoryAddress,
        layout: Layout,
        new_size: usize,
    ) -> Result<Excess, AllocErr> {
        self.Alloc_realloc_excess(ptr, layout, new_size)
    }
    #[inline(always)]
    unsafe fn grow_in_place(
        &mut self,
        ptr: MemoryAddress,
        layout: Layout,
        new_size: usize,
    ) -> Result<(), CannotReallocInPlace> {
        self.Alloc_grow_in_place(ptr, layout, new_size)
    }
    #[inline(always)]
    unsafe fn shrink_in_place(
        &mut self,
        ptr: MemoryAddress,
        layout: Layout,
        new_size: usize,
    ) -> Result<(), CannotReallocInPlace> {
        self.Alloc_shrink_in_place(ptr, layout, new_size)
    }
}

impl Allocator for GlobalThreadAndCoroutineSwitchableAllocatorInstance {
    #[inline(always)]
    fn allocate(
        &self,
        non_zero_size: NonZeroUsize,
        non_zero_power_of_two_alignment: NonZeroUsize,
    ) -> Result<MemoryAddress, AllocErr> {
        use self::CurrentAllocatorInUse::*;
        match self.save_current_allocator_in_use() {
            CoroutineLocal => self
                .coroutine_local_allocator()
                .expect("Should have assigned a coroutine local allocator")
                .allocate(non_zero_size, non_zero_power_of_two_alignment),
            ThreadLocal => self
                .thread_local_allocator()
                .expect("Should have assigned a thread local allocator")
                .allocate(non_zero_size, non_zero_power_of_two_alignment),
            Global => self
                .global_allocator()
                .allocate(non_zero_size, non_zero_power_of_two_alignment),
        }
    }
    #[inline(always)]
    fn deallocate(
        &self,
        non_zero_size: NonZeroUsize,
        non_zero_power_of_two_alignment: NonZeroUsize,
        current_memory: MemoryAddress,
    ) {
        {
            if let Some(coroutine_local_allocator) = self.coroutine_local_allocator() {
                if unsafe {
                    ::std::intrinsics::likely(coroutine_local_allocator.contains(current_memory))
                } {
                    return coroutine_local_allocator.deallocate(
                        non_zero_size,
                        non_zero_power_of_two_alignment,
                        current_memory,
                    );
                }
            }
            if let Some(thread_local_allocator) = self.thread_local_allocator() {
                if unsafe {
                    ::std::intrinsics::likely(thread_local_allocator.contains(current_memory))
                } {
                    return thread_local_allocator.deallocate(
                        non_zero_size,
                        non_zero_power_of_two_alignment,
                        current_memory,
                    );
                }
            }
            self.global_allocator().deallocate(
                non_zero_size,
                non_zero_power_of_two_alignment,
                current_memory,
            )
        }
    }
    #[inline(always)]
    fn growing_reallocate(
        &self,
        non_zero_new_size: NonZeroUsize,
        non_zero_power_of_two_alignment: NonZeroUsize,
        non_zero_current_size: NonZeroUsize,
        current_memory: MemoryAddress,
    ) -> Result<MemoryAddress, AllocErr> {
        {
            if let Some(coroutine_local_allocator) = self.coroutine_local_allocator() {
                if unsafe {
                    ::std::intrinsics::likely(coroutine_local_allocator.contains(current_memory))
                } {
                    return coroutine_local_allocator.growing_reallocate(
                        non_zero_new_size,
                        non_zero_power_of_two_alignment,
                        non_zero_current_size,
                        current_memory,
                    );
                }
            }
            if let Some(thread_local_allocator) = self.thread_local_allocator() {
                if unsafe {
                    ::std::intrinsics::likely(thread_local_allocator.contains(current_memory))
                } {
                    return thread_local_allocator.growing_reallocate(
                        non_zero_new_size,
                        non_zero_power_of_two_alignment,
                        non_zero_current_size,
                        current_memory,
                    );
                }
            }
            self.global_allocator().growing_reallocate(
                non_zero_new_size,
                non_zero_power_of_two_alignment,
                non_zero_current_size,
                current_memory,
            )
        }
    }
    #[inline(always)]
    fn shrinking_reallocate(
        &self,
        non_zero_new_size: NonZeroUsize,
        non_zero_power_of_two_alignment: NonZeroUsize,
        non_zero_current_size: NonZeroUsize,
        current_memory: MemoryAddress,
    ) -> Result<MemoryAddress, AllocErr> {
        {
            if let Some(coroutine_local_allocator) = self.coroutine_local_allocator() {
                if unsafe {
                    ::std::intrinsics::likely(coroutine_local_allocator.contains(current_memory))
                } {
                    return coroutine_local_allocator.growing_reallocate(
                        non_zero_new_size,
                        non_zero_power_of_two_alignment,
                        non_zero_current_size,
                        current_memory,
                    );
                }
            }
            if let Some(thread_local_allocator) = self.thread_local_allocator() {
                if unsafe {
                    ::std::intrinsics::likely(thread_local_allocator.contains(current_memory))
                } {
                    return thread_local_allocator.growing_reallocate(
                        non_zero_new_size,
                        non_zero_power_of_two_alignment,
                        non_zero_current_size,
                        current_memory,
                    );
                }
            }
            self.global_allocator().growing_reallocate(
                non_zero_new_size,
                non_zero_power_of_two_alignment,
                non_zero_current_size,
                current_memory,
            )
        }
    }
}

impl GlobalThreadAndCoroutineSwitchableAllocator
    for GlobalThreadAndCoroutineSwitchableAllocatorInstance
{
    type CoroutineLocalAllocator = BumpAllocator<ArenaMemorySource<MemoryMapSource>>;
    type ThreadLocalAllocator = MultipleBinarySearchTreeAllocator<MemoryMapSource>;
    type GlobalAllocator = GlobalAllocToAllocatorAdaptor<System>;
    #[inline(always)]
    fn replace_coroutine_local_allocator(
        &self,
        replacement: Option<Self::CoroutineLocalAllocator>,
    ) -> Option<Self::CoroutineLocalAllocator> {
        unsafe { replace(&mut PER_THREAD_STATE.coroutine_local_allocator, replacement) }
    }
    #[inline(always)]
    fn initialize_thread_local_allocator(
        &self,
        thread_local_allocator: Self::ThreadLocalAllocator,
    ) {
        if true {
            if !unsafe { PER_THREAD_STATE.thread_local_allocator.is_none() } {
                {
                    ::std::rt::begin_panic(
                        "Already initialized thread local allocator",
                        &("allocator.rs", 146u32, 1u32),
                    )
                }
            };
        };
        unsafe { PER_THREAD_STATE.thread_local_allocator = Some(thread_local_allocator) }
    }
    #[inline(always)]
    fn drop_thread_local_allocator(&self) {
        if true {
            if !unsafe { PER_THREAD_STATE.thread_local_allocator.is_some() } {
                {
                    ::std::rt::begin_panic(
                        "Already deinitialized thread local allocator",
                        &("allocator.rs", 146u32, 1u32),
                    )
                }
            };
        };
        unsafe { PER_THREAD_STATE.thread_local_allocator = None }
    }
    #[inline(always)]
    fn save_current_allocator_in_use(&self) -> CurrentAllocatorInUse {
        unsafe { PER_THREAD_STATE.current_allocator_in_use }
    }
    #[inline(always)]
    fn restore_current_allocator_in_use(&self, restore_to: CurrentAllocatorInUse) {
        unsafe { PER_THREAD_STATE.current_allocator_in_use = restore_to }
    }
    #[inline(always)]
    fn coroutine_local_allocator(&self) -> Option<&Self::CoroutineLocalAllocator> {
        unsafe { PER_THREAD_STATE.coroutine_local_allocator.as_ref() }
    }
    #[inline(always)]
    fn thread_local_allocator(&self) -> Option<&Self::ThreadLocalAllocator> {
        unsafe { PER_THREAD_STATE.thread_local_allocator.as_ref() }
    }
    #[inline(always)]
    fn global_allocator(&self) -> &Self::GlobalAllocator {
        &self.global_allocator
    }
}
