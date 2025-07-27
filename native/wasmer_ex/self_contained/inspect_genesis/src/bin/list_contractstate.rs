//! Print every key in the `contractstate` column-family together with the
//! length (bytes) of its corresponding value.
//
//  Build / run:
//      cargo run --release --bin list_contractstate -- <path-to-rocksdb>

use rocksdb::{Options, DB, IteratorMode};
use std::{env, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    // ---------------------------- CLI
    let db_path = env::args()
        .nth(1)
        .expect("Usage: list_contractstate <path-to-rocksdb>");
    println!("Opening RocksDB at {db_path}");

    // ---------------------------- open DB (read-only)
    let opts = Options::default();
    let cf_names = DB::list_cf(&opts, &db_path)?;
    let db = DB::open_cf_for_read_only(&opts, &db_path, &cf_names, false)?;

    // ---------------------------- locate the CF
    let cf = db
        .cf_handle("contractstate")
        .expect("column-family `contractstate` not found");

    // ---------------------------- iterate
    for (key, value) in db.iterator_cf(&cf, IteratorMode::Start) {
        println!("{:02X?}  {}", key, value.len());
    }

    Ok(())
}
