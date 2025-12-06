//! Cloudflare AutoRAG (AI Search) Client
//!
//! Direct REST API integration for historical knowledge search.
//! Ported from evna/src/lib/autorag-client.ts

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// AutoRAG search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Search query
    pub query: String,
    /// RAG instance ID (default: "sysops-beta")
    pub rag_id: String,
    /// Maximum results to return (default: 10)
    pub max_results: usize,
    /// Enable query rewriting for better retrieval (default: true)
    pub rewrite_query: bool,
    /// Minimum score threshold (default: 0.3)
    pub score_threshold: f64,
    /// Enable BGE reranking (default: true)
    pub enable_reranking: bool,
    /// Filter by folder prefix (e.g., "bridges/")
    pub folder_filter: Option<String>,
    /// Model for AI search synthesis (default: llama-3.3-70b)
    pub model: String,
    /// System prompt for generating answer
    pub system_prompt: Option<String>,
    /// Model for reranking (default: bge-reranker-base)
    pub rerank_model: String,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            query: String::new(),
            rag_id: "sysops-beta".to_string(),
            max_results: 10,
            rewrite_query: true,
            score_threshold: 0.3,
            enable_reranking: true,
            folder_filter: None,
            model: "@cf/meta/llama-3.3-70b-instruct-fp8-fast".to_string(),
            system_prompt: None,
            rerank_model: "@cf/baai/bge-reranker-base".to_string(),
        }
    }
}

