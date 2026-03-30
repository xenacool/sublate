use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use gloo_worker::reactor::{Reactor, ReactorScope};
use serde::{Deserialize, Serialize};
use futures::{StreamExt, SinkExt};
use std::sync::{Arc, Mutex};
use rustpython_vm::{VirtualMachine, builtins::PyModule, PyPayload, class::StaticType};
use num_traits::ToPrimitive;

mod python;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Input {
    Compile(String),
    RunHegel(String),
    GetLottie(String, Vec<i32>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Output {
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

pub struct SamanthaWorker {
    scope: ReactorScope<ReliableInput, ReliableOutput>,
    next_output_seq: u64,
    last_received_seq: u64,
}

impl Reactor for SamanthaWorker {
    type Scope = ReactorScope<ReliableInput, ReliableOutput>;

    fn create(scope: Self::Scope) -> Self {
        Self {
            scope,
            next_output_seq: 0,
            last_received_seq: 0,
        }
    }
}

impl SamanthaWorker {
    fn run_python_hegel(&self, code: String) -> HegelResponse {
        let settings = rustpython_vm::Settings::default();
        let interp = rustpython_vm::Interpreter::with_init(settings, |vm: &mut VirtualMachine| {
            vm.add_native_module("samantha", Box::new(|vm| {
                python::SamBuilderWrapper::init_builtin_type();
                python::CompositionWrapper::init_builtin_type();
                python::LayerWrapper::init_builtin_type();

                let def = python::decl::__module_def(&vm.ctx);
                let module = PyPayload::into_ref(PyModule::from_def(def), &vm.ctx);
                python::decl::__init_attributes(vm, &module);
                
                module.set_attr("SamBuilder", python::SamBuilderWrapper::class(&vm.ctx).to_owned(), vm).unwrap();
                module.set_attr("Composition", python::CompositionWrapper::class(&vm.ctx).to_owned(), vm).unwrap();
                module.set_attr("Layer", python::LayerWrapper::class(&vm.ctx).to_owned(), vm).unwrap();
                module.into()
            }));
        });
        interp.enter(|vm: &VirtualMachine| {
            let scope = vm.new_scope_with_builtins();
            
            if let Err(e) = vm.run_block_expr(scope.clone(), &code) {
                return HegelResponse {
                    logs: vec![format!("Error running code: {:?}", e)],
                    data: vec![],
                    passed: false,
                };
            }

            if let Ok(build_test) = scope.globals.get_item("build_test", vm) {
                let py_data = build_test.call((), vm).unwrap_or(vm.ctx.none());
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

                if let Ok(build_algorithm) = scope.globals.get_item("build_algorithm", vm) {
                    let _ = build_algorithm.call((sam_fsm.clone(), py_data_clone.clone()), vm);
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
        let settings = rustpython_vm::Settings::default();
        let interp = rustpython_vm::Interpreter::with_init(settings, |vm: &mut VirtualMachine| {
            vm.add_native_module("samantha", Box::new(|vm| {
                python::SamBuilderWrapper::init_builtin_type();
                python::CompositionWrapper::init_builtin_type();
                python::LayerWrapper::init_builtin_type();

                let def = python::decl::__module_def(&vm.ctx);
                let module = PyPayload::into_ref(PyModule::from_def(def), &vm.ctx);
                python::decl::__init_attributes(vm, &module);
                
                module.set_attr("SamBuilder", python::SamBuilderWrapper::class(&vm.ctx).to_owned(), vm).unwrap();
                module.set_attr("Composition", python::CompositionWrapper::class(&vm.ctx).to_owned(), vm).unwrap();
                module.set_attr("Layer", python::LayerWrapper::class(&vm.ctx).to_owned(), vm).unwrap();
                module.into()
            }));
        });
        interp.enter(|vm: &VirtualMachine| {
            let scope = vm.new_scope_with_builtins();
            
            if let Err(_) = vm.run_block_expr(scope.clone(), &code) {
                return ui::state_machine::SAM::default();
            }

            let py_data = vm.ctx.new_list(data.iter().map(|&i| vm.ctx.new_int(i).into()).collect());

            let sam_builder = python::SamBuilderWrapper { 
                inner: Arc::new(Mutex::new(python::SamBuilder::new())) 
            };
            
            if let Ok(build_algorithm) = scope.globals.get_item("build_algorithm", vm) {
                let _ = build_algorithm.call((sam_builder.clone(), py_data.clone()), vm);
            }

            let logs = if let Ok(l) = sam_builder.logs(vm) {
                l
            } else {
                vm.ctx.new_list(vec![]).into()
            };

            if let Ok(build_animation) = scope.globals.get_item("build_animation", vm) {
                let _ = build_animation.call((sam_builder.clone(), py_data, logs), vm);
            }

            let builder = sam_builder.inner.lock().unwrap();
            builder.sam.clone()
        })
    }
}

impl Future for SamanthaWorker {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        while let Poll::Ready(Some(input)) = self.scope.poll_next_unpin(cx) {
            match input {
                ReliableInput::Msg(envelope) => {
                    self.last_received_seq = envelope.seq;
                    let response = match envelope.msg {
                        Input::Compile(code) => {
                            // Placeholder for Python compilation
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
                    
                    let _ = self.scope.start_send_unpin(ReliableOutput::Msg(out_envelope));
                    let last_received_seq = self.last_received_seq;
                    let _ = self.scope.start_send_unpin(ReliableOutput::Watermark(last_received_seq));
                }
                ReliableInput::Watermark(_seq) => {
                    // Handle ACK
                }
            }
        }
        
        let _ = self.scope.poll_flush_unpin(cx);
        Poll::Pending
    }
}

pub use gloo_worker::reactor::ReactorBridge;
use gloo_worker::reactor::ReactorSpawner;

pub fn spawn() -> ReactorBridge<SamanthaWorker> {
    ReactorSpawner::<SamanthaWorker>::new()
        .as_module(true)
        .spawn("/worker.js?v=1")
}
