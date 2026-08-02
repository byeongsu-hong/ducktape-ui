use crate::semantic::{BinaryOp, UnaryOp};

#[derive(Clone, Debug)]
pub enum Expr {
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Bytes(Vec<u8>),
    EmptyList,
    List(Vec<Expr>),
    None,
    Path(Vec<String>),
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}
