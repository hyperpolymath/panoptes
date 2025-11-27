// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

//! Source code file analyzer using tree-sitter

use async_trait::async_trait;
use std::path::Path;
use tracing::{debug, info, warn};

use super::{AnalysisResult, FileAnalyzer, calculate_file_hash, clean_filename, infer_category, extract_tags};
use crate::{AppConfig, Result, PanoptesError};
use crate::ollama::OllamaClient;

/// Analyzer for source code files
pub struct CodeAnalyzer;

impl CodeAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Detect language from extension
    fn detect_language(path: &Path) -> Option<&'static str> {
        let ext = path.extension()?.to_str()?;
        match ext.to_lowercase().as_str() {
            "rs" => Some("rust"),
            "py" => Some("python"),
            "js" | "mjs" => Some("javascript"),
            "ts" | "tsx" => Some("typescript"),
            "go" => Some("go"),
            "java" => Some("java"),
            "c" | "h" => Some("c"),
            "cpp" | "hpp" | "cc" | "cxx" => Some("cpp"),
            "rb" => Some("ruby"),
            "php" => Some("php"),
            "swift" => Some("swift"),
            "kt" | "kts" => Some("kotlin"),
            "scala" => Some("scala"),
            "ex" | "exs" => Some("elixir"),
            "hs" => Some("haskell"),
            "sh" | "bash" | "zsh" => Some("shell"),
            "sql" => Some("sql"),
            _ => None,
        }
    }

    /// Extract code structure summary
    fn extract_structure(content: &str, language: &str) -> CodeStructure {
        let mut structure = CodeStructure::default();

        // Simple pattern matching for common structures
        // In a full implementation, we'd use tree-sitter parsers

        let lines: Vec<&str> = content.lines().collect();
        structure.line_count = lines.len();

        for line in &lines {
            let trimmed = line.trim();

            // Count comments
            if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("--") {
                structure.comment_lines += 1;
            }

            // Detect main entry points
            if trimmed.contains("fn main") || trimmed.contains("def main") ||
               trimmed.contains("function main") || trimmed.contains("public static void main") {
                structure.has_main = true;
            }

            // Count function definitions (simplified)
            if (language == "rust" && trimmed.starts_with("fn ")) ||
               (language == "python" && trimmed.starts_with("def ")) ||
               (language == "javascript" && (trimmed.starts_with("function ") || trimmed.contains("=> {"))) ||
               (language == "go" && trimmed.starts_with("func ")) {
                structure.function_count += 1;

                // Extract function name
                if let Some(name) = Self::extract_function_name(trimmed, language) {
                    structure.functions.push(name);
                }
            }

            // Count class/struct definitions
            if trimmed.starts_with("class ") || trimmed.starts_with("struct ") ||
               trimmed.starts_with("interface ") || trimmed.starts_with("trait ") {
                structure.class_count += 1;
            }

            // Detect imports/includes
            if trimmed.starts_with("import ") || trimmed.starts_with("use ") ||
               trimmed.starts_with("from ") || trimmed.starts_with("#include") ||
               trimmed.starts_with("require") {
                structure.import_count += 1;
            }
        }

        structure
    }

    fn extract_function_name(line: &str, language: &str) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        match language {
            "rust" => {
                // fn function_name(...
                if parts.len() >= 2 && parts[0] == "fn" {
                    let name = parts[1].split('(').next()?;
                    if name != "main" && !name.starts_with("test_") {
                        return Some(name.to_string());
                    }
                }
            }
            "python" => {
                // def function_name(...
                if parts.len() >= 2 && parts[0] == "def" {
                    let name = parts[1].split('(').next()?;
                    if name != "__init__" && !name.starts_with("_") {
                        return Some(name.to_string());
                    }
                }
            }
            "javascript" | "typescript" => {
                // function name(...
                if parts.len() >= 2 && parts[0] == "function" {
                    let name = parts[1].split('(').next()?;
                    return Some(name.to_string());
                }
            }
            _ => {}
        }
        None
    }
}

