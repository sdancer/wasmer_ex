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

#[derive(Debug, Clone, Copy)]
pub struct ExitCode(u32);

#[derive(Clone)]
//struct HostEnv<'a> {
pub struct HostEnv {
    pub memory: Option<Memory>,
    pub readonly: bool,
    pub error: Option<Vec<u8>>,
    pub return_value: Option<Vec<u8>>,
    pub logs: Vec<Vec<u8>>,
    pub current_account: Vec<u8>,
    pub rpc_pid: Option<LocalPid>,
    //pub env: Env<'a>
    pub instance: Option<Arc<Instance>>,

    pub attached_symbol: Vec<u8>,
    pub attached_amount: Vec<u8>,

    pub writes: HashMap<Vec<u8>, Vec<u8>>,
}
//unsafe impl Sync for HostEnv<'_> {}
//unsafe impl Send for HostEnv<'_> {}

pub fn import_storage_kv_get_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    key_ptr: i32,
    key_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64)) * 100;

    todo!()
}

pub fn import_storage_kv_exists_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    key_ptr: i32,
    key_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64)) * 100;

    todo!()
}

pub fn import_storage_kv_get_prev_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    suffix_ptr: i32,
    suffix_len: i32,
    key_ptr: i32,
    key_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64)) * 100;

    todo!()
}

pub fn import_storage_kv_get_next_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    suffix_ptr: i32,
    suffix_len: i32,
    key_ptr: i32,
    key_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64)) * 100;

    todo!()
}

pub fn import_storage_kv_put_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    key_ptr: i32,
    key_len: i32,
    val_ptr: i32,
    val_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64) + (val_len as u64)) * 1000;

    let (data, store) = env.data_and_store_mut();
    if data.readonly {
        return Err(RuntimeError::new("read_only"));
    }

    let memory = data.memory.as_ref().unwrap();

    let key = read_memory(memory, &store, key_ptr)?; // -> Vec<u8>
    let val = read_memory(memory, &store, val_ptr)?;

    data.writes.insert(key, val);

    Ok(cost.try_into().unwrap())
}

pub fn import_storage_kv_increment_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    key_ptr: i32,
    key_len: i32,
    val_ptr: i32,
    val_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64) + (val_len as u64)) * 1000;

    let (data, mut store) = env.data_and_store_mut();
    if data.readonly {
        return Err(RuntimeError::new("read_only"));
    }

    todo!()
}

pub fn import_storage_kv_delete_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    key_ptr: i32,
    key_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (key_len as u64)) * 1000;

    let (data, mut store) = env.data_and_store_mut();
    if data.readonly {
        return Err(RuntimeError::new("read_only"));
    }

    todo!()
}

pub fn import_storage_kv_clear_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    prefix_ptr: i32,
    prefix_len: i32,
) -> Result<i32, RuntimeError> {
    let cost = (48 + (prefix_len as u64)) * 1000;

    let (data, mut store) = env.data_and_store_mut();
    if data.readonly {
        return Err(RuntimeError::new("read_only"));
    }

    todo!()
}

