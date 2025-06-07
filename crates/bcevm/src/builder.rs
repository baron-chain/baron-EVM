//bcmod [err#161]
use crate::{
    db::{Database, DatabaseRef, EmptyDB, WrapDatabaseRef},
    handler::register,
    primitives::{
        BlockEnv, CfgEnv, CfgEnvWithHandlerCfg, Env, EnvWithHandlerCfg, HandlerCfg, SpecId, TxEnv,
    },
    Context, ContextWithHandlerCfg, Evm, Handler,
};
use core::marker::PhantomData;
use std::boxed::Box;

pub struct EvmBuilder<'a, BuilderStage, EXT, DB: Database> {
    context: Context<EXT, DB>,
    handler: Handler<'a, Evm<'a, EXT, DB>, EXT, DB>,
    phantom: PhantomData<BuilderStage>,
}

pub struct SetGenericStage;
pub struct HandlerStage;

impl<'a> Default for EvmBuilder<'a, SetGenericStage, (), EmptyDB> {
    fn default() -> Self {
        let handler_cfg = if cfg!(all(feature = "optimism-default-handler", not(feature = "negate-optimism-default-handler"))) {
            let mut cfg = HandlerCfg::new(SpecId::LATEST);
            cfg.is_optimism = true;
            cfg
        } else {
            HandlerCfg::new(SpecId::LATEST)
        };

        Self {
            context: Context::default(),
            handler: Self::handler(handler_cfg),
            phantom: PhantomData,
        }
    }
}

impl<'a, EXT, DB: Database> EvmBuilder<'a, SetGenericStage, EXT, DB> {
    pub fn with_empty_db(self) -> EvmBuilder<'a, SetGenericStage, EXT, EmptyDB> {
        self.with_db(EmptyDB::default())
    }

    pub fn with_db<ODB: Database>(self, db: ODB) -> EvmBuilder<'a, SetGenericStage, EXT, ODB> {
        EvmBuilder {
            context: Context::new(self.context.evm.with_db(db), self.context.external),
            handler: Self::handler(self.handler.cfg()),
            phantom: PhantomData,
        }
    }

    pub fn with_ref_db<ODB: DatabaseRef>(
        self,
        db: ODB,
    ) -> EvmBuilder<'a, SetGenericStage, EXT, WrapDatabaseRef<ODB>> {
        self.with_db(WrapDatabaseRef(db))
    }

    pub fn with_external_context<OEXT>(
        self,
        external: OEXT,
    ) -> EvmBuilder<'a, SetGenericStage, OEXT, DB> {
        EvmBuilder {
            context: Context::new(self.context.evm, external),
            handler: Self::handler(self.handler.cfg()),
            phantom: PhantomData,
        }
    }

    pub fn with_env_with_handler_cfg(
        mut self,
        EnvWithHandlerCfg { env, handler_cfg }: EnvWithHandlerCfg,
    ) -> EvmBuilder<'a, HandlerStage, EXT, DB> {
        self.context.evm.env = env;
        EvmBuilder {
            context: self.context,
            handler: Self::handler(handler_cfg),
            phantom: PhantomData,
        }
    }

    pub fn with_context_with_handler_cfg<OEXT, ODB: Database>(
        self,
        context_with_handler_cfg: ContextWithHandlerCfg<OEXT, ODB>,
    ) -> EvmBuilder<'a, HandlerStage, OEXT, ODB> {
        EvmBuilder {
            context: context_with_handler_cfg.context,
            handler: Self::handler(context_with_handler_cfg.cfg),
            phantom: PhantomData,
        }
    }

    pub fn with_cfg_env_with_handler_cfg(
        mut self,
        cfg_env_and_spec_id: CfgEnvWithHandlerCfg,
    ) -> EvmBuilder<'a, HandlerStage, EXT, DB> {
        self.context.evm.env.cfg = cfg_env_and_spec_id.cfg_env;
        EvmBuilder {
            context: self.context,
            handler: Self::handler(cfg_env_and_spec_id.handler_cfg),
            phantom: PhantomData,
        }
    }

    pub fn with_handler_cfg(
        self,
        handler_cfg: HandlerCfg,
    ) -> EvmBuilder<'a, HandlerStage, EXT, DB> {
        EvmBuilder {
            context: self.context,
            handler: Self::handler(handler_cfg),
            phantom: PhantomData,
        }
    }

    #[cfg(feature = "optimism")]
    pub fn optimism(mut self) -> EvmBuilder<'a, HandlerStage, EXT, DB> {
        self.handler = Handler::optimism_with_spec(self.handler.cfg.spec_id);
        EvmBuilder {
            context: self.context,
            handler: self.handler,
            phantom: PhantomData,
        }
    }

    #[cfg(feature = "optimism-default-handler")]
    pub fn mainnet(mut self) -> EvmBuilder<'a, HandlerStage, EXT, DB> {
        self.handler = Handler::mainnet_with_spec(self.handler.cfg.spec_id);
        EvmBuilder {
            context: self.context,
            handler: self.handler,
            phantom: PhantomData,
        }
    }
}

