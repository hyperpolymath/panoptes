// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Ollama API client for local AI inference

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, warn};

use crate::{PanoptesError, Result};

/// Ollama API client
pub struct OllamaClient {
    client: Client,
    base_url: String,
}

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    name: String,
}

impl OllamaClient {
    /// Create a new Ollama client
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        // Normalize URL
        let base_url = base_url
            .trim_end_matches('/')
            .replace("/api/generate", "")
            .replace("/api/chat", "");

        Self { client, base_url }
    }

    /// Check if Ollama is available
    pub async fn health_check(&self) -> Result<()> {
        let url = format!("{}/api/tags", self.base_url);

        self.client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| {
                PanoptesError::OllamaUnavailable(format!(
                    "Cannot connect to Ollama at {}: {}",
                    self.base_url, e
                ))
            })?;

        Ok(())
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self.client
            .get(&url)
            .send()
            .await?;

        let tags: TagsResponse = response.json().await?;
        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }

    /// Check if a specific model is available
    pub async fn model_available(&self, model: &str) -> Result<bool> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| {
            m.starts_with(model) || m == &format!("{}:latest", model)
        }))
    }

    /// Generate text completion
    pub async fn generate(&self, model: &str, prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);

        let request = GenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
            images: None,
        };

        debug!("Sending request to Ollama: model={}", model);

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(PanoptesError::OllamaUnavailable(format!(
                "Ollama returned status {}",
                response.status()
            )));
        }

        let result: GenerateResponse = response.json().await?;
        Ok(result.response)
    }

    /// Generate with image (for vision models)
    pub async fn generate_with_image(
        &self,
        model: &str,
        prompt: &str,
        image_base64: &str,
    ) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);

        let request = GenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
            images: Some(vec![image_base64.to_string()]),
        };

        debug!("Sending vision request to Ollama: model={}", model);

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(PanoptesError::OllamaUnavailable(format!(
                "Ollama returned status {}",
                response.status()
            )));
        }

        let result: GenerateResponse = response.json().await?;
        Ok(result.response)
    }

    /// Generate with retry logic
    pub async fn generate_with_retry(
        &self,
        model: &str,
        prompt: &str,
        retries: u32,
    ) -> Result<String> {
        let mut last_error = None;

        for attempt in 0..=retries {
            if attempt > 0 {
                let delay = Duration::from_secs(2u64.pow(attempt - 1));
                warn!("Retrying Ollama request in {:?} (attempt {})", delay, attempt + 1);
                tokio::time::sleep(delay).await;
            }

            match self.generate(model, prompt).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            PanoptesError::OllamaUnavailable("Unknown error".to_string())
        }))
    }
}