pub fn import_call_4_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    module_ptr: i32,
    module_len: i32,
    function_ptr: i32,
    function_len: i32,
    arg_1_ptr: i32,
    arg_1_len: i32,
    arg_2_ptr: i32,
    arg_2_len: i32,
    arg_3_ptr: i32,
    arg_3_len: i32,
    arg_4_ptr: i32,
    arg_4_len: i32,
) -> Result<i32, RuntimeError> {
    let cost =
        (48 + (arg_1_len as u64) + (arg_2_len as u64) + (arg_3_len as u64) + (arg_4_len as u64))
            * 1000;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut module_buffer = vec![0u8; module_len as usize];
    let Ok(_) = view.read(module_ptr as u64, &mut module_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut function_buffer = vec![0u8; function_len as usize];
    let Ok(_) = view.read(function_ptr as u64, &mut function_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let mut arg_1_buffer = vec![0u8; arg_1_len as usize];
    let Ok(_) = view.read(arg_1_ptr as u64, &mut arg_1_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut arg_2_buffer = vec![0u8; arg_2_len as usize];
    let Ok(_) = view.read(arg_2_ptr as u64, &mut arg_2_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut arg_3_buffer = vec![0u8; arg_3_len as usize];
    let Ok(_) = view.read(arg_3_ptr as u64, &mut arg_3_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut arg_4_buffer = vec![0u8; arg_4_len as usize];
    let Ok(_) = view.read(arg_4_ptr as u64, &mut arg_4_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    let mut args = Vec::with_capacity(4);
    args.push(arg_1_buffer);
    args.push(arg_2_buffer);
    args.push(arg_3_buffer);
    args.push(arg_4_buffer);

    todo!()
    /*
    let (rx, request_id) = request_from_rust_call(
        data.rpc_pid.expect("should have an rpcid, now its fucked"),
        remaining_u64,
        module_buffer,
        function_buffer,
        args,
        data.attached_symbol.clone(),
        data.attached_amount.clone(),
    );

    match rx.recv_timeout(std::time::Duration::from_secs(6)) {
        Ok((error, logs, remaining_exec, result)) => {
            if error != b"ok" {
                return Err(RuntimeError::new("xcc_failed"));
            }

            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            write_i32(&view, 30_000, error.len() as i32)?;
            write_bin(&view, 30_004, &error)?;

            match result {
                Some(bytes) => {
                    write_i32(&view, 30_004 + (error.len() as u64), bytes.len() as i32)?;
                    write_bin(&view, 30_004 + (error.len() as u64) + 4, &bytes)?;
                }
                None => {
                    write_i32(&view, 30_004 + (error.len() as u64), 0)?;
                }
            }

            data.logs.extend(logs);
            set_remaining_points(&mut store, instance_arc.as_ref(), remaining_exec);

            Ok(30_000)
        }
        Err(_) => {
            data.attached_symbol = Vec::new();
            data.attached_amount = Vec::new();

            let mut map = REQ_REGISTRY_CALL.lock().unwrap();
            map.remove(&request_id);
            Err(RuntimeError::new("no_elixir_callback"))
        }
    }
    */
}

//AssemblyScript specific
pub fn abort_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    msg_ptr: i32,
    filename_ptr: i32,
    line: i32,
    column: i32,
) -> Result<(), RuntimeError> {
    let (data, store) = env.data_and_store_mut();
    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    //I kill thee
    let mut msg_size_bytes = [0u8; 4];
    let Ok(_) = view.read((msg_ptr as u64) - 4, &mut msg_size_bytes) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let msg_size: i32 = i32::from_le_bytes(msg_size_bytes);
    let mut msg_buff_utf16 = vec![0u8; msg_size as usize];
    let Ok(_) = view.read(msg_ptr as u64, &mut msg_buff_utf16) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let msg_buff_utf16_b4collect = msg_buff_utf16
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));
    let msg_buff_utf16_collected: Vec<u16> = msg_buff_utf16_b4collect.collect();

    let mut filename_size_bytes = [0u8; 4];
    let Ok(_) = view.read((filename_ptr as u64) - 4, &mut filename_size_bytes) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let filename_size: i32 = i32::from_le_bytes(filename_size_bytes);
    let mut filename_buff_utf16 = vec![0u8; filename_size as usize];
    let Ok(_) = view.read(filename_ptr as u64, &mut filename_buff_utf16) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let filename_buff_utf16_b4collect = filename_buff_utf16
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));
    let filename_buff_utf16_collected: Vec<u16> = filename_buff_utf16_b4collect.collect();

    let msg_utf8 = match String::from_utf16(&msg_buff_utf16_collected) {
        Ok(s) => s,
        Err(_) => {
            return Err(RuntimeError::new("invalid_memory"));
        }
    };
    let filename_utf8 = match String::from_utf16(&filename_buff_utf16_collected) {
        Ok(s) => s,
        Err(_) => {
            return Err(RuntimeError::new("invalid_memory"));
        }
    };

    let formatted = format!("{} | {} {} {}", msg_utf8, filename_utf8, line, column);
    data.return_value = Some(formatted.into_bytes());

    //println!("{} {} {} {}", msg_utf8, filename_utf8, line, column);

    Err(RuntimeError::new("abort"))
}

pub fn import_log_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    ptr: i32,
    len: i32,
) -> Result<(), RuntimeError> {
    let cost = (len as u64) * 1000;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut buffer = vec![0u8; len as usize];
    match view.read(ptr as u64, &mut buffer) {
        Ok(_) => {
            //print!("log>> {} \n", String::from_utf8_lossy(&buffer));
            data.logs.push(buffer);
            Ok(())
        }
        Err(read_err) => Err(RuntimeError::new("invalid_memory")),
    }
}

