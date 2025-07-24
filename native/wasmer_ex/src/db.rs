#![allow(unused_variables)]
#![allow(dead_code)]

use rustler::types::atom;
use rustler::{
    Atom, Binary, Encoder, Env, Error, ListIterator, NifResult, OwnedBinary, ResourceArc, Term,
};
use std::sync::Mutex;

use rocksdb::BoundColumnFamily;
use rocksdb::DBCommon;
use rocksdb::Transaction;
use rocksdb::{
    ColumnFamily, ColumnFamilyDescriptor, DBAccess, DBIterator, Direction, IteratorMode,
    MultiThreaded, OptimisticTransactionDB, OptimisticTransactionOptions, Options, ReadOptions,
    WriteOptions,
};
use std::path::Path;
// --- NIF Resources ---
use std::sync::Arc;

/// A resource holding a thread-safe reference to an open OptimisticTransactionDB.
pub struct DbResource {
    pub db: OptimisticTransactionDB<MultiThreaded>,
}

/// A resource holding a transaction.
/// A transaction's lifetime is tied to its parent DB. By holding a `ResourceArc<DbResource>`,
/// we ensure the DB is not garbage collected by the BEAM while a transaction is still live.
/// The transaction itself is wrapped in a `Mutex<Option<...>>` to allow it to be
/// consumed by `commit` or `rollback` operations.
pub struct TransactionResource {
    // A transaction's lifetime is tied to its DB.
    // By holding a `ResourceArc<DbResource>`, we guarantee the DB is not
    // garbage collected while this transaction resource exists.
    _db_holder: ResourceArc<DbResource>,

    // The transaction object itself. We wrap it in a Mutex and Option
    // so we can "take" it when we commit or rollback, preventing reuse.
    // The lifetime is 'static because we've guaranteed safety with `_db_holder`.
    txn: Mutex<Option<Transaction<'static, OptimisticTransactionDB<MultiThreaded>>>>,
}

struct IteratorResource {
    iter: Mutex<DBIterator<'static>>,
    _db_holder: ResourceArc<DbResource>,
}

pub fn load(env: Env, _: Term) -> bool {
    // Use the `resource!` macro to implement the `Resource` trait for your structs.
    rustler::resource!(DbResource, env);
    rustler::resource!(TransactionResource, env);
    rustler::resource!(IteratorResource, env); // Don't forget this one too!
    true
}

// --- Atoms ---

mod atoms {
    rustler::atoms! {
        // General atoms
        ok,
        error,
        nil,
        finished,

        // Option key atoms
        create_if_missing,
        create_missing_column_families,
        target_file_size_base,
        target_file_size_multiplier,

        // Iterator control atoms
        iterator_mode,
        start,
        end,
        from,
        forward,
        reverse,
        next,
    }
}

/// Parses a keyword list of options from Elixir into a `rocksdb::Options` struct.
fn parse_db_options(opts_term: Term) -> NifResult<Options> {
    let mut opts = Options::default();
    if !opts_term.is_list() {
        return Ok(opts);
    }
    let opts_iter: ListIterator = opts_term.decode()?;

    for opt_term in opts_iter {
        let (key, value): (atom::Atom, Term) = opt_term.decode()?;

        if key == atoms::create_if_missing() {
            opts.create_if_missing(value.decode()?);
        } else if key == atoms::create_missing_column_families() {
            opts.create_missing_column_families(value.decode()?);
        } else if key == atoms::target_file_size_base() {
            opts.set_target_file_size_base(value.decode()?);
        } else if key == atoms::target_file_size_multiplier() {
            opts.set_target_file_size_multiplier(value.decode()?);
        }
        // Add more supported DB options here...
    }
    Ok(opts)
}

fn to_nif_err(err: rocksdb::Error) -> Error {
    Error::Term(Box::new(err.to_string()))
}

/// A helper to safely get a `ColumnFamily` handle from a `DbResource`.
fn get_cf_handle<'a>(
    db_res: &'a DbResource,
    cf_name: &'a str,
) -> NifResult<Arc<BoundColumnFamily<'a>>> {
    db_res
        .db
        .cf_handle(cf_name)
        .ok_or_else(|| Error::Term(Box::new(format!("Column family not found: {}", cf_name))))
}

