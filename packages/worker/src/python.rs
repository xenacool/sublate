use rustpython_vm::{
    pyclass, pymodule, PyPayload, Traverse,
    builtins::{PyStrRef},
    PyObjectRef, PyResult, VirtualMachine,
    class::StaticType, AsObject, PyRef,
};
use ui::state_machine::{SAM, AnimationState, LoopBehavior};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde_json::json;
use num_traits::ToPrimitive;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub name: String,
    pub data: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct SamBuilder {
    pub states: std::collections::HashMap<String, AnimationState>,
    pub transitions: Vec<ui::state_machine::Transition>,
    pub entry_state: String,
    pub lottie_json: serde_json::Value,
    pub logs: Vec<LogEntry>,
}

impl SamBuilder {
    pub fn new() -> Self {
        Self {
            states: std::collections::HashMap::new(),
            transitions: Vec::new(),
            entry_state: String::new(),
            lottie_json: json!({
                "v": "5.5.2",
                "fr": 30,
                "ip": 0,
                "op": 100,
                "w": 800,
                "h": 600,
                "nm": "Samantha Animation",
                "ddd": 0,
                "assets": [],
                "layers": []
            }),
            logs: Vec::new(),
        }
    }

    pub fn into_sam(self) -> SAM {
        SAM {
            states: self.states,
            transitions: self.transitions,
            entry_state: self.entry_state,
            lottie_json: serde_json::to_string(&self.lottie_json).unwrap_or_else(|_| "{}".to_string()),
        }
    }
}

#[pyclass(module = "samantha", name = "SamBuilder")]
#[derive(Debug, Clone, PyPayload, Traverse)]
pub struct SamBuilderWrapper {
    #[pytraverse(skip)]
    pub inner: Arc<Mutex<SamBuilder>>,
}

#[pyclass]
impl SamBuilderWrapper {
    #[pymethod]
    pub fn logs(&self, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
        let builder = self.inner.lock().unwrap();
        let py_logs = vm.ctx.new_list(builder.logs.iter().map(|log| {
            let dict = vm.ctx.new_dict();
            let _ = dict.set_item("name", vm.ctx.new_str(log.name.clone()).into(), vm);
            for (k, v) in &log.data {
                let _ = dict.set_item(k, vm.ctx.new_str(v.clone()).into(), vm);
            }
            dict.into()
        }).collect());
        Ok(py_logs.into())
    }

    #[pymethod]
    pub fn log(&self, name: String, kwargs: rustpython_vm::function::KwArgs, vm: &VirtualMachine) -> PyResult<()> {
        let mut builder = self.inner.lock().unwrap();
        let log_entry = LogEntry {
            name,
            data: kwargs.into_iter().map(|(k, v)| {
                let v_str = v.repr(vm).map(|s| s.as_str().to_string()).unwrap_or_else(|_| "?".to_string());
                (k, v_str)
            }).collect(),
        };
        builder.logs.push(log_entry);
        Ok(())
    }

    #[pygetset]
    pub fn composition(&self, vm: &VirtualMachine) -> PyRef<CompositionWrapper> {
        PyPayload::into_ref(CompositionWrapper {
            sam: Arc::clone(&self.inner),
        }, &vm.ctx)
    }