pub fn import_attach_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    symbol_ptr: i32,
    symbol_len: i32,
    amount_ptr: i32,
    amount_len: i32,
) -> Result<(), RuntimeError> {
    let cost = 1000 + (symbol_len as u64) * 100 + (amount_len as u64) * 100;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut symbol_buffer = vec![0u8; symbol_len as usize];
    let Ok(_) = view.read(symbol_ptr as u64, &mut symbol_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let mut amount_buffer = vec![0u8; amount_len as usize];
    let Ok(_) = view.read(amount_ptr as u64, &mut amount_buffer) else {
        return Err(RuntimeError::new("invalid_memory"));
    };

    data.attached_symbol = symbol_buffer;
    data.attached_amount = amount_buffer;
    Ok(())
}

pub fn import_return_value_implementation(
    mut env: FunctionEnvMut<HostEnv>,
    ptr: i32,
    len: i32,
) -> Result<(), RuntimeError> {
    let cost = (len as u64) * 1000;

    let (data, mut store) = env.data_and_store_mut();

    let instance_arc = data
        .instance
        .as_ref()
        .ok_or_else(|| RuntimeError::new("invalid_instance"))?;
    let remaining_u64 = charge_points(&mut store, instance_arc.as_ref(), cost)?;

    let Some(memory) = &data.memory else {
        return Err(RuntimeError::new("invalid_memory"));
    };
    let view: MemoryView = memory.view(&store);

    let mut buffer = vec![0u8; len as usize];
    match view.read(ptr as u64, &mut buffer) {
        Ok(_) => {
            //println!("return was {}", String::from_utf8_lossy(&buffer));
            data.return_value = Some(buffer);
            Err(RuntimeError::new("return_value"))
        }
        Err(read_err) => Err(RuntimeError::new("invalid_memory")),
    }
}

#[inline]
fn charge_points<S>(store: &mut S, instance: &Instance, cost: u64) -> Result<u64, RuntimeError>
where
    S: AsStoreMut,
{
    let remaining = match get_remaining_points(store, instance) {
        MeteringPoints::Remaining(v) => v,
        MeteringPoints::Exhausted => 0,
    };

    if cost > remaining {
        return Err(RuntimeError::new("unreachable"));
    }
    let new_remaining = remaining - cost;
    set_remaining_points(store, instance, new_remaining);
    Ok(new_remaining)
}

fn build_prefixed_key(
    view: &MemoryView,
    prefix: &[u8],
    ptr: i32,
    len: i32,
) -> Result<Vec<u8>, RuntimeError> {
    const CONTRACT: &[u8] = b"c:";

    let mut body = vec![0u8; len as usize];
    view.read(ptr as u64, &mut body)
        .map_err(|_| RuntimeError::new("invalid_memory"))?;

    let mut out = Vec::with_capacity(CONTRACT.len() + prefix.len() + 1 + body.len());
    out.extend_from_slice(CONTRACT);
    out.extend_from_slice(prefix);
    out.push(b':');
    out.extend_from_slice(&body);
    Ok(out)
}

#[inline]
fn write_i32(view: &MemoryView, offset: u64, value: i32) -> Result<(), RuntimeError> {
    view.write(offset, &value.to_le_bytes())
        .map_err(|_| RuntimeError::new("invalid_memory"))
}

#[inline]
fn write_bin(view: &MemoryView, offset: u64, slice: &[u8]) -> Result<(), RuntimeError> {
    view.write(offset, slice)
        .map_err(|_| RuntimeError::new("invalid_memory"))
}

#[inline]
fn read_memory(memory: &Memory, store: &StoreMut, ptr: i32) -> Result<Vec<u8>, RuntimeError> {
    let view = memory.view(store);

    let mut len_bytes = [0u8; 4];
    view.read(ptr as u64, &mut len_bytes)
        .map_err(|_| RuntimeError::new("invalid_memory"))?;
    let len = i32::from_le_bytes(len_bytes) as usize;

    let mut buffer = vec![0u8; len];
    view.read((ptr as u64) + 4, &mut buffer)
        .map_err(|_| RuntimeError::new("invalid_memory"))?;

    Ok(buffer)
}

fn cost_function(operator: &Operator) -> u64 {
    10
    /*match operator {
        Operator::Loop { .. }
        | Operator::Block { .. }
        | Operator::If { .. }
        | Operator::Else { .. }
        | Operator::End { .. }
        | Operator::Br { .. }
        | Operator::BrIf { .. }
        | Operator::Return { .. }
        | Operator::Unreachable { .. } => 1,

        Operator::Call { .. } | Operator::CallIndirect { .. } => 5,

        Operator::I32Load { .. }
        | Operator::I64Load { .. }
        | Operator::F32Load { .. }
        | Operator::F64Load { .. }
        | Operator::I32Store { .. }
        | Operator::I64Store { .. }
        | Operator::F32Store { .. }
        | Operator::F64Store { .. } => 3,

        _ => 2,
    }*/
}

/*
fn get_or_compile_module(wasm_bytes: &[u8]) -> Result<(Arc<Engine>, Arc<Module>), rustler::Error> {
    let cache_mutex = MODULE_CACHE.get_or_init(|| {
        Mutex::new(HashMap::new())
    });

    let mut hasher = Sha256::new();
    hasher.update(wasm_bytes);
    let hash = hasher.finalize().into();
    {
        let cache = cache_mutex.lock().unwrap();
        if let Some((cached_engine, cached_module)) = cache.get(&hash) {
            return Ok((Arc::clone(cached_engine), Arc::clone(cached_module)));
        }
    }

    let metering = Arc::new(Metering::new(10_000_000, cost_function));
    let mut compiler = Singlepass::default();
    compiler.canonicalize_nans(true);

    use wasmer::CompilerConfig;
    compiler.push_middleware(metering);

    let mut features = Features::new();
    //features.bulk_memory(false); #required for modern compilers to WASM
    features.threads(false);
    features.reference_types(false);
    features.simd(false);
    features.multi_value(false);
    features.tail_call(false);
    features.module_linking(false);
    features.multi_memory(false);
    features.memory64(false);

    //let engine = EngineBuilder::new(compiler).set_features(Some(features));
    //let mut store = Store::new(engine);

    let engine = Arc::new(EngineBuilder::new(compiler).set_features(Some(features)));
    let store = Store::new(Arc::clone(&engine));

    let module = Module::new(&store, &wasm_bytes).map_err(|err| rustler::Error::Term(Box::new(err.to_string())))?;

    let arc_module = Arc::new(module);
    {
        let mut cache = cache_mutex.lock().unwrap();
        cache.insert(hash, (Arc::clone(&engine), Arc::clone(&arc_module)));
    }

    Ok((engine, arc_module))
}
*/

#[inline]
pub fn write_to_memory(
    memory: &Memory,
    store: &mut Store,
    offset: u64,
    data: &[u8],
) -> Result<(), rustler::Error> {
    let view = memory.view(store);
    let data_len_bytes = (data.len() as i32).to_le_bytes();

    view.write(offset, &data_len_bytes)
        .and_then(|_| view.write(offset + 4, data))
        .map_err(|err| rustler::Error::Term(Box::new(err.to_string())))
}

use wasmer::CompilerConfig;

#[derive(Debug, Clone)]
pub struct RuntimeEnv {
    pub seed: Vec<u8>,
    pub entry_signer: Vec<u8>,
    pub entry_prev_hash: Vec<u8>,
    pub entry_vr: Vec<u8>,
    pub entry_dr: Vec<u8>,
    pub tx_signer: Vec<u8>,
    pub account_current: Vec<u8>,
    pub account_caller: Vec<u8>,
    pub account_origin: Vec<u8>,
    pub attached_symbol: Vec<u8>,
    pub attached_amount: Vec<u8>,

    pub readonly: bool,
    pub call_exec_points_remaining: u64,

    pub entry_slot: i64,
    pub entry_prev_slot: i64,
    pub entry_height: i64,
    pub entry_epoch: i64,
    pub tx_nonce: i64,

    pub seedf64: f64,
}

#[derive(Debug, Clone)]
pub enum WasmArg {
    I64(i64),
    Bytes(Vec<u8>),
}

pub fn run_wasm(
    env: &RuntimeEnv,
    wasm_bytes: &[u8],
    function_name: &str,
    function_args: &[WasmArg],
) -> Result<(), Error> {
    // ---------------------------------------------------------------------
    // 1. metering / compiler setup
    // ---------------------------------------------------------------------
    let exec_points = env.call_exec_points_remaining;
    let metering = Arc::new(Metering::new(exec_points, cost_function));

    let mut compiler = Singlepass::default();
    compiler.canonicalize_nans(true);
    compiler.push_middleware(metering);

    let mut features = Features::new();
    features.threads(false);
    features.reference_types(false);
    features.simd(false);
    features.multi_value(false);
    features.tail_call(false);
    features.module_linking(false);
    features.multi_memory(false);
    features.memory64(false);

    let engine = wasmer::EngineBuilder::new(compiler).set_features(Some(features));
    let mut store = Store::new(engine);

    // ---------------------------------------------------------------------
    // 2. compile module & create linear memory
    // ---------------------------------------------------------------------
    let module =
        Module::new(&store, wasm_bytes).map_err(|e| Error::Term(Box::new(e.to_string())))?;

    let memory = Memory::new(&mut store, MemoryType::new(Pages(8), None, false))
        .map_err(|e| Error::Term(Box::new(e.to_string())))?;

    // ---------------------------------------------------------------------
    // 3. write predefined keys/values into guest memory
    // ---------------------------------------------------------------------
    let keys: &[(&[u8], u64)] = &[
        (&env.seed, 10_000),
        (&env.entry_signer, 10_100),
        (&env.entry_prev_hash, 10_200),
        (&env.entry_vr, 10_300),
        (&env.entry_dr, 10_400),
        (&env.tx_signer, 11_000),
        (&env.account_current, 12_000),
        (&env.account_caller, 13_000),
        (&env.account_origin, 14_000),
        (&env.attached_symbol, 15_000),
        (&env.attached_amount, 16_000),
    ];

    for (bytes, offset) in keys {
        write_to_memory(&memory, &mut store, *offset, bytes)?;
    }

    // ---------------------------------------------------------------------
    // 4. process function arguments
    // ---------------------------------------------------------------------
    let mut offset: u64 = 20_000;
    let mut wasm_args = Vec::with_capacity(function_args.len());

    for arg in function_args {
        match arg {
            WasmArg::I64(i) => wasm_args.push(Value::I64(*i)),
            WasmArg::Bytes(b) => {
                write_to_memory(&memory, &mut store, offset, b)?;
                wasm_args.push(Value::I32(offset as i32));
                offset += 4 + b.len() as u64;
            }
        }
    }

    // ---------------------------------------------------------------------
    // 5. build host-environment + import object
    // ---------------------------------------------------------------------
    let mut host_env = FunctionEnv::new(
        &mut store,
        HostEnv {
            memory: None,
            error: None,
            return_value: None,
            logs: vec![],
            readonly: env.readonly,
            rpc_pid: None,
            current_account: env.account_current.clone(),
            instance: None,
            attached_symbol: Vec::new(),
            attached_amount: Vec::new(),
            writes: HashMap::new(),
        },
    );

    let import_object = imports! {
        "env" => {
            "memory"                => memory,
            "seed_ptr"              => Global::new(&mut store, Value::I32(10_000)),
            "entry_signer_ptr"      => Global::new(&mut store, Value::I32(10_100)),
            "entry_prev_hash_ptr"   => Global::new(&mut store, Value::I32(10_200)),
            "entry_slot"            => Global::new(&mut store, Value::I64(env.entry_slot)),
            "entry_prev_slot"       => Global::new(&mut store, Value::I64(env.entry_prev_slot)),
            "entry_height"          => Global::new(&mut store, Value::I64(env.entry_height)),
            "entry_epoch"           => Global::new(&mut store, Value::I64(env.entry_epoch)),
            "entry_vr_ptr"          => Global::new(&mut store, Value::I32(10_300)),
            "entry_dr_ptr"          => Global::new(&mut store, Value::I32(10_400)),
            "tx_signer_ptr"         => Global::new(&mut store, Value::I32(11_000)),
            "tx_nonce"              => Global::new(&mut store, Value::I64(env.tx_nonce)),
            "account_current_ptr"   => Global::new(&mut store, Value::I32(12_000)),
            "account_caller_ptr"    => Global::new(&mut store, Value::I32(13_000)),
            "account_origin_ptr"    => Global::new(&mut store, Value::I32(14_000)),
            "attached_symbol_ptr"   => Global::new(&mut store, Value::I32(15_000)),
            "attached_amount_ptr"   => Global::new(&mut store, Value::I32(16_000)),

            // ---- host functions (unchanged, shown for context) ----
            "import_attach"         => Function::new_typed_with_env(&mut store, &host_env, import_attach_implementation),
            "import_log"            => Function::new_typed_with_env(&mut store, &host_env, import_log_implementation),
            "import_return_value"   => Function::new_typed_with_env(&mut store, &host_env, import_return_value_implementation),
            "import_call_0"         => Function::new_typed_with_env(&mut store, &host_env, import_call_4_implementation),
            "import_call_1"         => Function::new_typed_with_env(&mut store, &host_env, import_call_4_implementation),
            "import_call_2"         => Function::new_typed_with_env(&mut store, &host_env, import_call_4_implementation),
            "import_call_3"         => Function::new_typed_with_env(&mut store, &host_env, import_call_4_implementation),
            "import_call_4"         => Function::new_typed_with_env(&mut store, &host_env, import_call_4_implementation),
            "import_kv_put"         => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_put_implementation),
            "import_kv_increment"   => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_increment_implementation),
            "import_kv_delete"      => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_delete_implementation),
            "import_kv_clear"       => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_clear_implementation),
            "import_kv_get"         => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_get_implementation),
            "import_kv_exists"      => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_exists_implementation),
            "import_kv_get_prev"    => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_get_prev_implementation),
            "import_kv_get_next"    => Function::new_typed_with_env(&mut store, &host_env, import_storage_kv_get_next_implementation),
            "abort"                => Function::new_typed_with_env(&mut store, &host_env, abort_implementation),
            "seed"                  => Global::new(&mut store, Value::F64(env.seedf64)),
        }
    };

    // ---------------------------------------------------------------------
    // 6. instantiate & invoke
    // ---------------------------------------------------------------------
    let instance = Instance::new(&mut store, &module, &import_object)
        .map_err(|e| Error::Term(Box::new(e.to_string())))?;
    host_env.as_mut(&mut store).instance = Some(Arc::new(instance.clone()));

    let instance_memory = instance
        .exports
        .get_memory("memory")
        .map_err(|e| Error::Term(Box::new(format!("Failed to get memory export: {}", e))))?;
    host_env.as_mut(&mut store).memory = Some(instance_memory.clone());

    let entry_to_call = instance
        .exports
        .get_function(function_name)
        .map_err(|e| Error::Term(Box::new(e.to_string())))?;

    let call_result = entry_to_call.call(&mut store, &wasm_args);

    // meter
    let remaining_u64 = match get_remaining_points(&mut store, &instance) {
        MeteringPoints::Remaining(v) => v,
        MeteringPoints::Exhausted => 0,
    };

    // you can still plumb `logs`, `return_value`, etc. exactly as before.
    // For this minimal port we keep the same external contract:
    let _ = call_result; // ignored on purpose
    let _ = remaining_u64; // ditto

    println!("result? {:?}", call_result);

    Ok(())
}
