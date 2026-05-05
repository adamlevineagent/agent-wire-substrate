#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use agent_wire_foundation::canonical_ops::{
    CanonicalOp, CompilerOp, InvocationMode, LlmPrimitive, StepModifier, TaskPrimitive,
    WirePrimitive,
};
use agent_wire_foundation::{
    CreditAmount, CrossGraphRef, FoundationError, IdempotencyKey, QuoteReceipt,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireContributionType {
    Skill,
    Template,
    Action,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireActionKind {
    Single,
    Chain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    Low,
    Mid,
    High,
    Max,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnErrorPolicy {
    Abort,
    Skip,
    Retry { attempts: u8 },
}

impl OnErrorPolicy {
    pub fn retry(attempts: u8) -> Result<Self, CompileError> {
        if attempts == 0 || attempts > MAX_RETRY_ATTEMPTS {
            return Err(CompileError::OutOfRange {
                field: "retry_attempts",
            });
        }
        Ok(Self::Retry { attempts })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ForEachSpec {
    Reference(String),
    Items {
        reference: String,
        max_iterations: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireActionPermissions {
    pub query: bool,
    pub contribute: bool,
    pub access: bool,
    pub rate: bool,
    pub flag: bool,
    pub retract: bool,
    pub list_manage: bool,
    pub message: bool,
    pub market_create: bool,
    pub task_board: bool,
    pub max_contributions: u32,
    pub max_cost: CreditAmount,
}

impl WireActionPermissions {
    pub fn trusted_local(max_cost: CreditAmount) -> Self {
        Self {
            query: true,
            contribute: true,
            access: true,
            rate: true,
            flag: true,
            retract: true,
            list_manage: true,
            message: true,
            market_create: true,
            task_board: true,
            max_contributions: 64,
            max_cost,
        }
    }

    fn allows_wire(&self, primitive: WirePrimitive) -> bool {
        match primitive {
            WirePrimitive::Query | WirePrimitive::Browse => self.query,
            WirePrimitive::Contribute | WirePrimitive::Supersede => self.contribute,
            WirePrimitive::Access => self.access,
            WirePrimitive::Rate => self.rate,
            WirePrimitive::Flag => self.flag,
            WirePrimitive::Retract => self.retract,
            WirePrimitive::ListCreate
            | WirePrimitive::ListPin
            | WirePrimitive::ListSubscribe
            | WirePrimitive::ListQuery => self.list_manage,
            WirePrimitive::MessageSend | WirePrimitive::MessageBroadcast => self.message,
            WirePrimitive::MarketCreate
            | WirePrimitive::MarketSeed
            | WirePrimitive::MarketStake
            | WirePrimitive::MarketResolve
            | WirePrimitive::MarketClaim => self.market_create,
            WirePrimitive::CircleCreate
            | WirePrimitive::CircleInvite
            | WirePrimitive::CircleAssign
            | WirePrimitive::MonitorTopic
            | WirePrimitive::MonitorEntity
            | WirePrimitive::MonitorChain
            | WirePrimitive::SubscribeChain => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireActionStep {
    pub name: String,
    pub operation: CompilerOp,
    pub primitive: Option<LlmPrimitive>,
    pub wire: Option<WirePrimitive>,
    pub task: Option<TaskPrimitive>,
    pub instruction: Option<String>,
    pub input: Option<Value>,
    pub params: BTreeMap<String, Value>,
    pub output_schema: Option<Value>,
    pub model_tier: Option<ModelTier>,
    pub when: Option<String>,
    pub for_each: Option<ForEachSpec>,
    pub action_id: Option<String>,
    pub on_error: OnErrorPolicy,
    pub wait_for_completion: bool,
    pub game_type: Option<String>,
    pub formation: Option<String>,
    pub duration: Option<String>,
    pub entry_fee: Option<CreditAmount>,
    pub bounty: Option<CreditAmount>,
}

impl WireActionStep {
    pub fn llm(name: impl Into<String>, primitive: LlmPrimitive) -> Self {
        Self::new(name, CompilerOp::Llm).with_llm_primitive(primitive)
    }

    pub fn wire(name: impl Into<String>, primitive: WirePrimitive) -> Self {
        Self::new(name, CompilerOp::Wire).with_wire_primitive(primitive)
    }

    pub fn task(name: impl Into<String>, primitive: TaskPrimitive) -> Self {
        Self::new(name, CompilerOp::Task).with_task_primitive(primitive)
    }

    pub fn game(name: impl Into<String>) -> Self {
        Self::new(name, CompilerOp::Game)
    }

    pub fn new(name: impl Into<String>, operation: CompilerOp) -> Self {
        Self {
            name: name.into(),
            operation,
            primitive: None,
            wire: None,
            task: None,
            instruction: None,
            input: None,
            params: BTreeMap::new(),
            output_schema: None,
            model_tier: None,
            when: None,
            for_each: None,
            action_id: None,
            on_error: OnErrorPolicy::Abort,
            wait_for_completion: false,
            game_type: None,
            formation: None,
            duration: None,
            entry_fee: None,
            bounty: None,
        }
    }

    pub fn with_llm_primitive(mut self, primitive: LlmPrimitive) -> Self {
        self.primitive = Some(primitive);
        self
    }

    pub fn with_wire_primitive(mut self, primitive: WirePrimitive) -> Self {
        self.wire = Some(primitive);
        self
    }

    pub fn with_task_primitive(mut self, primitive: TaskPrimitive) -> Self {
        self.task = Some(primitive);
        self
    }

    pub fn with_action_id(mut self, action_id: impl Into<String>) -> Self {
        self.action_id = Some(action_id.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireActionDefinition {
    pub name: String,
    pub contribution_type: WireContributionType,
    pub action_kind: WireActionKind,
    pub permissions: WireActionPermissions,
    pub steps: Vec<WireActionStep>,
}

impl WireActionDefinition {
    pub fn single(
        name: impl Into<String>,
        step: WireActionStep,
        permissions: WireActionPermissions,
    ) -> Self {
        Self {
            name: name.into(),
            contribution_type: WireContributionType::Action,
            action_kind: WireActionKind::Single,
            permissions,
            steps: vec![step],
        }
    }

    pub fn chain(
        name: impl Into<String>,
        steps: Vec<WireActionStep>,
        permissions: WireActionPermissions,
    ) -> Self {
        Self {
            name: name.into(),
            contribution_type: WireContributionType::Action,
            action_kind: WireActionKind::Chain,
            permissions,
            steps,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionDisposition {
    InScope,
    NotImplementedInV1 { reason: String },
    OutOfV1Scope { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompiledOperation {
    Llm(LlmPrimitive),
    Wire(WirePrimitive),
    Task(TaskPrimitive),
    Game,
}

impl CompiledOperation {
    pub fn canonical_name(&self) -> &'static str {
        match self {
            Self::Llm(op) => op.name(),
            Self::Wire(op) => op.name(),
            Self::Task(op) => op.name(),
            Self::Game => CompilerOp::Game.name(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledStep {
    pub name: String,
    pub top_level: CompilerOp,
    pub operation: CompiledOperation,
    pub operation_name: String,
    pub cost: CreditAmount,
    pub disposition: ExecutionDisposition,
    pub resolved_action_ref: Option<CrossGraphRef>,
    pub modifiers: Vec<StepModifier>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireCompiledPlan {
    pub total_steps: usize,
    pub max_cost: CreditAmount,
    pub operations_used: Vec<String>,
    pub resolved_actions: BTreeMap<String, CrossGraphRef>,
    pub compiled_at_ms: u64,
    pub invocation_mode: InvocationMode,
    pub quote_receipt: Option<QuoteReceipt>,
    pub steps: Vec<CompiledStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompilerContext {
    pub compiled_at_ms: u64,
    pub quote_ref: CrossGraphRef,
    pub quote_key: IdempotencyKey,
    pub quote_expires_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalWireCompiledPlan {
    pub total_steps: usize,
    pub max_cost: CreditAmount,
    pub operations_used: Vec<String>,
    pub resolved_actions: BTreeMap<String, String>,
    pub compiled_at: String,
}

impl From<&WireCompiledPlan> for CanonicalWireCompiledPlan {
    fn from(plan: &WireCompiledPlan) -> Self {
        Self {
            total_steps: plan.total_steps,
            max_cost: plan.max_cost,
            operations_used: plan.operations_used.clone(),
            resolved_actions: plan
                .resolved_actions
                .iter()
                .map(|(action_id, reference)| (action_id.clone(), reference.to_string()))
                .collect(),
            compiled_at: unix_ms_to_rfc3339(plan.compiled_at_ms),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalWireActionPermissions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contribute: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_manage: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<CanonicalMessagePermission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_create: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_contributions: Option<u32>,
    pub max_cost: CreditAmount,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalMessagePermission {
    pub scope: CanonicalMessageScope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalMessageScope {
    Fleet,
    Circle,
}

impl From<&WireActionPermissions> for CanonicalWireActionPermissions {
    fn from(permissions: &WireActionPermissions) -> Self {
        Self {
            query: permissions.query.then_some(true),
            contribute: permissions.contribute.then_some(true),
            access: permissions.access.then_some(true),
            rate: permissions.rate.then_some(true),
            flag: permissions.flag.then_some(true),
            list_manage: permissions.list_manage.then_some(true),
            message: permissions.message.then_some(CanonicalMessagePermission {
                scope: CanonicalMessageScope::Fleet,
            }),
            market_create: permissions.market_create.then_some(true),
            max_contributions: Some(permissions.max_contributions),
            max_cost: permissions.max_cost,
        }
    }
}

impl From<CanonicalWireActionPermissions> for WireActionPermissions {
    fn from(permissions: CanonicalWireActionPermissions) -> Self {
        Self {
            query: permissions.query.unwrap_or(false),
            contribute: permissions.contribute.unwrap_or(false),
            access: permissions.access.unwrap_or(false),
            rate: permissions.rate.unwrap_or(false),
            flag: permissions.flag.unwrap_or(false),
            retract: false,
            list_manage: permissions.list_manage.unwrap_or(false),
            message: permissions.message.is_some(),
            market_create: permissions.market_create.unwrap_or(false),
            task_board: true,
            max_contributions: permissions.max_contributions.unwrap_or(64),
            max_cost: permissions.max_cost,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalWireActionStep {
    pub name: String,
    pub operation: CompilerOp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primitive: Option<LlmPrimitive>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_tier: Option<ModelTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_each: Option<CanonicalForEachSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait_for: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_fee: Option<CreditAmount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounty: Option<CreditAmount>,
}

impl CanonicalWireActionStep {
    pub fn from_internal(step: &WireActionStep) -> Self {
        Self {
            name: step.name.clone(),
            operation: step.operation,
            primitive: step.primitive,
            tool: tool_name_for_step(step),
            instruction: step.instruction.clone(),
            input: step.input.clone(),
            params: (!step.params.is_empty()).then_some(step.params.clone()),
            output_schema: step.output_schema.clone(),
            model_tier: step.model_tier.clone(),
            when: step.when.clone(),
            for_each: step.for_each.as_ref().map(CanonicalForEachSpec::from),
            max_iterations: None,
            action_id: step.action_id.clone(),
            on_error: canonical_on_error(&step.on_error),
            wait_for: step.wait_for_completion.then(|| "completion".to_owned()),
            game_type: step.game_type.clone(),
            formation: step.formation.clone(),
            duration: step.duration.clone(),
            entry_fee: step.entry_fee,
            bounty: step.bounty,
        }
    }

    pub fn into_internal(self) -> Result<WireActionStep, CanonicalActionError> {
        let mut step = WireActionStep::new(self.name, self.operation);
        step.primitive = self.primitive;
        step.instruction = self.instruction;
        step.input = self.input;
        step.params = self.params.unwrap_or_default();
        step.output_schema = self.output_schema;
        step.model_tier = self.model_tier;
        step.when = self.when;
        step.for_each = match (self.for_each, self.max_iterations) {
            (Some(CanonicalForEachSpec::Reference(reference)), Some(max_iterations)) => {
                Some(ForEachSpec::Items {
                    reference,
                    max_iterations,
                })
            }
            (Some(for_each), _) => Some(for_each.into_internal()),
            (None, _) => None,
        };
        step.action_id = self.action_id;
        step.on_error = parse_on_error(self.on_error.as_deref())?;
        step.wait_for_completion = self.wait_for.as_deref() == Some("completion");
        step.game_type = self.game_type;
        step.formation = self.formation;
        step.duration = self.duration;
        step.entry_fee = self.entry_fee;
        step.bounty = self.bounty;
        match step.operation {
            CompilerOp::Wire => {
                let tool = self.tool.ok_or(CanonicalActionError::MissingTool {
                    operation: CompilerOp::Wire,
                })?;
                step.wire = Some(
                    parse_wire_tool(&tool)
                        .ok_or_else(|| CanonicalActionError::UnknownTool(tool.clone()))?,
                );
            }
            CompilerOp::Task => {
                let tool = self.tool.ok_or(CanonicalActionError::MissingTool {
                    operation: CompilerOp::Task,
                })?;
                step.task = Some(
                    parse_task_tool(&tool)
                        .ok_or_else(|| CanonicalActionError::UnknownTool(tool.clone()))?,
                );
            }
            CompilerOp::Llm | CompilerOp::Game => {}
        }
        Ok(step)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CanonicalForEachSpec {
    Reference(String),
    Items {
        items: String,
        #[serde(rename = "maxIterations")]
        max_iterations: u32,
    },
}

impl CanonicalForEachSpec {
    fn into_internal(self) -> ForEachSpec {
        match self {
            Self::Reference(reference) => ForEachSpec::Reference(reference),
            Self::Items {
                items,
                max_iterations,
            } => ForEachSpec::Items {
                reference: items,
                max_iterations,
            },
        }
    }
}

impl From<&ForEachSpec> for CanonicalForEachSpec {
    fn from(spec: &ForEachSpec) -> Self {
        match spec {
            ForEachSpec::Reference(reference) => Self::Reference(reference.clone()),
            ForEachSpec::Items {
                reference,
                max_iterations,
            } => Self::Items {
                items: reference.clone(),
                max_iterations: *max_iterations,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalActionType {
    Chain,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalWireActionDefinition {
    pub schema_version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<CompilerOp>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primitive: Option<LlmPrimitive>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_tier: Option<ModelTier>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<CanonicalActionType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<CanonicalWireActionStep>>,
    pub permissions: CanonicalWireActionPermissions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiled_plan: Option<CanonicalWireCompiledPlan>,
}

impl CanonicalWireActionDefinition {
    pub fn from_internal(
        definition: &WireActionDefinition,
        plan: Option<&WireCompiledPlan>,
    ) -> Self {
        let compiled_plan = plan.map(CanonicalWireCompiledPlan::from);
        let permissions = CanonicalWireActionPermissions::from(&definition.permissions);
        if definition.action_kind == WireActionKind::Chain {
            Self {
                schema_version: 1,
                operation: None,
                primitive: None,
                instruction: None,
                input_schema: None,
                output_schema: None,
                constraints: None,
                model_tier: None,
                action_type: Some(CanonicalActionType::Chain),
                steps: Some(
                    definition
                        .steps
                        .iter()
                        .map(CanonicalWireActionStep::from_internal)
                        .collect(),
                ),
                permissions,
                compiled_plan,
            }
        } else {
            let step = definition.steps.first();
            Self {
                schema_version: 1,
                operation: step.map(|step| step.operation),
                primitive: step.and_then(|step| step.primitive),
                instruction: step.and_then(|step| step.instruction.clone()),
                input_schema: None,
                output_schema: step.and_then(|step| step.output_schema.clone()),
                constraints: None,
                model_tier: step.and_then(|step| step.model_tier.clone()),
                action_type: None,
                steps: None,
                permissions,
                compiled_plan,
            }
        }
    }

    pub fn into_internal(self) -> Result<WireActionDefinition, CanonicalActionError> {
        if self.schema_version != 1 {
            return Err(CanonicalActionError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        let permissions = self.permissions.into();
        if self.action_type == Some(CanonicalActionType::Chain) {
            let steps = self
                .steps
                .ok_or(CanonicalActionError::EmptySteps)?
                .into_iter()
                .map(CanonicalWireActionStep::into_internal)
                .collect::<Result<Vec<_>, _>>()?;
            if steps.is_empty() {
                return Err(CanonicalActionError::EmptySteps);
            }
            return Ok(WireActionDefinition::chain(
                "canonical-chain",
                steps,
                permissions,
            ));
        }

        let operation = self
            .operation
            .ok_or(CanonicalActionError::MissingSingleStepOperation)?;
        let canonical_step = CanonicalWireActionStep {
            name: "single".to_owned(),
            operation,
            primitive: self.primitive,
            tool: None,
            instruction: self.instruction,
            input: None,
            params: None,
            output_schema: self.output_schema,
            model_tier: self.model_tier,
            when: None,
            for_each: None,
            max_iterations: None,
            action_id: None,
            on_error: None,
            wait_for: None,
            game_type: None,
            formation: None,
            duration: None,
            entry_fee: None,
            bounty: None,
        };
        Ok(WireActionDefinition::single(
            "canonical-single",
            canonical_step.into_internal()?,
            permissions,
        ))
    }
}

pub trait ActionResolver {
    fn resolve_action_id(&self, action_id: &str) -> Option<CrossGraphRef>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopActionResolver;

impl ActionResolver for NoopActionResolver {
    fn resolve_action_id(&self, _action_id: &str) -> Option<CrossGraphRef> {
        None
    }
}

pub trait StepCostModel {
    fn estimate_step_cost(&self, step: &WireActionStep) -> CreditAmount;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultStepCostModel;

impl StepCostModel for DefaultStepCostModel {
    fn estimate_step_cost(&self, step: &WireActionStep) -> CreditAmount {
        match step.operation {
            CompilerOp::Llm => CreditAmount::from_sats(10),
            CompilerOp::Wire => CreditAmount::from_sats(1),
            CompilerOp::Task => CreditAmount::from_sats(2),
            CompilerOp::Game => CreditAmount::zero(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WireCompiler<R = NoopActionResolver, C = DefaultStepCostModel> {
    resolver: R,
    costs: C,
}

impl Default for WireCompiler<NoopActionResolver, DefaultStepCostModel> {
    fn default() -> Self {
        Self {
            resolver: NoopActionResolver,
            costs: DefaultStepCostModel,
        }
    }
}

impl<R, C> WireCompiler<R, C>
where
    R: ActionResolver,
    C: StepCostModel,
{
    pub fn new(resolver: R, costs: C) -> Self {
        Self { resolver, costs }
    }

    pub fn compile(
        &self,
        definition: &WireActionDefinition,
        mode: InvocationMode,
        context: &CompilerContext,
    ) -> Result<WireCompiledPlan, CompileError> {
        if definition.name.trim().is_empty() {
            return Err(CompileError::EmptyField {
                field: "action_name",
            });
        }
        if definition.steps.is_empty() {
            return Err(CompileError::EmptySteps);
        }

        let mut max_cost = CreditAmount::zero();
        let mut operations_used = Vec::<String>::new();
        let mut resolved_actions = BTreeMap::<String, CrossGraphRef>::new();
        let mut compiled_steps = Vec::<CompiledStep>::with_capacity(definition.steps.len());

        for step in &definition.steps {
            let compiled = self.compile_step(step, &definition.permissions)?;
            max_cost = max_cost
                .checked_add(compiled.cost)
                .ok_or(CompileError::CostOverflow)?;
            if let Some(action_id) = &step.action_id {
                let resolved = self
                    .resolver
                    .resolve_action_id(action_id)
                    .ok_or_else(|| CompileError::UnresolvedAction(action_id.clone()))?;
                resolved_actions.insert(action_id.clone(), resolved.clone());
                compiled_steps.push(CompiledStep {
                    resolved_action_ref: Some(resolved),
                    ..compiled
                });
            } else {
                compiled_steps.push(compiled);
            }
            let operation_name = compiled_steps
                .last()
                .expect("compiled step was just pushed")
                .operation_name
                .clone();
            if !operations_used.iter().any(|name| name == &operation_name) {
                operations_used.push(operation_name);
            }
        }

        if max_cost > definition.permissions.max_cost {
            return Err(CompileError::BudgetExceeded {
                estimated: max_cost,
                max_allowed: definition.permissions.max_cost,
            });
        }

        let quote_receipt = if mode == InvocationMode::Quote {
            Some(QuoteReceipt::new(
                context.quote_ref.clone(),
                context.quote_key.clone(),
                max_cost,
                context.quote_expires_at_ms,
            )?)
        } else {
            None
        };

        Ok(WireCompiledPlan {
            total_steps: compiled_steps.len(),
            max_cost,
            operations_used,
            resolved_actions,
            compiled_at_ms: context.compiled_at_ms,
            invocation_mode: mode,
            quote_receipt,
            steps: compiled_steps,
        })
    }

    fn compile_step(
        &self,
        step: &WireActionStep,
        permissions: &WireActionPermissions,
    ) -> Result<CompiledStep, CompileError> {
        if step.name.trim().is_empty() {
            return Err(CompileError::EmptyField { field: "step_name" });
        }
        validate_modifiers(step)?;
        let operation = compile_operation(step, permissions)?;
        let disposition = disposition_for(&operation);
        let operation_name = operation.canonical_name().to_owned();
        Ok(CompiledStep {
            name: step.name.clone(),
            top_level: step.operation,
            operation,
            operation_name,
            cost: self.costs.estimate_step_cost(step),
            disposition,
            resolved_action_ref: None,
            modifiers: modifiers_used(step),
        })
    }
}

fn compile_operation(
    step: &WireActionStep,
    permissions: &WireActionPermissions,
) -> Result<CompiledOperation, CompileError> {
    match step.operation {
        CompilerOp::Llm => {
            step.primitive
                .map(CompiledOperation::Llm)
                .ok_or(CompileError::MissingSubPrimitive {
                    operation: CompilerOp::Llm,
                })
        }
        CompilerOp::Wire => {
            let primitive = step.wire.ok_or(CompileError::MissingSubPrimitive {
                operation: CompilerOp::Wire,
            })?;
            if !permissions.allows_wire(primitive) {
                return Err(CompileError::PermissionDenied {
                    operation: primitive.name(),
                });
            }
            Ok(CompiledOperation::Wire(primitive))
        }
        CompilerOp::Task => {
            if !permissions.task_board {
                return Err(CompileError::PermissionDenied {
                    operation: CompilerOp::Task.name(),
                });
            }
            step.task
                .map(CompiledOperation::Task)
                .ok_or(CompileError::MissingSubPrimitive {
                    operation: CompilerOp::Task,
                })
        }
        CompilerOp::Game => Ok(CompiledOperation::Game),
    }
}

fn disposition_for(operation: &CompiledOperation) -> ExecutionDisposition {
    match operation {
        CompiledOperation::Game => ExecutionDisposition::OutOfV1Scope {
            reason: "game op is intentionally stubbed until V2+".to_owned(),
        },
        _ => ExecutionDisposition::InScope,
    }
}

fn validate_modifiers(step: &WireActionStep) -> Result<(), CompileError> {
    if let Some(ForEachSpec::Items { max_iterations, .. }) = &step.for_each {
        if *max_iterations == 0 || *max_iterations > MAX_ITERATIONS {
            return Err(CompileError::OutOfRange {
                field: "max_iterations",
            });
        }
    }
    if let OnErrorPolicy::Retry { attempts } = step.on_error {
        if attempts == 0 || attempts > MAX_RETRY_ATTEMPTS {
            return Err(CompileError::OutOfRange {
                field: "retry_attempts",
            });
        }
    }
    Ok(())
}

fn modifiers_used(step: &WireActionStep) -> Vec<StepModifier> {
    let mut modifiers = Vec::new();
    if step.when.is_some() {
        modifiers.push(StepModifier::When);
    }
    if step.for_each.is_some() {
        modifiers.push(StepModifier::ForEach);
    }
    if step.action_id.is_some() {
        modifiers.push(StepModifier::ActionId);
    }
    if !matches!(step.on_error, OnErrorPolicy::Abort) {
        modifiers.push(StepModifier::OnError);
    }
    if step.wait_for_completion {
        modifiers.push(StepModifier::WaitFor);
    }
    if step.output_schema.is_some() {
        modifiers.push(StepModifier::OutputSchema);
    }
    if step.model_tier.is_some() {
        modifiers.push(StepModifier::ModelTier);
    }
    if step.game_type.is_some() {
        modifiers.push(StepModifier::GameType);
    }
    if step.formation.is_some() {
        modifiers.push(StepModifier::Formation);
    }
    if step.duration.is_some() {
        modifiers.push(StepModifier::Duration);
    }
    if step.entry_fee.is_some() {
        modifiers.push(StepModifier::EntryFee);
    }
    if step.bounty.is_some() {
        modifiers.push(StepModifier::Bounty);
    }
    modifiers
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompilerOpManifest {
    pub top_level_ops: Vec<CompilerOp>,
    pub llm_primitives: Vec<LlmPrimitive>,
    pub wire_primitives: Vec<WirePrimitive>,
    pub task_primitives: Vec<TaskPrimitive>,
    pub step_modifiers: Vec<StepModifier>,
    pub invocation_modes: Vec<InvocationMode>,
}

impl CompilerOpManifest {
    pub fn v1() -> Self {
        Self {
            top_level_ops: CompilerOp::ALL.to_vec(),
            llm_primitives: LlmPrimitive::ALL.to_vec(),
            wire_primitives: WirePrimitive::ALL.to_vec(),
            task_primitives: TaskPrimitive::ALL.to_vec(),
            step_modifiers: StepModifier::ALL.to_vec(),
            invocation_modes: InvocationMode::ALL.to_vec(),
        }
    }

    pub fn logical_leaf_count(&self) -> usize {
        self.top_level_ops.len()
            + self.llm_primitives.len()
            + self.wire_primitives.len()
            + self.task_primitives.len()
            + self.step_modifiers.len()
            + self.invocation_modes.len()
    }

    pub fn all_canonical_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        names.extend(self.top_level_ops.iter().map(CanonicalOp::name));
        names.extend(self.llm_primitives.iter().map(CanonicalOp::name));
        names.extend(self.wire_primitives.iter().map(CanonicalOp::name));
        names.extend(self.task_primitives.iter().map(CanonicalOp::name));
        names.extend(self.step_modifiers.iter().map(CanonicalOp::name));
        names.extend(self.invocation_modes.iter().map(CanonicalOp::name));
        names
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CompileError {
    #[error("{field} must not be empty")]
    EmptyField { field: &'static str },

    #[error("action definition has no steps")]
    EmptySteps,

    #[error("{operation:?} is missing required sub-primitive")]
    MissingSubPrimitive { operation: CompilerOp },

    #[error("permission denied for {operation}")]
    PermissionDenied { operation: &'static str },

    #[error("action reference could not be resolved: {0}")]
    UnresolvedAction(String),

    #[error("{field} is outside the supported range")]
    OutOfRange { field: &'static str },

    #[error("estimated cost {estimated} exceeds maximum {max_allowed}")]
    BudgetExceeded {
        estimated: CreditAmount,
        max_allowed: CreditAmount,
    },

    #[error("compiled plan cost overflowed")]
    CostOverflow,

    #[error(transparent)]
    Foundation(#[from] FoundationError),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CanonicalActionError {
    #[error("canonical action schema version {0} is not supported")]
    UnsupportedSchemaVersion(u8),

    #[error("canonical chain action has no steps")]
    EmptySteps,

    #[error("canonical single-step action is missing operation")]
    MissingSingleStepOperation,

    #[error("{operation:?} step is missing canonical tool")]
    MissingTool { operation: CompilerOp },

    #[error("unknown canonical action tool: {0}")]
    UnknownTool(String),

    #[error("invalid canonical onError value: {0}")]
    InvalidOnError(String),
}

const MAX_ITERATIONS: u32 = 1_000;
const MAX_RETRY_ATTEMPTS: u8 = 5;

fn tool_name_for_step(step: &WireActionStep) -> Option<String> {
    match step.operation {
        CompilerOp::Wire => step.wire.map(|primitive| primitive.name().to_owned()),
        CompilerOp::Task => step.task.map(|primitive| primitive.name().to_owned()),
        CompilerOp::Llm | CompilerOp::Game => None,
    }
}

fn canonical_on_error(policy: &OnErrorPolicy) -> Option<String> {
    match policy {
        OnErrorPolicy::Abort => None,
        OnErrorPolicy::Skip => Some("skip".to_owned()),
        OnErrorPolicy::Retry { attempts } => Some(format!("retry({attempts})")),
    }
}

fn parse_on_error(value: Option<&str>) -> Result<OnErrorPolicy, CanonicalActionError> {
    let Some(value) = value else {
        return Ok(OnErrorPolicy::Abort);
    };
    match value {
        "abort" => Ok(OnErrorPolicy::Abort),
        "skip" => Ok(OnErrorPolicy::Skip),
        retry if retry.starts_with("retry(") && retry.ends_with(')') => {
            let attempts = retry
                .trim_start_matches("retry(")
                .trim_end_matches(')')
                .parse::<u8>()
                .map_err(|_| CanonicalActionError::InvalidOnError(retry.to_owned()))?;
            OnErrorPolicy::retry(attempts)
                .map_err(|_| CanonicalActionError::InvalidOnError(retry.to_owned()))
        }
        other => Err(CanonicalActionError::InvalidOnError(other.to_owned())),
    }
}

fn parse_wire_tool(value: &str) -> Option<WirePrimitive> {
    WirePrimitive::ALL
        .iter()
        .copied()
        .find(|primitive| primitive.name() == value)
}

fn parse_task_tool(value: &str) -> Option<TaskPrimitive> {
    TaskPrimitive::ALL
        .iter()
        .copied()
        .find(|primitive| primitive.name() == value)
}

fn unix_ms_to_rfc3339(value: u64) -> String {
    let nanos = i128::from(value) * 1_000_000;
    OffsetDateTime::from_unix_timestamp_nanos(nanos)
        .ok()
        .and_then(|timestamp| timestamp.format(&Rfc3339).ok())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_wire_foundation::NamespaceId;

    fn playful_ref(sequence: u32) -> CrossGraphRef {
        CrossGraphRef {
            namespace: NamespaceId::new("playful").unwrap(),
            day: 124,
            slug: Some("compiler".to_owned()),
            sequence,
        }
    }

    fn context() -> CompilerContext {
        CompilerContext {
            compiled_at_ms: 1_000,
            quote_ref: playful_ref(1),
            quote_key: IdempotencyKey::new("compiler-quote-1").unwrap(),
            quote_expires_at_ms: 31_000,
        }
    }

    fn permissions() -> WireActionPermissions {
        WireActionPermissions::trusted_local(CreditAmount::from_sats(1_000))
    }

    #[test]
    fn manifest_covers_v1_compiler_surface_from_sealed_registry() {
        let manifest = CompilerOpManifest::v1();
        let names = manifest.all_canonical_names();

        assert_eq!(manifest.top_level_ops.len(), 4);
        assert_eq!(manifest.llm_primitives.len(), 29);
        assert_eq!(manifest.wire_primitives.len(), 26);
        assert_eq!(manifest.task_primitives.len(), 3);
        assert_eq!(manifest.step_modifiers.len(), 12);
        assert_eq!(manifest.invocation_modes.len(), 3);
        assert_eq!(manifest.logical_leaf_count(), 77);
        assert!(names.contains(&"llm"));
        assert!(names.contains(&"wire.retract"));
        assert!(names.contains(&"task.complete"));
        assert!(names.contains(&"trusted"));
    }

    #[test]
    fn compiles_llm_wire_and_task_chain_with_quote_receipt() {
        let compiler = WireCompiler::default();
        let mut llm = WireActionStep::llm("extract", LlmPrimitive::Extract);
        llm.instruction = Some("extract the claim".to_owned());
        llm.model_tier = Some(ModelTier::Mid);
        let mut wire = WireActionStep::wire("publish", WirePrimitive::Contribute);
        wire.output_schema = Some(serde_json::json!({"type": "object"}));
        let mut task = WireActionStep::task("follow-up", TaskPrimitive::Create);
        task.wait_for_completion = true;
        task.bounty = Some(CreditAmount::from_sats(5));

        let definition =
            WireActionDefinition::chain("claim-flow", vec![llm, wire, task], permissions());
        let plan = compiler
            .compile(&definition, InvocationMode::Quote, &context())
            .unwrap();

        assert_eq!(plan.total_steps, 3);
        assert_eq!(plan.max_cost.as_sats(), 13);
        assert_eq!(
            plan.operations_used,
            ["extract", "wire.contribute", "task.create"]
        );
        assert_eq!(
            plan.quote_receipt
                .as_ref()
                .unwrap()
                .idempotency_key()
                .as_str(),
            "compiler-quote-1"
        );
        assert_eq!(plan.steps[0].modifiers, vec![StepModifier::ModelTier]);
        assert_eq!(plan.steps[1].modifiers, vec![StepModifier::OutputSchema]);
        assert_eq!(
            plan.steps[2].modifiers,
            vec![StepModifier::WaitFor, StepModifier::Bounty]
        );
    }

    #[test]
    fn nested_action_ids_are_resolved_into_compiled_plan() {
        struct Resolver;
        impl ActionResolver for Resolver {
            fn resolve_action_id(&self, action_id: &str) -> Option<CrossGraphRef> {
                (action_id == "nested-action").then(|| playful_ref(9))
            }
        }

        let compiler = WireCompiler::new(Resolver, DefaultStepCostModel);
        let step =
            WireActionStep::wire("nested", WirePrimitive::Query).with_action_id("nested-action");
        let definition = WireActionDefinition::single("nested-flow", step, permissions());

        let plan = compiler
            .compile(&definition, InvocationMode::Trusted, &context())
            .unwrap();

        assert_eq!(
            plan.resolved_actions.get("nested-action"),
            Some(&playful_ref(9))
        );
        assert_eq!(plan.steps[0].resolved_action_ref, Some(playful_ref(9)));
        assert_eq!(plan.steps[0].modifiers, vec![StepModifier::ActionId]);
    }

    #[test]
    fn game_op_compiles_as_explicit_out_of_v1_scope_stub() {
        let compiler = WireCompiler::default();
        let mut step = WireActionStep::game("play");
        step.game_type = Some("forecast".to_owned());
        let definition = WireActionDefinition::single("game-flow", step, permissions());

        let plan = compiler
            .compile(&definition, InvocationMode::Review, &context())
            .unwrap();

        assert_eq!(plan.steps[0].operation_name, "game");
        assert!(matches!(
            plan.steps[0].disposition,
            ExecutionDisposition::OutOfV1Scope { .. }
        ));
        assert_eq!(plan.steps[0].modifiers, vec![StepModifier::GameType]);
    }

    #[test]
    fn compile_rejects_missing_subprimitive_permission_and_budget_issues() {
        let compiler = WireCompiler::default();
        let missing = WireActionDefinition::single(
            "bad",
            WireActionStep::new("missing", CompilerOp::Wire),
            permissions(),
        );
        assert_eq!(
            compiler.compile(&missing, InvocationMode::Trusted, &context()),
            Err(CompileError::MissingSubPrimitive {
                operation: CompilerOp::Wire
            })
        );

        let mut denied_permissions = permissions();
        denied_permissions.contribute = false;
        let denied = WireActionDefinition::single(
            "denied",
            WireActionStep::wire("publish", WirePrimitive::Contribute),
            denied_permissions,
        );
        assert_eq!(
            compiler.compile(&denied, InvocationMode::Trusted, &context()),
            Err(CompileError::PermissionDenied {
                operation: "wire.contribute"
            })
        );

        let tiny_budget = WireActionPermissions::trusted_local(CreditAmount::from_sats(1));
        let over_budget = WireActionDefinition::single(
            "expensive",
            WireActionStep::llm("extract", LlmPrimitive::Extract),
            tiny_budget,
        );
        assert_eq!(
            compiler.compile(&over_budget, InvocationMode::Trusted, &context()),
            Err(CompileError::BudgetExceeded {
                estimated: CreditAmount::from_sats(10),
                max_allowed: CreditAmount::from_sats(1),
            })
        );
    }

    #[test]
    fn canonical_action_definition_uses_goodnews_everyone_json_shape() {
        struct Resolver;
        impl ActionResolver for Resolver {
            fn resolve_action_id(&self, action_id: &str) -> Option<CrossGraphRef> {
                (action_id == "nested-action").then(|| playful_ref(9))
            }
        }

        let compiler = WireCompiler::new(Resolver, DefaultStepCostModel);
        let mut llm = WireActionStep::llm("extract", LlmPrimitive::Extract);
        llm.output_schema = Some(serde_json::json!({"type": "object"}));
        llm.model_tier = Some(ModelTier::High);
        let mut wire = WireActionStep::wire("publish", WirePrimitive::Contribute);
        wire.for_each = Some(ForEachSpec::Items {
            reference: "$extract.claims".to_owned(),
            max_iterations: 4,
        });
        wire.action_id = Some("nested-action".to_owned());
        wire.on_error = OnErrorPolicy::retry(2).unwrap();
        let mut task = WireActionStep::task("claim", TaskPrimitive::Claim);
        task.wait_for_completion = true;
        task.bounty = Some(CreditAmount::from_sats(7));

        let definition =
            WireActionDefinition::chain("ws3-parity", vec![llm, wire, task], permissions());
        let plan = compiler
            .compile(&definition, InvocationMode::Quote, &context())
            .unwrap();
        let canonical = CanonicalWireActionDefinition::from_internal(&definition, Some(&plan));
        let value = serde_json::to_value(&canonical).unwrap();

        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["actionType"], "chain");
        assert_eq!(value["steps"][1]["tool"], "wire.contribute");
        assert_eq!(value["steps"][1]["forEach"]["items"], "$extract.claims");
        assert_eq!(value["steps"][1]["forEach"]["maxIterations"], 4);
        assert_eq!(value["steps"][1]["actionId"], "nested-action");
        assert_eq!(value["steps"][1]["onError"], "retry(2)");
        assert_eq!(value["steps"][2]["tool"], "task.claim");
        assert_eq!(value["steps"][2]["waitFor"], "completion");
        assert_eq!(value["steps"][2]["bounty"], 7);
        assert_eq!(value["compiledPlan"]["totalSteps"], 3);
        assert_eq!(value["compiledPlan"]["maxCost"], 13);
        assert_eq!(value["compiledPlan"]["operationsUsed"][0], "extract");
        assert_eq!(value["compiledPlan"]["compiledAt"], "1970-01-01T00:00:01Z");
        assert!(value.get("contribution_type").is_none());
        assert!(value.get("action_kind").is_none());
        assert!(value["steps"][0].get("output_schema").is_none());
        assert!(value["steps"][0].get("model_tier").is_none());
        assert!(value["steps"][1].get("wire").is_none());
        assert!(value["compiledPlan"].get("compiled_at_ms").is_none());
        assert!(value["compiledPlan"].get("quote_receipt").is_none());
        assert!(value["compiledPlan"].get("steps").is_none());
    }

    #[test]
    fn canonical_action_definition_round_trips_into_typed_internal_plan() {
        let canonical = serde_json::json!({
            "schemaVersion": 1,
            "actionType": "chain",
            "permissions": {
                "query": true,
                "contribute": true,
                "message": { "scope": "fleet" },
                "maxContributions": 8,
                "maxCost": 1_000
            },
            "steps": [
                {
                    "name": "extract",
                    "operation": "llm",
                    "primitive": "extract",
                    "instruction": "extract claims",
                    "outputSchema": { "type": "object" },
                    "modelTier": "mid"
                },
                {
                    "name": "publish",
                    "operation": "wire",
                    "tool": "wire.contribute",
                    "forEach": { "items": "$extract.claims", "maxIterations": 3 },
                    "onError": "skip"
                },
                {
                    "name": "claim",
                    "operation": "task",
                    "tool": "task.claim",
                    "waitFor": "completion"
                }
            ]
        });
        let dto: CanonicalWireActionDefinition = serde_json::from_value(canonical).unwrap();
        let internal = dto.into_internal().unwrap();

        assert_eq!(internal.action_kind, WireActionKind::Chain);
        assert_eq!(internal.permissions.max_contributions, 8);
        assert!(internal.permissions.message);
        assert_eq!(internal.steps[0].operation, CompilerOp::Llm);
        assert_eq!(internal.steps[0].model_tier, Some(ModelTier::Mid));
        assert_eq!(internal.steps[1].wire, Some(WirePrimitive::Contribute));
        assert_eq!(
            internal.steps[1].for_each,
            Some(ForEachSpec::Items {
                reference: "$extract.claims".to_owned(),
                max_iterations: 3,
            })
        );
        assert_eq!(internal.steps[1].on_error, OnErrorPolicy::Skip);
        assert_eq!(internal.steps[2].task, Some(TaskPrimitive::Claim));
        assert!(internal.steps[2].wait_for_completion);
    }
}
