use crate::key_vec::{Index, KeyVec, Sentinels, SharedIndex};

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SynKind {
    // lhs: syn | none -- statement, lhs is only none if the tree is empty
    // rhs: syn.root | none -- next
    Root,
    // lhs: token.ident
    Ident,
    // lhs: token.false
    False,
    // lhs: token.true
    True,
    // lhs: token.number
    Number,
    // lhs: syn
    // rhs: syn
    Equal,
    // lhs: syn
    // rhs: syn
    Add,
    // lhs: syn
    // rhs: syn
    Subtract,
    // lhs: syn
    // rhs: syn
    Multiply,
    // lhs: syn
    // rhs: syn
    Divide,
    // lhs: syn -- pattern
    // rhs: syn -- value
    Binding,
    // lhs: token.mut
    Mut,
    // lhs: syn -- pattern
    // rhs: syn -- value
    Assignment,
    // lhs: syn -- pattern
    // rhs: syn -- body
    Function,
    // lhs: syn
    // rhs: syn -- return type
    ReturnAscription,
    // lhs: syn
    // rhs: syn -- type
    Ascription,
    // lhs: syn
    // rhs: syn -- key
    Access,
    // lhs: token.left_paren
    // rhs: syn | syn.none
    Paren,
    // lhs: token.left_curly
    // lhs: syn | syn.none
    Curly,
    // lhs: syn -- field
    // rhs: syn | syn.tuple | none -- next, the tuple is closed if the last rhs is none.
    Tuple,
    // lhs: syn -- callee
    // rhs: syn -- argument
    Application,
    // lhs: syn
    Loop,
    // lhs: syn
    Match,
    // lhs: syn -- condition
    // rhs: syn | syn.else -- then block or syn.else which contains the then block and the else block
    If,
    // lhs: syn -- then block
    // rhs: syn -- else block
    Else,
    // lhs: syn -- field
    // rhs: syn | syn.chain | none -- next, the chain is closed if the last rhs is none.
    Chain,
    // lhs: token.string_start
    // rhs: syn.string_interpolation | syn.string_segment | none -- next
    String,
    // lhs: token.string_segment
    // rhs: syn.string_interpolation | none -- next
    StringSegment,
    // lhs: syn
    // rhs: syn.string_segment | none -- next
    StringInterpolation,
}

#[derive(Sentinels, Clone, Copy, Debug)]
#[repr(u32)]
pub enum SynSentinel {
    None = u32::MAX,
}

#[derive(Default)]
pub struct Syntax {
    pub kinds: KeyVec<SynSentinel, SynKind>,
    pub lhs: KeyVec<SynSentinel, SharedIndex>,
    pub rhs: KeyVec<SynSentinel, SharedIndex>,
}

impl Syntax {
    pub fn push(
        &mut self,
        kind: SynKind,
        lhs: impl Into<SharedIndex>,
        rhs: impl Into<SharedIndex>,
    ) -> Syn {
        let syn = self.kinds.push(kind);
        let lhs_syn = self.lhs.push(lhs.into());
        let rhs_syn = self.rhs.push(rhs.into());

        debug_assert_eq!(syn, lhs_syn);
        debug_assert_eq!(syn, rhs_syn);

        syn
    }
}

pub type Syn = Index<SynSentinel>;

pub const ROOT_SYN: Syn = Syn::from_u32_index(0);
