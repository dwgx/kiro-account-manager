use super::model::AvailableModel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAvailableModelsResponse {
    #[serde(
        default,
        alias = "models",
        deserialize_with = "super::model::null_models_as_default"
    )]
    pub available_models: Vec<AvailableModel>,
    pub next_token: Option<String>,
    pub default_model: Option<AvailableModel>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AvailableProfile {
    #[serde(default)]
    arn: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListAvailableProfilesResponse {
    #[serde(default)]
    profiles: Vec<AvailableProfile>,
}

pub(super) fn first_available_profile_arn(
    response: ListAvailableProfilesResponse,
) -> Option<String> {
    response.profiles.into_iter().find_map(|profile| {
        profile
            .arn
            .map(|arn| arn.trim().to_string())
            .filter(|arn| !arn.is_empty())
    })
}

pub(super) fn normalize_list_available_models_response(response: &mut ListAvailableModelsResponse) {
    response
        .available_models
        .retain(|model| !model.model_id.trim().is_empty());

    let default_model_id = response
        .default_model
        .as_ref()
        .map(|model| model.model_id.trim().to_string())
        .filter(|model_id| !model_id.is_empty());

    if let Some(default_id) = default_model_id.as_deref() {
        mark_default_model(&mut response.available_models, Some(default_id));

        if let Some(default_from_list) = response
            .available_models
            .iter()
            .find(|model| model.model_id == default_id)
            .cloned()
        {
            response.default_model = Some(default_from_list);
        }
    }

    if let Some(default_model) = response.default_model.as_mut() {
        default_model.is_default = Some(true);
    }

    ensure_default_model_present(response);
    sort_available_models_for_display(&mut response.available_models);
}

pub(super) fn mark_default_model(models: &mut [AvailableModel], default_model_id: Option<&str>) {
    if let Some(default_id) = default_model_id {
        for model in models {
            if model.model_id == default_id {
                model.is_default = Some(true);
            }
        }
    }
}

pub(super) fn ensure_default_model_present(response: &mut ListAvailableModelsResponse) {
    if let Some(default_model) = response.default_model.clone() {
        if !default_model.model_id.trim().is_empty()
            && response
                .available_models
                .iter()
                .all(|model| model.model_id != default_model.model_id)
        {
            response.available_models.insert(0, default_model);
        }
    }
}

pub(super) fn sort_available_models_for_display(models: &mut [AvailableModel]) {
    models.sort_by_key(|model| !model.is_default.unwrap_or(false));
}
