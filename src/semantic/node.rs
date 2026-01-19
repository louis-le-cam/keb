use crate::{
    key_vec::{Index, KeyVec, Sentinel},
    semantic::Type,
    token::Token,
};

#[derive(Clone, Debug)]
pub struct NodeData {
    pub kind: NodeKind,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub enum NodeKind {
    False(Token),
    True(Token),
    Number(Token),
    Module {
        bindings: Vec<(String, Node)>,
    },
    Function {
        argument: String,
        body: Node,
    },
    Binding {
        name: String,
        value: Node,
        body: Node,
    },
    Reference {
        name: String,
    },
    Access {
        field: String,
        expr: Node,
    },
    Application {
        function: Node,
        argument: Node,
    },
    Loop(Node),
    BuildStruct {
        fields: Vec<(String, Node)>,
    },
    ChainOpen {
        statements: Vec<Node>,
        expression: Node,
    },
    ChainClosed {
        statements: Vec<Node>,
    },
}

#[derive(Sentinel, Clone, Copy, Debug)]
pub enum NodeSentinel {}

pub type Node = Index<NodeSentinel>;
pub type Nodes = KeyVec<NodeSentinel, NodeData>;

pub const ROOT_NODE: Node = Node::from_u32_index(0);
