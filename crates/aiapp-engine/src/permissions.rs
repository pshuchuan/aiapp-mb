//! Permission gate: checks the permissions declared in the manifest before invoking host capabilities.
//!
//! Aligned with the Android permission model: the app declares required permissions in `aiapp.json`,
//! and the host grants them as needed. Before calling a capability, the runtime checks through this
//! module whether it has been granted.

use aiapp_format::manifest::permissions;
use aiapp_format::AppManifest;

/// Permission check result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permission {
    /// Granted.
    Granted,
    /// Not declared (the manifest does not include this permission).
    NotDeclared,
    /// Declared but not granted by the host (e.g. the host config only allows some permissions).
    Denied,
}

/// Authorization decision maker: determined jointly by the `AppManifest` + host policy.
#[derive(Debug, Clone)]
pub struct PermissionChecker {
    /// Declared permissions in the app manifest.
    manifest_permissions: Vec<String>,
    /// Permissions actually granted by the host (default = all declared in the manifest).
    granted: Vec<String>,
}

impl PermissionChecker {
    /// Build from a manifest: by default grant all permissions declared in the manifest.
    pub fn from_manifest(manifest: &AppManifest) -> Self {
        let manifest_permissions = manifest.permissions.clone();
        PermissionChecker {
            granted: manifest_permissions.clone(),
            manifest_permissions,
        }
    }

    /// Restrict the permissions actually granted by the host (e.g. an enterprise policy allows only storage).
    pub fn with_granted(mut self, granted: Vec<String>) -> Self {
        self.granted = granted;
        self
    }

    /// Check whether a permission is available.
    pub fn check(&self, perm: &str) -> Permission {
        if !self.manifest_permissions.iter().any(|p| p == perm) {
            return Permission::NotDeclared;
        }
        if self.granted.iter().any(|p| p == perm) {
            Permission::Granted
        } else {
            Permission::Denied
        }
    }

    /// Check the storage permission (commonly used by data apps).
    pub fn can_use_storage(&self) -> bool {
        self.check(permissions::STORAGE) == Permission::Granted
    }

    /// Check the notifications permission.
    pub fn can_notify(&self) -> bool {
        self.check(permissions::NOTIFICATIONS) == Permission::Granted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiapp_format::AppManifest;

    #[test]
    fn storage_permission() {
        let m = AppManifest::new("Todo", "todo", "todo_app");
        let checker = PermissionChecker::from_manifest(&m);
        assert_eq!(checker.check(permissions::STORAGE), Permission::Granted);
        // the todo template does not declare notifications
        assert_eq!(
            checker.check(permissions::NOTIFICATIONS),
            Permission::NotDeclared
        );
    }

    #[test]
    fn grant_policy() {
        let m = AppManifest::new("Todo", "todo", "todo_app");
        let checker = PermissionChecker::from_manifest(&m).with_granted(vec![]);
        assert_eq!(checker.check(permissions::STORAGE), Permission::Denied);
    }
}
