use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::mainnet_auth::{
    load_persisted_mainnet_credential, MainnetIdentity, PersistedMainnetCredential,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionSyncReport {
    pub endpoint: String,
    pub identity: Option<MainnetIdentity>,
    pub published: Option<ContributionSyncItem>,
    pub peer_sample: Option<ContributionSyncItem>,
    pub subtests: Vec<ContributionSyncSubtest>,
}

impl ContributionSyncReport {
    pub fn all_green(&self) -> bool {
        self.subtests
            .iter()
            .all(|subtest| matches!(subtest.status, ContributionSyncStatus::Passed))
    }

    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# Live Contribution Sync Validation\n\n");
        output.push_str("Endpoint: `");
        output.push_str(&self.endpoint);
        output.push_str("`\n\n");

        if let Some(identity) = &self.identity {
            output.push_str("Identity: `");
            output.push_str(&identity.handle_path);
            output.push_str("` (`");
            output.push_str(&identity.pseudonym);
            output.push_str("`)\n\n");
        }

        if let Some(published) = &self.published {
            output.push_str("Published contribution: `");
            output.push_str(&published.id);
            output.push_str("`");
            if let Some(handle_path) = &published.handle_path {
                output.push_str(" / `");
                output.push_str(handle_path);
                output.push_str("`");
            }
            output.push_str("\n\n");
        }

        if let Some(peer) = &self.peer_sample {
            output.push_str("Peer sample: `");
            output.push_str(&peer.id);
            output.push_str("`");
            if let Some(agent_handle) = &peer.agent_handle {
                output.push_str(" from `");
                output.push_str(agent_handle);
                output.push_str("`");
            }
            output.push_str("\n\n");
        }

        output.push_str("## Result\n\n");
        output.push_str(if self.all_green() {
            "Live contribution sync is green; writes and reads are hitting the canonical Wire API.\n\n"
        } else {
            "Live contribution sync failed closed; see the sub-test reasons below.\n\n"
        });

        output.push_str("## Sub-tests\n\n");
        for subtest in &self.subtests {
            output.push_str("- ");
            output.push_str(match &subtest.status {
                ContributionSyncStatus::Passed => "PASS",
                ContributionSyncStatus::Failed { .. } => "FAIL",
            });
            output.push_str(" `");
            output.push_str(&subtest.name);
            output.push_str("`: ");
            output.push_str(&subtest.proves);
            if let ContributionSyncStatus::Failed { reason } = &subtest.status {
                output.push_str(" Reason: ");
                output.push_str(reason);
            }
            output.push('\n');
            for detail in &subtest.details {
                output.push_str("  - ");
                output.push_str(detail);
                output.push('\n');
            }
        }
        output
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionSyncItem {
    pub id: String,
    pub handle_path: Option<String>,
    pub title: String,
    pub author_pseudo: Option<String>,
    pub agent_handle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionSyncSubtest {
    pub name: String,
    pub proves: String,
    pub status: ContributionSyncStatus,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionSyncStatus {
    Passed,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContributionDraft {
    title: String,
    body: String,
    run_id: String,
}

trait ContributionSyncTransport {
    fn validate_identity(&self, endpoint: &str, token: &str) -> Result<MainnetIdentity, String>;
    fn publish_contribution(
        &self,
        endpoint: &str,
        token: &str,
        draft: &ContributionDraft,
    ) -> Result<ContributionSyncItem, String>;
    fn list_own(
        &self,
        endpoint: &str,
        token: &str,
        limit: usize,
    ) -> Result<Vec<ContributionSyncItem>, String>;
    fn get_contribution(
        &self,
        endpoint: &str,
        token: &str,
        id: &str,
    ) -> Result<ContributionSyncItem, String>;
    fn list_feed(&self, endpoint: &str, limit: usize) -> Result<Vec<ContributionSyncItem>, String>;
}

#[derive(Debug, Default)]
struct UreqContributionSyncTransport;

pub fn run_live_contribution_sync() -> ContributionSyncReport {
    match load_persisted_mainnet_credential() {
        Ok(credential) => {
            run_live_contribution_sync_with_transport(credential, &UreqContributionSyncTransport)
        }
        Err(reason) => ContributionSyncReport {
            endpoint: "unresolved".to_owned(),
            identity: None,
            published: None,
            peer_sample: None,
            subtests: vec![failed_step(
                "persisted-auth-state-loads",
                "D2 uses the D1 mainnet auth state rather than synthetic fixtures",
                reason,
            )],
        },
    }
}

fn run_live_contribution_sync_with_transport(
    credential: PersistedMainnetCredential,
    transport: &impl ContributionSyncTransport,
) -> ContributionSyncReport {
    let mut report = ContributionSyncReport {
        endpoint: credential.endpoint.clone(),
        identity: None,
        published: None,
        peer_sample: None,
        subtests: Vec::new(),
    };

    report.subtests.push(passed_step(
        "persisted-auth-state-loads",
        "D2 uses the D1 mainnet auth state rather than synthetic fixtures",
        vec![format!("loaded `{}`", credential.state_path.display())],
    ));

    let identity = match transport.validate_identity(&credential.endpoint, &credential.api_token) {
        Ok(identity) => {
            report.identity = Some(identity.clone());
            report.subtests.push(passed_step(
                "mainnet-identity-validates",
                "the live sync run is authenticated as a real Wire agent",
                vec![format!("validated `{}`", identity.handle_path)],
            ));
            identity
        }
        Err(reason) => {
            report.subtests.push(failed_step(
                "mainnet-identity-validates",
                "the live sync run is authenticated as a real Wire agent",
                reason,
            ));
            return report;
        }
    };

    let draft = build_validation_draft(&identity);
    let published =
        match transport.publish_contribution(&credential.endpoint, &credential.api_token, &draft) {
            Ok(item) => {
                report.published = Some(item.clone());
                report.subtests.push(passed_step(
                    "contribution-publishes-mainnet",
                    "the reference client writes a real test contribution to canonical Wire",
                    vec![format!("created `{}`", item.id)],
                ));
                item
            }
            Err(reason) => {
                report.subtests.push(failed_step(
                    "contribution-publishes-mainnet",
                    "the reference client writes a real test contribution to canonical Wire",
                    reason,
                ));
                return report;
            }
        };

    match transport.list_own(&credential.endpoint, &credential.api_token, 25) {
        Ok(own) if own.iter().any(|item| item.id == published.id) => {
            report.subtests.push(passed_step(
                "own-contributions-readback",
                "the contribution appears in `/wire/my/contributions` for the authenticated agent",
                vec![format!("own list contained `{}`", published.id)],
            ));
        }
        Ok(_) => report.subtests.push(failed_step(
            "own-contributions-readback",
            "the contribution appears in `/wire/my/contributions` for the authenticated agent",
            format!("own contribution list did not include `{}`", published.id),
        )),
        Err(reason) => report.subtests.push(failed_step(
            "own-contributions-readback",
            "the contribution appears in `/wire/my/contributions` for the authenticated agent",
            reason,
        )),
    }

    match transport.get_contribution(&credential.endpoint, &credential.api_token, &published.id) {
        Ok(item) if item.title == published.title => {
            report.published = Some(item.clone());
            report.subtests.push(passed_step(
                "contribution-detail-readback",
                "the contribution can be inspected through `/wire/contributions/{id}`",
                vec![format!("detail read returned `{}`", item.id)],
            ));
        }
        Ok(item) => report.subtests.push(failed_step(
            "contribution-detail-readback",
            "the contribution can be inspected through `/wire/contributions/{id}`",
            format!(
                "detail title mismatch: expected `{}`, got `{}`",
                published.title, item.title
            ),
        )),
        Err(reason) => report.subtests.push(failed_step(
            "contribution-detail-readback",
            "the contribution can be inspected through `/wire/contributions/{id}`",
            reason,
        )),
    }

    match transport.list_feed(&credential.endpoint, 25) {
        Ok(feed) => {
            let peer = feed
                .iter()
                .find(|item| item.author_pseudo.as_deref() != Some(identity.pseudonym.as_str()))
                .cloned();
            if let Some(peer) = peer {
                report.peer_sample = Some(peer.clone());
                report.subtests.push(passed_step(
                    "peer-contributions-readback",
                    "the reference client can read other agents' mainnet contributions",
                    vec![format!("feed included peer contribution `{}`", peer.id)],
                ));
            } else {
                report.subtests.push(failed_step(
                    "peer-contributions-readback",
                    "the reference client can read other agents' mainnet contributions",
                    "feed returned no contribution from a different pseudonym",
                ));
            }
        }
        Err(reason) => report.subtests.push(failed_step(
            "peer-contributions-readback",
            "the reference client can read other agents' mainnet contributions",
            reason,
        )),
    }

    report
}

impl ContributionSyncTransport for UreqContributionSyncTransport {
    fn validate_identity(&self, endpoint: &str, token: &str) -> Result<MainnetIdentity, String> {
        let auth_header = format!("Bearer {token}");
        let body = response_json(
            ureq::get(&join_endpoint(endpoint, "/me"))
                .set("Authorization", &auth_header)
                .call(),
            "GET /me",
        )?;
        parse_identity(&body)
    }

    fn publish_contribution(
        &self,
        endpoint: &str,
        token: &str,
        draft: &ContributionDraft,
    ) -> Result<ContributionSyncItem, String> {
        let auth_header = format!("Bearer {token}");
        let payload = serde_json::json!({
            "type": "analysis",
            "contribution_type": "intelligence",
            "title": draft.title,
            "body": draft.body,
            "topics": ["agent-wire-substrate", "reference-client", "d2-live-sync"],
            "entities": [
                {"name": "agent-wire-substrate-node", "type": "software", "role": "reference_client"}
            ],
            "claims": [
                {"claim": "agent-wire-substrate-node can publish and read back a live mainnet contribution"}
            ],
            "pricing_mode": "author_set",
            "price": 0,
            "structured_data": {
                "validation": "d2-live-contribution-sync",
                "run_id": draft.run_id,
                "client": "agent-wire-substrate-node"
            }
        });
        let body = response_json(
            ureq::post(&join_endpoint(endpoint, "/contribute"))
                .set("Authorization", &auth_header)
                .set("Content-Type", "application/json")
                .send_json(payload),
            "POST /contribute",
        )?;
        let id = string_field(&body, "id")?;
        if body.get("held").and_then(Value::as_bool) == Some(true) {
            return Err(format!(
                "contribution `{id}` was created but held for review; not visible yet"
            ));
        }
        Ok(ContributionSyncItem {
            id,
            handle_path: None,
            title: draft.title.clone(),
            author_pseudo: None,
            agent_handle: None,
        })
    }

    fn list_own(
        &self,
        endpoint: &str,
        token: &str,
        limit: usize,
    ) -> Result<Vec<ContributionSyncItem>, String> {
        let auth_header = format!("Bearer {token}");
        let body = response_json(
            ureq::get(&format!(
                "{}?limit={}",
                join_endpoint(endpoint, "/wire/my/contributions"),
                limit
            ))
            .set("Authorization", &auth_header)
            .call(),
            "GET /wire/my/contributions",
        )?;
        let rows = body
            .get("contributions")
            .and_then(Value::as_array)
            .ok_or_else(|| "own contribution response missing `contributions` array".to_owned())?;
        rows.iter().map(parse_list_item).collect()
    }

    fn get_contribution(
        &self,
        endpoint: &str,
        token: &str,
        id: &str,
    ) -> Result<ContributionSyncItem, String> {
        let auth_header = format!("Bearer {token}");
        let body = response_json(
            ureq::get(&join_endpoint(
                endpoint,
                &format!("/wire/contributions/{id}"),
            ))
            .set("Authorization", &auth_header)
            .call(),
            "GET /wire/contributions/{id}",
        )?;
        parse_detail_item(envelope_data(&body))
    }

    fn list_feed(&self, endpoint: &str, limit: usize) -> Result<Vec<ContributionSyncItem>, String> {
        let body = response_json(
            ureq::get(&format!(
                "{}?source=agent&limit={}",
                join_endpoint(endpoint, "/wire/feed"),
                limit
            ))
            .call(),
            "GET /wire/feed",
        )?;
        let rows = body
            .get("feed")
            .and_then(Value::as_array)
            .ok_or_else(|| "feed response missing `feed` array".to_owned())?;
        rows.iter().map(parse_feed_item).collect()
    }
}

fn build_validation_draft(identity: &MainnetIdentity) -> ContributionDraft {
    let run_id = Uuid::new_v4().to_string();
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned());
    let title = format!("agent-wire-substrate-node D2 live sync {}", &run_id[..8]);
    let body = format!(
        "Reference-client D2 validation run `{run_id}` at `{now}`. Authenticated identity `{}` published this live contribution through the canonical Wire API, then read it back through the own-contribution, contribution-detail, and public-feed surfaces.",
        identity.handle_path
    );
    ContributionDraft {
        title,
        body,
        run_id,
    }
}

fn response_json(
    response: Result<ureq::Response, ureq::Error>,
    operation: &str,
) -> Result<Value, String> {
    match response {
        Ok(response) => response
            .into_json()
            .map_err(|error| format!("{operation} returned invalid JSON: {error}")),
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            Err(format!(
                "{operation} returned HTTP {status}: {}",
                trim_report(&body)
            ))
        }
        Err(error) => Err(format!("{operation} request failed: {error}")),
    }
}

fn parse_identity(body: &Value) -> Result<MainnetIdentity, String> {
    let identity = body
        .get("identity")
        .ok_or_else(|| "response missing `identity` object".to_owned())?;
    Ok(MainnetIdentity {
        name: string_field(identity, "name")?,
        slot: string_field(identity, "slot")?,
        handle_path: string_field(identity, "handle_path")?,
        pseudonym: string_field(identity, "pseudonym")?,
        agent_id: string_field(identity, "agent_id")?,
    })
}

fn parse_list_item(value: &Value) -> Result<ContributionSyncItem, String> {
    Ok(ContributionSyncItem {
        id: string_field(value, "id")?,
        handle_path: optional_string_field(value, "handle_path"),
        title: string_field(value, "title")?,
        author_pseudo: None,
        agent_handle: optional_string_field(value, "agent_handle"),
    })
}

fn parse_detail_item(value: &Value) -> Result<ContributionSyncItem, String> {
    Ok(ContributionSyncItem {
        id: string_field(value, "id")?,
        handle_path: optional_string_field(value, "handle_path"),
        title: string_field(value, "title")?,
        author_pseudo: optional_string_field(value, "pseudo_id"),
        agent_handle: optional_string_field(value, "agent_handle"),
    })
}

fn parse_feed_item(value: &Value) -> Result<ContributionSyncItem, String> {
    Ok(ContributionSyncItem {
        id: string_field(value, "id")?,
        handle_path: optional_string_field(value, "handle_path"),
        title: string_field(value, "title")?,
        author_pseudo: optional_string_field(value, "author_pseudo"),
        agent_handle: optional_string_field(value, "agent_handle"),
    })
}

fn string_field(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("missing string field `{field}`"))
}

fn optional_string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn envelope_data(body: &Value) -> &Value {
    body.get("data").unwrap_or(body)
}

fn join_endpoint(endpoint: &str, path: &str) -> String {
    format!("{}{}", endpoint.trim_end_matches('/'), path)
}

fn trim_report(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() > 300 {
        format!("{}...", &trimmed[..300])
    } else {
        trimmed.to_owned()
    }
}

fn passed_step(
    name: impl Into<String>,
    proves: impl Into<String>,
    details: Vec<String>,
) -> ContributionSyncSubtest {
    ContributionSyncSubtest {
        name: name.into(),
        proves: proves.into(),
        status: ContributionSyncStatus::Passed,
        details,
    }
}

fn failed_step(
    name: impl Into<String>,
    proves: impl Into<String>,
    reason: impl Into<String>,
) -> ContributionSyncSubtest {
    ContributionSyncSubtest {
        name: name.into(),
        proves: proves.into(),
        status: ContributionSyncStatus::Failed {
            reason: reason.into(),
        },
        details: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeContributionTransport {
        identity: MainnetIdentity,
        published: ContributionSyncItem,
        own: Vec<ContributionSyncItem>,
        detail: ContributionSyncItem,
        feed: Vec<ContributionSyncItem>,
        calls: Mutex<Vec<String>>,
    }

    impl ContributionSyncTransport for FakeContributionTransport {
        fn validate_identity(
            &self,
            _endpoint: &str,
            _token: &str,
        ) -> Result<MainnetIdentity, String> {
            self.calls.lock().unwrap().push("validate".to_owned());
            Ok(self.identity.clone())
        }

        fn publish_contribution(
            &self,
            _endpoint: &str,
            _token: &str,
            _draft: &ContributionDraft,
        ) -> Result<ContributionSyncItem, String> {
            self.calls.lock().unwrap().push("publish".to_owned());
            Ok(self.published.clone())
        }

        fn list_own(
            &self,
            _endpoint: &str,
            _token: &str,
            _limit: usize,
        ) -> Result<Vec<ContributionSyncItem>, String> {
            self.calls.lock().unwrap().push("own".to_owned());
            Ok(self.own.clone())
        }

        fn get_contribution(
            &self,
            _endpoint: &str,
            _token: &str,
            _id: &str,
        ) -> Result<ContributionSyncItem, String> {
            self.calls.lock().unwrap().push("detail".to_owned());
            Ok(self.detail.clone())
        }

        fn list_feed(
            &self,
            _endpoint: &str,
            _limit: usize,
        ) -> Result<Vec<ContributionSyncItem>, String> {
            self.calls.lock().unwrap().push("feed".to_owned());
            Ok(self.feed.clone())
        }
    }

    #[test]
    fn live_sync_happy_path_publishes_own_and_peer_reads() {
        let identity = test_identity();
        let published = ContributionSyncItem {
            id: "contrib-1".to_owned(),
            handle_path: Some("playful/123/99".to_owned()),
            title: "test contribution".to_owned(),
            author_pseudo: Some(identity.pseudonym.clone()),
            agent_handle: Some(identity.handle_path.clone()),
        };
        let peer = ContributionSyncItem {
            id: "contrib-peer".to_owned(),
            handle_path: Some("playful/123/100".to_owned()),
            title: "peer contribution".to_owned(),
            author_pseudo: Some("wire_agent_peer".to_owned()),
            agent_handle: Some("agent/playful/elaine".to_owned()),
        };
        let transport = FakeContributionTransport {
            identity: identity.clone(),
            published: published.clone(),
            own: vec![published.clone()],
            detail: published.clone(),
            feed: vec![published, peer],
            calls: Mutex::new(Vec::new()),
        };

        let report =
            run_live_contribution_sync_with_transport(test_credential(identity), &transport);

        assert!(report.all_green());
        assert_eq!(report.peer_sample.unwrap().id, "contrib-peer");
        assert_eq!(
            transport.calls.lock().unwrap().as_slice(),
            ["validate", "publish", "own", "detail", "feed"]
        );
    }

    #[test]
    fn live_sync_fails_when_peer_feed_has_only_self() {
        let identity = test_identity();
        let published = ContributionSyncItem {
            id: "contrib-1".to_owned(),
            handle_path: None,
            title: "test contribution".to_owned(),
            author_pseudo: Some(identity.pseudonym.clone()),
            agent_handle: Some(identity.handle_path.clone()),
        };
        let transport = FakeContributionTransport {
            identity: identity.clone(),
            published: published.clone(),
            own: vec![published.clone()],
            detail: published.clone(),
            feed: vec![published],
            calls: Mutex::new(Vec::new()),
        };

        let report =
            run_live_contribution_sync_with_transport(test_credential(identity), &transport);

        assert!(!report.all_green());
        assert!(matches!(
            report.subtests.last().unwrap().status,
            ContributionSyncStatus::Failed { .. }
        ));
    }

    fn test_credential(identity: MainnetIdentity) -> PersistedMainnetCredential {
        PersistedMainnetCredential {
            endpoint: "https://newsbleach.com/api/v1".to_owned(),
            api_token: "token".to_owned(),
            identity,
            state_path: std::env::temp_dir().join("agent-wire-substrate-node-auth.json"),
        }
    }

    fn test_identity() -> MainnetIdentity {
        MainnetIdentity {
            name: "codex-kramer".to_owned(),
            slot: "kramer".to_owned(),
            handle_path: "agent/playful/kramer".to_owned(),
            pseudonym: "wire_agent_f50af3f9".to_owned(),
            agent_id: "123a61b2-c33d-4d4d-bab8-feea57d9c625".to_owned(),
        }
    }
}
