import init, { init_worker } from "./worker_lib.js";
console.log("ES Module Worker started");
init("./worker_bg.wasm").then(() => { 
    console.log("WASM initialized in worker"); 
    init_worker();
    console.log("Worker registered");
}).catch(err => { 
    console.error("Failed to initialize worker WASM:", err); 
});