/// Search result from AutoRAG
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    pub file_id: String,
    pub filename: String,
    pub score: f64,
    pub attributes: ResultAttributes,
    pub content: Vec<ContentChunk>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResultAttributes {
    pub modified_date: Option<i64>,
    pub folder: Option<String>,
    pub file: Option<FileInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileInfo {
    pub url: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentChunk {
    pub id: String,
    #[serde(rename = "type")]
    pub chunk_type: String,
    pub text: String,
}

/// AI Search response (includes synthesized answer)
#[derive(Debug, Clone)]
pub struct AiSearchResponse {
    pub answer: String,
    pub sources: Vec<SearchResult>,
    pub search_query: String,
}

/// Raw API response structure
#[derive(Debug, Deserialize)]
struct ApiResponse {
    success: bool,
    result: ApiResult,
}

#[derive(Debug, Deserialize)]
struct ApiResult {
    search_query: String,
    response: Option<String>,
    data: Vec<SearchResult>,
    has_more: bool,
    next_page: Option<String>,
}

/// Request body for API
#[derive(Debug, Serialize)]
struct SearchRequest {
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_prompt: Option<String>,
    rewrite_query: bool,
    max_num_results: usize,
    ranking_options: RankingOptions,
    reranking: RerankingOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<FilterSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize)]
struct RankingOptions {
    score_threshold: f64,
}

#[derive(Debug, Serialize)]
struct RerankingOptions {
    enabled: bool,
    model: String,
}

#[derive(Debug, Serialize)]
struct FilterSpec {
    #[serde(rename = "type")]
    filter_type: String,
    filters: Vec<FilterCondition>,
}

#[derive(Debug, Serialize)]
struct FilterCondition {
    #[serde(rename = "type")]
    condition_type: String,
    key: String,
    value: String,
}

/// Cloudflare AutoRAG Client
pub struct AutoRAGClient {
    client: Client,
    account_id: String,
    api_token: String,
    base_url: String,
}

impl AutoRAGClient {
    /// Create a new AutoRAG client
    pub fn new(account_id: impl Into<String>, api_token: impl Into<String>) -> Self {
        let account_id = account_id.into();
        let base_url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/autorag/rags",
            account_id
        );
        Self {
            client: Client::new(),
            account_id,
            api_token: api_token.into(),
            base_url,
        }
    }

    /// Create client from environment variables
    /// Reads CLOUDFLARE_ACCOUNT_ID and CLOUDFLARE_API_TOKEN (or AUTORAG_API_TOKEN)
    pub fn from_env() -> Result<Self> {
        let account_id = std::env::var("CLOUDFLARE_ACCOUNT_ID")
            .context("CLOUDFLARE_ACCOUNT_ID not set")?;
        // Try CLOUDFLARE_API_TOKEN first, then AUTORAG_API_TOKEN for compatibility
        let api_token = std::env::var("CLOUDFLARE_API_TOKEN")
            .or_else(|_| std::env::var("AUTORAG_API_TOKEN"))
            .context("CLOUDFLARE_API_TOKEN or AUTORAG_API_TOKEN not set")?;
        Ok(Self::new(account_id, api_token))
    }

    /// AI Search - Retrieval + LLM synthesis
    /// Returns synthesized answer + source documents
    pub async fn ai_search(&self, options: SearchOptions) -> Result<AiSearchResponse> {
        let url = format!("{}/{}/ai-search", self.base_url, options.rag_id);

        let request = self.build_request(&options, true);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&request)
            .send()
            .await
            .context("Failed to send ai-search request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            // Truncate error response to avoid leaking sensitive API details in logs
            let truncated = if error_text.len() > 500 {
                format!("{}...", &error_text[..500])
            } else {
                error_text
            };
            anyhow::bail!("AutoRAG ai-search failed ({}): {}", status, truncated);
        }

        let data: ApiResponse = response.json().await.context("Failed to parse response")?;

        Ok(AiSearchResponse {
            answer: data.result.response.unwrap_or_else(|| "No answer generated".to_string()),
            sources: data.result.data,
            search_query: data.result.search_query,
        })
    }

    /// Search only - Retrieval without LLM synthesis
    /// Returns raw document chunks
    pub async fn search(&self, options: SearchOptions) -> Result<Vec<SearchResult>> {
        let url = format!("{}/{}/search", self.base_url, options.rag_id);

        let request = self.build_request(&options, false);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&request)
            .send()
            .await
            .context("Failed to send search request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            // Truncate error response to avoid leaking sensitive API details in logs
            let truncated = if error_text.len() > 500 {
                format!("{}...", &error_text[..500])
            } else {
                error_text
            };
            anyhow::bail!("AutoRAG search failed ({}): {}", status, truncated);
        }

        let data: ApiResponse = response.json().await.context("Failed to parse response")?;

        Ok(data.result.data)
    }

    fn build_request(&self, options: &SearchOptions, include_model: bool) -> SearchRequest {
        let filters = options.folder_filter.as_ref().map(|folder| {
            // WORKAROUND: Cloudflare AutoRAG has no `startswith` operator.
            // We simulate prefix matching using ASCII range: gt "folder/" excludes exact match
            // but includes "folder/a...", while lte "folderz" caps before "foldera...".
            // LIMITATION: Fails for folders starting with 'z' or special chars after 'z'.
            // See: https://developers.cloudflare.com/ai-search/configuration/metadata/
            FilterSpec {
                filter_type: "and".to_string(),
                filters: vec![
                    FilterCondition {
                        condition_type: "gt".to_string(),
                        key: "folder".to_string(),
                        value: format!("{}/", folder),
                    },
                    FilterCondition {
                        condition_type: "lte".to_string(),
                        key: "folder".to_string(),
                        value: format!("{}z", folder),
                    },
                ],
            }
        });

        SearchRequest {
            query: options.query.clone(),
            model: if include_model {
                Some(options.model.clone())
            } else {
                None
            },
            system_prompt: if include_model {
                options.system_prompt.clone()
            } else {
                None
            },
            rewrite_query: options.rewrite_query,
            max_num_results: options.max_results,
            ranking_options: RankingOptions {
                score_threshold: options.score_threshold,
            },
            reranking: RerankingOptions {
                enabled: options.enable_reranking,
                model: options.rerank_model.clone(),
            },
            filters,
            stream: if include_model { Some(false) } else { None },
        }
    }

    /// Format results as markdown for display
    pub fn format_results(answer: &str, sources: &[SearchResult]) -> String {
        let mut output = format!("## AI Search Results\n\n{}\n\n", answer);

        if !sources.is_empty() {
            output.push_str(&format!("### Sources ({})\n\n", sources.len()));
            for (i, source) in sources.iter().enumerate() {
                let folder = source.attributes.folder.as_deref().unwrap_or("");
                let score = (source.score * 100.0).round() as i32;
                output.push_str(&format!(
                    "{}. **{}** ({}% match)\n",
                    i + 1,
                    source.filename,
                    score
                ));
                output.push_str(&format!("   Folder: {}\n", folder));
                if let Some(chunk) = source.content.first() {
                    let preview: String = chunk.text.chars().take(200).collect();
                    output.push_str(&format!("   Preview: {}...\n", preview));
                }
                output.push('\n');
            }
        }

        output
    }

    /// Format results as JSON for machine consumption
    pub fn format_json(answer: &str, sources: &[SearchResult]) -> Result<String> {
        #[derive(Serialize)]
        struct JsonOutput<'a> {
            answer: &'a str,
            sources: &'a [SearchResult],
        }
        let output = JsonOutput { answer, sources };
        serde_json::to_string_pretty(&output).context("Failed to serialize to JSON")
    }
}

// Implement Serialize for SearchResult so we can output JSON
impl Serialize for SearchResult {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SearchResult", 5)?;
        state.serialize_field("file_id", &self.file_id)?;
        state.serialize_field("filename", &self.filename)?;
        state.serialize_field("score", &self.score)?;
        state.serialize_field("folder", &self.attributes.folder)?;
        // Serialize first content chunk text as preview
        let preview = self.content.first().map(|c| &c.text);
        state.serialize_field("preview", &preview)?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = SearchOptions::default();
        assert_eq!(opts.rag_id, "sysops-beta");
        assert_eq!(opts.max_results, 10);
        assert!(opts.rewrite_query);
    }

    #[test]
    fn test_format_results() {
        let answer = "Test answer";
        let sources = vec![];
        let output = AutoRAGClient::format_results(answer, &sources);
        assert!(output.contains("Test answer"));
    }
}
