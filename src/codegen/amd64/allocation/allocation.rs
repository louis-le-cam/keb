use std::borrow::Cow;

use crate::{
    key_vec::KeyVec,
    ssa::{BlockSentinel, Inst, InstSentinel},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Allocation {
    Unit,
    Stack { offset: u64, size: u64 },
    StackArgument { offset: u64, size: u64 },
    Eax,
    Ebx,
    Ecx,
    Edx,
    Esi,
    Edi,
    R8d,
    Immediate(u32),
}

pub struct InstAllocations {
    pub reallocations: Vec<(Allocation, Allocation)>,
    pub allocation: Allocation,
    pub deallocations: Vec<Inst>,
}

pub struct Allocations {
    /// The allocation of the argument of a function from the function's
    /// perspective (i.e. stack allocation are relative to the callee stack).
    ///
    /// Block arguments are also specified here.
    pub arguments: KeyVec<BlockSentinel, Allocation>,
    /// The allocation of the return value of a function from the function's
    /// perspective (i.e. stack allocation are relative to the callee stack).
    ///
    /// Block don't have any return value so their return allocation should be
    /// [`Allocation::Unit`].
    pub returns: KeyVec<BlockSentinel, Allocation>,
    /// The allocation of the result of an instruction.
    ///
    /// If an instruction doesn't result in any value, the allocation should be
    /// [`Allocation::Unit`].
    pub instructions: KeyVec<InstSentinel, InstAllocations>,
}

impl Allocation {
    pub fn asm(&self) -> Option<Cow<'static, str>> {
        match self {
            Allocation::Unit => None,
            Allocation::Stack { offset, .. } => Some(Cow::Owned(format!("-{}(%rbp)", offset))),
            Allocation::StackArgument { offset, size: _ } => {
                Some(Cow::Owned(format!("{}(%rbp)", offset + 16)))
            }
            Allocation::Eax => Some(Cow::Borrowed("%eax")),
            Allocation::Ebx => Some(Cow::Borrowed("%ebx")),
            Allocation::Ecx => Some(Cow::Borrowed("%ecx")),
            Allocation::Edx => Some(Cow::Borrowed("%edx")),
            Allocation::Esi => Some(Cow::Borrowed("%esi")),
            Allocation::Edi => Some(Cow::Borrowed("%edi")),
            Allocation::R8d => Some(Cow::Borrowed("%r8d")),
            Allocation::Immediate(value) => Some(Cow::Owned(format!("${value}"))),
        }
    }

    pub fn is_memory(&self) -> bool {
        matches!(
            self,
            Allocation::Stack { .. } | Allocation::StackArgument { .. },
        )
    }

    pub fn move_to(&self, destination: &Allocation) -> String {
        if self == destination {
            return String::new();
        }

        match destination.size() {
            0 => String::new(),
            4 if self.is_memory() && destination.is_memory() => format!(
                "  movl {}, {}\n  movl {}, {}\n",
                self.asm().unwrap(),
                // FIXME: Took a random register to move memory to memory, not
                // the greatest idea.
                Allocation::R8d.asm().unwrap(),
                Allocation::R8d.asm().unwrap(),
                destination.asm().unwrap(),
            ),
            4 => format!(
                "  movl {}, {}\n",
                self.asm().unwrap(),
                destination.asm().unwrap()
            ),
            8 => format!(
                "  movl {}, {}\n  movl {}, {}\n  movl {}, {}\n  movl {}, {}\n",
                self.offset(0, 4).asm().unwrap(),
                // FIXME: Took a random register to move memory to memory, not
                // the greatest idea.
                Allocation::R8d.asm().unwrap(),
                Allocation::R8d.asm().unwrap(),
                destination.offset(0, 4).asm().unwrap(),
                self.offset(4, 4).asm().unwrap(),
                Allocation::R8d.asm().unwrap(),
                Allocation::R8d.asm().unwrap(),
                destination.offset(4, 4).asm().unwrap(),
            ),
            _ => todo!(),
        }
    }

    pub fn size(&self) -> u64 {
        match self {
            Allocation::Unit => 0,
            Allocation::Stack { size, .. } | Allocation::StackArgument { size, .. } => *size,
            Allocation::Eax
            | Allocation::Ebx
            | Allocation::Ecx
            | Allocation::Edx
            | Allocation::Esi
            | Allocation::Edi
            | Allocation::R8d
            | Allocation::Immediate(_) => 4,
        }
    }

    pub fn offset(&self, offset: u64, size: u64) -> Allocation {
        match self {
            Allocation::Stack {
                offset: base_offset,
                ..
            } => Allocation::Stack {
                offset: base_offset - offset,
                size,
            },
            Allocation::StackArgument {
                offset: base_offset,
                ..
            } => Allocation::StackArgument {
                offset: base_offset + offset,
                size,
            },
            _ => panic!(),
        }
    }
}
