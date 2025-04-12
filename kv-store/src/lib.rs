use thiserror::Error;

pub mod backend;

/// Prelude module to automatically include all necessary traits
pub mod prelude {
    pub use super::{
        Db, KvDatabase, KvHandle, RawRead, RawWrite, RoCursor, RoTx, RwCursor, RwTx, ToCursorOp,
    };
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("The key already exists.")]
    KeyExist,
    #[error("The data is corrupted.")]
    Corrupted,
    #[error("The database '{0}' was not found.")]
    DbNotFound(String),
    #[error("The argument is invalid.")]
    InvalidArgument,
    #[error("An I/O error occurred.")]
    IOError,
    #[error("The operation timed out.")]
    TimedOut,
    #[error("The resource is busy - {0}")]
    Busy(String),
    #[error("An unknown error occurred.")]
    Unknown,
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub enum CursorOp {
    First,
    Last,
    Next,
    Prev,
    Current,
}

pub trait ToCursorOp<DB: KvDatabase> {
    fn to_op(&self) -> u32;
}

/// Marker trait representing a key-value database.
///
/// This trait does not define any methods and serves as a placeholder
/// for types that represent a database in this abstraction.
/// Implementing this trait indicates that a type can function as a database
/// within the key-value store system.
pub trait KvDatabase {}

/// Trait representing a generic transaction.
///
/// Transactions provide a context for executing a series of database operations atomically.
/// This trait offers methods to commit or abort a transaction, ensuring that all operations
/// within the transaction are either fully applied or completely discarded.
pub trait Tx: Sized {
    /// Commits the transaction.
    ///
    /// Applies all changes made during the transaction to the database.
    /// If the commit is successful, returns `Ok(())`.
    /// If the commit fails (e.g., due to a conflict or I/O error), returns an error of type `E`.
    fn commit(self) -> Result<(), Error>;

    /// Aborts the transaction.
    ///
    /// Discards all changes made during the transaction.
    /// After calling `abort`, the transaction is considered terminated,
    /// and further operations on it may not be valid.
    fn abort(self);
}

/// Trait for reading raw data from the database within a transaction.
///
/// Provides a method to read the raw byte value associated with a given key.
/// The use of lifetimes and generics ensures data safety and flexibility across different database implementations.
pub trait RawRead<'env, DB>
where
    DB: KvDatabase,
{
    /// Reads the raw bytes associated with the given key from the database.
    ///
    /// - `db`: Reference to the database handle where the key is stored.
    /// - `key`: Reference to the key to be read.
    ///
    /// Returns a slice of bytes if the key exists, or an error if the key does not exist
    /// or if the operation fails.
    fn read(&self, db: &impl KvHandle<DB>, key: &impl AsRef<[u8]>) -> Result<Option<&[u8]>, Error>;
}

/// Trait for writing raw data to the database within a transaction.
///
/// Provides methods to write and delete key-value pairs.
/// This trait extends the capabilities of a transaction to modify data within the database.
pub trait RawWrite<'env, DB>
where
    DB: KvDatabase,
{
    /// Writes raw bytes to the database associated with the given key.
    ///
    /// - `db`: Reference to the database handle where the data will be written.
    /// - `key`: Reference to the key under which the data will be stored.
    /// - `data`: Slice of bytes representing the data to be stored.
    ///
    /// If the key already exists, its value is overwritten.
    /// Returns `Ok(())` if the operation is successful, or an error of type `E` if it fails.
    fn write(
        &mut self,
        db: &impl KvHandle<DB>,
        key: &impl AsRef<[u8]>,
        data: &impl AsRef<[u8]>,
    ) -> Result<(), Error>;

    /// Deletes the key-value pair associated with the given key from the database.
    ///
    /// - `db`: Reference to the database handle from which the key will be deleted.
    /// - `key`: Reference to the key to be deleted.
    ///
    /// Returns `Ok(())` if the deletion is successful, or an error of type `E` if it fails.
    fn delete(&mut self, db: &impl KvHandle<DB>, key: &impl AsRef<[u8]>) -> Result<(), Error>;
}

