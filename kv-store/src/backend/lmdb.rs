use crate::{CursorOp, Error, KvDatabase, KvHandle, RoCursor, RoTx, RwCursor, RwTx, ToCursorOp};
use lmdb::EnvironmentFlags;
use lmdb_sys::{MDB_FIRST, MDB_GET_CURRENT, MDB_LAST, MDB_NEXT, MDB_PREV};
use std::{marker::PhantomData, path::Path, sync::Arc};

use crate::{Db, RawRead, RawWrite, Tx};

/// Converts LMDB-specific errors (`lmdb::Error`) into the database interface `Error` type.
///
/// This implementation maps LMDB errors to a meaningful representation in the library context,
/// allowing better handling of database errors.
///
/// ### Notes:
/// - The `NotFound` error is deliberately not converted to the custom `Error` type because it is
///   represented by the `Option` type in the interface itself.
/// - For errors like `Other`, additional contextual information is included for debugging purposes.
impl From<lmdb::Error> for Error {
    fn from(value: lmdb::Error) -> Error {
        match value {
            lmdb::Error::KeyExist => Error::KeyExist,
            lmdb::Error::NotFound => panic!("Wrong implementation, the key not found error is pictured by the option type in the interface iteself."),
            lmdb::Error::PageNotFound => Error::Other("Database page not found".to_string()),
            lmdb::Error::Corrupted => Error::Corrupted,
            lmdb::Error::Panic => Error::Other("panic occured".to_string()),
            lmdb::Error::VersionMismatch => Error::Other("version mismatch".to_string()),
            lmdb::Error::Invalid => Error::InvalidArgument,
            lmdb::Error::MapResized => Error::Other("map resized".to_string()),
            lmdb::Error::Incompatible => Error::Other("incopatible format".to_string()),
            lmdb::Error::BadRslot => Error::Other("bad reader slot".to_string()),
            lmdb::Error::BadTxn => Error::Other("bad transaction".to_string()),
            lmdb::Error::BadValSize => Error::Other("bad value size".to_string()),
            lmdb::Error::BadDbi => Error::Other("bad database index".to_string()),
            lmdb::Error::Other(code) => Error::Other(format!("unknown lmdb error code: {}", code)),
            e => Error::Busy(e.to_string())
        }
    }
}

impl ToCursorOp<lmdb::Database> for CursorOp {
    fn to_op(&self) -> u32 {
        match self {
            CursorOp::First => MDB_FIRST,
            CursorOp::Last => MDB_LAST,
            CursorOp::Next => MDB_NEXT,
            CursorOp::Prev => MDB_PREV,
            CursorOp::Current => MDB_GET_CURRENT,
        }
    }
}

/// Marks LMDB's `Database` type as implementing the `KvDatabase` trait.
///
/// This allows LMDB's native database type to integrate with the `KvDatabase` trait, enabling
/// it to be used seamlessly with the broader key-value store interface.
impl KvDatabase for lmdb::Database {}

// TODO: Remove this type, we could just implement the trait on the raw handle
/// Represents a handle to an LMDB database.
///
/// This structure provides a layer of abstraction over the LMDB `Database`, ensuring type safety
/// and lifetime management within the environment.
pub struct LmdbHandle<'env> {
    db: lmdb::Database,
    // Tracks the lifetime of the handle relative to the environment.
    _lt: PhantomData<&'env u8>,
}

impl<'env> LmdbHandle<'env> {
    /// Creates a new handle for a given LMDB database.
    ///
    /// ### Parameters:
    /// - `db`: The LMDB database instance to wrap.
    ///
    /// ### Returns:
    /// - A new `LmdbHandle` instance tied to the provided database.
    pub fn new(db: lmdb::Database) -> LmdbHandle<'env> {
        LmdbHandle {
            db,
            _lt: PhantomData,
        }
    }
}

/// Implements the `KvHandle` trait for `LmdbHandle`, providing database access.
///
/// This implementation ensures compatibility with the key-value interface by exposing the
/// underlying LMDB database.
impl<'env> KvHandle<'env, lmdb::Database> for LmdbHandle<'env> {
    /// Returns a reference to the underlying LMDB database.
    ///
    /// ### Returns:
    /// - A reference to the database managed by this handle.
    fn db(&self) -> &lmdb::Database {
        &self.db
    }
}

