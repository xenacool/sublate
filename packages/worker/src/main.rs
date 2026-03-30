use worker::SamanthaWorker;
use gloo_worker::reactor::ReactorRegistrar;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    ReactorRegistrar::<SamanthaWorker>::new().register();
}

fn main() {
    start();
}