/// Trait representing a read-write cursor over the database within a transaction.
///
/// A cursor allows sequential access over key-value pairs in the database,
/// which can be efficient for iterating over large datasets or performing range queries.
/// This trait provides an iterator that yields mutable references to the data.
pub trait RwCursor<'txn, DB: KvDatabase> {
    /// Iterator type over key-value pairs, yielding references to keys and values.
    ///
    /// The iterator borrows data for the lifetime `'txn`, ensuring that data
    /// remains valid for the duration of the transaction.
    type Iter: Iterator<Item = (&'txn [u8], &'txn [u8])>;

    // Retrieves a key/data pair from the cursor. Depending on the cursor op,
    // the current key may be returned.
    fn get<K, V>(
        &self,
        key: Option<&K>,
        value: Option<&V>,
        op: impl ToCursorOp<DB>,
    ) -> Result<(Option<&'txn [u8]>, &'txn [u8]), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>;

    // Puts a key/data pair into the database. The cursor will be positioned
    // at the new data item, or on failure usually near it.
    fn put<K, V>(&mut self, key: &K, value: &V) -> Result<(), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>;

    /// Deletes the current key/data pair.
    fn del(&mut self) -> Result<(), Error>;

    /// Returns an iterator over key-value pairs starting from the current cursor position.
    ///
    /// May return an error if the cursor is invalid or if an I/O error occurs.
    fn iter(&mut self) -> Self::Iter;

    /// Iterate over database items starting from the beginning of the database.
    fn iter_start(&mut self) -> Self::Iter;

    /// Iterate over database items starting from the given key.
    fn iter_from<K>(&mut self, key: &K) -> Self::Iter
    where
        K: AsRef<[u8]>;
}

/// Trait representing a read-only cursor over the database within a transaction.
///
/// Similar to `RwCursor`, but does not allow modification of the data.
/// Useful for read-only transactions where data integrity must be preserved.
/// Provides an iterator that yields references to the data without permitting changes.
pub trait RoCursor<'txn, DB: KvDatabase> {
    /// Iterator type over key-value pairs, yielding references to keys and values.
    ///
    /// The iterator borrows data for the lifetime `'txn`.
    type Iter: Iterator<Item = (&'txn [u8], &'txn [u8])>;

    fn get<K, V>(
        &self,
        key: Option<&K>,
        value: Option<&V>,
        op: impl ToCursorOp<DB>,
    ) -> Result<(Option<&'txn [u8]>, &'txn [u8]), Error>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>;

    /// Iterate over database items. The iterator will begin with item next after the cursor,
    /// and continue until the end of the database. For new cursors,
    /// the iterator will begin with the first item in the database.
    fn iter(&mut self) -> Self::Iter;

    // Iterate over database items starting from the beginning of the database.
    fn iter_start(&mut self) -> Self::Iter;

    // Iterate over database items starting from the given key.
    fn iter_from<K>(&mut self, key: &K) -> Self::Iter
    where
        K: AsRef<[u8]>;
}

/// Trait representing a read-only transaction.
///
/// Extends the `Tx` and `RawRead` traits and provides cursor functionality.
/// Read-only transactions ensure that data cannot be modified during the transaction,
/// which can optimize performance and allow for higher concurrency.
pub trait RoTx<'env, DB>: Tx + RawRead<'env, DB>
where
    DB: KvDatabase,
{
    /// Cursor type for iterating over key-value pairs within the transaction.
    ///
    /// The cursor is bound by the lifetime `'txn`, which cannot outlive the transaction.
    type Cursor<'txn>: RoCursor<'txn, DB>
    where
        Self: 'txn;

    /// Creates a read-only cursor for the given database handle.
    ///
    /// - `db`: Reference to the database handle for which the cursor is created.
    ///
    /// Returns a cursor that allows iteration over the data in a read-only fashion.
    /// May return an error if the cursor cannot be created.
    fn ro_cursor<'txn>(&'txn self, db: &impl KvHandle<DB>) -> Result<Self::Cursor<'txn>, Error>;
}

