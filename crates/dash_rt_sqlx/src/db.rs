use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::Sender;

use dash_vm::delegate;
use dash_vm::gc::trace::Trace;
use dash_vm::local::LocalScope;
use dash_vm::value::object::NamedObject;
use dash_vm::value::object::Object;
use rusqlite::Connection;
use rusqlite::ToSql;

enum SqliteEventMessage {
    Execute {
        sql: String,
        params: Vec<Box<dyn ToSql + Send + Sync + 'static>>,
    },
    Query {
        sql: String,
        params: Vec<Box<dyn ToSql + Send + Sync + 'static>>,
    },
}

#[derive(Debug)]
pub struct Database {
    event_sender: Sender<SqliteEventMessage>,
    object: NamedObject,
}

unsafe impl Trace for Database {
    fn trace(&self) {
        #[allow(unused)]
        let Database { object, event_sender } = self;
        object.trace();
    }
}

impl Object for Database {
    delegate!(
        object,
        get_own_property_descriptor,
        get_property,
        get_property_descriptor,
        set_property,
        delete_property,
        set_prototype,
        get_prototype,
        as_any,
        apply,
        own_keys
    );
}

impl Database {
    pub fn connect(path: &str, sc: &mut LocalScope) -> Self {
        let path = path.to_string();

        let (tx, rx) = mpsc::channel();
        // tokio::task::spawn_blocking(move || {
        let conn = Connection::open(path).unwrap();

        // while let Ok(message) = rx.recv() {
        // let message = SqliteEventMessage::Execute {
        //     // sql: "CREATE TABLE USER (`name` VARCHAR);".to_string(),
        //     sql: "INSERT INTO USER VALUES (?)".to_string(),
        //     params: vec![Box::new("hmm")],
        // };
        let message = SqliteEventMessage::Query {
            sql: "SELECT * FROM USER".to_string(),
            params: Vec::new(),
        };
        match message {
            SqliteEventMessage::Query { sql, params } => {
                let params = params.iter().map(|x| &**x as &dyn ToSql).collect::<Vec<_>>();
                let mut stmt = conn.prepare(&sql).unwrap();
                let mut rows = stmt.query(params.as_slice()).unwrap();
                let columns = rows
                    .as_ref()
                    .unwrap()
                    .column_names()
                    .into_iter()
                    .map(|x| x.to_owned())
                    .collect::<Vec<_>>();

                let mut result = Vec::new();
                while let Some(row) = rows.next().unwrap() {
                    let mut map: HashMap<String, rusqlite::types::Value> = HashMap::new();
                    for column in &columns {
                        map.insert(column.to_string(), row.get_unwrap(column.as_str()));
                    }
                    result.push(map);
                }
                dbg!(&result);
            }
            SqliteEventMessage::Execute { sql, params } => {
                let params = params.iter().map(|x| &**x as &dyn ToSql).collect::<Vec<_>>();
                let mut stmt = conn.prepare(&sql).unwrap();
                let mut rows = stmt.execute(params.as_slice()).unwrap();
            }
        }
        // }
        // });

        let object = NamedObject::new(sc);

        Self {
            event_sender: tx,
            object,
        }
    }
}
