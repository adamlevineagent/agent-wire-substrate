use agent_wire_foundation::{EventKind, TriggerFilter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundWorkerLifecycle {
    pub workers: Vec<BackgroundWorker>,
    pub wake_filter: TriggerFilter,
}

impl BackgroundWorkerLifecycle {
    pub fn substrate_default() -> Self {
        Self {
            workers: vec![
                BackgroundWorker::ContributionSync,
                BackgroundWorker::ComputeProvider,
                BackgroundWorker::ComputeRequester,
                BackgroundWorker::TunnelDelivery,
                BackgroundWorker::VocabularySync,
                BackgroundWorker::WakeTriggerListener,
            ],
            wake_filter: TriggerFilter {
                triggers: vec![
                    agent_wire_foundation::EventTrigger::Kind(EventKind::MessageUnread),
                    agent_wire_foundation::EventTrigger::Kind(EventKind::TaskAssigned),
                    agent_wire_foundation::EventTrigger::Kind(EventKind::TaskMoved),
                    agent_wire_foundation::EventTrigger::Kind(EventKind::ContributionPublished),
                ],
                after: None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundWorker {
    ContributionSync,
    ComputeProvider,
    ComputeRequester,
    TunnelDelivery,
    VocabularySync,
    WakeTriggerListener,
}
