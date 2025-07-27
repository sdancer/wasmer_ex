#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]

use rustler::types::{Binary, LocalPid, OwnedBinary};
use rustler::{Atom, Encoder, Env, Error, NifResult, OwnedEnv, ResourceArc, Term};
use wasmer::StoreMut;

use wasmer::{
    imports,
    sys::{EngineBuilder, Features},
    wasmparser::Operator,
    AsStoreMut, Engine, Function, FunctionEnv, FunctionEnvMut, FunctionType, Global, Instance,
    Memory, MemoryType, MemoryView, Module, Pages, RuntimeError, Store, Type, Value,
};
use wasmer_compiler_singlepass::Singlepass;

use std::sync::{Arc, Mutex, OnceLock};
use wasmer_middlewares::{
    metering::{get_remaining_points, set_remaining_points, MeteringPoints},
    Metering,
};

use std::collections::HashMap;

//use sha2::{Sha256, Digest};
//use rand::random;
use std::sync::{mpsc, LazyLock};
use std::time::{Duration, SystemTime};

pub mod atoms;
pub mod db;
pub mod entry;
pub mod wasm;

static MODULE_CACHE: OnceLock<Mutex<HashMap<[u8; 32], (Arc<Engine>, Arc<Module>)>>> =
    OnceLock::new();

use crate::wasm::*;

#[rustler::nif(schedule = "DirtyCpu")]
fn call<'a>(
    env: Env<'a>,
    rpc_pid: LocalPid,
    mapenv: Term<'a>,
    wasm_bytes: Binary,
    function_name: String,
    function_args: Vec<Term<'a>>,
) -> Result<rustler::Term<'a>, rustler::Error> {
    todo!()
}

#[rustler::nif(schedule = "DirtyCpu")]
fn validate_contract<'a>(
    env: Env<'a>,
    mapenv: Term<'a>,
    wasm_bytes: Binary,
) -> Result<rustler::Term<'a>, rustler::Error> {
    todo!()
}

rustler::init!("Elixir.WasmerEx", load = db::load);
