use once_cell::sync::Lazy;
use serde_json::Value;

static SKILL_SCHEMA: Lazy<Value> = Lazy::new(|| {
    serde_json::json!({
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "type": "object",
      "additionalProperties": false,
      "required": [
        "name",
        "version",
        "description",
        "author",
        "license",
        "system_prompt",
        "allowed_tools"
      ],
      "properties": {
        "name": { "type": "string", "minLength": 1, "maxLength": 128 },
        "version": { "type": "string", "minLength": 1, "maxLength": 32 },
        "description": { "type": "string", "minLength": 1, "maxLength": 2048 },
        "author": { "type": "string", "minLength": 1, "maxLength": 256 },
        "license": { "type": "string", "minLength": 1, "maxLength": 64 },
        "system_prompt": { "type": "string", "minLength": 1, "maxLength": 16384 },
        "allowed_tools": {
          "type": "array",
          "items": { "type": "string", "minLength": 1, "maxLength": 128 },
          "maxItems": 128
        }
      }
    })
});

pub fn skill_schema() -> &'static Value {
    &SKILL_SCHEMA
}
