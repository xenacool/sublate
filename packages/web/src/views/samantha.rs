use dioxus::prelude::*;
use worker::{Input, Output, ReliableInput, ReliableOutput, Envelope};
use futures::{StreamExt, SinkExt};
use ui::state_machine::{SAM, AnimationState};
use velato::{Renderer, model};

#[component]
pub fn Samantha() -> Element {
    let mut code = use_signal(|| String::new());
    let mut hegel_logs = use_signal(|| Vec::<String>::new());
    let mut lottie_fsm = use_signal(|| SAM::default());

    let mut lottie_coroutine = use_coroutine(move |mut rx: UnboundedReceiver<(String, Vec<i32>)>| async move {
        let mut bridge = worker::spawn();
        let mut next_input_seq = 0u64;

        loop {
            tokio::select! {
                Some((code_str, data)) = rx.next() => {
                    let envelope = Envelope {
                        seq: next_input_seq,
                        msg: Input::GetLottie(code_str, data),
                    };
                    next_input_seq += 1;
                    let _ = bridge.send(ReliableInput::Msg(envelope)).await;
                }
                Some(output) = bridge.next() => {
                    match output {
                        ReliableOutput::Msg(envelope) => {
                            if let Output::LottieResult(res) = envelope.msg {
                                lottie_fsm.set(res);
                            }
                        }
                        ReliableOutput::Watermark(_seq) => {
                            // Ack
                        }
                    }
                }
            }
        }
    });

    let lottie_h = lottie_coroutine.clone();
    let mut hegel_coroutine = use_coroutine(move |mut rx: UnboundedReceiver<String>| {
        let lottie_h = lottie_h.clone();
        async move {
            let mut bridge = worker::spawn();
            let mut next_input_seq = 0u64;

            loop {
                tokio::select! {
                    Some(code_str) = rx.next() => {
                        let envelope = Envelope {
                            seq: next_input_seq,
                            msg: Input::RunHegel(code_str.clone()),
                        };
                        next_input_seq += 1;
                        let _ = bridge.send(ReliableInput::Msg(envelope)).await;
                        
                        // Wait for result and then trigger lottie
                        if let Some(ReliableOutput::Msg(out_envelope)) = bridge.next().await {
                            if let Output::HegelResult(res) = out_envelope.msg {
                                hegel_logs.write().push(format!("Hegel ({}): {:?}", if res.passed { "passed" } else { "failed" }, res.logs));
                                lottie_h.send((code_str, res.data));
                            }
                        }
                    }
                }
            }
        }
    });

    rsx! {
        div {
            class: "samantha-container",
            div {
                class: "control-bar",
                h1 { "Samantha Editor" }
                button {
                    onclick: move |_| hegel_logs.write().clear(),
                    "Clear Logs"
                }
                button {
                    onclick: move |_| {
                        hegel_coroutine.send(code());
                    },
                    "Run (Manual)"
                }
            }
            div {
                class: "editor-split",
                div {
                    class: "editor-area",
                    textarea {
                        placeholder: "Enter Python code here...",
                        class: "python-editor",
                        oninput: move |e| {
                            code.set(e.value());
                            hegel_coroutine.send(e.value());
                        }
                    }
                }
                    div {
                        class: "preview-area",
                        div {
                            class: "visualization-canvas",
                            h2 { "Lottie Preview" }
                            p { "SAM States: {lottie_fsm().states.len()}" }
                            if lottie_fsm().states.len() > 0 {
                                SvgPreview { sam: lottie_fsm() }
                            }
                        }
                        div {
                            class: "log-viewer",
                            h2 { "Hegel Results" }
                            ul {
                                for log in hegel_logs().iter().rev().take(10) {
                                    li { "{log}" }
                                }
                            }
                        }
                    }
            }
        }
    }
}

#[component]
fn SvgPreview(sam: SAM) -> Element {
    let mut current_state_name = use_signal(|| "step_0".to_string());
    let current_state = sam.states.get(&current_state_name()).cloned();

    if let Some(state) = current_state {
        let composition = velato::Composition::from_json(sam.lottie_json.clone()).unwrap();
        let frame = state.frame_range.map(|(s, _)| s).unwrap_or(0.0);
        
        let mut renderer = Renderer::new();
        let mut sink = ui::state_machine::render::VanillaSink::new();
        
        renderer.append(&composition, frame, kurbo::Affine::IDENTITY, 1.0, &mut sink);
        
        rsx! {
            div {
                div {
                    class: "state-selector",
                    select {
                        onchange: move |e| current_state_name.set(e.value()),
                        for name in sam.states.keys() {
                            option { value: "{name}", selected: *name == current_state_name(), "{name}" }
                        }
                    }
                }
                svg {
                    view_box: "0 0 800 600",
                    width: "100%",
                    height: "400",
                    style: "background: #1a1a1a; border: 1px solid #333;",
                    
                    for path in &sink.paths {
                        path {
                            d: "{path_to_svg(&path.elements)}",
                            fill: "{brush_to_svg(&path.brush)}",
                            stroke: "{stroke_to_svg(&path.stroke)}",
                            transform: "{affine_to_svg(&path.transform)}",
                        }
                    }
                }
            }
        }
    } else {
        rsx! { div { "State {current_state_name} not found" } }
    }
}

fn path_to_svg(elements: &[kurbo::PathEl]) -> String {
    let mut s = String::new();
    for el in elements {
        match el {
            kurbo::PathEl::MoveTo(p) => s.push_str(&format!("M {} {} ", p.x, p.y)),
            kurbo::PathEl::LineTo(p) => s.push_str(&format!("L {} {} ", p.x, p.y)),
            kurbo::PathEl::QuadTo(p1, p2) => s.push_str(&format!("Q {} {}, {} {} ", p1.x, p1.y, p2.x, p2.y)),
            kurbo::PathEl::CurveTo(p1, p2, p3) => s.push_str(&format!("C {} {}, {} {}, {} {} ", p1.x, p1.y, p2.x, p2.y, p3.x, p3.y)),
            kurbo::PathEl::ClosePath => s.push_str("Z "),
        }
    }
    s
}

fn affine_to_svg(affine: &kurbo::Affine) -> String {
    let coeffs = affine.as_coeffs();
    format!("matrix({} {} {} {} {} {})", coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4], coeffs[5])
}

fn brush_to_svg(brush: &velato::model::fixed::Brush) -> String {
    match brush {
        velato::model::fixed::Brush::Solid(color) => {
            let rgba = color.to_rgba8();
            format!("rgba({}, {}, {}, {})", rgba.r, rgba.g, rgba.b, rgba.a as f32 / 255.0)
        }
        _ => "white".to_string(),
    }
}

fn stroke_to_svg(stroke: &Option<velato::model::fixed::Stroke>) -> String {
    if let Some(_s) = stroke {
        "white".to_string() // Simplified
    } else {
        "none".to_string()
    }
}