impl<'a, EXT, DB: Database> EvmBuilder<'a, HandlerStage, EXT, DB> {
    pub fn new(evm: Evm<'a, EXT, DB>) -> Self {
        Self {
            context: evm.context,
            handler: evm.handler,
            phantom: PhantomData,
        }
    }

    pub fn reset_handler_with_empty_db(self) -> EvmBuilder<'a, HandlerStage, EXT, EmptyDB> {
        self.reset_handler_with_db(EmptyDB::default())
    }

    #[cfg(feature = "optimism-default-handler")]
    pub fn reset_handler_with_mainnet(mut self) -> Self {
        self.handler = Handler::mainnet_with_spec(self.handler.cfg.spec_id);
        self
    }

    pub fn reset_handler_with_db<ODB: Database>(
        self,
        db: ODB,
    ) -> EvmBuilder<'a, SetGenericStage, EXT, ODB> {
        EvmBuilder {
            context: Context::new(self.context.evm.with_db(db), self.context.external),
            handler: Self::handler(self.handler.cfg()),
            phantom: PhantomData,
        }
    }

    pub fn reset_handler_with_ref_db<ODB: DatabaseRef>(
        self,
        db: ODB,
    ) -> EvmBuilder<'a, SetGenericStage, EXT, WrapDatabaseRef<ODB>> {
        self.reset_handler_with_db(WrapDatabaseRef(db))
    }

    pub fn reset_handler_with_external_context<OEXT>(
        self,
        external: OEXT,
    ) -> EvmBuilder<'a, SetGenericStage, OEXT, DB> {
        EvmBuilder {
            context: Context::new(self.context.evm, external),
            handler: Self::handler(self.handler.cfg()),
            phantom: PhantomData,
        }
    }
}

impl<'a, BuilderStage, EXT, DB: Database> EvmBuilder<'a, BuilderStage, EXT, DB> {
    fn handler(handler_cfg: HandlerCfg) -> Handler<'a, Evm<'a, EXT, DB>, EXT, DB> {
        Handler::new(handler_cfg)
    }

    pub fn with_handler(
        self,
        handler: Handler<'a, Evm<'a, EXT, DB>, EXT, DB>,
    ) -> EvmBuilder<'a, BuilderStage, EXT, DB> {
        EvmBuilder {
            context: self.context,
            handler,
            phantom: PhantomData,
        }
    }

    pub fn build(self) -> Evm<'a, EXT, DB> {
        Evm::new(self.context, self.handler)
    }

    pub fn append_handler_register(
        mut self,
        handle_register: register::HandleRegister<EXT, DB>,
    ) -> EvmBuilder<'a, HandlerStage, EXT, DB> {
        self.handler
            .append_handler_register(register::HandleRegisters::Plain(handle_register));
        EvmBuilder {
            context: self.context,
            handler: self.handler,
            phantom: PhantomData,
        }
    }

    pub fn append_handler_register_box(
        mut self,
        handle_register: register::HandleRegisterBox<EXT, DB>,
    ) -> EvmBuilder<'a, HandlerStage, EXT, DB> {
        self.handler
            .append_handler_register(register::HandleRegisters::Box(handle_register));
        EvmBuilder {
            context: self.context,
            handler: self.handler,
            phantom: PhantomData,
        }
    }

    pub fn with_spec_id(mut self, spec_id: SpecId) -> Self {
        self.handler.modify_spec_id(spec_id);
        self
    }

    pub fn modify_db(mut self, f: impl FnOnce(&mut DB)) -> Self {
        f(&mut self.context.evm.db);
        self
    }

    pub fn modify_external_context(mut self, f: impl FnOnce(&mut EXT)) -> Self {
        f(&mut self.context.external);
        self
    }

    pub fn modify_env(mut self, f: impl FnOnce(&mut Box<Env>)) -> Self {
        f(&mut self.context.evm.env);
        self
    }

    pub fn with_env(mut self, env: Box<Env>) -> Self {
        self.context.evm.env = env;
        self
    }

    pub fn modify_tx_env(mut self, f: impl FnOnce(&mut TxEnv)) -> Self {
        f(&mut self.context.evm.env.tx);
        self
    }

    pub fn with_tx_env(mut self, tx_env: TxEnv) -> Self {
        self.context.evm.env.tx = tx_env;
        self
    }

    pub fn modify_block_env(mut self, f: impl FnOnce(&mut BlockEnv)) -> Self {
        f(&mut self.context.evm.env.block);
        self
    }

    pub fn with_block_env(mut self, block_env: BlockEnv) -> Self {
        self.context.evm.env.block = block_env;
        self
    }

    pub fn modify_cfg_env(mut self, f: impl FnOnce(&mut CfgEnv)) -> Self {
        f(&mut self.context.evm.env.cfg);
        self
    }

    pub fn with_clear_env(mut self) -> Self {
        self.context.evm.env.clear();
        self
    }

    pub fn with_clear_tx_env(mut self) -> Self {
        self.context.evm.env.tx.clear();
        self
    }

    pub fn with_clear_block_env(mut self) -> Self {
        self.context.evm.env.block.clear();
        self
    }

    pub fn reset_handler(mut self) -> Self {
        self.handler = Self::handler(self.handler.cfg());
        self
    }
}

#[cfg(test)]
mod test {
    // Test module implementation remains unchanged
}
