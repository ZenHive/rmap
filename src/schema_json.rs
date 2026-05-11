use crate::schema::Tasks;

pub fn schema_json_str() -> serde_json::Result<String> {
    serde_json::to_string_pretty(&schemars::schema_for!(Tasks))
}
