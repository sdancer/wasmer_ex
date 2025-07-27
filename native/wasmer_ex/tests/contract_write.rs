#[test]
fn test_run_wasm_kv_put() {
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    use wasmer::wat2wasm;
    use wasmer_ex::wasm::{run_wasm, Layer, RuntimeEnv, WasmArg};

    // Minimal WASM that calls the imported `import_kv_put` function
    let wat = r#"
        (module
            (import "env" "import_kv_put" (func $kv_put (param i32 i32 i32 i32) (result i32)))
            (memory (export "memory") 1)

            (data (i32.const 1000) "\04\00\00\00test")   ;; key = "test", offset 1000
            (data (i32.const 2000) "\05\00\00\00value")  ;; val = "value", offset 2000

            (func (export "main")
                i32.const 1000  ;; key_ptr
                i32.const 4     ;; key_len
                i32.const 2000  ;; val_ptr
                i32.const 5     ;; val_len
                call $kv_put
                drop
            )
        )
    "#;

    let wasm_bytes = wat2wasm(wat.as_bytes()).expect("Failed to compile WAT");

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

    let args = vec![]; // `main` takes no parameters

    let writes: Arc<RwLock<Layer>> = Arc::new(RwLock::new(HashMap::new()));
    let result = run_wasm(&env, &wasm_bytes, "main", &args, Arc::clone(&writes));

    assert!(result.is_ok());

    let map = writes.read().unwrap();
    let rv = map.get(b"test".as_ref()).unwrap().clone();
    assert_eq!(rv, Some(b"value".to_vec()));
}
