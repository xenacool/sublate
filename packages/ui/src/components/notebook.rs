use dioxus::prelude::*;

#[component]
pub fn Notebook() -> Element {
    let mut code = use_signal(|| "def bubble_sort(arr):\n    n = len(arr)\n    for i in range(n):\n        for j in range(0, n-i-1):\n            if arr[j] > arr[j+1]:\n                arr[j], arr[j+1] = arr[j+1], arr[j]".to_string());
    
    rsx! {
        div { class: "notebook",
            div { class: "cell",
                textarea {
                    value: "{code}",
                    oninput: move |evt| code.set(evt.value().clone()),
                    rows: "10",
                    cols: "50",
                }
                button {
                    onclick: move |_| {
                        // Trigger PythonVM step execution
                    },
                    "Step"
                }
            }
            div { class: "visualization",
                // Roughr-based canvas will go here
                "Visualization Canvas"
            }
        }
    }
}