/// Represents an LMDB environment for managing databases and transactions.
///
/// The `Lmdb` provides a high-level abstraction over the LMDB `Environment`,
/// enabling the creation and management of databases, as well as read-only and
/// read-write transactions.
#[derive(Clone)]
pub struct Lmdb {
    env: Arc<lmdb::Environment>,
}

impl Lmdb {
    /// Initializes a new LMDB environment at the specified path with a given maximum number of databases.
    ///
    /// ### Parameters:
    /// - `path`: The filesystem path where the LMDB environment will be created or accessed.
    /// - `max_dbs`: The maximum number of named databases that can be created within this environment.
    ///
    /// ### Returns:
    /// - `Ok(Lmdb)` if the environment was successfully initialized.
    /// - `Err(Error)` if an error occurred during initialization.
    pub fn new(path: &Path, max_dbs: u32) -> Result<Lmdb, Error> {
        // We can do some further optimizations, if we want to increase the performance:
        let flags = EnvironmentFlags::default()
            | EnvironmentFlags::WRITE_MAP     // Faster writes, backed by OS memory mapping. Safe on modern OSes. (incompatible with nested transactions !)
            | EnvironmentFlags::NO_META_SYNC; // Skips metadata sync — tiny risk on power loss, big write speed boost.

        let env = lmdb::Environment::new()
            .set_max_dbs(max_dbs)
            // NOTE: we have to maintain the map size
            // in future. A mechanism to increase this size
            // is a good idea.
            .set_map_size(1_099_511_627_776)
            .set_max_readers(2048) // allow more concurrent readers
            .set_flags(flags)
            .open(path)?;

        Ok(Lmdb { env: Arc::new(env) })
    }
}

/// Implements the `Db` trait for `Lmdb`.
///
/// This allows `Lmdb` to serve as a fully functional key-value environment
/// compatible with the application's interface.
impl Db for Lmdb {
    type DB = lmdb::Database;
    type Handle<'env> = LmdbHandle<'env>;
    type RoTx<'env> = lmdb::RoTransaction<'env>;
    type RwTx<'env> = lmdb::RwTransaction<'env>;

    /// Opens a database by name or the default database if the name is "default".
    ///
    /// ### Parameters:
    /// - `name`: The name of the database to open, or "default" to open the unnamed default database.
    ///
    /// ### Returns:
    /// - `Ok(Handle<'env>)` containing a handle to the opened database.
    /// - `Err(Error)` if the database could not be opened.
    fn open_sub_db(&self, name: &str) -> Result<Self::Handle<'_>, Error> {
        // NOTE: This can also cause Error::NotFound
        let res = if name.to_ascii_lowercase() == "default" {
            self.env.open_db(None)
        } else {
            self.env.open_db(Some(name))
        };

        let db = match res {
            Ok(db) => db,
            Err(lmdb::Error::NotFound) => return Err(Error::DbNotFound(name.to_string())),
            Err(e) => return Err(Error::from(e)),
        };

        let handle = LmdbHandle::new(db);
        Ok(handle)
    }

    /// Create a database by name or the default database if the name is "default".
    ///
    /// ### Parameters:
    /// - `name`: The name of the database to open, or "default" to open the unnamed default database.
    ///
    /// ### Returns:
    /// - `Ok(Handle<'env>)` containing a handle to the opened database.
    /// - `Err(Error)` if the database could not be opened.
    fn create_sub_db(&self, name: &str) -> Result<Self::Handle<'_>, Error> {
        let db = if name == "default" {
            self.env.create_db(None, lmdb::DatabaseFlags::empty())?
        } else {
            self.env
                .create_db(Some(name), lmdb::DatabaseFlags::empty())?
        };

        let handle = LmdbHandle::new(db);
        Ok(handle)
    }

    /// Begins a new read-only transaction within the environment.
    ///
    /// ### Returns:
    /// - `Ok(RoTx<'env>)` containing the transaction object.
    /// - `Err(Error)` if the transaction could not be started.
    fn begin_ro_txn(&self) -> Result<Self::RoTx<'_>, Error> {
        let txn = self.env.begin_ro_txn()?;
        Ok(txn)
    }

    /// Begins a new read-write transaction within the environment.
    ///
    /// ### Returns:
    /// - `Ok(RwTx<'env>)` containing the transaction object.
    /// - `Err(Error)` if the transaction could not be started.
    fn begin_rw_txn(&self) -> Result<Self::RwTx<'_>, Error> {
        let txn = self.env.begin_rw_txn()?;
        Ok(txn)
    }
}

