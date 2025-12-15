use crate::{
    key_vec::Val,
    syntax::{ROOT_SYN, Syn, SynData, Syns},
};

pub fn debug(syns: &Syns) {
    struct DebugSyn<'a> {
        syns: &'a Syns,
        syn: Syn,
    }

    impl std::fmt::Debug for DebugSyn<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let dbg_syn = |syn| DebugSyn {
                syns: self.syns,
                syn,
            };

            match self.syns.get(self.syn) {
                Val::None => panic!(),
                Val::Value(syn_data) => match syn_data {
                    SynData::Root(syns) => syns
                        .iter()
                        .fold(&mut f.debug_tuple("root"), |tuple, expr| {
                            tuple.field(&dbg_syn(*expr))
                        })
                        .finish(),
                    SynData::Ident { .. } => f.debug_tuple("ident").finish(),
                    SynData::False { .. } => f.debug_tuple("false").finish(),
                    SynData::True { .. } => f.debug_tuple("true").finish(),
                    SynData::Number { .. } => f.debug_tuple("number").finish(),
                    SynData::Add(lhs, rhs) => f
                        .debug_tuple("add")
                        .field(&dbg_syn(*lhs))
                        .field(&dbg_syn(*rhs))
                        .finish(),
                    SynData::Subtract(lhs, rhs) => f
                        .debug_tuple("subtract")
                        .field(&dbg_syn(*lhs))
                        .field(&dbg_syn(*rhs))
                        .finish(),
                    SynData::Binding { pattern, value } => f
                        .debug_tuple("binding")
                        .field(&dbg_syn(*pattern))
                        .field(&dbg_syn(*value))
                        .finish(),
                    SynData::Function { pattern, body } => f
                        .debug_tuple("function")
                        .field(&dbg_syn(*pattern))
                        .field(&dbg_syn(*body))
                        .finish(),
                    SynData::ReturnAscription {
                        syn: pattern,
                        type_,
                    } => f
                        .debug_tuple("return_ascription")
                        .field(&dbg_syn(*pattern))
                        .field(&dbg_syn(*type_))
                        .finish(),
                    SynData::Ascription { syn, type_ } => f
                        .debug_tuple("ascription")
                        .field(&dbg_syn(*syn))
                        .field(&dbg_syn(*type_))
                        .finish(),
                    SynData::Access { syn, key } => f
                        .debug_tuple("access")
                        .field(&dbg_syn(*syn))
                        .field(&dbg_syn(*key))
                        .finish(),
                    SynData::EmptyParen { .. } => f.debug_tuple("empty_paren").finish(),
                    SynData::Paren(expr) => f.debug_tuple("paren").field(&dbg_syn(*expr)).finish(),
                    SynData::EmptyCurly { .. } => f.debug_tuple("empty_curly").finish(),
                    SynData::Curly(expr) => f.debug_tuple("curly").field(&dbg_syn(*expr)).finish(),
                    SynData::Tuple(syns) => syns
                        .iter()
                        .fold(&mut f.debug_tuple("tuple"), |tuple, expr| {
                            tuple.field(&dbg_syn(*expr))
                        })
                        .finish(),
                    SynData::Application { function, argument } => f
                        .debug_tuple("application")
                        .field(&dbg_syn(*function))
                        .field(&dbg_syn(*argument))
                        .finish(),
                    SynData::Chain(syns) => syns
                        .iter()
                        .fold(&mut f.debug_tuple("tuple"), |tuple, expr| {
                            tuple.field(&dbg_syn(*expr))
                        })
                        .finish(),
                    SynData::ChainClosed(syns) => syns
                        .iter()
                        .fold(&mut f.debug_tuple("tuple"), |tuple, expr| {
                            tuple.field(&dbg_syn(*expr))
                        })
                        .finish(),
                },
            }
        }
    }

    println!(
        "{:#?}",
        DebugSyn {
            syns,
            syn: ROOT_SYN
        }
    );
}
