use crate::{
    inspectors::GasInspector,
    interpreter::{CallInputs, CallOutcome, CreateInputs, CreateOutcome, Interpreter, InterpreterResult},
    primitives::{db::Database, hex, HashMap, B256, U256},
    EvmContext, Inspector,
};
use bcevm_interpreter::OpCode;
use serde::Serialize;
use std::io::Write;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Output {
    pc: u64,
    op: u8,
    gas: String,
    gas_cost: String,
    stack: Vec<String>,
    depth: u64,
    return_data: String,
    refund: String,
    mem_size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    op_name: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    return_stack: Option<Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Summary {
    state_root: String,
    output: String,
    gas_used: String,
    pass: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fork: Option<String>,
}

pub struct TracerEip3155 {
    output: Box<dyn Write>,
    gas_inspector: GasInspector,
    print_summary: bool,
    stack: Vec<U256>,
    pc: usize,
    opcode: u8,
    gas: u64,
    refunded: i64,
    mem_size: usize,
    skip: bool,
    include_memory: bool,
    memory: Option<String>,
}

impl TracerEip3155 {
    pub fn new(output: Box<dyn Write>) -> Self {
        Self {
            output,
            gas_inspector: GasInspector::default(),
            print_summary: true,
            include_memory: false,
            stack: Default::default(),
            memory: Default::default(),
            pc: 0,
            opcode: 0,
            gas: 0,
            refunded: 0,
            mem_size: 0,
            skip: false,
        }
    }

    pub fn set_writer(&mut self, writer: Box<dyn Write>) {
        self.output = writer;
    }

    pub fn clear(&mut self) {
        self.gas_inspector = GasInspector::default();
        self.stack.clear();
        self.pc = 0;
        self.opcode = 0;
        self.gas = 0;
        self.refunded = 0;
        self.mem_size = 0;
        self.skip = false;
    }

    pub fn without_summary(mut self) -> Self {
        self.print_summary = false;
        self
    }

    pub fn with_memory(mut self) -> Self {
        self.include_memory = true;
        self
    }

    fn write_value(&mut self, value: &impl serde::Serialize) -> std::io::Result<()> {
        serde_json::to_writer(&mut *self.output, value)?;
        self.output.write_all(b"\n")?;
        self.output.flush()
    }

    fn print_summary<DB: Database>(&mut self, result: &InterpreterResult, context: &mut EvmContext<DB>) {
        if self.print_summary {
            let spec_name: &str = context.spec_id().into();
            let value = Summary {
                state_root: B256::ZERO.to_string(),
                output: result.output.to_string(),
                gas_used: hex_number(context.inner.env().tx.gas_limit - self.gas_inspector.gas_remaining()),
                pass: result.is_ok(),
                time: None,
                fork: Some(spec_name.to_string()),
            };
            let _ = self.write_value(&value);
        }
    }
}

impl<DB: Database> Inspector<DB> for TracerEip3155 {
    fn initialize_interp(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        self.gas_inspector.initialize_interp(interp, context);
    }

    fn step(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        self.gas_inspector.step(interp, context);
        self.stack = interp.stack.data().clone();
        self.memory = self.include_memory.then(|| hex::encode_prefixed(interp.shared_memory.context_memory()));
        self.pc = interp.program_counter();
        self.opcode = interp.current_opcode();
        self.mem_size = interp.shared_memory.len();
        self.gas = interp.gas.remaining();
        self.refunded = interp.gas.refunded();
    }

    fn step_end(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        self.gas_inspector.step_end(interp, context);
        if self.skip {
            self.skip = false;
            return;
        }

        let value = Output {
            pc: self.pc as u64,
            op: self.opcode,
            gas: hex_number(self.gas),
            gas_cost: hex_number(self.gas_inspector.last_gas_cost()),
            stack: self.stack.iter().map(hex_number_u256).collect(),
            depth: context.journaled_state.depth(),
            return_data: "0x".to_string(),
            refund: hex_number(self.refunded as u64),
            mem_size: self.mem_size.to_string(),
            op_name: OpCode::new(self.opcode).map(|i| i.as_str()),
            error: (!interp.instruction_result.is_ok()).then(|| format!("{:?}", interp.instruction_result)),
            memory: self.memory.take(),
            storage: None,
            return_stack: None,
        };
        let _ = self.write_value(&value);
    }

    fn call_end(&mut self, context: &mut EvmContext<DB>, inputs: &CallInputs, outcome: CallOutcome) -> CallOutcome {
        let outcome = self.gas_inspector.call_end(context, inputs, outcome);
        if context.journaled_state.depth() == 0 {
            self.print_summary(&outcome.result, context);
            self.clear();
        }
        outcome
    }

    fn create_end(&mut self, context: &mut EvmContext<DB>, inputs: &CreateInputs, outcome: CreateOutcome) -> CreateOutcome {
        let outcome = self.gas_inspector.create_end(context, inputs, outcome);
        if context.journaled_state.depth() == 0 {
            self.print_summary(&outcome.result, context);
            self.clear();
        }
        outcome
    }
}

fn hex_number(uint: u64) -> String {
    format!("0x{uint:x}")
}

fn hex_number_u256(b: &U256) -> String {
    let s = hex::encode(b.to_be_bytes::<32>()).trim_start_matches('0').to_string();
    if s.is_empty() { "0x0".to_string() } else { format!("0x{s}") }
}