// TODO: The function names clash with those of RawRead !!
/// Trait representing a read-write transaction.
///
/// Extends `Tx`, `RawWrite`, `RawRead`, and `CRUD` traits and provides cursor functionality.
/// Read-write transactions allow for both reading and modifying data within the database.
/// This trait also supports nested transactions and mutable cursors.
pub trait RwTx<'env, DB>: Tx + RawWrite<'env, DB> + RawRead<'env, DB>
where
    DB: KvDatabase,
{
    /// Cursor type for iterating over key-value pairs within the transaction.
    ///
    /// The cursor allows for modifying data as it iterates.
    type Cursor<'txn>: RwCursor<'txn, DB>
    where
        Self: 'txn;

    /// Nested transaction type.
    ///
    /// Allows for creating nested transactions within the current transaction,
    /// which can be committed or aborted independently of the parent transaction.
    type RwTx<'txn>: RwTx<'txn, DB>
    where
        Self: 'txn;

    /// Creates a read-write cursor for the given database handle.
    ///
    /// - `db`: Reference to the database handle for which the cursor is created.
    ///
    /// Returns a cursor that allows for iterating and modifying data within the database.
    /// May return an error if the cursor cannot be created.
    fn rw_cursor<'txn>(&'txn mut self, db: &impl KvHandle<DB>)
        -> Result<Self::Cursor<'txn>, Error>;

    /// Begins a nested read-write transaction.
    ///
    /// Nested transactions can be useful for batching operations or handling
    /// transactions within transactions, providing finer control over commits and rollbacks.
    ///
    /// Returns a new read-write transaction that is nested within the current one.
    /// May return an error if the nested transaction cannot be created.
    fn nested_txn(&mut self) -> Result<Self::RwTx<'_>, Error>;
}

/// Trait representing the key-value database.
///
/// Provides methods to open databases and begin transactions.
/// The environment encapsulates the overall state of the database system,
/// managing resources and providing access to databases and transactions.
pub trait Db: Clone + std::marker::Send + std::marker::Sync {
    type DB: KvDatabase;

    /// Handle type for a database.
    ///
    /// The handle provides access to a specific database within the environment.
    type Handle: KvHandle<Self::DB>;

    /// Read-only transaction type.
    ///
    /// Allows for creating read-only transactions within the environment.
    type RoTx<'env>: RoTx<'env, Self::DB>
    where
        Self: 'env;

    /// Read-write transaction type.
    ///
    /// Allows for creating read-write transactions within the environment.
    type RwTx<'env>: RwTx<'env, Self::DB>
    where
        Self: 'env;

    /// Opens a database with the given name.
    ///
    /// - `name`: Name of the database to open.
    ///
    /// Returns a handle to the database if it exists, or an error if it does not
    /// or if the operation fails.
    fn open_sub_db(&self, name: &str) -> Result<Self::Handle, Error>;

    /// Creates a database with the given name.
    /// If the database is already created, this function will open the database.
    ///
    /// - `name`: Name of the database to open.
    ///
    /// Returns a handle to the database if it exists, or an error if it does not
    /// or if the operation fails.
    fn create_sub_db(&self, name: &str) -> Result<Self::Handle, Error>;

    /// Begins a new read-only transaction.
    ///
    /// Read-only transactions are optimized for situations where data integrity is critical,
    /// and no modifications are required.
    ///
    /// Returns a new read-only transaction, or an error if the transaction cannot be started.
    fn begin_ro_txn(&self) -> Result<Self::RoTx<'_>, Error>;

    /// Begins a new read-write transaction.
    ///
    /// Read-write transactions allow for both reading and modifying data.
    ///
    /// Returns a new read-write transaction, or an error if the transaction cannot be started.
    fn begin_rw_txn(&self) -> Result<Self::RwTx<'_>, Error>;
}

/// Trait representing a handle to a database.
///
/// Provides access to the database name and the underlying database object.
/// The handle is used in transaction operations to specify which database to interact with.
/// This abstraction allows the same transaction methods to operate on different databases.
pub trait KvHandle<DB>: std::marker::Send + std::marker::Sync
where
    DB: KvDatabase,
{
    /// Returns a reference to the underlying database object.
    ///
    /// This can be used to access database-specific functionality if needed.
    /// The lifetime `'env` ensures that the database reference does not outlive the environment.
    fn db(&self) -> &DB;
}

