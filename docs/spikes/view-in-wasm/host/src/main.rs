//! THROWAWAY SPIKE host: times the same view session natively and inside
//! wasmtime, checks the two produce identical primitives, and prices the
//! pixels-out alternative.

use std::time::Instant;
use wasmtime::{Config, Engine, Linker, Module, OptLevel, Store, TypedFunc};

const WARMUP: usize = 5;
const FRAMES: usize = 41;

fn median_us(mut f: impl FnMut()) -> f64 {
    for _ in 0..WARMUP {
        f();
    }
    let mut samples: Vec<f64> = (0..FRAMES)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed().as_secs_f64() * 1e6
        })
        .collect();
    samples.sort_by(f64::total_cmp);
    samples[FRAMES / 2]
}

struct Guest {
    store: Store<()>,
    init: TypedFunc<u32, ()>,
    frame: TypedFunc<u32, u32>,
    raster: TypedFunc<(), u32>,
    out_ptr: TypedFunc<(), u32>,
    memory: wasmtime::Memory,
}

impl Guest {
    fn load(path: &str) -> (Self, f64, f64) {
        let mut config = Config::new();
        config.cranelift_opt_level(OptLevel::Speed);
        let engine = Engine::new(&config).unwrap();
        let start = Instant::now();
        let module = Module::from_file(&engine, path).unwrap();
        let compile_ms = start.elapsed().as_secs_f64() * 1e3;
        let mut store = Store::new(&engine, ());
        // web_time/js-sys imports (Instant::now on wasm32) — never on the
        // measured path; stubbed to zeros so instantiation succeeds.
        let mut linker = Linker::new(&engine);
        linker.define_unknown_imports_as_default_values(&mut store, &module).unwrap();
        let start = Instant::now();
        let instance = linker.instantiate(&mut store, &module).unwrap();
        let instantiate_us = start.elapsed().as_secs_f64() * 1e6;
        let init = instance.get_typed_func::<u32, ()>(&mut store, "init").unwrap();
        let frame = instance.get_typed_func::<u32, u32>(&mut store, "frame").unwrap();
        let raster = instance.get_typed_func::<(), u32>(&mut store, "raster").unwrap();
        let out_ptr = instance.get_typed_func::<(), u32>(&mut store, "out_ptr").unwrap();
        let memory = instance.get_memory(&mut store, "memory").unwrap();
        (Self { store, init, frame, raster, out_ptr, memory }, compile_ms, instantiate_us)
    }

    fn init(&mut self, rows: u32) {
        self.init.call(&mut self.store, rows).unwrap();
    }

    fn take(&mut self, len: u32) -> Vec<u8> {
        let ptr = self.out_ptr.call(&mut self.store, ()).unwrap() as usize;
        self.memory.data(&self.store)[ptr..ptr + len as usize].to_vec()
    }

    fn frame(&mut self, changed: bool) -> Vec<u8> {
        let len = self.frame.call(&mut self.store, changed as u32).unwrap();
        self.take(len)
    }

    fn raster(&mut self) -> Vec<u8> {
        let len = self.raster.call(&mut self.store, ()).unwrap();
        self.take(len)
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("guest.wasm path");
    let wasm_bytes = std::fs::metadata(&path).unwrap().len();
    let (mut guest, compile_ms, instantiate_us) = Guest::load(&path);
    println!("guest.wasm: {} KB | cranelift compile {compile_ms:.0} ms | instantiate {instantiate_us:.0} µs", wasm_bytes / 1024);
    println!();
    println!("rows | native steady µs | wasm steady µs | ratio | native changed µs | wasm changed µs | ratio | prims | bytes | host decode µs | parity");
    for rows in [40usize, 200, 1000] {
        // Cold = session creation (font system, renderer) + the first frame,
        // which shapes every paragraph.
        let start = Instant::now();
        let mut native = viewcore::Session::new(rows);
        let native_first = native.frame_bytes(false);
        let native_cold = start.elapsed().as_secs_f64() * 1e6;
        let start = Instant::now();
        guest.init(rows as u32);
        let wasm_first = guest.frame(false);
        let wasm_cold = start.elapsed().as_secs_f64() * 1e6;
        let parity = native_first == wasm_first;
        if !parity {
            let a = viewcore::decode(&native_first);
            let b = viewcore::decode(&wasm_first);
            println!("       parity: native {} prims, wasm {} prims", a.len(), b.len());
            if let Some(i) = (0..a.len().min(b.len())).find(|&i| a[i] != b[i]) {
                println!("       first diff at #{i}:\n         native {:?}\n         wasm   {:?}", a[i], b[i]);
            }
        }

        let native_steady = median_us(|| { native.frame_bytes(false); });
        let wasm_steady = median_us(|| { guest.frame(false); });
        let native_changed = median_us(|| { native.frame_bytes(true); });
        let wasm_changed = median_us(|| { guest.frame(true); });
        let prims = viewcore::decode(&native_first);
        let decode = median_us(|| { viewcore::decode(&native_first); });
        println!(
            "{rows:>4} | {native_steady:>16.1} | {wasm_steady:>14.1} | {:>5.2}x | {native_changed:>17.1} | {wasm_changed:>15.1} | {:>5.2}x | {:>5} | {:>6} | {decode:>14.1} | {}",
            wasm_steady / native_steady,
            wasm_changed / native_changed,
            prims.len(),
            native_first.len(),
            if parity { "identical" } else { "DIFFERS" }
        );
        println!("       cold first frame: native {native_cold:.0} µs, wasm {wasm_cold:.0} µs");
    }

    println!();
    println!("pixels-out alternative (1024x768 @1x, full-frame raster each frame)");
    println!("rows | native raster µs | wasm raster µs | ratio | bytes/frame");
    for rows in [40usize, 200] {
        let mut native = viewcore::Session::new(rows);
        guest.init(rows as u32);
        let native_r = median_us(|| { native.raster(); });
        let wasm_r = median_us(|| { guest.raster(); });
        let bytes = native.raster().len();
        println!("{rows:>4} | {native_r:>16.0} | {wasm_r:>14.0} | {:>5.2}x | {bytes}", wasm_r / native_r);
    }
}
