use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;
use std::ptr::null_mut;

use crate::dynamic_invoke;
use crate::recovery::helpers::dynamic_api::{djb2_hash, WINSQLITE3_HASH, load_library};

#[repr(C)]
struct Sqlite3 {
    _unused: [u8; 0],
}

#[repr(C)]
struct Sqlite3Stmt {
    _unused: [u8; 0],
}

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;

fn sqlite_transient() -> Option<unsafe extern "C" fn(*mut c_void)> {
    unsafe { std::mem::transmute(-1isize) }
}

pub struct Connection {
    db: *mut Sqlite3,
}

impl Connection {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        // Ensure winsqlite3.dll is loaded
        unsafe { load_library("winsqlite3.dll") };

        let path_str = path.as_ref().to_string_lossy();
        let c_path = CString::new(path_str.as_ref()).map_err(|e| e.to_string())?;
        let mut db = null_mut();
        
        type FnSqlite3Open = unsafe extern "C" fn(filename: *const c_char, pp_db: *mut *mut Sqlite3) -> c_int;

        let res = unsafe {
            dynamic_invoke!(
                WINSQLITE3_HASH,
                djb2_hash("sqlite3_open"),
                FnSqlite3Open,
                c_path.as_ptr(),
                &mut db
            )
        }.ok_or("Failed to invoke sqlite3_open")?;

        if res == SQLITE_OK {
            Ok(Self { db })
        } else {
            Err(format!("failed to open sqlite database: {res}"))
        }
    }

    pub fn prepare(&self, sql: &str) -> Result<Statement, String> {
        let c_sql = CString::new(sql).map_err(|e| e.to_string())?;
        let mut stmt = null_mut();
        
        type FnSqlite3PrepareV2 = unsafe extern "C" fn(
            db: *mut Sqlite3,
            z_sql: *const c_char,
            n_byte: c_int,
            pp_stmt: *mut *mut Sqlite3Stmt,
            pz_tail: *mut *const c_char,
        ) -> c_int;

        let res = unsafe {
            dynamic_invoke!(
                WINSQLITE3_HASH,
                djb2_hash("sqlite3_prepare_v2"),
                FnSqlite3PrepareV2,
                self.db,
                c_sql.as_ptr(),
                -1,
                &mut stmt,
                null_mut()
            )
        }.ok_or("Failed to invoke sqlite3_prepare_v2")?;

        if res == SQLITE_OK {
            Ok(Statement { stmt })
        } else {
            Err(format!("failed to prepare sqlite statement: {res}"))
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        type FnSqlite3Close = unsafe extern "C" fn(db: *mut Sqlite3) -> c_int;
        unsafe {
            dynamic_invoke!(WINSQLITE3_HASH, djb2_hash("sqlite3_close"), FnSqlite3Close, self.db);
        }
    }
}

pub struct Statement {
    stmt: *mut Sqlite3Stmt,
}

impl Statement {
    pub fn bind_text(&mut self, index: i32, text: &str) -> Result<(), String> {
        let c_text = CString::new(text).map_err(|e| e.to_string())?;
        
        type FnSqlite3BindText = unsafe extern "C" fn(
            p_stmt: *mut Sqlite3Stmt,
            i: c_int,
            z: *const c_char,
            n: c_int,
            x_del: Option<unsafe extern "C" fn(*mut c_void)>,
        ) -> c_int;

        let res = unsafe {
            dynamic_invoke!(
                WINSQLITE3_HASH,
                djb2_hash("sqlite3_bind_text"),
                FnSqlite3BindText,
                self.stmt,
                index,
                c_text.as_ptr(),
                -1,
                sqlite_transient()
            )
        }.ok_or("Failed to invoke sqlite3_bind_text")?;

        if res == SQLITE_OK {
            Ok(())
        } else {
            Err(format!("failed to bind text: {res}"))
        }
    }

    pub fn next(&mut self) -> Result<Option<Row>, String> {
        type FnSqlite3Step = unsafe extern "C" fn(p_stmt: *mut Sqlite3Stmt) -> c_int;

        let res = unsafe {
            dynamic_invoke!(
                WINSQLITE3_HASH,
                djb2_hash("sqlite3_step"),
                FnSqlite3Step,
                self.stmt
            )
        }.ok_or("Failed to invoke sqlite3_step")?;

        if res == SQLITE_ROW {
            Ok(Some(Row { stmt: self.stmt }))
        } else if res == SQLITE_DONE {
            Ok(None)
        } else {
            Err(format!("sqlite step failed: {res}"))
        }
    }
}

impl Drop for Statement {
    fn drop(&mut self) {
        type FnSqlite3Finalize = unsafe extern "C" fn(p_stmt: *mut Sqlite3Stmt) -> c_int;
        unsafe {
            dynamic_invoke!(WINSQLITE3_HASH, djb2_hash("sqlite3_finalize"), FnSqlite3Finalize, self.stmt);
        }
    }
}

pub struct Row {
    stmt: *mut Sqlite3Stmt,
}

impl Row {
    pub fn get_text(&self, index: i32) -> Option<String> {
        type FnSqlite3ColumnText = unsafe extern "C" fn(p_stmt: *mut Sqlite3Stmt, i_col: c_int) -> *const c_char;

        let ptr = unsafe {
            dynamic_invoke!(
                WINSQLITE3_HASH,
                djb2_hash("sqlite3_column_text"),
                FnSqlite3ColumnText,
                self.stmt,
                index
            )
        }.unwrap_or(null_mut());

        if ptr.is_null() {
            None
        } else {
            unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
        }
    }

    pub fn get_blob(&self, index: i32) -> Option<Vec<u8>> {
        type FnSqlite3ColumnBlob = unsafe extern "C" fn(p_stmt: *mut Sqlite3Stmt, i_col: c_int) -> *const c_void;
        type FnSqlite3ColumnBytes = unsafe extern "C" fn(p_stmt: *mut Sqlite3Stmt, i_col: c_int) -> c_int;

        let ptr = unsafe {
            dynamic_invoke!(
                WINSQLITE3_HASH,
                djb2_hash("sqlite3_column_blob"),
                FnSqlite3ColumnBlob,
                self.stmt,
                index
            )
        }.unwrap_or(null_mut());

        let len = unsafe {
            dynamic_invoke!(
                WINSQLITE3_HASH,
                djb2_hash("sqlite3_column_bytes"),
                FnSqlite3ColumnBytes,
                self.stmt,
                index
            )
        }.unwrap_or(0);

        if ptr.is_null() {
            None
        } else {
            unsafe { Some(std::slice::from_raw_parts(ptr as *const u8, len as usize).to_vec()) }
        }
    }
}
