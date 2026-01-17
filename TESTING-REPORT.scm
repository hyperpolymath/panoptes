;; SPDX-License-Identifier: MIT
;; SPDX-FileCopyrightText: 2025 Jonathan D. A. Jewell <hyperpolymath>
;;
;; Panoptes Testing Report - Guile Scheme Format
;; Generated: 2025-12-29
;; Generator: Claude Code Autonomous Testing

(testing-report
  (metadata
    (project "panoptes")
    (version "3.0.0")
    (test-date "2025-12-29")
    (schema-version "1.0.0")
    (generator "claude-code-autonomous")
    (report-type "comprehensive"))

  (summary
    (status pass)
    (overall-health good)
    (issues-found 1)
    (issues-fixed 1)
    (tests-run 3)
    (tests-passed 3)
    (tests-failed 0)
    (build-status success)
    (build-duration-seconds 635))

  (build-results
    (target dev)
    (profile debug)
    (crates-compiled 434)
    (warnings
      (library 21)
      (binary 2)
      (total 23))
    (errors 0)
    (success #t))

  (test-results
    (test-suites
      (test-suite
        (name "lib")
        (file "src/lib.rs")
        (passed 0)
        (failed 0)
        (ignored 0))
      (test-suite
        (name "main")
        (file "src/main.rs")
        (passed 3)
        (failed 0)
        (ignored 0)
        (tests
          (test (name "test_cli_parsing") (status pass))
          (test (name "test_cli_analyze_command") (status pass))
          (test (name "test_cli_watch_command") (status pass))))
      (test-suite
        (name "panoptes-undo")
        (file "src/bin/panoptes-undo.rs")
        (passed 0)
        (failed 0)
        (ignored 0))
      (test-suite
        (name "panoptes-web")
        (file "src/bin/panoptes-web.rs")
        (passed 0)
        (failed 0)
        (ignored 0))
      (test-suite
        (name "doctests")
        (passed 0)
        (failed 0)
        (ignored 0))))

  (issues
    (issue
      (id "PANOPTES-2025-001")
      (severity medium)
      (type bug)
      (status fixed)
      (title "CLI short option conflict in panoptes-undo")
      (description "Short option -h was used for --history-file but conflicts with auto-generated --help flag")
      (location "src/bin/panoptes-undo.rs:20")
      (symptom "Panic on panoptes-undo --help with message about duplicate short option")
      (fix
        (before "#[arg(short, long, default_value = \"panoptes_history.jsonl\")]")
        (after "#[arg(short = 'H', long, default_value = \"panoptes_history.jsonl\")]"))
      (verified #t)))

  (runtime-verification
    (binary
      (name "panoptes")
      (help-works #t)
      (subcommands
        (subcommand (name "watch") (status verified))
        (subcommand (name "analyze") (status verified))
        (subcommand (name "db") (status verified))
        (subcommand (name "history") (status verified))
        (subcommand (name "config") (status verified))
        (subcommand (name "status") (status verified))
        (subcommand (name "init") (status verified))))
    (binary
      (name "panoptes-web")
      (help-works #t))
    (binary
      (name "panoptes-undo")
      (help-works #t)))

  (static-analysis
    (tool "clippy")
    (warnings 31)
    (errors 0)
    (warning-categories
      (category (name "unused_imports") (count 19))
      (category (name "unused_variables") (count 2))
      (category (name "dead_code") (count 1))
      (category (name "field_reassign_with_default") (count 2))
      (category (name "redundant_closure") (count 4))
      (category (name "collapsible_if") (count 1))
      (category (name "too_many_arguments") (count 1))))

  (project-structure
    (binaries
      (binary (name "panoptes") (main #t))
      (binary (name "panoptes-web"))
      (binary (name "panoptes-undo")))
    (library
      (name "panoptes")
      (modules
        (module (name "config"))
        (module (name "db"))
        (module (name "error"))
        (module (name "history"))
        (module (name "ollama"))
        (module (name "watcher"))
        (module (name "web"))
        (module (name "analyzers")
          (submodules
            (submodule (name "image"))
            (submodule (name "pdf"))
            (submodule (name "audio"))
            (submodule (name "video"))
            (submodule (name "code"))
            (submodule (name "document"))
            (submodule (name "archive")))))))

  (dependencies
    (total-count 434)
    (key-dependencies
      (dependency (name "axum") (purpose "web-framework"))
      (dependency (name "tokio") (purpose "async-runtime"))
      (dependency (name "reqwest") (purpose "http-client"))
      (dependency (name "rusqlite") (purpose "database"))
      (dependency (name "image") (purpose "image-processing"))
      (dependency (name "pdf-extract") (purpose "pdf-text-extraction"))
      (dependency (name "lopdf") (purpose "pdf-metadata"))
      (dependency (name "symphonia") (purpose "audio-decoding"))
      (dependency (name "id3") (purpose "audio-metadata"))
      (dependency (name "tree-sitter") (purpose "code-parsing"))
      (dependency (name "clap") (purpose "cli-parsing"))
      (dependency (name "serde") (purpose "serialization"))
      (dependency (name "tracing") (purpose "logging"))))

  (recommendations
    (recommendation
      (priority low)
      (type code-cleanup)
      (description "Remove unused imports with cargo fix"))
    (recommendation
      (priority low)
      (type code-cleanup)
      (description "Remove or use extract_function_name_fixed function"))
    (recommendation
      (priority medium)
      (type test-coverage)
      (description "Add unit tests for analyzer modules"))
    (recommendation
      (priority low)
      (type code-style)
      (description "Refactor insert_file to use struct parameter instead of 8 arguments")))

  (conclusion
    (production-ready #f)
    (development-ready #t)
    (notes
      "Project builds and runs successfully. All tests pass. One bug was found and fixed. "
      "Recommended for development use. Minor code cleanup suggested before production release.")))
