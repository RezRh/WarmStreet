use crux_core::capability::{Capability, CapabilityContext, Operation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Event;

#[derive(Clone)]
pub struct Push<E> {
    context: CapabilityContext<PushOperation, E>,
}

impl<Ev> Capability<Ev> for Push<Ev> {
    type Operation = PushOperation;
    type MappedSelf<MappedEv> = Push<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + 'static,
        Ev: 'static,
        NewEv: 'static,
    {
        Push::new(self.context.map_event(f))
    }
}

impl<E: 'static> Push<E>
where
    E: 'static,
{
    pub fn new(context: CapabilityContext<PushOperation, E>) -> Self {
        Self { context }
    }

    pub fn request_permission<F>(&self, _callback: F)
    where
        F: FnOnce(bool) -> E + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let _ = ctx.request_from_shell(PushOperation::RequestPermission).await;
        });
    }

    pub fn get_token<F>(&self, callback: F)
    where
        F: FnOnce(Result<PushOutput, PushError>) -> E + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let result = ctx.request_from_shell(PushOperation::GetToken).await;
            ctx.update_app(callback(result));
        });
    }

    pub fn register<F>(&self, token: String, _callback: F)
    where
        F: FnOnce(Result<(), String>) -> E + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let _ = ctx.request_from_shell(PushOperation::Register { token }).await;
        });
    }

    pub fn unregister<F>(&self, _callback: F)
    where
        F: FnOnce(Result<(), String>) -> E + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let _ = ctx.request_from_shell(PushOperation::Unregister).await;
        });
    }
}

pub type PushCapability = Push<Event>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PushOperation {
    RequestPermission,
    GetToken,
    Register { token: String },
    Unregister,
}

impl Operation for PushOperation {
    type Output = PushResult;
}

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum PushError {
    #[error("permission denied")]
    PermissionDenied,

    #[error("push not available")]
    NotAvailable,

    #[error("registration failed: {0}")]
    RegistrationFailed(String),

    #[error("{0}")]
    Other(String),
}

pub type PushResult = Result<PushOutput, PushError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PushOutput {
    PermissionGranted(bool),
    Token(String),
    Registered,
    Unregistered,
}
