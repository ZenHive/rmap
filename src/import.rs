use crate::schema_json::schema_json_str;

const PROMPT_TEMPLATE: &str = include_str!("../templates/import_prompt.md");

pub fn format_import_prompt(project: &str) -> String {
    let schema = schema_json_str().expect("schema generation infallible");
    PROMPT_TEMPLATE
        .replace("{{project}}", project)
        .replace("{{schema_json}}", &schema)
}
