use crate::{db::Database, handler::Handler, Evm};
use std::boxed::Box;

pub type EvmHandler<'a, EXT, DB> = Handler<'a, Evm<'a, EXT, DB>, EXT, DB>;
pub type HandleRegister<EXT, DB> = for<'a> fn(&mut EvmHandler<'a, EXT, DB>);
pub type HandleRegisterBox<EXT, DB> = Box<dyn for<'a> Fn(&mut EvmHandler<'a, EXT, DB>)>;

pub enum HandleRegisters<EXT, DB: Database> {
    Plain(HandleRegister<EXT, DB>),
    Box(HandleRegisterBox<EXT, DB>),
}

impl<EXT, DB: Database> HandleRegisters<EXT, DB> {
    pub fn register(&self, handler: &mut EvmHandler<'_, EXT, DB>) {
        match self {
            Self::Plain(f) => f(handler),
            Self::Box(f) => f(handler),
        }
    }
}
