# Tool Catalog — Xplorer commands → MCP tools

`mcp_host.rs` currently exposes six tools. This is the target surface. Every
row: validate args → delegate to the existing registered Tauri command → write
an audit entry → return structured JSON.

## Already implemented (verify, then harden)

| MCP tool | Status |
|---|---|
| `read_file` | exists — add path guard + size cap |
| `write_file` | exists — add version snapshot before write |
| `list_directory` | exists — add pagination |
| `search_files` | exists — add result cap |
| `run_command` | exists — **must** route through `is_blocked_command` + `is_network_command` |
| `get_git_status` | exists |

## Phase 8 additions

### Mutation (all require approval; all reversible)

| MCP tool | Backing command | Guard notes |
|---|---|---|
| `move_file` | `operations::move_file` / `move_with_progress` | validate src **and** dest; re-key path-scoped metadata |
| `copy_file` | `operations::copy` / `copy_with_progress` | dest guard |
| `rename` | `operations::rename` | reject reserved Windows names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`) |
| `bulk_rename` | `operations::bulk_rename` | dry-run first; cap batch size |
| `create_directory` | `operations::create_dir_recursive` | |
| `trash` | `operations::move_to_trash` | **the only deletion path** |
| `restore_from_trash` | `operations::restore_trash_item` | |
| `set_tags` | `storage::set_file_tags`, `batch_add_tags`, `batch_remove_tags` | |
| `set_note` | `storage::add_file_note` / `batch_set_notes` | |
| `set_metadata` | `storage::set_file_metadata` | |

### Analysis (read-only, no approval needed)

| MCP tool | Backing command |
|---|---|
| `get_file_properties` | `operations::get_detailed_file_properties` |
| `get_directory_size` | `operations::get_directory_size`, `get_directory_item_count` |
| `analyze_storage` | `operations::analyze_storage` |
| `find_duplicates` | `duplicate_finder::find_duplicates` |
| `compute_hash` | `operations::compute_file_hash` |
| `extract_text` | `operations::extract_document_text` (PDF/DOCX/XLSX/PPTX) |
| `get_image_info` | `operations::get_image_info` |
| `check_conflicts` | `operations::file_ops::check_conflicts` |
| `get_tags` | `storage::get_file_tags_batch`, `get_all_file_tags` |

### Retrieval (the reason this base was chosen)

| MCP tool | Backing command |
|---|---|
| `semantic_search` | `search::compat_commands::semantic_search` |
| `hybrid_search` | `search::compat_commands::hybrid_search` |
| `find_similar_files` | `search::compat_commands::find_similar_files` |
| `natural_language_search` | `search::compat_commands::natural_language_search` |
| `grep` | `operations::grep_search` / `search_in_files` |
| `index_status` | `search::compat_commands::get_ai_index_status` |

### Organization (the preview/commit pair)

| MCP tool | Backing command | Notes |
|---|---|---|
| `analyze_directory` | `file_organizer::analyze_directory` | read-only |
| `preview_organization` | `file_organizer::preview_organization` | **returns the plan** — surface as `plan` update |
| `execute_organization` | `file_organizer::execute_organization` | requires approval of the previewed plan id |

### Reversibility (expose to agent for self-correction)

| MCP tool | Backing command |
|---|---|
| `undo_last` | `operations::undo_redo::undo_operation` |
| `get_undo_history` | `operations::undo_redo::get_undo_history` |
| `list_versions` | `file_versions::list_versions` |
| `restore_version` | `file_versions::restore_version` |

## Never expose

`remove_file`, `remove_dir`, `empty_recycle_bin`,
`permanently_delete_trash_item`, `secure_delete`, `set_file_permissions`,
`set_default_folder_handler`, `add_context_menu_entry`,
`remove_context_menu_entry`, `install_cli`, `execute_sqlite_query`,
`update_agent_permissions`, `update_agent_api_keys`,
`native_plugin_invoke`, `extension_backend_call`, all `gdrive_*` write
operations, all `git::remote_ops::*` (push/pull/fetch), all `docker_*`
lifecycle mutations.

Rationale: permanent destruction, privilege escalation, self-modification of
the agent's own permission set, arbitrary code execution, or network egress.

## Tool schema conventions

- Every path arg: absolute, forward slashes on the wire.
- Every mutating tool: return `{"ok":bool,"undo_token":string,"audit_id":int}`.
- Every listing tool: accept `limit` + `offset`, return `{"items":[],"total":n,"truncated":bool}`.
- Errors: return `McpToolResult::err` with a human-readable reason. Never panic.
