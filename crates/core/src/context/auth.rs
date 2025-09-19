use core::fmt;

use alloc::{string::ToString, vec::Vec};
use msgpacker::MsgPacker;
use serde::{Deserialize, Serialize};
use valence_coprocessor_types::{DataBackend, Hasher};

use crate::{ExecutionContext, Hash};

/// Authorizations table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, MsgPacker)]
pub enum Permission {
    /// Write permission to the controller bytecode.
    CircuitControllerWrite(Hash),

    /// Write permission to the circuit storage.
    CircuitStorageWrite(Hash),
}

impl Permission {
    /// Returns the identifier of the resource, if present.
    pub fn resource(&self) -> Option<Hash> {
        use Permission::*;

        match self {
            CircuitControllerWrite(h) | CircuitStorageWrite(h) => Some(*h),
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Permission::*;

        match self {
            CircuitControllerWrite(h) => {
                write!(f, "CircuitControllerWrite({})", const_hex::encode(h))
            }
            CircuitStorageWrite(h) => write!(f, "CircuitStorageWrite({})", const_hex::encode(h)),
        }
    }
}

impl<H, D> ExecutionContext<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    /// Data backend prefix for the authorizations.
    pub const PREFIX_AUTH: &[u8] = b"context-auth";

    /// Data backend prefix of a locked resource.
    pub const PREFIX_AUTH_LOCKED: &[u8] = b"context-auth-locked";

    /// Associates the authorization with the execution context.
    pub fn with_owner(mut self, owner: Vec<u8>) -> Self {
        self.owner.replace(owner);
        self
    }

    /// Returns the associated owner, if present.
    pub fn owner(&self) -> Option<&[u8]> {
        self.owner.as_deref()
    }

    /// Asserts the owner is present and has the permission.
    fn _assert(&self, permission: &Permission) -> anyhow::Result<()> {
        let owner = self
            .owner
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not authorized"))?;

        let permission = permission.to_string();
        let token = H::key(&permission, owner.as_slice());

        tracing::debug!("checking resource `{}`...", const_hex::encode(token));

        self.data
            .get(Self::PREFIX_AUTH, &token)?
            .ok_or_else(|| anyhow::anyhow!("not authorized..."))?;

        Ok(())
    }

    fn _grant(&self, permission: &Permission, owner: &[u8]) -> anyhow::Result<()> {
        tracing::debug!(
            "granting `{permission}` to `{}`...",
            const_hex::encode(owner)
        );

        if permission.resource().is_some() {
            let resource = permission.to_string();
            let resource = H::hash(resource.as_bytes());

            tracing::debug!("resource `{}` locked...", const_hex::encode(resource));

            self.data.set(Self::PREFIX_AUTH_LOCKED, &resource, &[])?;
        }

        let token = permission.to_string();
        let token = H::key(&token, owner);

        tracing::debug!("resource `{}` granted...", const_hex::encode(token));

        self.data.set(Self::PREFIX_AUTH, &token, &[])?;

        Ok(())
    }

    /// Grants the provided permission to the owner
    pub fn allow(&self, permission: &Permission) -> anyhow::Result<()> {
        if let Some(owner) = &self.owner {
            self._grant(permission, owner)?;
        }

        Ok(())
    }

    /// Asserts the authorization has the provided permission.
    pub fn ensure(&self, permission: &Permission) -> anyhow::Result<()> {
        if permission.resource().is_some() {
            let resource = permission.to_string();
            let resource = H::hash(resource.as_bytes());

            if self
                .data
                .get(Self::PREFIX_AUTH_LOCKED, &resource)?
                .is_some()
            {
                tracing::debug!(
                    "resource `{}` locked; ensuring permission...",
                    const_hex::encode(resource)
                );

                self._assert(permission)?;
            }
        }

        if let Some(owner) = &self.owner {
            tracing::debug!(
                "`{permission}` to `{}` ensured...",
                const_hex::encode(owner)
            );
        }

        Ok(())
    }

    /// Delegates the permission to another authorization.
    pub fn delegate(&self, permission: &Permission, delegated: &[u8]) -> anyhow::Result<()> {
        self._assert(permission)?;
        self._grant(permission, delegated)?;

        Ok(())
    }
}
