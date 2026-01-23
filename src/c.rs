use std::collections::HashSet;

use crate::{
    key_vec::Val,
    semantic::{Type, TypeData, TypeSentinel, Types, types_equals},
    ssa::{Block, BlockData, ConstData, ConstSentinel, Expr, Inst, InstData, Ssa},
};

pub fn generate(types: &Types, ssa: &Ssa) -> String {
    let mut generator = Generator {
        types,
        ssa,
        functions: String::new(),
        structs: Vec::new(),
    };

    generator.generate();

    generator.result()
}

struct Generator<'a> {
    types: &'a Types,
    ssa: &'a Ssa,
    functions: String,
    structs: Vec<(Type, String)>,
}

impl Generator<'_> {
    fn result(self) -> String {
        format!(
            "#include<stdio.h>\n\nvoid builtin_print(unsigned int x) {{ printf(\"%u\\n\", x); }}\n\n{}\n\n{}int main() {{ f{}_main(); return 0; }}\n",
            self.structs
                .into_iter()
                .map(|(_, definition)| definition)
                .collect::<Vec<String>>()
                .join("\n\n"),
            self.functions,
            self.ssa
                .blocks
                .entries()
                .find(|(_, block_data)| match block_data {
                    BlockData::Function { name, .. } if name == "main" => true,
                    _ => false,
                })
                .unwrap()
                .0
                .as_u32()
        )
    }

    fn generate(&mut self) {
        for (block, block_data) in self.ssa.blocks.entries() {
            match block_data {
                BlockData::ExternFunction { .. } => {
                    let function = self.generate_extern_function(block);
                    self.functions.push_str(&function);
                    self.functions.push_str("\n\n");
                }
                BlockData::Function { .. } => {
                    let function = self.generate_function(block);
                    self.functions.push_str(&function);
                    self.functions.push_str("\n\n");
                }
                BlockData::Block { .. } => {}
            }
        }
    }

    fn generate_extern_function(&mut self, function: Block) -> String {
        let BlockData::ExternFunction { name, arg, ret } = &self.ssa.blocks[function] else {
            panic!()
        };

        let return_type = self.generate_type(*ret);
        let argument_type = self.generate_type(*arg);

        let head = if argument_type == "void" {
            format!("{} f{}_{name}()", return_type, function.as_u32())
        } else {
            format!(
                "{} f{}_{name}({} a{})",
                return_type,
                function.as_u32(),
                argument_type,
                function.as_u32(),
            )
        };

        let body = if return_type == "void" {
            format!("    {name}(a{});\n", function.as_u32())
        } else {
            format!("    return {name}(a{});\n", function.as_u32())
        };

        format!("{head} {{\n{body}}}")
    }

    fn generate_function(&mut self, function: Block) -> String {
        let BlockData::Function {
            name,
            arg,
            ret,
            insts,
        } = &self.ssa.blocks[function]
        else {
            panic!()
        };

        let return_type = self.generate_type(*ret);
        let argument_type = self.generate_type(*arg);

        let head = if argument_type == "void" {
            format!("{} f{}_{name}()", return_type, function.as_u32())
        } else {
            format!(
                "{} f{}_{name}({} a{})",
                return_type,
                function.as_u32(),
                argument_type,
                function.as_u32(),
            )
        };

        let mut body = String::new();

        let blocks = self.function_blocks(function);

        for block in &blocks {
            let BlockData::Block { arg, insts: _ } = &self.ssa.blocks[*block] else {
                panic!()
            };

            match arg.sentinel() {
                Some(TypeSentinel::Unit) => {}
                _ => {
                    body.push_str(&format!(
                        "    {} a{};\n",
                        self.generate_type(*arg),
                        block.as_u32(),
                    ));
                }
            }
        }

        body.push_str(&self.generate_statements(insts.iter().copied()));

        for block in &blocks {
            let BlockData::Block { insts, .. } = &self.ssa.blocks[*block] else {
                panic!()
            };

            body.push_str(&format!("b{}:\n", block.as_u32()));
            body.push_str(&self.generate_statements(insts.iter().copied()));
        }

        format!("{head} {{\n{body}}}")
    }

    fn function_blocks(&mut self, function: Block) -> HashSet<Block> {
        let BlockData::Function { insts, .. } = &self.ssa.blocks[function] else {
            panic!()
        };

        let mut blocks = HashSet::new();

        for inst in insts {
            match &self.ssa.insts[*inst] {
                InstData::Jump { block, .. } => {
                    blocks.insert(*block);
                }
                InstData::JumpCondition { then, else_, .. } => {
                    blocks.insert(*then);
                    blocks.insert(*else_);
                }
                _ => {}
            }
        }

        let mut checked_blocks = HashSet::new();

        while let Some(&block) = blocks.difference(&checked_blocks).next() {
            checked_blocks.insert(block);

            let BlockData::Block { insts, .. } = &self.ssa.blocks[block] else {
                panic!();
            };

            for inst in insts {
                match &self.ssa.insts[*inst] {
                    InstData::Jump { block, .. } => {
                        blocks.insert(*block);
                    }
                    InstData::JumpCondition { then, else_, .. } => {
                        blocks.insert(*then);
                        blocks.insert(*else_);
                    }
                    _ => {}
                }
            }
        }

        blocks
    }

    fn generate_statements(&mut self, insts: impl IntoIterator<Item = Inst>) -> String {
        let mut body = String::new();

        for inst in insts {
            body.push_str("    ");

            let type_ = self.ssa.instruction_type(self.types, inst);
            let c_type = self.generate_type(type_);
            if c_type != "void" {
                body.push_str(&format!("{c_type} i{} = ", inst.as_u32()));
            }

            match &self.ssa.insts[inst] {
                InstData::Field(expr, field) => {
                    body.push_str(&format!("{}.f{field}", self.generate_expr(*expr)))
                }
                InstData::Record(fields, _) => body.push_str(&format!(
                    "{{ {} }}",
                    fields
                        .iter()
                        .map(|field| format!("{}, ", self.generate_expr(*field)))
                        .collect::<String>()
                )),
                InstData::Add(lhs, rhs) => body.push_str(&format!(
                    "{} + {}",
                    self.generate_expr(*lhs),
                    self.generate_expr(*rhs),
                )),
                InstData::Sub(lhs, rhs) => body.push_str(&format!(
                    "{} - {}",
                    self.generate_expr(*lhs),
                    self.generate_expr(*rhs),
                )),
                InstData::Mul(lhs, rhs) => body.push_str(&format!(
                    "{} * {}",
                    self.generate_expr(*lhs),
                    self.generate_expr(*rhs),
                )),
                InstData::Div(lhs, rhs) => body.push_str(&format!(
                    "{} / {}",
                    self.generate_expr(*lhs),
                    self.generate_expr(*rhs),
                )),
                InstData::Call { function, argument } => match &self.ssa.blocks[*function] {
                    BlockData::ExternFunction { name, .. } | BlockData::Function { name, .. } => {
                        let argument_text = match argument {
                            Expr::Const(const_)
                                if const_.sentinel() == Some(ConstSentinel::Unit) =>
                            {
                                ""
                            }
                            _ => &self.generate_expr(*argument),
                        };

                        body.push_str(&format!("f{}_{name}({argument_text})", function.as_u32(),));
                    }
                    BlockData::Block { .. } => panic!(),
                },
                InstData::Jump { block, argument } => {
                    body.push_str(&format!(
                        "a{} = {};\n    goto b{}",
                        block.as_u32(),
                        self.generate_expr(*argument),
                        block.as_u32(),
                    ));
                }
                InstData::JumpCondition {
                    condition,
                    then,
                    else_,
                } => body.push_str(&format!(
                    "if ({}) {{ goto b{}; }} else {{ goto b{}; }}",
                    self.generate_expr(*condition),
                    then.as_u32(),
                    else_.as_u32(),
                )),
                InstData::Return(expr) => {
                    if let Some(TypeSentinel::Unit) =
                        self.ssa.expression_type(self.types, *expr).sentinel()
                    {
                        body.push_str("return");
                    } else {
                        body.push_str(&format!("return {}", self.generate_expr(*expr)))
                    }
                }
            }

            body.push_str(";\n");
        }

        body
    }

    fn generate_type(&mut self, type_: Type) -> String {
        let c_type = match self.types.get(type_) {
            Val::None => panic!(),
            Val::Sentinel(sentinel) => match sentinel {
                TypeSentinel::Unknown => panic!(),
                TypeSentinel::Unit => "void".to_string(),
                TypeSentinel::Uint32 => "unsigned int".to_string(),
                TypeSentinel::Bool | TypeSentinel::False | TypeSentinel::True => "bool".to_string(),
            },
            Val::Value(type_data) => match type_data {
                TypeData::Function { .. } => todo!(),
                TypeData::Product { fields } if fields.len() == 0 => "void".to_string(),
                TypeData::Product { fields } => {
                    if let Some((ty, _)) = self
                        .structs
                        .iter()
                        .find(|(ty, _)| types_equals(self.types, type_, *ty))
                    {
                        return format!("struct t{}", ty.as_u32());
                    }

                    let value = format!(
                        "struct t{} {{ {} }};",
                        type_.as_u32(),
                        fields
                            .iter()
                            .enumerate()
                            .map(|(i, (_, type_))| format!("{} f{i};", self.generate_type(*type_)))
                            .collect::<String>()
                    );

                    self.structs.push((type_, value));

                    return format!("struct t{}", type_.as_u32());
                }
            },
        };

        c_type
    }

    fn generate_expr(&mut self, expr: Expr) -> String {
        match expr {
            Expr::Const(const_) => match self.ssa.consts.get(const_) {
                Val::None => panic!(),
                Val::Sentinel(sentinel) => match sentinel {
                    ConstSentinel::Unit => panic!(),
                    ConstSentinel::False => "false".to_string(),
                    ConstSentinel::True => "true".to_string(),
                },
                Val::Value(value) => match value {
                    ConstData::Uint32(value) => value.to_string(),
                    ConstData::Product(_, _) => todo!(),
                },
            },
            Expr::Inst(inst) => format!("i{}", inst.as_u32()),
            Expr::BlockArg(block) => format!("a{}", block.as_u32()),
        }
    }
}
