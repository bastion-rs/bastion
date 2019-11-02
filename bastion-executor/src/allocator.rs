// Allocator generator macro
use allocator_suite::switchable_allocator;

// General imports
use allocator_suite::adaptors::prelude::*;
use std::alloc::System;

switchable_allocator!(
    application_allocator,
    BumpAllocator<ArenaMemorySource<MemoryMapSource>>,
    MultipleBinarySearchTreeAllocator<MemoryMapSource>,
    GlobalAllocToAllocatorAdaptor<System>,
    GlobalAllocToAllocatorAdaptor(System)
);