/// Implements the `Tx` trait for LMDB's read-only transactions (`RoTransaction`).
///
/// This trait provides methods to manage the lifecycle of read-only transactions,
/// including committing and aborting.
impl<'env> Tx for lmdb::RoTransaction<'env> {
    /// Commits the transaction, making all changes visible to other transactions.
    ///
    /// ### Returns:
    /// - `Ok(())` if the transaction was successfully committed.
    /// - `Err(Error)` if an error occurred during the commit operation.
    fn commit(self) -> Result<(), Error> {
        <Self as lmdb::Transaction>::commit(self)?;
        Ok(())
    }

    /// Aborts the transaction, discarding all changes made during its lifetime.
    ///
    /// ### Notes:
    /// - This method ensures that the transaction is cleanly terminated without affecting the database.
    fn abort(self) {
        <Self as lmdb::Transaction>::abort(self);
    }
}

/// Implements the `RawRead` trait for LMDB's read-only transaction (`RoTransaction`).
///
/// This allows performing read operations within the context of a read-only transaction.
/// It retrieves data from the database based on a specified key.
///
/// ### Methods:
/// - `read`: Fetches the value associated with a given key in the specified database.
impl<'env> RawRead<'env, lmdb::Database> for lmdb::RoTransaction<'env> {
    /// Reads a value from the database associated with the provided key.
    ///
    /// ### Parameters:
    /// - `db`: A handle to the database to query.
    /// - `key`: The key to search for in the database.
    ///
    /// ### Returns:
    /// - `Ok(Some(&[u8]))`: If the key exists, returns a reference to the value.
    /// - `Ok(None)`: If the key is not found.
    /// - `Err(Error)`: If an error occurs during the operation.
    fn read(
        &self,
        db: &impl KvHandle<'env, lmdb::Database>,
        key: &impl AsRef<[u8]>,
    ) -> Result<Option<&[u8]>, Error> {
        let res = <Self as lmdb::Transaction>::get(self, *db.db(), key);

        let data = match res {
            Ok(buf) => Some(buf),
            Err(lmdb::Error::NotFound) => None,
            Err(e) => return Err(Error::from(e)),
        };

        Ok(data)
    }
}

/// Implements the `RoTx` trait for LMDB's read-only transactions.
///
/// This provides additional methods specific to read-only transactions, such as creating cursors.
impl<'env> RoTx<'env, lmdb::Database> for lmdb::RoTransaction<'env> {
    /// Type definition for the cursor used in read-only transactions.
    type Cursor<'txn> = lmdb::RoCursor<'txn>
    where
        Self: 'txn;

    /// Creates a cursor for iterating over the database in the context of a read-only transaction.
    ///
    /// ### Parameters:
    /// - `db`: A handle to the database for which the cursor is created.
    ///
    /// ### Returns:
    /// - `Ok(Cursor)`: If the cursor is successfully created.
    /// - `Err(Error)`: If an error occurs.
    fn ro_cursor<'txn>(
        &'txn self,
        db: &impl KvHandle<'env, lmdb::Database>,
    ) -> Result<Self::Cursor<'txn>, Error> {
        let cursor = <Self as lmdb::Transaction>::open_ro_cursor(self, *db.db())?;
        Ok(cursor)
    }
}

