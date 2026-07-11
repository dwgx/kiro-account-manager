use serde::{Deserialize, Deserializer, Serialize};

fn null_string_as_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

fn null_string_vec_as_default<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<Vec<Option<String>>>::deserialize(deserializer)?
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect())
}

pub(super) fn null_models_as_default<'de, D>(deserializer: D) -> Result<Vec<AvailableModel>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<Vec<AvailableModel>>::deserialize(deserializer)?.unwrap_or_default())
}

fn extract_string_enum(schema: &serde_json::Value, path: &[&str]) -> Vec<String> {
    let mut current = schema;
    for key in path {
        current = match current.get(*key) {
            Some(value) => value,
            None => return Vec::new(),
        };
    }

    current
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

const EFFORT_SCHEMA_PATHS: &[(&str, &[&str])] = &[
    (
        "output_config",
        &[
            "properties",
            "output_config",
            "properties",
            "effort",
            "enum",
        ],
    ),
    (
        "reasoning",
        &["properties", "reasoning", "properties", "effort", "enum"],
    ),
];

fn extract_effort_metadata(schema: &Option<serde_json::Value>) -> (Vec<String>, Option<String>) {
    let Some(schema) = schema.as_ref().filter(|value| value.is_object()) else {
        return (Vec::new(), None);
    };

    for (schema_path, enum_path) in EFFORT_SCHEMA_PATHS {
        let levels = extract_string_enum(schema, enum_path);
        if !levels.is_empty() {
            return (levels, Some((*schema_path).to_string()));
        }
    }

    (Vec::new(), None)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModelTokenLimits {
    pub max_input_tokens: Option<i64>,
    pub max_output_tokens: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModelPromptCaching {
    pub maximum_cache_checkpoints_per_request: Option<i64>,
    pub minimum_tokens_per_cache_checkpoint: Option<i64>,
    pub supports_prompt_caching: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailableModel {
    #[serde(default, deserialize_with = "null_string_as_default")]
    pub model_id: String,
    #[serde(default, deserialize_with = "null_string_as_default")]
    pub model_name: String,
    #[serde(default, deserialize_with = "null_string_as_default")]
    pub description: String,
    pub provider: Option<String>,
    #[serde(default, deserialize_with = "null_string_vec_as_default")]
    pub capabilities: Vec<String>,
    pub context_window: Option<i64>,
    pub is_default: Option<bool>,
    pub rate_multiplier: Option<f64>,
    #[serde(default)]
    pub rate_unit: Option<String>,
    pub prompt_caching: Option<AvailableModelPromptCaching>,
    #[serde(default, deserialize_with = "null_string_vec_as_default")]
    pub supported_input_types: Vec<String>,
    pub token_limits: Option<AvailableModelTokenLimits>,
    #[serde(default)]
    pub additional_model_request_fields_schema: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effort_levels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort_schema_path: Option<String>,
}

impl<'de> Deserialize<'de> for AvailableModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct AvailableModelWire {
            #[serde(default, deserialize_with = "null_string_as_default")]
            model_id: String,
            #[serde(default, deserialize_with = "null_string_as_default")]
            model_name: String,
            #[serde(default, deserialize_with = "null_string_as_default")]
            description: String,
            provider: Option<String>,
            #[serde(default, deserialize_with = "null_string_vec_as_default")]
            capabilities: Vec<String>,
            context_window: Option<i64>,
            is_default: Option<bool>,
            rate_multiplier: Option<f64>,
            #[serde(default)]
            rate_unit: Option<String>,
            prompt_caching: Option<AvailableModelPromptCaching>,
            #[serde(default, deserialize_with = "null_string_vec_as_default")]
            supported_input_types: Vec<String>,
            token_limits: Option<AvailableModelTokenLimits>,
            #[serde(default)]
            additional_model_request_fields_schema: Option<serde_json::Value>,
            #[serde(default)]
            effort_levels: Vec<String>,
            #[serde(default)]
            effort_schema_path: Option<String>,
        }

        let wire = AvailableModelWire::deserialize(deserializer)?;
        let (schema_effort_levels, schema_effort_path) =
            extract_effort_metadata(&wire.additional_model_request_fields_schema);

        Ok(Self {
            model_id: wire.model_id,
            model_name: wire.model_name,
            description: wire.description,
            provider: wire.provider,
            capabilities: wire.capabilities,
            context_window: wire.context_window,
            is_default: wire.is_default,
            rate_multiplier: wire.rate_multiplier,
            rate_unit: wire.rate_unit,
            prompt_caching: wire.prompt_caching,
            supported_input_types: wire.supported_input_types,
            token_limits: wire.token_limits,
            additional_model_request_fields_schema: wire.additional_model_request_fields_schema,
            effort_levels: if wire.effort_levels.is_empty() {
                schema_effort_levels
            } else {
                wire.effort_levels
            },
            effort_schema_path: wire.effort_schema_path.or(schema_effort_path),
        })
    }
}
