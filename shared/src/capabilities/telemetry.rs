use crux_core::capability::{Capability, CapabilityContext, Operation};
use serde::{Deserialize, Serialize};

use crate::Event;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TelemetryOperation {
    Log { level: String, message: String, tags: std::collections::HashMap<String, String> },
    Counter { name: String, value: i64, tags: std::collections::HashMap<String, String> },
    Gauge { name: String, value: f64, tags: std::collections::HashMap<String, String> },
}

impl Operation for TelemetryOperation {
    type Output = ();
}

#[derive(Clone)]
pub struct Telemetry<Ev> {
    context: CapabilityContext<TelemetryOperation, Ev>,
}

impl<Ev> std::fmt::Debug for Telemetry<Ev> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Telemetry").finish()
    }
}

impl<Ev: 'static> Capability<Ev> for Telemetry<Ev> {
    type Operation = TelemetryOperation;
    type MappedSelf<MappedEv> = Telemetry<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + 'static,
        NewEv: 'static + Send,
    {
        Telemetry::new(self.context.map_event(f))
    }
}

impl<Ev> Telemetry<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<TelemetryOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn info(&self, message: impl Into<String>, tags: std::collections::HashMap<String, String>) {
        let message = message.into();
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let _ = ctx.request_from_shell(TelemetryOperation::Log {
                level: "info".to_string(),
                message,
                tags,
            }).await;
        });
    }

    pub fn error(&self, message: impl Into<String>, tags: std::collections::HashMap<String, String>) {
        let message = message.into();
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let _ = ctx.request_from_shell(TelemetryOperation::Log {
                level: "error".to_string(),
                message,
                tags,
            }).await;
        });
    }

    pub fn info_msg(&self, message: impl Into<String>) {
        self.info(message, std::collections::HashMap::new());
    }

    pub fn error_msg(&self, message: impl Into<String>) {
        self.error(message, std::collections::HashMap::new());
    }

    pub fn warn_msg(&self, message: impl Into<String>) {
        self.warn(message, std::collections::HashMap::new());
    }

    pub fn warn(&self, message: impl Into<String>, tags: std::collections::HashMap<String, String>) {
        let message = message.into();
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let _ = ctx.request_from_shell(TelemetryOperation::Log {
                level: "warn".to_string(),
                message,
                tags,
            }).await;
        });
    }

    pub fn counter(&self, name: impl Into<String>, value: i64, tags: std::collections::HashMap<String, String>) {
        let name = name.into();
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let _ = ctx.request_from_shell(TelemetryOperation::Counter {
                name,
                value,
                tags,
            }).await;
        });
    }

    pub fn gauge(&self, name: impl Into<String>, value: f64, tags: std::collections::HashMap<String, String>) {
        let name = name.into();
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let _ = ctx.request_from_shell(TelemetryOperation::Gauge {
                name,
                value,
                tags,
            }).await;
        });
    }
}

pub type TelemetryCapability = Telemetry<Event>;