// /// Trait providing CRUD (Create, Read, Update, Delete) operations.
// ///
// /// Extends functionality for read-write transactions.
// /// These methods provide a higher-level abstraction over `RawRead` and `RawWrite`,
// /// working with `Option` types and `Vec<u8>` for more ergonomic usage.
// pub trait CRUD<'env, DB>: RwTx<'env, DB>
// where
//     DB: KvDatabase,
// {
//     /// Reads the value associated with the given key.
//     ///
//     /// - `db`: Reference to the database handle.
//     /// - `key`: Reference to the key to be read.
//     ///
//     /// Returns:
//     /// - `Ok(Some(Vec<u8>))` if the key exists, containing the value.
//     /// - `Ok(None)` if the key does not exist.
//     /// - `Err(E)` if an error occurs during the operation.
//     fn read(
//         &self,
//         db: &impl KvHandle<DB>,
//         key: &impl AsRef<[u8]>,
//     ) -> Result<Option<Vec<u8>>, Error> {
//         let val = <Self as RawRead<DB>>::read(self, db, key)?.map(|v| v.to_vec());

//         Ok(val)
//     }

//     /// Creates a new key-value pair in the database.
//     /// This method overwrite an existing key!
//     ///
//     /// - `db`: Reference to the database handle.
//     /// - `key`: Reference to the key under which the data will be stored.
//     /// - `data`: Slice of bytes representing the data to be stored.
//     ///
//     /// Returns:
//     /// - `Ok(())` if the key-value pair was successfully created.
//     /// - `Err(E)` if the key already exists or if an error occurs.
//     fn create(
//         &mut self,
//         db: &impl KvHandle<DB>,
//         key: &impl AsRef<[u8]>,
//         data: &impl AsRef<[u8]>,
//     ) -> Result<(), Error> {
//         <Self as RawWrite<DB>>::write(self, db, key, data)?;
//         Ok(())
//     }

//     /// Updates the value associated with the given key.
//     ///
//     /// - `db`: Reference to the database handle.
//     /// - `key`: Reference to the key to be updated.
//     /// - `data`: Slice of bytes representing the new data to be stored.
//     ///
//     /// Returns:
//     /// - `Ok(Some(Vec<u8>))` containing the old value if the key existed and was updated.
//     /// - `Ok(None)` if the key did not exist.
//     /// - `Err(E)` if an error occurs during the operation.
//     fn update(
//         &mut self,
//         db: &impl KvHandle<DB>,
//         key: &impl AsRef<[u8]>,
//         data: &impl AsRef<[u8]>,
//     ) -> Result<Option<Vec<u8>>, Error> {
//         let old = <Self as CRUD<DB>>::read(self, db, key)?;
//         <Self as RawWrite<DB>>::write(self, db, key, data)?;
//         Ok(old)
//     }

//     /// Deletes the key-value pair associated with the given key.
//     ///
//     /// - `db`: Reference to the database handle.
//     /// - `key`: Reference to the key to be deleted.
//     ///
//     /// Returns:
//     /// - `Ok(Some(Vec<u8>))` containing the old value if the key existed and was deleted.
//     /// - `Ok(None)` if the key did not exist.
//     /// - `Err(E)` if an error occurs during the operation.
//     fn delete(
//         &mut self,
//         db: &impl KvHandle<DB>,
//         key: &impl AsRef<[u8]>,
//     ) -> Result<Option<Vec<u8>>, Error> {
//         let old = <Self as CRUD<DB>>::read(self, db, key)?;
//         <Self as RawWrite<DB>>::delete(self, db, key)?;
//         Ok(old)
//     }
// }

// /// Implement the CRUD Interface for all Types T
// /// implementing the RwTx super trait.
// impl<'env, DB, T> CRUD<'env, DB> for T
// where
//     DB: KvDatabase,
//     T: RwTx<'env, DB>,
// {
// }
