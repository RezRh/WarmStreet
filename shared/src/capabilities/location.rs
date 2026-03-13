use crux_core::capability::{Capability, CapabilityContext, Operation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Event;

#[derive(Clone)]
pub struct Location<E> {
    context: CapabilityContext<LocationOperation, E>,
}

impl<Ev> Capability<Ev> for Location<Ev> {
    type Operation = LocationOperation;
    type MappedSelf<MappedEv> = Location<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + 'static,
        Ev: 'static,
        NewEv: 'static,
    {
        Location::new(self.context.map_event(f))
    }
}

impl<E: 'static> Location<E>
where
    E: 'static,
{
    pub fn new(context: CapabilityContext<LocationOperation, E>) -> Self {
        Self { context }
    }

    pub fn request_permission<F>(&self, _callback: F)
    where
        F: FnOnce(bool) -> E + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let _ = ctx.request_from_shell(LocationOperation::RequestPermission).await;
            // Shell will send the result via event
        });
    }

    pub fn get_current<F>(&self, callback: F)
    where
        F: FnOnce(Result<(f64, f64, Option<f64>), String>) -> E + Send + 'static,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let result = ctx.request_from_shell(LocationOperation::GetCurrentLocation).await;
            ctx.update_app(callback(result.map_err(|e| format!("{:?}", e))));
        });
    }
}

pub type LocationCapability = Location<Event>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LocationOperation {
    RequestPermission,
    GetCurrentLocation,
}

impl Operation for LocationOperation {
    type Output = LocationResult;
}

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum LocationError {
    #[error("permission denied")]
    PermissionDenied,

    #[error("location unavailable")]
    Unavailable,

    #[error("timeout")]
    Timeout,

    #[error("{0}")]
    Other(String),
}

pub type LocationResult = Result<(f64, f64, Option<f64>), LocationError>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum PermissionState {
    #[default]
    NotDetermined,
    Requesting,
    Granted,
    Denied,
}

impl PermissionState {
    #[must_use]
    pub const fn is_granted(self) -> bool {
        matches!(self, Self::Granted)
    }

    #[must_use]
    pub const fn is_denied(self) -> bool {
        matches!(self, Self::Denied)
    }

    #[must_use]
    pub const fn is_requesting(self) -> bool {
        matches!(self, Self::Requesting)
    }
}
