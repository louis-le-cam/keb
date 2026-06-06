mod allocation;
mod allocator;

pub use self::{
    allocation::{Allocation, Allocations, InstAllocations},
    allocator::allocate,
};
