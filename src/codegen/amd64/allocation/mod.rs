mod allocation;
mod allocator;
mod debug;

pub use self::{
    allocation::{Allocation, Allocations, InstAllocations},
    allocator::allocate,
    debug::debug,
};
