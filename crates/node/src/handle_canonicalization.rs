use agent_wire_foundation::canonical_ops::McpTool;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HandleCanonicalizationOptions {
    pub include_ids: bool,
}

pub fn canonicalize_mcp_response(
    tool: McpTool,
    response: Value,
    options: HandleCanonicalizationOptions,
) -> Value {
    if tool == McpTool::WireWait {
        return response;
    }
    canonicalize_value(response, options)
}

fn canonicalize_value(value: Value, options: HandleCanonicalizationOptions) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| canonicalize_value(item, options))
                .collect(),
        ),
        Value::Object(object) => Value::Object(canonicalize_object(object, options)),
        other => other,
    }
}

fn canonicalize_object(
    object: Map<String, Value>,
    options: HandleCanonicalizationOptions,
) -> Map<String, Value> {
    let has_primary_handle = object
        .keys()
        .any(|key| PRIMARY_HANDLE_KEYS.contains(&key.as_str()));
    object
        .into_iter()
        .filter_map(|(key, value)| {
            if has_primary_handle
                && !options.include_ids
                && DIAGNOSTIC_ID_KEYS.contains(&key.as_str())
            {
                None
            } else {
                Some((key, canonicalize_value(value, options)))
            }
        })
        .collect()
}

const PRIMARY_HANDLE_KEYS: &[&str] = &[
    "handle_path",
    "message_handle",
    "operator_handle",
    "agent_handle",
    "handle",
    "from_handle",
    "to_handle",
];

const DIAGNOSTIC_ID_KEYS: &[&str] = &[
    "id",
    "agent_id",
    "pseudo_id",
    "pseudonym",
    "message_id",
    "contribution_id",
    "item_id",
    "operator_id",
    "from_id",
    "to_id",
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn contribution_responses_default_to_handle_paths() {
        let shaped = canonicalize_mcp_response(
            McpTool::WireContribute,
            json!({
                "id": "d0d1d2d3-0000-4000-8000-000000000001",
                "handle_path": "agent/playful/kramer/contributions/hello",
                "contribution": {
                    "id": "d0d1d2d3-0000-4000-8000-000000000002",
                    "agent_id": "wire_agent_f50af3f9",
                    "handle_path": "agent/playful/kramer/contributions/hello",
                    "title": "hello"
                }
            }),
            HandleCanonicalizationOptions::default(),
        );

        assert_eq!(
            shaped["handle_path"],
            "agent/playful/kramer/contributions/hello"
        );
        assert!(shaped.get("id").is_none());
        assert!(shaped["contribution"].get("id").is_none());
        assert!(shaped["contribution"].get("agent_id").is_none());
        assert_eq!(shaped["contribution"]["title"], "hello");
    }

    #[test]
    fn include_ids_preserves_diagnostic_identifiers() {
        let shaped = canonicalize_mcp_response(
            McpTool::WireInspect,
            json!({
                "id": "d0d1d2d3-0000-4000-8000-000000000003",
                "handle_path": "agent/playful/kramer/contributions/inspect-me"
            }),
            HandleCanonicalizationOptions { include_ids: true },
        );

        assert_eq!(
            shaped["handle_path"],
            "agent/playful/kramer/contributions/inspect-me"
        );
        assert_eq!(shaped["id"], "d0d1d2d3-0000-4000-8000-000000000003");
    }

    #[test]
    fn message_responses_default_to_message_handles() {
        let shaped = canonicalize_mcp_response(
            McpTool::WireMessages,
            json!({
                "message": {
                    "id": "d0d1d2d3-0000-4000-8000-000000000004",
                    "message_id": "d0d1d2d3-0000-4000-8000-000000000004",
                    "message_handle": "msg/kramer/2026-05-09/dm-123abc",
                    "from_id": "wire_agent_partner",
                    "from_handle": "agent/playful/partner-orchestrator",
                    "to_id": "wire_agent_f50af3f9",
                    "to_handle": "agent/playful/codex-kramer"
                }
            }),
            HandleCanonicalizationOptions::default(),
        );

        let message = &shaped["message"];
        assert_eq!(message["message_handle"], "msg/kramer/2026-05-09/dm-123abc");
        assert_eq!(message["from_handle"], "agent/playful/partner-orchestrator");
        assert_eq!(message["to_handle"], "agent/playful/codex-kramer");
        assert!(message.get("id").is_none());
        assert!(message.get("message_id").is_none());
        assert!(message.get("from_id").is_none());
        assert!(message.get("to_id").is_none());
    }

    #[test]
    fn wire_wait_event_payloads_are_not_rewritten() {
        let payload = json!({
            "events": [{
                "id": "14690",
                "message_id": "d0d1d2d3-0000-4000-8000-000000000005",
                "message_handle": "msg/kramer/2026-05-09/dm-456def"
            }],
            "next_cursor": "wv1.cursor"
        });

        assert_eq!(
            canonicalize_mcp_response(
                McpTool::WireWait,
                payload.clone(),
                HandleCanonicalizationOptions::default()
            ),
            payload
        );
    }

    #[test]
    fn objects_without_handle_context_keep_identifiers() {
        let shaped = canonicalize_mcp_response(
            McpTool::WireStatus,
            json!({
                "cursor": {
                    "id": "14691",
                    "next": "wv1.cursor"
                }
            }),
            HandleCanonicalizationOptions::default(),
        );

        assert_eq!(shaped["cursor"]["id"], "14691");
    }
}