    #[pymethod]
    pub fn add_state(
        &self, 
        args: rustpython_vm::function::KwArgs,
        vm: &VirtualMachine
    ) -> PyResult<()> {
        let mut builder = self.inner.lock().unwrap();
        
        let steps = if let Some(s) = args.clone().into_iter().find(|(k, _)| k == "steps").map(|(_, v)| v) {
            s.repr(vm)?.as_str().to_string()
        } else {
            "0".to_string()
        };
        
        let frame = if let Some(f) = args.into_iter().find(|(k, _)| k == "frame").map(|(_, v)| v) {
            if let Some(float) = f.payload::<rustpython_vm::builtins::PyFloat>() {
                float.to_f64()
            } else if let Some(int) = f.payload::<rustpython_vm::builtins::PyInt>() {
                int.as_bigint().to_f64().unwrap_or(0.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let state = AnimationState {
            name: format!("step_{}", steps),
            loop_behavior: LoopBehavior::Hold,
            frame_range: Some((frame, frame)),
        };
        
        if builder.states.is_empty() {
            builder.entry_state = state.name.clone();
        }
        
        builder.states.insert(state.name.clone(), state);
        Ok(())
    }
}

#[pyclass(module = "samantha", name = "Composition")]
#[derive(Debug, Clone, PyPayload, Traverse)]
pub struct CompositionWrapper {
    #[pytraverse(skip)]
    pub sam: Arc<Mutex<SamBuilder>>,
}

#[pyclass]
impl CompositionWrapper {
    #[pymethod]
    fn add_layer(&self, name: String, vm: &VirtualMachine) -> PyResult<PyRef<LayerWrapper>> {
        let mut sam_builder = self.sam.lock().unwrap();
        let layers = sam_builder.lottie_json.get_mut("layers").and_then(|l| l.as_array_mut()).unwrap();
        
        let layer_index = layers.len();
        layers.push(json!({
            "ty": 4,
            "nm": name.to_string(),
            "ks": {
                "p": { "k": [0, 0, 0] },
                "s": { "k": [100, 100, 100] },
                "r": { "k": 0 },
                "o": { "k": 100 },
                "a": { "k": [0, 0, 0] }
            },
            "shapes": [
                {
                    "ty": "rc",
                    "nm": "RectanglePath",
                    "s": { "k": [24, 100] },
                    "p": { "k": [0, 0] },
                    "r": { "k": 0 }
                },
                {
                    "ty": "fl",
                    "nm": "Fill",
                    "c": { "k": [1, 1, 1, 1] },
                    "o": { "k": 100 },
                    "r": 1,
                    "bm": 0
                }
            ],
            "ip": 0,
            "op": 100,
            "st": 0,
            "bm": 0
        }));

        Ok(PyPayload::into_ref(LayerWrapper {
            layer_index,
            sam: Arc::clone(&self.sam),
        }, &vm.ctx))
    }
}

#[pyclass(module = "samantha", name = "Layer")]
#[derive(Debug, Clone, PyPayload, Traverse)]
pub struct LayerWrapper {
    #[pytraverse(skip)]
    pub layer_index: usize,
    #[pytraverse(skip)]
    pub sam: Arc<Mutex<SamBuilder>>,
}

#[pyclass]
impl LayerWrapper {
    #[pymethod]
    fn set_position(&self, x: f64, y: f64, _vm: &VirtualMachine) -> PyResult<()> {
        let mut sam_builder = self.sam.lock().unwrap();
        let layers = sam_builder.lottie_json.get_mut("layers").and_then(|l| l.as_array_mut()).unwrap();
        let layer = &mut layers[self.layer_index];
        layer["ks"]["p"]["k"] = json!([x, y, 0]);
        Ok(())
    }

    #[pymethod]
    fn set_size(&self, w: f64, h: f64, _vm: &VirtualMachine) -> PyResult<()> {
        let mut sam_builder = self.sam.lock().unwrap();
        let layers = sam_builder.lottie_json.get_mut("layers").and_then(|l| l.as_array_mut()).unwrap();
        let layer = &mut layers[self.layer_index];
        layer["shapes"][0]["s"]["k"] = json!([w, h]);
        Ok(())
    }

    #[pymethod]
    fn set_color(&self, r: f64, g: f64, b: f64, _vm: &VirtualMachine) -> PyResult<()> {
        let mut sam_builder = self.sam.lock().unwrap();
        let layers = sam_builder.lottie_json.get_mut("layers").and_then(|l| l.as_array_mut()).unwrap();
        let layer = &mut layers[self.layer_index];
        layer["shapes"][1]["c"]["k"] = json!([r, g, b, 1.0]);
        Ok(())
    }
}

#[pymodule(name = "samantha")]
pub(crate) mod decl {
    use super::*;

    #[pyattr]
    fn SamBuilder(vm: &VirtualMachine) -> PyObjectRef {
        SamBuilderWrapper::static_type().to_owned().into()
    }

    #[pyattr]
    fn Composition(vm: &VirtualMachine) -> PyObjectRef {
        CompositionWrapper::static_type().to_owned().into()
    }

    #[pyattr]
    fn Layer(vm: &VirtualMachine) -> PyObjectRef {
        LayerWrapper::static_type().to_owned().into()
    }

    #[pyfunction]
    pub fn build(func: PyObjectRef, data: PyObjectRef, vm: &VirtualMachine) -> PyResult<PyRef<SamBuilderWrapper>> {
        let builder = SamBuilder::new();
        let wrapper = SamBuilderWrapper {
            inner: Arc::new(Mutex::new(builder)),
        };
        let wrapper_ref = PyPayload::into_ref(wrapper, &vm.ctx);
        let _ = func.call((wrapper_ref.clone(), data), vm)?;
        Ok(wrapper_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_vm::Interpreter;
    use rustpython_vm::builtins::PyModule;

    #[test]
    fn test_python_lottie_conversion() {
        let settings = rustpython_vm::Settings::default();
        let interp = Interpreter::with_init(settings, |vm: &mut VirtualMachine| {
            vm.add_native_module("samantha", Box::new(|vm| {
                SamBuilderWrapper::init_builtin_type();
                CompositionWrapper::init_builtin_type();
                LayerWrapper::init_builtin_type();
                
                let def = decl::__module_def(&vm.ctx);
                let module = PyPayload::into_ref(PyModule::from_def(def), &vm.ctx);
                decl::__init_attributes(vm, &module);
                
                module.set_attr("SamBuilder", SamBuilderWrapper::class(&vm.ctx).to_owned(), vm).unwrap();
                module.set_attr("Composition", CompositionWrapper::class(&vm.ctx).to_owned(), vm).unwrap();
                module.set_attr("Layer", LayerWrapper::class(&vm.ctx).to_owned(), vm).unwrap();
                module.into()
            }));
        });

        interp.enter(|vm| {
            let scope = vm.new_scope_with_builtins();
            let sam_module = vm.import("samantha", 0).unwrap();
            scope.globals.set_item("samantha", sam_module.into(), vm).unwrap();
            
            let source = r#"
def build_animation(sam, initial_data, logs):
    comp = sam.composition
    layer = comp.add_layer("test-layer")
    layer.set_position(10.0, 20.0)
    sam.add_state(steps=1, frame=30.0)
"#;
            let code_obj = vm.compile(source, rustpython_vm::compiler::Mode::Exec, "<test>".to_string()).unwrap();
            vm.run_code_obj(code_obj, scope.clone()).unwrap();
            
            let sam_builder = SamBuilderWrapper { 
                inner: Arc::new(Mutex::new(SamBuilder::new())) 
            };
            let sam_builder_ref = PyPayload::into_ref(sam_builder, &vm.ctx);
            
            let build_animation = scope.globals.get_item("build_animation", vm).unwrap();
            build_animation.call((sam_builder_ref.clone(), vm.ctx.new_list(vec![]), vm.ctx.new_list(vec![])), vm).unwrap();

            let builder = sam_builder_ref.inner.lock().unwrap();
            let json = &builder.lottie_json;
            let layers = json["layers"].as_array().unwrap();
            assert_eq!(layers.len(), 1);
            assert_eq!(layers[0]["nm"], "test-layer");
            
            let pos = &layers[0]["ks"]["p"]["k"];
            assert_eq!(pos[0], 10.0);
            assert_eq!(pos[1], 20.0);

            assert!(builder.states.contains_key("step_1"));
            let state = builder.states.get("step_1").unwrap();
            assert_eq!(state.frame_range, Some((30.0, 30.0)));
        });
    }
}