/// Implements the `RoCursor` trait for LMDB's read-only cursors.
///
/// Provides methods to iterate over the key-value pairs in the database.
impl<'txn> RoCursor<'txn, lmdb::Database> for lmdb::RoCursor<'txn> {
    /// Type definition for the iterator used in the cursor.
    type Iter = lmdb::Iter<'txn>;

    fn get<K, V>(
        &self,
        key: Option<&K>,
        value: Option<&V>,
        op: impl ToCursorOp<lmdb::Database>,
    ) -> Result<(Option<&'txn [u8]>, &'txn [u8]), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let res = <Self as lmdb::Cursor>::get(
            self,
            key.map(|k| k.as_ref()),
            value.map(|v| v.as_ref()),
            op.to_op(),
        )?;

        Ok(res)
    }

    /// Creates an iterator over the database starting from the current cursor position.
    ///
    /// ### Returns:
    /// - An iterator over key-value pairs in the database.
    fn iter(&mut self) -> Self::Iter {
        <Self as lmdb::Cursor>::iter(self)
    }

    fn iter_start(&mut self) -> Self::Iter {
        <Self as lmdb::Cursor>::iter_start(self)
    }

    fn iter_from<K>(&mut self, key: &K) -> Self::Iter
    where
        K: AsRef<[u8]>,
    {
        <Self as lmdb::Cursor>::iter_from(self, key)
    }
}

/// Implements the `Tx` trait for LMDB's read-write transactions.
///
/// Provides methods to manage the lifecycle of read-write transactions, such as committing or aborting.
impl<'env> Tx for lmdb::RwTransaction<'env> {
    /// Commits the transaction, applying all changes to the database.
    ///
    /// ### Returns:
    /// - `Ok(())`: If the transaction was successfully committed.
    /// - `Err(Error)`: If an error occurs during the commit operation.
    fn commit(self) -> Result<(), Error> {
        <Self as lmdb::Transaction>::commit(self)?;
        Ok(())
    }

    /// Aborts the transaction, discarding all changes made within it.
    ///
    /// ### Notes:
    /// - This ensures no changes from the transaction are persisted in the database.
    fn abort(self) {
        <Self as lmdb::Transaction>::abort(self);
    }
}

/// Implements the `RawRead` trait for LMDB's read-write transaction (`RwTransaction`).
///
/// Enables reading data within the context of a read-write transaction.
impl<'env> RawRead<'env, lmdb::Database> for lmdb::RwTransaction<'env> {
    /// Reads a value from the database associated with the provided key.
    ///
    /// ### Parameters:
    /// - `db`: A handle to the database to query.
    /// - `key`: The key to search for in the database.
    ///
    /// ### Returns:
    /// - `Ok(Some(&[u8]))`: If the key exists, returns a reference to the value.
    /// - `Ok(None)`: If the key is not found.
    /// - `Err(Error)`: If an error occurs during the operation.
    fn read(
        &self,
        db: &impl KvHandle<'env, lmdb::Database>,
        key: &impl AsRef<[u8]>,
    ) -> Result<Option<&[u8]>, Error> {
        let res = <Self as lmdb::Transaction>::get(self, *db.db(), key);

        let data = match res {
            Ok(buf) => Some(buf),
            Err(lmdb::Error::NotFound) => None,
            Err(e) => return Err(Error::from(e)),
        };

        Ok(data)
    }
}

