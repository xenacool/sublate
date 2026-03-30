import init from "./worker_lib.js";
console.log("ES Module Worker started");
init("./worker_bg.wasm").then(() => { console.log("WASM initialized in worker"); }).catch(err => { console.error("Failed to initialize worker WASM:", err); });
