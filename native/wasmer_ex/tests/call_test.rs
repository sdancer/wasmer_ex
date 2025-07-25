
#[cfg(test)]
mod tests {
    use crate::atoms;
    use rustler::{OwnedEnv, Encoder, Term};
    use rustler::types::Binary;
    use std::collections::HashMap;
    use wasmer::wat2wasm;

    #[test]
    fn test_run_wasm_minimal() {
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
    }
}

