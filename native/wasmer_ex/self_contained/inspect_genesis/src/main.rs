use rocksdb::{Options, DB};
use eetf::Term;
use std::{env, error::Error};
use std::io::Cursor;

/// 32‑byte hash of the genesis entry (copy‑paste from your note)
const GENESIS_HASH: [u8; 32] = [
    250, 154, 199, 170, 114, 250, 155,  84,
      2, 215,  37, 236, 138,  98,  19,  87,
     19, 163,  21, 138, 131, 205, 205, 189,
    176, 217,   5, 112, 225,  13,  15, 217,
];

fn main() -> Result<(), Box<dyn Error>> {
    // ------------------------------------------------------------------ CLI
    let db_path = env::args()
        .nth(1)
        .expect("Usage: inspect_genesis <path‑to‑rocksdb>");
    println!("Opening RocksDB at {db_path}");

    // ------------------------------------------------------------------ DB
    let opts = Options::default();
    // We only need the default CF, but open all in case others exist
    let cf_names = DB::list_cf(&opts, &db_path)?;
    let db       = DB::open_cf_for_read_only(&opts, &db_path, &cf_names, /*error_if_log_file_exists*/ false)?;

    let default_cf = db
        .cf_handle("default")
        .expect("default column family missing");

    // ------------------------------------------------------------------ lookup
    match db.get_cf(&default_cf, GENESIS_HASH)? {
        Some(val) => {
            // ----------------------------------------------------------- decode
            let term = Term::decode(Cursor::new(&val))
                .map_err(|e| format!("ETF decode failed: {e}"))?;
            println!("\n=== Genesis entry decoded ===\n{:#?}", term);

            // (Optional) pull out the header sub‑map for quick inspection
            if let Term::Map(map) = &term {
                let header_key = Term::Atom("header".into());
                if let Some(header_term) = map.map.get(&header_key) {
                    println!("\nHeader field:\n{:#?}", header_term);
                }
            }
        }
        None => eprintln!("Genesis hash not found in this DB"),
    }

    Ok(())
}