#[derive(Default, Debug)]
struct CodeStructure {
    line_count: usize,
    comment_lines: usize,
    function_count: usize,
    class_count: usize,
    import_count: usize,
    has_main: bool,
    functions: Vec<String>,
}

// Fix the startswith typo
impl CodeAnalyzer {
    fn extract_function_name_fixed(line: &str, language: &str) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        match language {
            "rust" => {
                if parts.len() >= 2 && parts[0] == "fn" {
                    let name = parts[1].split('(').next()?;
                    if name != "main" && !name.starts_with("test_") {
                        return Some(name.to_string());
                    }
                }
            }
            "python" => {
                if parts.len() >= 2 && parts[0] == "def" {
                    let name = parts[1].split('(').next()?;
                    if name != "__init__" && !name.starts_with("_") {
                        return Some(name.to_string());
                    }
                }
            }
            "javascript" | "typescript" => {
                if parts.len() >= 2 && parts[0] == "function" {
                    let name = parts[1].split('(').next()?;
                    return Some(name.to_string());
                }
            }
            _ => {}
        }
        None
    }
}

impl Default for CodeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileAnalyzer for CodeAnalyzer {
    fn name(&self) -> &'static str {
        "code"
    }

    fn supported_extensions(&self) -> &[&str] {
        &[
            "rs", "py", "js", "mjs", "ts", "tsx", "go", "java",
            "c", "h", "cpp", "hpp", "cc", "cxx",
            "rb", "php", "swift", "kt", "kts", "scala",
            "ex", "exs", "hs", "sh", "bash", "zsh", "sql"
        ]
    }

    fn priority(&self) -> u8 {
        60
    }

    async fn analyze(&self, path: &Path, config: &AppConfig) -> Result<AnalysisResult> {
        info!("Analyzing code: {:?}", path);

        let file_hash = calculate_file_hash(path)?;
        let content = std::fs::read_to_string(path)?;
        let language = Self::detect_language(path).unwrap_or("unknown");
        let structure = Self::extract_structure(&content, language);

        let metadata = serde_json::json!({
            "language": language,
            "line_count": structure.line_count,
            "comment_lines": structure.comment_lines,
            "function_count": structure.function_count,
            "class_count": structure.class_count,
            "import_count": structure.import_count,
            "has_main": structure.has_main,
            "top_functions": structure.functions.iter().take(5).collect::<Vec<_>>(),
        });

        // Build a summary for the LLM
        let summary = format!(
            "Language: {}\nLines: {}\nFunctions: {}\nClasses: {}\nHas main: {}\nTop functions: {:?}",
            language,
            structure.line_count,
            structure.function_count,
            structure.class_count,
            structure.has_main,
            structure.functions.iter().take(3).collect::<Vec<_>>()
        );

        // Use code model for analysis
        let client = OllamaClient::new(&config.ai_engine.url);
        let prompt = format!(
            "{}\n\nCode summary:\n{}\n\nFirst 50 lines:\n{}",
            config.prompts.code,
            summary,
            content.lines().take(50).collect::<Vec<_>>().join("\n")
        );

        let suggested_name = match client.generate(&config.ai_engine.models.code, &prompt).await {
            Ok(response) => {
                let name = clean_filename(&response);
                if name.is_empty() {
                    // Fallback: use primary function name or language
                    structure.functions.first()
                        .map(|f| format!("{}_{}", f, language))
                        .unwrap_or_else(|| format!("{}_code", language))
                } else {
                    name
                }
            }
            Err(e) => {
                warn!("Code model failed: {}", e);
                structure.functions.first()
                    .map(|f| format!("{}_{}", f, language))
                    .unwrap_or_else(|| format!("{}_code", language))
            }
        };

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt");
        let category = Some("Code".to_string());

        let mut tags = vec![language.to_string()];
        if structure.has_main {
            tags.push("executable".to_string());
        }
        tags.extend(extract_tags(&suggested_name, &metadata));

        Ok(AnalysisResult {
            suggested_name,
            confidence: 0.70,
            category,
            tags,
            file_hash,
            metadata,
        })
    }
}