/// Implements the `RawWrite` trait for LMDB's read-write transaction (`RwTransaction`).
///
/// Enables writing and deleting data within the context of a read-write transaction.
impl<'env> RawWrite<'env, lmdb::Database> for lmdb::RwTransaction<'env> {
    /// Writes a key-value pair to the database.
    ///
    /// ### Parameters:
    /// - `db`: A handle to the database where the key-value pair will be stored.
    /// - `key`: The key to store.
    /// - `data`: The value associated with the key.
    ///
    /// ### Returns:
    /// - `Ok(())`: If the write operation was successful.
    /// - `Err(Error)`: If an error occurs during the operation.
    fn write(
        &mut self,
        db: &impl KvHandle<'env, lmdb::Database>,
        key: &impl AsRef<[u8]>,
        data: &impl AsRef<[u8]>,
    ) -> Result<(), Error> {
        self.put(*db.db(), key, &data, lmdb::WriteFlags::empty())?;
        Ok(())
    }

    /// Deletes a key-value pair from the database.
    ///
    /// ### Parameters:
    /// - `db`: A handle to the database from which the key-value pair will be deleted.
    /// - `key`: The key to delete.
    ///
    /// ### Returns:
    /// - `Ok(())`: If the deletion was successful.
    /// - `Err(Error)`: If an error occurs during the operation.
    fn delete(
        &mut self,
        db: &impl KvHandle<'env, lmdb::Database>,
        key: &impl AsRef<[u8]>,
    ) -> Result<(), Error> {
        let res = self.del(*db.db(), key, None);

        match res {
            Ok(_) => Ok(()),
            Err(lmdb::Error::NotFound) => Ok(()),
            Err(e) => Err(Error::from(e)),
        }
    }
}

/// Implements the `RwTx` trait for LMDB's read-write transactions.
///
/// Provides methods to create cursors and nested transactions within the context of a read-write transaction.
impl<'env> RwTx<'env, lmdb::Database> for lmdb::RwTransaction<'env> {
    /// Type definition for cursors used in read-write transactions.
    type Cursor<'txn> = lmdb::RwCursor<'txn>
    where
        Self: 'txn;

    /// Type definition for nested read-write transactions.
    type RwTx<'txn> = lmdb::RwTransaction<'txn>
    where
        Self: 'txn;

    /// Creates a cursor for iterating over the database in the context of a read-write transaction.
    ///
    /// ### Parameters:
    /// - `db`: A handle to the database for which the cursor is created.
    ///
    /// ### Returns:
    /// - `Ok(Cursor)`: If the cursor is successfully created.
    /// - `Err(Error)`: If an error occurs.
    fn rw_cursor<'txn>(
        &'txn mut self,
        db: &impl KvHandle<'env, lmdb::Database>,
    ) -> Result<Self::Cursor<'txn>, Error> {
        let cursor = self.open_rw_cursor(*db.db())?;
        Ok(cursor)
    }

    /// Begins a nested transaction within the current transaction.
    ///
    /// ### Returns:
    /// - `Ok(RwTx<'txn>)`: If the nested transaction is successfully started.
    /// - `Err(Error)`: If an error occurs during the operation.
    fn nested_txn(&mut self) -> Result<Self::RwTx<'_>, Error> {
        let ntx = self.begin_nested_txn()?;
        Ok(ntx)
    }
}