/// Opens an `OptimisticTransactionDB` with specified column families and options.
///
/// Returns `{:ok, db_resource, list_of_cf_names}`.
#[rustler::nif]
fn open_optimistic_transaction_db<'a>(
    env: Env<'a>,
    path: String,
    db_opts_term: Term<'a>,
    cf_descriptors_term: Term<'a>,
) -> NifResult<Term<'a>> {
    let db_opts = parse_db_options(db_opts_term)?;

    let cf_list_iter: ListIterator = cf_descriptors_term.decode()?;
    let mut cf_descriptors = Vec::new();
    let mut cf_names = Vec::new();

    for cf_desc_term in cf_list_iter {
        let (cf_name, cf_opts_term) = cf_desc_term.decode::<(String, Term)>()?;
        let cf_opts = parse_db_options(cf_opts_term)?;
        cf_names.push(cf_name.clone());
        cf_descriptors.push(ColumnFamilyDescriptor::new(cf_name, cf_opts));
    }

    let path = Path::new(&path);
    match OptimisticTransactionDB::open_cf_descriptors(&db_opts, path, cf_descriptors) {
        Ok(db) => {
            let resource = ResourceArc::new(DbResource { db });
            let cf_names_term = cf_names.encode(env);
            Ok((atoms::ok(), resource, cf_names_term).encode(env))
        }
        Err(e) => Err(to_nif_err(e)),
    }
}

#[rustler::nif]
fn put_cf(
    db_res: ResourceArc<DbResource>,
    cf_name: String,
    key: Binary,
    value: Binary,
) -> NifResult<atom::Atom> {
    // `get_cf_handle` now returns an `Arc<ColumnFamily>`.
    let cf_handle: Arc<BoundColumnFamily> = get_cf_handle(&db_res, &cf_name)?;

    // When we pass `cf_handle` to `put_cf`, the compiler automatically
    // dereferences it from `Arc<ColumnFamily>` to the `&ColumnFamily` that the method expects.
    // No change is needed in this line!
    db_res
        .db
        .put_cf(&cf_handle, key.as_slice(), value.as_slice())
        .map(|_| atoms::ok())
        .map_err(to_nif_err)
}

#[rustler::nif]
fn get_cf<'a>(
    env: Env<'a>,
    db_res: ResourceArc<DbResource>,
    cf_name: String,
    key: Binary,
) -> NifResult<Term<'a>> {
    let cf_handle = get_cf_handle(&db_res, &cf_name)?; // This is now an Arc
    match db_res.db.get_cf(&cf_handle, key.as_slice()) {
        // Pass it as a reference
        Ok(Some(value)) => {
            let mut bin = OwnedBinary::new(value.len()).unwrap();
            bin.as_mut_slice().copy_from_slice(&value);
            Ok((atoms::ok(), bin.release(env)).encode(env))
        }
        Ok(None) => Ok((atoms::ok(), atoms::nil()).encode(env)),
        Err(e) => Err(to_nif_err(e)),
    }
}

#[rustler::nif]
fn delete_cf(
    db_res: ResourceArc<DbResource>,
    cf_name: String,
    key: Binary,
) -> NifResult<atom::Atom> {
    let cf = get_cf_handle(&db_res, &cf_name)?;
    db_res
        .db
        .delete_cf(&cf, key.as_slice())
        .map(|_| atoms::ok())
        .map_err(to_nif_err)
}

#[rustler::nif]
fn begin_transaction<'a>(env: Env<'a>, db_res: ResourceArc<DbResource>) -> NifResult<Term<'a>> {
    let db = &db_res.db;
    let write_opts = WriteOptions::default();
    let tx_opts = OptimisticTransactionOptions::default();

    // The transaction's lifetime is bound to `db`. We use `unsafe transmute` to make it
    // `'static` because `TransactionResource` holds a `ResourceArc` to the DB,
    // guaranteeing the DB lives as long as the transaction resource.
    let txn = db.transaction_opt(&write_opts, &tx_opts);
    let static_txn: Transaction<'static, _> = unsafe { std::mem::transmute(txn) };

    let txn_res = ResourceArc::new(TransactionResource {
        txn: Mutex::new(Some(static_txn)),
        _db_holder: db_res.clone(),
    });

    Ok((atoms::ok(), txn_res).encode(env))
}

