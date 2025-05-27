use limbo_ext::{
    register_extension, Connection, ConstraintInfo, IndexInfo, OrderByInfo, ResultCode, Statement,
    StepResult, VTabCursor, VTabKind, VTabModule, VTabModuleDerive, VTable, Value, ValueType,
};
use std::rc::Rc;

register_extension! {
    vtabs: { PragmaVTabModule }
}

#[derive(Debug, VTabModuleDerive, Default)]
struct PragmaVTabModule;

impl VTabModule for PragmaVTabModule {
    type Table = PragmaTable;
    const VTAB_KIND: VTabKind = VTabKind::TableValuedFunction;
    const NAME: &'static str = "pragma_table_info_ext";

    fn create(args: &[Value]) -> Result<(String, Self::Table), ResultCode> {
        if args.is_empty() {
            return Err(ResultCode::InvalidArgs);
        }
        let schema = "CREATE TABLE x(
            \"name\",
            \"type\",
            \"notnull\",
            \"dflt_value\",
            \"pk\"
        )"
        .into();
        let table_name = args.get(0).unwrap().to_text().unwrap();
        let table = PragmaTable {
            arg: Some(table_name.to_string()),
        };
        Ok((schema, table))
    }
}

struct PragmaTable {
    arg: Option<String>,
}

impl VTable for PragmaTable {
    type Cursor = PragmaCursor;
    type Error = ResultCode;

    fn open(&self, conn: Option<Rc<Connection>>) -> Result<Self::Cursor, Self::Error> {
        Ok(PragmaCursor {
            pos: 0,
            eof: false,
            conn,
            stmt: None,
            arg: self.arg.clone(),
        })
    }

    fn update(&mut self, _rowid: i64, _args: &[Value]) -> Result<(), Self::Error> {
        Err(ResultCode::ReadOnly)
    }

    fn insert(&mut self, _args: &[Value]) -> Result<i64, Self::Error> {
        Err(ResultCode::ReadOnly)
    }

    fn delete(&mut self, _rowid: i64) -> Result<(), Self::Error> {
        Err(ResultCode::ReadOnly)
    }

    fn best_index(_constraints: &[ConstraintInfo], _order_by: &[OrderByInfo]) -> IndexInfo {
        Default::default()
    }
}

struct PragmaCursor {
    pos: usize,
    eof: bool,
    conn: Option<Rc<Connection>>,
    stmt: Option<Statement>,
    arg: Option<String>,
}

impl VTabCursor for PragmaCursor {
    type Error = ResultCode;

    fn filter(&mut self, _args: &[Value], _idx_info: Option<(&str, i32)>) -> ResultCode {
        let Some(conn) = &self.conn else {
            return ResultCode::Error;
        };

        let stmt = conn
            .prepare(
                format!(
                    "PRAGMA table_info({});",
                    self.arg
                        .as_ref()
                        .unwrap_or(&"".to_string())
                        .replace("'", "")
                )
                .as_str(),
            )
            .map_err(|_| ResultCode::Error)
            .unwrap();

        self.stmt = Some(stmt);
        self.next()
    }

    fn rowid(&self) -> i64 {
        self.pos as i64
    }

    fn column(&self, idx: u32) -> Result<Value, Self::Error> {
        self.stmt.as_ref().unwrap().get_row()
            .get(idx as usize)
            .map(|col| match col.value_type() {
                ValueType::Null => Value::null(),
                ValueType::Integer => Value::from_integer(col.to_integer().unwrap()),
                ValueType::Float => Value::from_float(col.to_float().unwrap()),
                ValueType::Text => Value::from_text(col.to_text().unwrap().to_string()),
                ValueType::Blob => Value::from_blob(col.to_blob().unwrap()),
                ValueType::Error => Value::error(col.to_error().unwrap()),
            })
            .ok_or(ResultCode::OutOfRange)
    }

    fn eof(&self) -> bool {
        self.eof
    }

    fn next(&mut self) -> ResultCode {
        let result = self.stmt.as_ref().unwrap().step();

        self.eof = result == StepResult::Done;
        if self.eof {
            return ResultCode::EOF;
        }

        self.pos += 1;
        ResultCode::OK
    }
}