/// Implements the `RwCursor` trait for LMDB's read-write cursors.
///
/// Provides methods to iterate over the key-value pairs in the database.
impl<'txn> RwCursor<'txn, lmdb::Database> for lmdb::RwCursor<'txn> {
    /// Type definition for the iterator used in the cursor.
    type Iter = lmdb::Iter<'txn>;

    fn get<K, V>(
        &self,
        key: Option<&K>,
        value: Option<&V>,
        op: impl ToCursorOp<lmdb::Database>,
    ) -> Result<(Option<&'txn [u8]>, &'txn [u8]), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let res = <Self as lmdb::Cursor>::get(
            self,
            key.map(|k| k.as_ref()),
            value.map(|v| v.as_ref()),
            op.to_op(),
        )?;

        Ok(res)
    }

    fn put<K, V>(&mut self, key: &K, value: &V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.put(key, value, lmdb::WriteFlags::empty())?;
        Ok(())
    }

    fn del(&mut self) -> Result<(), Error> {
        self.del(lmdb::WriteFlags::empty())?;
        Ok(())
    }

    /// Creates an iterator over the database starting from the current cursor position.
    ///
    /// ### Returns:
    /// - An iterator over key-value pairs in the database.
    fn iter(&mut self) -> Self::Iter {
        <Self as lmdb::Cursor>::iter(self)
    }

    fn iter_start(&mut self) -> Self::Iter {
        <Self as lmdb::Cursor>::iter_start(self)
    }

    fn iter_from<K>(&mut self, key: &K) -> Self::Iter
    where
        K: AsRef<[u8]>,
    {
        <Self as lmdb::Cursor>::iter_from(self, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use tempfile::tempdir;

    const TEST_REPEATS: usize = 4;

    fn open_tmp_lmdb() -> Lmdb {
        let temp_dir = tempdir().unwrap();
        let env = Lmdb::new(temp_dir.path(), 1).unwrap();
        env
    }

    fn create_test_handle<'env, DB: KvDatabase, Env: Db>(env: &'env Env) -> impl KvHandle<'env, DB>
    where
        <Env as Db>::Handle<'env>: KvHandle<'env, DB>,
    {
        let handle = env.create_sub_db("test").unwrap();
        handle
    }

    fn write_with_rw_txn<'env, DB: KvDatabase, Txn: RwTx<'env, DB>>(
        txn: &mut Txn,
        handle: &impl KvHandle<'env, DB>,
        key: &impl AsRef<[u8]>,
        data: &impl AsRef<[u8]>,
    ) -> Result<(), Error> {
        txn.write(handle, &key, &data)?;
        Ok(())
    }

    fn read_with_ro_txn<'env, DB: KvDatabase, Txn: RoTx<'env, DB>>(
        txn: &Txn,
        handle: &impl KvHandle<'env, DB>,
        key: &impl AsRef<[u8]>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let buf = txn.read(handle, &key)?;
        let out: Option<Vec<u8>> = buf.map(|v| v.to_vec());
        Ok(out)
    }

    fn delete_with_rw_txn<'env, DB: KvDatabase, Txn: RwTx<'env, DB>>(
        txn: &mut Txn,
        handle: &impl KvHandle<'env, DB>,
        key: &impl AsRef<[u8]>,
    ) -> Result<(), Error> {
        txn.delete(handle, &key)?;
        Ok(())
    }

    #[test]
    fn read_write_delete() -> Result<(), Box<dyn std::error::Error>> {
        let env = open_tmp_lmdb();
        let handle = create_test_handle(&env);

        for _ in 0..TEST_REPEATS {
            let mut rng = rand::thread_rng();
            let test_key: Vec<u8> = (0..256).map(|_| rng.gen()).collect(); // 32 byte random key
            let test_data: Vec<u8> = (0..1048576).map(|_| rng.gen()).collect(); // 1 MiB random data
            {
                // write test value to db
                let mut txn = env.begin_rw_txn()?;
                write_with_rw_txn(&mut txn, &handle, &test_key, &test_data)?;
                Tx::commit(txn)?;
            }

            {
                // read test value from db
                let txn = env.begin_ro_txn()?;
                let res = read_with_ro_txn(&txn, &handle, &test_key)?;
                assert!(res.is_some(), "failed to read value from db");
                assert_eq!(test_data, res.unwrap(), "data read is corrupted");
            }

            {
                // delete test value from db
                let mut txn = env.begin_rw_txn()?;
                delete_with_rw_txn(&mut txn, &handle, &test_key)?;
                Tx::commit(txn)?;

                // try to read deleted value
                let txn = env.begin_ro_txn()?;
                let res = read_with_ro_txn(&txn, &handle, &test_key)?;
                assert!(res.is_none(), "could read deleted value");
            }
        }
        Ok(())
    }

    #[test]
    fn not_found_is_none() -> Result<(), Box<dyn std::error::Error>> {
        let env = open_tmp_lmdb();
        let handle = create_test_handle(&env);

        let txn = env.begin_ro_txn()?;
        let not_found = txn.read(&handle, &[0, 0, 0, 0])?;
        assert!(not_found.is_none());
        Ok(())
    }

    #[test]
    fn non_existing_db() {
        let env = open_tmp_lmdb();
        let db_name = "does-not-exist";
        let res = env.open_sub_db(db_name);
        assert!(res.is_err());
        if let Err(e) = res {
            match e {
                Error::DbNotFound(name) => {
                    assert_eq!(name, db_name, "error should include db-name")
                }
                _ => panic!("expected error 'DbNotFound'"),
            }
        }
    }
}
