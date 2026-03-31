use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use gloo_worker::reactor::{Reactor, ReactorScope};
use serde::{Deserialize, Serialize};
use futures::{StreamExt, SinkExt};
use std::sync::{Arc, Mutex};
use rustpython_vm::{VirtualMachine, builtins::PyModule, PyPayload, class::StaticType, class::PyClassImpl, AsObject};
use num_traits::ToPrimitive;

mod python;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Input {
    Init(String),
    Compile(String),
    RunHegel(String),
    GetLottie(String, Vec<i32>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Output {
    InitResult(String),
    Result(String),
    HegelResult(HegelResponse),
    LottieResult(ui::state_machine::SAM),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HegelResponse {
    pub logs: Vec<String>,
    pub data: Vec<i32>,
    pub passed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Envelope<T> {
    pub seq: u64,
    pub msg: T,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReliableInput {
    Msg(Envelope<Input>),
    Watermark(u64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReliableOutput {
    Msg(Envelope<Output>),
    Watermark(u64),
}

use std::collections::VecDeque;

pub struct SamanthaWorker {
    scope: ReactorScope<ReliableInput, ReliableOutput>,
    next_output_seq: u64,
    last_received_seq: u64,
    outbox: VecDeque<ReliableOutput>,
    interp: rustpython_vm::Interpreter,
    py_scope: rustpython_vm::scope::Scope,
}

impl Reactor for SamanthaWorker {
    type Scope = ReactorScope<ReliableInput, ReliableOutput>;

    fn create(scope: Self::Scope) -> Self {
        let settings = rustpython_vm::Settings::default();
        let interp = rustpython_vm::Interpreter::with_init(settings, |vm: &mut VirtualMachine| {
            let genesis = rustpython_vm::Context::genesis();
            
            let sam_builder_type = python::SamBuilderWrapper::init_builtin_type();
            <python::SamBuilderWrapper as PyClassImpl>::extend_class(genesis, sam_builder_type);
            
            let composition_type = python::CompositionWrapper::init_builtin_type();
            <python::CompositionWrapper as PyClassImpl>::extend_class(genesis, composition_type);
            
            let layer_type = python::LayerWrapper::init_builtin_type();
            <python::LayerWrapper as PyClassImpl>::extend_class(genesis, layer_type);

            vm.add_native_module("samantha", Box::new(|vm| {
                let def = python::decl::__module_def(&vm.ctx);
                let module = PyPayload::into_ref(PyModule::from_def(def), &vm.ctx);
                python::decl::__init_attributes(vm, &module);
                module.into()
            }));
        });
        
        let py_scope = interp.enter(|vm| vm.new_scope_with_builtins());

        Self {
            scope,
            next_output_seq: 0,
            last_received_seq: 0,
            outbox: VecDeque::new(),
            interp,
            py_scope,
        }
    }
}

impl SamanthaWorker {
    fn run_python_init(&self, code: String) -> String {
        self.interp.enter(|vm: &VirtualMachine| {
            let scope = vm.new_scope_with_builtins();
            match vm.run_block_expr(scope, &code) {
                Ok(obj) => format!("Init OK: {:?}", obj),
                Err(e) => format!("Init Error: {:?}", e),
            }
        })
    }

    fn run_python_hegel(&self, code: String) -> HegelResponse {
        self.interp.enter(|vm: &VirtualMachine| {
            let scope = self.py_scope.clone();
            
            if let Err(e) = vm.run_block_expr(scope.clone(), &code) {
                return HegelResponse {
                    logs: vec![format!("Error running code: {:?}", e)],
                    data: vec![],
                    passed: false,
                };
            }

            if let Ok(build_test) = scope.globals.get_item("build_test", vm) {
                let py_data = match build_test.call((), vm) {
                    Ok(d) => d,
                    Err(e) => {
                        return HegelResponse {
                            logs: vec![format!("Hegel: build_test failed: {:?}", e)],
                            data: vec![],
                            passed: false,
                        };
                    }
                };
                let data: Vec<i32> = if let Some(list) = py_data.payload::<rustpython_vm::builtins::PyList>() {
                    list.borrow_vec().iter().map(|item| {
                        if let Some(int) = item.payload::<rustpython_vm::builtins::PyInt>() {
                            ToPrimitive::to_f64(int.as_bigint()).unwrap_or(0.0) as i32
                        } else {
                            0
                        }
                    }).collect()
                } else {
                    vec![]
                };

                let py_data_clone = vm.ctx.new_list(data.iter().map(|&i| vm.ctx.new_int(i).into()).collect());
                let sam_fsm = python::SamBuilderWrapper { 
                    inner: Arc::new(Mutex::new(python::SamBuilder::new())) 
                };
                let sam_fsm_py = PyPayload::into_ref(sam_fsm, &vm.ctx);

                if let Ok(build_algorithm) = scope.globals.get_item("build_algorithm", vm) {
                    if let Err(e) = build_algorithm.call((sam_fsm_py.clone(), py_data_clone.clone()), vm) {
                        let mut msg = String::new();
                        vm.write_exception(&mut msg, &e).unwrap();
                        return HegelResponse {
                            logs: vec![format!("Hegel: build_algorithm failed: {}", msg)],
                            data: vec![],
                            passed: false,
                        };
                    }
                }

                // Check sorting invariant (Selection Sort example)
                let mut passed = true;
                let mut logs = vec!["Hegel: Algorithm executed".to_string()];
                
                let resulting_data: Vec<i32> = py_data_clone.borrow_vec().iter().map(|item| {
                    if let Some(int) = item.payload::<rustpython_vm::builtins::PyInt>() {
                        ToPrimitive::to_f64(int.as_bigint()).unwrap_or(0.0) as i32
                    } else {
                        0
                    }
                }).collect();

                for i in 1..resulting_data.len() {
                    if resulting_data[i-1] > resulting_data[i] {
                        passed = false;
                        logs.push(format!("Invariant failed: item at {} ({}) > item at {} ({})", i-1, resulting_data[i-1], i, resulting_data[i]));
                        break;
                    }
                }

                if passed {
                    logs.push("Hegel: All invariants passed".to_string());
                }

                HegelResponse {
                    logs,
                    data,
                    passed,
                }
            } else {
                HegelResponse {
                    logs: vec!["Hegel: No build_test found".to_string()],
                    data: vec![],
                    passed: false,
                }
            }
        })
    }

    fn run_python_get_sam(&self, code: String, data: Vec<i32>) -> ui::state_machine::SAM {
        self.interp.enter(|vm: &VirtualMachine| {
            let scope = self.py_scope.clone();
            
            if let Err(_) = vm.run_block_expr(scope.clone(), &code) {
                return ui::state_machine::SAM::default();
            }

            let py_data = vm.ctx.new_list(data.iter().map(|&i| vm.ctx.new_int(i).into()).collect());

            let sam_builder = python::SamBuilderWrapper { 
                inner: Arc::new(Mutex::new(python::SamBuilder::new())) 
            };
            let sam_builder_py = PyPayload::into_ref(sam_builder, &vm.ctx);

            if let Ok(build_algorithm) = scope.globals.get_item("build_algorithm", vm) {
                let _ = build_algorithm.call((sam_builder_py.clone(), py_data.clone()), vm);
            }

            let logs = if let Ok(l) = sam_builder_py.logs(vm) {
                l
            } else {
                vm.ctx.new_list(vec![]).into()
            };

            if let Ok(build_animation) = scope.globals.get_item("build_animation", vm) {
                web_sys::console::log_1(&"Calling build_animation".into());
                match build_animation.call((sam_builder_py.clone(), py_data, logs), vm) {
                    Ok(_) => {
                        web_sys::console::log_1(&"build_animation success".into());
                    }
                    Err(e) => {
                        web_sys::console::log_1(&format!("build_animation failed: {:?}", e).into());
                    }
                }
            } else {
                web_sys::console::log_1(&"build_animation not found in scope".into());
            }

            let sam_builder_inner = Arc::clone(&sam_builder_py.inner);
            let builder = sam_builder_inner.lock().unwrap();
            web_sys::console::log_1(&format!("Final SAM states count: {}", builder.states.len()).into());
            builder.to_owned().into_sam()
        })
    }
}

impl Future for SamanthaWorker {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Drain outbox
        while let Some(msg) = self.outbox.front() {
            match self.scope.poll_ready_unpin(cx) {
                Poll::Ready(Ok(())) => {
                    let msg = self.outbox.pop_front().unwrap();
                    let _ = self.scope.start_send_unpin(msg);
                }
                Poll::Ready(Err(_)) => {
                    self.outbox.pop_front();
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        while let Poll::Ready(Some(input)) = self.scope.poll_next_unpin(cx) {
            match input {
                ReliableInput::Msg(envelope) => {
                    web_sys::console::log_1(&format!("Worker received seq: {}", envelope.seq).into());
                    
                    // Logic for reliable messaging: only process if seq is next expected
                    // For now, we just process everything but keep track of last received
                    self.last_received_seq = envelope.seq;
                    
                    let response = match envelope.msg {
                        Input::Init(code) => {
                            Output::InitResult(self.run_python_init(code))
                        }
                        Input::Compile(code) => {
                            Output::Result(format!("Compiled: {} bytes", code.len()))
                        }
                        Input::RunHegel(code) => {
                            Output::HegelResult(self.run_python_hegel(code))
                        }
                        Input::GetLottie(code, data) => {
                            Output::LottieResult(self.run_python_get_sam(code, data))
                        }
                    };
                    
                    let out_envelope = Envelope {
                        seq: self.next_output_seq,
                        msg: response,
                    };
                    self.next_output_seq += 1;
                    
                    self.outbox.push_back(ReliableOutput::Msg(out_envelope));
                    let ack_seq = self.last_received_seq;
                    self.outbox.push_back(ReliableOutput::Watermark(ack_seq));
                }
                ReliableInput::Watermark(_seq) => {
                    // Handle ACK
                }
            }
        }
        
        // Re-drain outbox if anything was added
        while let Some(_) = self.outbox.front() {
            match self.scope.poll_ready_unpin(cx) {
                Poll::Ready(Ok(())) => {
                    let msg = self.outbox.pop_front().unwrap();
                    let _ = self.scope.start_send_unpin(msg);
                }
                Poll::Ready(Err(_)) => {
                    self.outbox.pop_front();
                }
                Poll::Pending => break,
            }
        }

        let _ = self.scope.poll_flush_unpin(cx);
        Poll::Pending
    }
}

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn init_worker() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"Worker init_worker() called".into());
    gloo_worker::reactor::ReactorRegistrar::<SamanthaWorker>::new().register();
}

pub use gloo_worker::reactor::ReactorBridge;
use gloo_worker::reactor::ReactorSpawner;

pub fn spawn() -> ReactorBridge<SamanthaWorker> {
    gloo_worker::reactor::ReactorSpawner::<SamanthaWorker>::new()
        .as_module(true)
        .with_loader(true)
        .spawn("/worker.js?v=2")
}
