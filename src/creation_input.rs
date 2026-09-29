//! Creation input and borrowed mutator arguments derived from the task registry.
use crate::schema::{CrossRepo, Scores, TaskId};
use toml_edit::{Array, InlineTable, Item, Table, Value};

macro_rules! creation_value {
    (id, $task:ident, $buffers:ident, $today:ident, $field:ident) => {
        $buffers.id
    };
    (copy, $task:ident, $buffers:ident, $today:ident, $field:ident) => {
        $task.$field
    };
    (string, $task:ident, $buffers:ident, $today:ident, $field:ident) => {
        $task.$field.as_str()
    };
    (optional, $task:ident, $buffers:ident, $today:ident, $field:ident) => {
        $task.$field.as_deref()
    };
    (date, $task:ident, $buffers:ident, $today:ident, $field:ident) => {
        Some($task.$field.as_deref().unwrap_or($today))
    };
    (slice, $task:ident, $buffers:ident, $today:ident, $field:ident) => {
        &$task.$field
    };
    (strings, $task:ident, $buffers:ident, $today:ident, $field:ident) => {
        &$buffers.$field
    };
    (deps, $task:ident, $buffers:ident, $today:ident, $field:ident) => {
        &$buffers.depends_on
    };
    (status, $task:ident, $buffers:ident, $today:ident, $field:ident) => {
        "pending"
    };
    (scores, $task:ident, $buffers:ident, $today:ident, $field:ident) => {
        ($task.scores.d, $task.scores.b, $task.scores.u)
    };
}

macro_rules! write_creation_field {
    (skip, $table:ident, $fields:ident, $name:ident) => {};
    (required, $table:ident, $fields:ident, $name:ident) => {
        $table[stringify!($name)] = Item::Value(Value::from($fields.$name));
    };
    (number, $table:ident, $fields:ident, $name:ident) => {
        $table[stringify!($name)] = Item::Value(Value::from($fields.$name as i64));
    };
    (optional, $table:ident, $fields:ident, $name:ident) => {
        if let Some(value) = $fields.$name {
            $table[stringify!($name)] = Item::Value(Value::from(value));
        }
    };
    (numbers, $table:ident, $fields:ident, $name:ident) => {
        if !$fields.$name.is_empty() {
            let array: Array = $fields
                .$name
                .iter()
                .map(|value| Value::from(*value as i64))
                .collect();
            $table[stringify!($name)] = Item::Value(Value::Array(array));
        }
    };
    (array, $table:ident, $fields:ident, $name:ident) => {
        if !$fields.$name.is_empty() {
            let array: Array = $fields
                .$name
                .iter()
                .map(|value| Value::from(value.clone()))
                .collect();
            $table[stringify!($name)] = Item::Value(Value::Array(array));
        }
    };
    (status, $table:ident, $fields:ident, $name:ident) => {
        let status = if $fields.status.is_empty() {
            "pending"
        } else {
            $fields.status
        };
        $table["status"] = Item::Value(Value::from(status));
    };
    (scores, $table:ident, $fields:ident, $name:ident) => {
        let mut scores = InlineTable::new();
        scores.insert("d", Value::from($fields.scores.0 as i64));
        scores.insert("b", Value::from($fields.scores.1 as i64));
        scores.insert("u", Value::from($fields.scores.2 as i64));
        $table["scores"] = Item::Value(Value::InlineTable(scores));
    };
    (cross_repo, $table:ident, $fields:ident, $name:ident) => {
        if !$fields.cross_repo.is_empty() {
            let mut array = Array::new();
            for entry in $fields.cross_repo {
                let mut inline = InlineTable::new();
                inline.insert("repo", Value::from(entry.repo.as_str()));
                let task_id = entry.task_id.to_string();
                let value = task_id
                    .parse::<i64>()
                    .map(Value::from)
                    .unwrap_or_else(|_| Value::from(task_id.as_str()));
                inline.insert("task_id", value);
                if let Some(linear_id) = &entry.linear_id {
                    inline.insert("linear_id", Value::from(linear_id.as_str()));
                }
                inline.insert("relation", Value::from(entry.relation.as_str()));
                array.push(Value::InlineTable(inline));
            }
            $table["cross_repo"] = Item::Value(Value::Array(array));
        }
    };
}

// Accumulate creation entries before emitting whole structs: declarative macros
// cannot expand directly into struct fields.
macro_rules! define_creation {
    (@collect [$($fields:tt)*]) => {
        define_creation!(@emit $($fields)*);
    };
    (@collect [$($fields:tt)*]
        $(#[$attr:meta])* $name:ident: $ty:ty => $metadata:tt [];
        $($rest:tt)*) => {
        define_creation!(@collect [$($fields)*] $($rest)*);
    };
    (@collect [$($fields:tt)*]
        $(#[$attr:meta])* $name:ident: $ty:ty => $metadata:tt
        [$stdin:ty; $borrowed:ty; $convert:ident; $write:ident];
        $($rest:tt)*) => {
        define_creation!(@collect [$($fields)*
            $(#[$attr])* $name: $stdin, $borrowed, $convert, $write;
        ] $($rest)*);
    };
    (@emit $( $(#[$attr:meta])* $name:ident: $stdin:ty, $borrowed:ty, $convert:ident, $write:ident; )*) => {
        #[derive(Debug, Default, serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct StdinTask {
            $( $(#[$attr])* pub $name: $stdin, )*
        }
        /// Borrowed creation fields. Empty status defaults to pending; an absent
        /// id requests allocation. Lifecycle fields are deliberately excluded.
        #[derive(Debug, Default, Clone)]
        pub struct NewTaskFields<'a> {
            $(pub $name: $borrowed,)*
        }
        pub const STDIN_TASK_FIELDS: &[&str] = &[$(stringify!($name),)*];
        impl StdinTask {
            pub fn new_fields<'a>(&'a self, buffers: &'a CreationBuffers<'a>, today: &'a str) -> NewTaskFields<'a> {
                NewTaskFields {
                    $($name: creation_value!($convert, self, buffers, today, $name),)*
                }
            }
        }
        impl NewTaskFields<'_> {
            pub(crate) fn write_fields(&self, table: &mut Table) {
                $(write_creation_field!($write, table, self, $name);)*
                table.sort_values_by(|a, _, b, _| {
                    crate::mutate::canonical_task_key_index(a.get())
                        .cmp(&crate::mutate::canonical_task_key_index(b.get()))
                });
            }
        }
    };
    ($($input:tt)*) => { define_creation!(@collect [] $($input)*); };
}

crate::task_fields!(define_creation);

/// Temporary numeric ids and string slices backing the borrowed public input.
pub struct CreationBuffers<'a> {
    pub id: Option<u32>,
    pub depends_on: Vec<u32>,
    pub markers: Vec<&'a str>,
    pub acceptance_criteria: Vec<&'a str>,
    pub out_of_scope: Vec<&'a str>,
}
