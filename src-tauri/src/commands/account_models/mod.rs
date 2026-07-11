// 命令层：可用模型(ListAvailableModels)数据模型 / 归一化 / 缓存 / 抓取
// 门面 pub use 用于保持旧路径稳定可达(FetchAvailableModelsResult/AvailableModel* 无按名消费者)，
// 会触发 unused_imports；此处 allow 以保持对外接口不变。
#![allow(unused_imports)]

mod cache;
mod fetch;
mod model;
mod response;

// 对外稳定接口(供 crate::commands::account_cmd 使用，路径必须不变):
pub use cache::{
    clear_available_models_cache, read_available_models_cache, write_available_models_cache,
};
pub use fetch::{fetch_all_available_models, FetchAvailableModelsResult};
pub use response::ListAvailableModelsResponse;

// model 侧的 pub 结构保守再导出，保持旧路径可达:
pub use model::{AvailableModel, AvailableModelPromptCaching, AvailableModelTokenLimits};

#[cfg(test)]
mod tests {
    use super::cache::{
        clear_available_models_cache, is_available_models_cache_fresh, read_available_models_cache,
        write_available_models_cache, AVAILABLE_MODELS_CACHE_TTL_SECONDS,
    };
    use super::model::AvailableModel;
    use super::response::{
        ensure_default_model_present, first_available_profile_arn, mark_default_model,
        normalize_list_available_models_response, sort_available_models_for_display,
        ListAvailableModelsResponse, ListAvailableProfilesResponse,
    };
    use crate::core::account::Account;

