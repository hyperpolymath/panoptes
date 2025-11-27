# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>

# Panoptes Configuration
# Nickel configuration with type contracts for validation

let Config = {
  # The directory to watch for new files
  watch_path
    | String
    | doc "Directory path to monitor for incoming files"
    = "/var/home/core/Downloads/scan_input",

  # Connection details for the AI Engine (Ollama)
  ai_engine
    | {
        url | String | doc "Ollama API endpoint URL",
        model | String | doc "Vision model to use (moondream recommended)",
      }
    = {
        url = "http://localhost:11434/api/generate",
        model = "moondream",
      },

  # Rules for filename generation
  rules
    | {
        sanitize | Bool | doc "Convert spaces to underscores",
        date_prefix | Bool | doc "Prefix files with ISO date (YYYY-MM-DD)",
        max_length | Number | doc "Maximum length of generated filename",
      }
    = {
        sanitize = true,
        date_prefix = true,
        max_length = 50,
      },

  # Prompt engineering for the Vision Model
  prompts
    | {
        image | String | doc "Prompt for image analysis",
        document | String | doc "Prompt for document text analysis",
      }
    = {
        image = m%"
          Analyze this image and generate a concise, descriptive filename
          (max 5 words). Use snake_case. Do not include the file extension.
          Return ONLY the filename, nothing else.
        "%,
        document = m%"
          Summarize the header or title of this document text into a
          concise filename (max 5 words). Use snake_case. Return ONLY
          the filename, nothing else.
        "%,
      },
}
in Config
