use rocksdb::{Options, DB, IteratorMode};
use std::{env, error::Error, str};

/// ASCII prefix and suffix we’re interested in
const PREFIX: &[u8] = b"bic:coin:balance:";
const SUFFIX: &[u8] = b":AMA";

/// Convert raw bytes → continuous upper-case hex string
fn hexdump(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02X}", b)).collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    // ---------------- CLI
    let db_path = env::args()
        .nth(1)
        .expect("Usage: list_contractstate <path-to-rocksdb>");
    println!("Opening RocksDB at {db_path}");

    // ---------------- open DB (read-only)
    let opts     = Options::default();
    let cf_names = DB::list_cf(&opts, &db_path)?;
    let db       = DB::open_cf_for_read_only(&opts, &db_path, &cf_names, false)?;

    // ---------------- locate CF
    let cf = db
        .cf_handle("contractstate")
        .expect("column-family `contractstate` not found");

    // ---------------- collect matching entries
    let mut rows: Vec<(u128, Vec<u8>)> = Vec::new();

    for entry in db.iterator_cf(&cf, IteratorMode::Start) {
        let (key, value) = entry?;                      // propagate RocksDB errors

        // prefix … suffix
        if key.starts_with(PREFIX) && key.ends_with(SUFFIX) {
            // ---- strip prefix & suffix (no assumption on middle length)
            let core = &key[PREFIX.len() .. key.len() - SUFFIX.len()];

            // ---- parse amount
            if let Ok(text) = str::from_utf8(&value) {
                if let Ok(n) = text.parse::<u128>() {
                    rows.push((n, core.to_vec()));     // keep ONLY the stripped key
                }
            }
        }
    }

    // ---------------- sort biggest → smallest
    rows.sort_by(|a, b| b.0.cmp(&a.0));

    // ---------------- print
    for (n, core_key) in rows {
        let amount = n as f64 / 1e9;                   // convert to “coins” (10^9 divider)
        println!("{} => {}", hexdump(&core_key), amount);
    }

    Ok(())
}