    #[test]
    fn deserialize_list_available_models_response_supports_known_fields() {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "models": [
                {
                    "modelId": "claude-sonnet-4.5",
                    "modelName": "Claude Sonnet 4.5",
                    "description": "The Claude Sonnet 4.5 model",
                    "rateMultiplier": 1.3,
                    "rateUnit": "Credit",
                    "supportedInputTypes": ["TEXT", "IMAGE"],
                    "tokenLimits": {
                        "maxInputTokens": 200000,
                        "maxOutputTokens": 64000
                    }
                }
            ],
            "nextToken": "page-2"
        }))
        .expect("response should deserialize");

        assert_eq!(response.available_models.len(), 1);
        assert_eq!(response.available_models[0].model_id, "claude-sonnet-4.5");
        assert_eq!(response.available_models[0].model_name, "Claude Sonnet 4.5");
        assert_eq!(
            response.available_models[0].supported_input_types,
            vec!["TEXT".to_string(), "IMAGE".to_string()]
        );
        assert_eq!(
            response.available_models[0]
                .token_limits
                .as_ref()
                .and_then(|limits| limits.max_input_tokens),
            Some(200000)
        );
        assert_eq!(
            response.available_models[0]
                .token_limits
                .as_ref()
                .and_then(|limits| limits.max_output_tokens),
            Some(64000)
        );
        assert_eq!(response.next_token.as_deref(), Some("page-2"));
    }

    #[test]
    fn deserialize_list_available_models_response_allows_null_next_token_and_nullable_display_fields(
    ) {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "models": [
                {
                    "modelId": "claude-sonnet-4.5",
                    "modelName": null,
                    "description": null,
                    "rateUnit": null
                }
            ],
            "nextToken": null
        }))
        .expect("response should deserialize null optional fields");

        assert_eq!(response.next_token, None);
        assert_eq!(response.available_models.len(), 1);
        assert_eq!(response.available_models[0].model_name, "");
        assert_eq!(response.available_models[0].description, "");
        assert_eq!(response.available_models[0].rate_unit, None);
    }

    #[test]
    fn deserialize_list_available_models_response_extracts_effort_levels_from_model_schema() {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "availableModels": [
                {
                    "modelId": "reasoning-model",
                    "modelName": "Reasoning Model",
                    "additionalModelRequestFieldsSchema": {
                        "type": "object",
                        "properties": {
                            "reasoning": {
                                "type": "object",
                                "properties": {
                                    "effort": {
                                        "type": "string",
                                        "enum": ["low", "medium", "high", "xhigh", "max"]
                                    }
                                }
                            }
                        }
                    }
                },
                {
                    "modelId": "plain-model",
                    "modelName": "Plain Model"
                }
            ],
            "nextToken": null
        }))
        .expect("response should deserialize effort metadata");

        assert_eq!(
            response.available_models[0].effort_levels,
            vec!["low", "medium", "high", "xhigh", "max"]
        );
        assert_eq!(
            response.available_models[0].effort_schema_path.as_deref(),
            Some("reasoning")
        );
        assert!(response.available_models[1].effort_levels.is_empty());
        assert_eq!(response.available_models[1].effort_schema_path, None);
    }

    #[test]
    fn deserialize_list_available_models_response_supports_full_default_model_shape() {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "models": [
                {
                    "modelId": "claude-sonnet-4",
                    "modelName": "Claude Sonnet 4",
                    "description": "Hybrid reasoning and coding for regular use",
                    "isDefault": true,
                    "promptCaching": {
                        "maximumCacheCheckpointsPerRequest": 4,
                        "minimumTokensPerCacheCheckpoint": 1024,
                        "supportsPromptCaching": true
                    },
                    "rateMultiplier": 1.3,
                    "rateUnit": "Credit",
                    "supportedInputTypes": ["TEXT", "IMAGE"],
                    "tokenLimits": {
                        "maxInputTokens": 200000,
                        "maxOutputTokens": 64000
                    }
                }
            ],
            "defaultModel": {
                "modelId": "claude-sonnet-4",
                "modelName": "Claude Sonnet 4",
                "description": "Hybrid reasoning and coding for regular use",
                "promptCaching": {
                    "maximumCacheCheckpointsPerRequest": 4,
                    "minimumTokensPerCacheCheckpoint": 1024,
                    "supportsPromptCaching": true
                },
                "rateMultiplier": 1.3,
                "rateUnit": "Credit",
                "supportedInputTypes": ["TEXT", "IMAGE"],
                "tokenLimits": {
                    "maxInputTokens": 200000,
                    "maxOutputTokens": 64000
                }
            }
        }))
        .expect("full response should deserialize");

        assert_eq!(response.available_models.len(), 1);
        assert_eq!(response.available_models[0].model_id, "claude-sonnet-4");
        assert_eq!(response.available_models[0].model_name, "Claude Sonnet 4");
        assert_eq!(
            response.available_models[0].description,
            "Hybrid reasoning and coding for regular use"
        );
        assert_eq!(response.available_models[0].is_default, Some(true));
        assert_eq!(
            response.available_models[0]
                .prompt_caching
                .as_ref()
                .and_then(|value| value.supports_prompt_caching),
            Some(true)
        );
        assert_eq!(
            response
                .default_model
                .as_ref()
                .map(|model| model.model_id.as_str()),
            Some("claude-sonnet-4")
        );
        assert_eq!(
            response
                .default_model
                .as_ref()
                .and_then(|model| model.prompt_caching.as_ref())
                .and_then(|value| value.minimum_tokens_per_cache_checkpoint),
            Some(1024)
        );
    }

    #[test]
    fn deserialize_list_available_models_response_supports_live_default_model_shape() {
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "description": "Models chosen by task for optimal usage and consistent quality",
                "modelId": "auto",
                "modelName": "Auto",
                "promptCaching": {
                    "maximumCacheCheckpointsPerRequest": 4,
                    "minimumTokensPerCacheCheckpoint": 1024,
                    "supportsPromptCaching": true
                },
                "rateMultiplier": 1.0,
                "rateUnit": "Credit",
                "supportedInputTypes": ["TEXT", "IMAGE"],
                "tokenLimits": {
                    "maxInputTokens": 200000,
                    "maxOutputTokens": 64000
                }
            },
            "models": [
                {
                    "description": "Models chosen by task for optimal usage and consistent quality",
                    "modelId": "auto",
                    "modelName": "Auto"
                }
            ],
            "nextToken": null
        }))
        .expect("live response shape should deserialize");

        let default_model = response
            .default_model
            .as_ref()
            .expect("default model should exist");
        assert_eq!(default_model.model_id, "auto");
        assert_eq!(default_model.model_name, "Auto");
        assert_eq!(
            default_model
                .prompt_caching
                .as_ref()
                .and_then(|value| value.supports_prompt_caching),
            Some(true)
        );
        assert_eq!(
            default_model
                .prompt_caching
                .as_ref()
                .and_then(|value| value.maximum_cache_checkpoints_per_request),
            Some(4)
        );
        assert_eq!(
            default_model
                .prompt_caching
                .as_ref()
                .and_then(|value| value.minimum_tokens_per_cache_checkpoint),
            Some(1024)
        );
        assert_eq!(
            default_model
                .token_limits
                .as_ref()
                .and_then(|limits| limits.max_output_tokens),
            Some(64000)
        );
    }

    #[test]
    fn sort_available_models_for_display_prioritizes_default_models() {
        let mut models: Vec<AvailableModel> = serde_json::from_value(serde_json::json!([
            {
                "modelId": "claude-sonnet-4.5",
                "modelName": "Claude Sonnet 4.5"
            },
            {
                "modelId": "auto",
                "modelName": "Auto",
                "isDefault": true
            },
            {
                "modelId": "claude-sonnet-4",
                "modelName": "Claude Sonnet 4"
            }
        ]))
        .expect("models should deserialize");

        sort_available_models_for_display(&mut models);

        let ordered_ids: Vec<_> = models.iter().map(|model| model.model_id.as_str()).collect();
        assert_eq!(
            ordered_ids,
            vec!["auto", "claude-sonnet-4.5", "claude-sonnet-4"]
        );
    }

    #[test]
    fn mark_default_model_sets_matching_entry() {
        let mut models: Vec<AvailableModel> = serde_json::from_value(serde_json::json!([
            { "modelId": "claude-sonnet-4.5", "modelName": "Claude Sonnet 4.5" },
            { "modelId": "auto", "modelName": "Auto" }
        ]))
        .expect("models should deserialize");

        mark_default_model(&mut models, Some("auto"));

        assert_eq!(models[0].is_default, None);
        assert_eq!(models[1].is_default, Some(true));
    }

    #[test]
    fn ensure_default_model_present_inserts_only_once() {
        let mut response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [
                {
                    "modelId": "claude-sonnet-4.5",
                    "modelName": "Claude Sonnet 4.5"
                }
            ],
            "nextToken": null
        }))
        .expect("response should deserialize");

        ensure_default_model_present(&mut response);
        ensure_default_model_present(&mut response);

        let auto_count = response
            .available_models
            .iter()
            .filter(|model| model.model_id == "auto")
            .count();
        assert_eq!(auto_count, 1);
        assert_eq!(
            response
                .available_models
                .first()
                .map(|model| model.model_id.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn normalize_list_available_models_response_expands_default_model_from_models_list() {
        let mut response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": { "modelId": "deepseek-3.2" },
            "models": [
                {
                    "description": "Experimental preview of DeepSeek V3.2",
                    "modelId": "deepseek-3.2",
                    "modelName": "Deepseek v3.2",
                    "promptCaching": { "supportsPromptCaching": false },
                    "rateMultiplier": 0.25,
                    "rateUnit": "Credit",
                    "supportedInputTypes": ["TEXT", "IMAGE"],
                    "tokenLimits": {
                        "maxInputTokens": 164000,
                        "maxOutputTokens": 64000
                    }
                },
                {
                    "description": "The MiniMax M2.5 model",
                    "modelId": "minimax-m2.5",
                    "modelName": "MiniMax M2.5",
                    "promptCaching": { "supportsPromptCaching": false },
                    "rateMultiplier": 0.25,
                    "rateUnit": "Credit",
                    "supportedInputTypes": ["TEXT"],
                    "tokenLimits": {
                        "maxInputTokens": 196000,
                        "maxOutputTokens": 64000
                    }
                }
            ],
            "nextToken": null
        }))
        .expect("captured response should deserialize");

        normalize_list_available_models_response(&mut response);

        assert_eq!(response.available_models.len(), 2);
        assert_eq!(response.available_models[0].model_id, "deepseek-3.2");
        assert_eq!(response.available_models[0].model_name, "Deepseek v3.2");
        assert_eq!(response.available_models[0].is_default, Some(true));
        assert_eq!(
            response
                .default_model
                .as_ref()
                .map(|model| model.model_name.as_str()),
            Some("Deepseek v3.2")
        );
        assert_eq!(
            response
                .default_model
                .as_ref()
                .and_then(|model| model.token_limits.as_ref())
                .and_then(|limits| limits.max_input_tokens),
            Some(164000)
        );
        assert_eq!(
            response
                .default_model
                .as_ref()
                .and_then(|model| model.prompt_caching.as_ref())
                .and_then(|prompt_caching| prompt_caching.supports_prompt_caching),
            Some(false)
        );
    }

    #[test]
    fn available_models_cache_round_trips_response() {
        let mut account = Account::new("cache@example.com".to_string(), "cache".to_string());
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [
                {
                    "modelId": "auto",
                    "modelName": "Auto"
                },
                {
                    "modelId": "claude-sonnet-4.5",
                    "modelName": "Claude Sonnet 4.5"
                }
            ],
            "nextToken": null
        }))
        .expect("response should deserialize");

        write_available_models_cache(&mut account, &response).expect("cache write should succeed");
        let cached =
            read_available_models_cache(&account, false).expect("cache should be readable");

        assert_eq!(cached.available_models.len(), 2);
        assert_eq!(
            cached
                .default_model
                .as_ref()
                .map(|model| model.model_id.as_str()),
            Some("auto")
        );
    }

    #[test]
    fn first_available_profile_arn_skips_empty_profiles() {
        let response: ListAvailableProfilesResponse = serde_json::from_value(serde_json::json!({
            "profiles": [
                { "arn": "   " },
                { "arn": null },
                { "arn": "arn:aws:codewhisperer:eu-central-1:123456789012:profile/REAL" }
            ]
        }))
        .expect("profiles response should deserialize");

        assert_eq!(
            first_available_profile_arn(response).as_deref(),
            Some("arn:aws:codewhisperer:eu-central-1:123456789012:profile/REAL")
        );
    }

    #[test]
    fn available_models_cache_expires_after_ttl() {
        assert!(is_available_models_cache_fresh(
            100,
            100 + AVAILABLE_MODELS_CACHE_TTL_SECONDS
        ));
        assert!(!is_available_models_cache_fresh(
            100,
            101 + AVAILABLE_MODELS_CACHE_TTL_SECONDS
        ));
    }

    #[test]
    fn clear_available_models_cache_removes_cached_response() {
        let mut account = Account::new("cache@example.com".to_string(), "cache".to_string());
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [],
            "nextToken": null
        }))
        .expect("response should deserialize");

        write_available_models_cache(&mut account, &response).expect("cache write should succeed");
        clear_available_models_cache(&mut account);

        assert!(read_available_models_cache(&account, false).is_none());
    }

    #[test]
    fn available_models_cache_skips_when_force_refresh_enabled() {
        let mut account = Account::new("cache@example.com".to_string(), "cache".to_string());
        let response: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "defaultModel": {
                "modelId": "auto",
                "modelName": "Auto"
            },
            "models": [],
            "nextToken": null
        }))
        .expect("response should deserialize");

        write_available_models_cache(&mut account, &response).expect("cache write should succeed");

        assert!(read_available_models_cache(&account, true).is_none());
    }

    #[test]
    fn deserialize_list_available_models_response_supports_both_models_and_available_models() {
        // 测试 AWS API 格式（models）
        let response_api: ListAvailableModelsResponse = serde_json::from_value(serde_json::json!({
            "models": [
                {
                    "modelId": "auto",
                    "modelName": "Auto"
                }
            ],
            "nextToken": null
        }))
        .expect("API format (models) should deserialize");
        assert_eq!(response_api.available_models.len(), 1);
        assert_eq!(response_api.available_models[0].model_id, "auto");

        // 测试缓存格式（availableModels）
        let response_cache: ListAvailableModelsResponse =
            serde_json::from_value(serde_json::json!({
                "availableModels": [
                    {
                        "modelId": "claude-sonnet-4.5",
                        "modelName": "Claude Sonnet 4.5"
                    }
                ],
                "nextToken": null
            }))
            .expect("Cache format (availableModels) should deserialize");
        assert_eq!(response_cache.available_models.len(), 1);
        assert_eq!(
            response_cache.available_models[0].model_id,
            "claude-sonnet-4.5"
        );
    }
}
