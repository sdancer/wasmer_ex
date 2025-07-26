use std::collections::HashMap;
use wasmer::wat2wasm;

use std::sync::{Arc, RwLock};
use wasmer_ex::wasm::{run_wasm, RuntimeEnv, WasmArg};

#[test]
fn test_run_wasm_minimal() {
    // --- compile a super-simple module ----
    let wat = r#"
        (module
            (memory (export "memory") 1)
            (func (export "add_one") (param i64) (result i64)
                local.get 0
                i64.const 1
                i64.add)
        )
    "#;

    let wasm_bytes = wat2wasm(wat.as_bytes()).expect("Failed to compile WAT");

    // --- construct a bare-bones environment (every vec empty) ----
    let env = RuntimeEnv {
        seed: vec![],
        entry_signer: vec![],
        entry_prev_hash: vec![],
        entry_vr: vec![],
        entry_dr: vec![],
        tx_signer: vec![],
        account_current: vec![],
        account_caller: vec![],
        account_origin: vec![],
        attached_symbol: vec![],
        attached_amount: vec![],
        readonly: false,
        call_exec_points_remaining: 10_000_000,
        entry_slot: 0,
        entry_prev_slot: 0,
        entry_height: 0,
        entry_epoch: 0,
        tx_nonce: 0,
        seedf64: 0.0,
    };

    // --- one 64-bit argument ----
    let args = vec![WasmArg::I64(42)];

    // --- invoke ----
    let write_layer: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>> = Arc::new(RwLock::new(HashMap::new()));
    let res = run_wasm(&env, &wasm_bytes, "add_one", &args, write_layer);
    assert!(res.is_ok());
}