/// Commits a transaction. The transaction resource cannot be used after this call.
#[rustler::nif]
fn commit_transaction(txn_res: ResourceArc<TransactionResource>) -> NifResult<atom::Atom> {
    let mut guard = txn_res.txn.lock().unwrap();
    if let Some(txn) = guard.take() {
        txn.commit().map(|_| atoms::ok()).map_err(to_nif_err)
    } else {
        Err(Error::Atom("transaction_already_consumed"))
    }
}

/// Rolls back a transaction. The transaction resource cannot be used after this call.
#[rustler::nif]
fn rollback_transaction(txn_res: ResourceArc<TransactionResource>) -> NifResult<atom::Atom> {
    let mut guard = txn_res.txn.lock().unwrap();
    if let Some(txn) = guard.take() {
        txn.rollback().map(|_| atoms::ok()).map_err(to_nif_err)
    } else {
        Err(Error::Atom("transaction_already_consumed"))
    }
}

/// Puts a key-value pair into a column family within a transaction.
#[rustler::nif]
fn transaction_put_cf(
    txn_res: ResourceArc<TransactionResource>,
    cf_name: String,
    key: Binary,
    value: Binary,
) -> NifResult<atom::Atom> {
    let mut guard = txn_res.txn.lock().unwrap();
    if let Some(txn) = guard.as_mut() {
        let cf = get_cf_handle(&txn_res._db_holder, &cf_name)?;
        txn.put_cf(&cf, key.as_slice(), value.as_slice())
            .map(|_| atoms::ok())
            .map_err(to_nif_err)
    } else {
        Err(Error::Atom("transaction_already_consumed"))
    }
}

/// Gets a value by key from a column family within a transaction.
#[rustler::nif]
fn transaction_get_cf<'a>(
    env: Env<'a>,
    txn_res: ResourceArc<TransactionResource>,
    cf_name: String,
    key: Binary,
) -> NifResult<Term<'a>> {
    let mut guard = txn_res.txn.lock().unwrap();
    if let Some(txn) = guard.as_mut() {
        let cf = get_cf_handle(&txn_res._db_holder, &cf_name)?;
        match txn.get_cf(&cf, key.as_slice()) {
            Ok(Some(value)) => {
                let mut bin = OwnedBinary::new(value.len()).unwrap();
                bin.as_mut_slice().copy_from_slice(&value);
                Ok((atoms::ok(), bin.release(env)).encode(env))
            }
            Ok(None) => Ok((atoms::ok(), atoms::nil()).encode(env)),
            Err(e) => Err(to_nif_err(e)),
        }
    } else {
        Err(Error::Atom("transaction_already_consumed"))
    }
}

/// Creates a new iterator over a column family.
#[rustler::nif]
fn iterator_cf<'a>(
    env: Env<'a>,
    db_res: ResourceArc<DbResource>,
    cf_name: String,
) -> NifResult<Term<'a>> {
    let cf = get_cf_handle(&db_res, &cf_name)?;
    let iter = db_res.db.iterator_cf(&cf, IteratorMode::Start);
    let static_iter: DBIterator = unsafe { std::mem::transmute(iter) };

    let iter_res = ResourceArc::new(IteratorResource {
        iter: Mutex::new(static_iter),
        _db_holder: db_res.clone(),
    });

    Ok((atoms::ok(), iter_res).encode(env))
}

fn vec_to_binary<'a>(env: Env<'a>, data: Vec<u8>) -> NifResult<Term<'a>> {
    let mut binary = OwnedBinary::new(data.len())
        .ok_or_else(|| Error::Term(Box::new("Failed to allocate binary")))?;
    binary.as_mut_slice().copy_from_slice(&data);
    Ok(binary.release(env).encode(env))
}

/// Moves the iterator to the next key-value pair.
///
/// Returns `{:ok, {key, value}}` or `:finished`.
#[rustler::nif]
fn iterator_next<'a>(env: Env<'a>, iter_res: ResourceArc<IteratorResource>) -> NifResult<Term<'a>> {
    let mut guard = iter_res.iter.lock().unwrap();
    match guard.next() {
        Some(Ok((key, value))) => {
            let key_bin = vec_to_binary(env, key.to_vec())?;
            let val_bin = vec_to_binary(env, value.to_vec())?;
            Ok((key_bin, val_bin).encode(env))
        }
        Some(Err(e)) => Err(to_nif_err(e)),
        None => Ok(atoms::finished().encode(env)),
    }
}

//rustler::init! {
//    "Elixir.RustlerRocksDB",
//    load = load
//}
