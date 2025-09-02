// Copyright 2019-2024 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Compile-time and runtime types for Tauri plugins.
#[cfg(feature = "build")]
pub use build::*;

#[cfg(feature = "build")]
mod build {
  use std::{
    env::{vars_os, var},
    fs,
    path::{Path, PathBuf},
  };

  const GLOBAL_API_SCRIPT_PATH_KEY: &str = "GLOBAL_API_SCRIPT_PATH";
  /// Known file name of the file that contains an array with the path of all API scripts defined with [`define_global_api_script_path`].
  pub const GLOBAL_API_SCRIPT_FILE_LIST_PATH: &str = "__global-api-script.js";

  /// Defines the path to the global API script using Cargo instructions.
  pub fn define_global_api_script_path(path: &Path) {
    // NOTE(lreyna): We want paths to the paths that are stored in the `.depenv` output to be relative.
    // Otherwise, you might get a path that doesn't exist on your system (either an old sandbox or a path from remote cache on jenkins)
    // We get the canonical path (resolved symlinks) and get the relative path of the global script.
    // When the path is read later, it will be resolved with the same bazel output base path
    // i.e. Example Output: DEP_TAURI_PLUGIN_CORS_FETCH_GLOBAL_API_SCRIPT_PATH=external/tauri-deps__tauri-plugin-cors-fetch-4.1.0/api-iife.js
    let bazel_output_base = PathBuf::from(var("BAZEL_OUTPUT_BASE").expect("BAZEL_OUTPUT_BASE not set"));

    let resolved_path = if path.is_relative() {
      PathBuf::from(var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set")).join(path)
    } else {
      path.to_path_buf()
    };
  
    let canon_path = resolved_path.canonicalize().expect("failed to canonicalize global API script path");
    let cleaned_canon_path = crate::config::parse::clean_canonical_path(canon_path);
    let relative_path = cleaned_canon_path.strip_prefix(&bazel_output_base).expect("failed to get relative path of global API script");

    println!(
      "cargo:{GLOBAL_API_SCRIPT_PATH_KEY}={}",
      relative_path.display()
    )
  }

  /// Collects the path of all the global API scripts defined with [`define_global_api_script_path`]
  /// and saves them to the out dir with filename [`GLOBAL_API_SCRIPT_FILE_LIST_PATH`].
  ///
  /// `tauri_global_scripts` is only used in Tauri's monorepo for the examples to work
  /// since they don't have a build script to run `tauri-build` and pull in the deps env vars
  pub fn save_global_api_scripts_paths(out_dir: &Path, mut tauri_global_scripts: Option<PathBuf>) {
    let mut scripts = Vec::new();

    for (key, value) in vars_os() {
      let key = key.to_string_lossy();

      if key == format!("DEP_TAURI_{GLOBAL_API_SCRIPT_PATH_KEY}") {
        tauri_global_scripts = Some(PathBuf::from(value));
      } else if key.starts_with("DEP_") && key.ends_with(GLOBAL_API_SCRIPT_PATH_KEY) {
        let script_path = PathBuf::from(value);
        scripts.push(script_path);
      }
    }

    if let Some(tauri_global_scripts) = tauri_global_scripts {
      scripts.insert(0, tauri_global_scripts);
    }

    fs::write(
      out_dir.join(GLOBAL_API_SCRIPT_FILE_LIST_PATH),
      serde_json::to_string(&scripts).expect("failed to serialize global API script paths"),
    )
    .expect("failed to write global API script");
  }

  /// Read global api scripts from [`GLOBAL_API_SCRIPT_FILE_LIST_PATH`]
  pub fn read_global_api_scripts(out_dir: &Path) -> Option<Vec<String>> {
    let global_scripts_path = out_dir.join(GLOBAL_API_SCRIPT_FILE_LIST_PATH);
    if !global_scripts_path.exists() {
      return None;
    }

    let global_scripts_str = fs::read_to_string(global_scripts_path)
      .expect("failed to read plugin global API script paths");
    let global_scripts = serde_json::from_str::<Vec<PathBuf>>(&global_scripts_str)
      .expect("failed to parse plugin global API script paths");

    Some(
      global_scripts
        .into_iter()
        .map(|p| {
          fs::read_to_string(&p).unwrap_or_else(|e| {
            panic!(
              "failed to read plugin global API script {}: {e}",
              p.display()
            )
          })
        })
        .collect(),
    )
  }
}
