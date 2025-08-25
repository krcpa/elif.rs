//! Role-Based Access Control (RBAC) system
//!
//! This module provides a comprehensive RBAC implementation with roles,
//! permissions, hierarchical role support, and efficient authorization checking.

use crate::traits::{Authenticatable, AuthorizationProvider};
use crate::{AuthError, AuthResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Represents a role in the RBAC system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Role {
    /// Unique role identifier
    pub id: String,

    /// Human-readable role name
    pub name: String,

    /// Optional description of the role
    pub description: Option<String>,

    /// Parent roles (for hierarchical RBAC)
    pub parent_roles: Vec<String>,

    /// Direct permissions assigned to this role
    pub permissions: Vec<String>,

    /// Whether this role is active
    pub is_active: bool,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Role {
    /// Create a new role
    pub fn new(id: String, name: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            description: None,
            parent_roles: vec![],
            permissions: vec![],
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a parent role (for hierarchical RBAC)
    pub fn add_parent_role(&mut self, role_id: String) {
        if !self.parent_roles.contains(&role_id) {
            self.parent_roles.push(role_id);
            self.updated_at = Utc::now();
        }
    }

    /// Add a permission to this role
    pub fn add_permission(&mut self, permission: String) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a permission from this role
    pub fn remove_permission(&mut self, permission: &str) {
        if let Some(pos) = self.permissions.iter().position(|p| p == permission) {
            self.permissions.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    /// Check if role has a specific permission directly
    pub fn has_direct_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }
}

/// Represents a permission in the RBAC system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Permission {
    /// Unique permission identifier
    pub id: String,

    /// Human-readable permission name
    pub name: String,

    /// Optional description of the permission
    pub description: Option<String>,

    /// Resource this permission applies to
    pub resource: String,

    /// Action this permission allows
    pub action: String,

    /// Optional conditions for this permission
    pub conditions: HashMap<String, serde_json::Value>,

    /// Whether this permission is active
    pub is_active: bool,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Permission {
    /// Create a new permission
    pub fn new(id: String, name: String, resource: String, action: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            description: None,
            resource,
            action,
            conditions: HashMap::new(),
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a condition to this permission
    pub fn add_condition(&mut self, key: String, value: serde_json::Value) {
        self.conditions.insert(key, value);
        self.updated_at = Utc::now();
    }

    /// Check if this permission matches a resource and action
    pub fn matches(&self, resource: &str, action: &str) -> bool {
        self.is_active
            && (self.resource == "*" || self.resource == resource)
            && (self.action == "*" || self.action == action)
    }

    /// Check if conditions are satisfied
    pub fn check_conditions(&self, context: &HashMap<String, serde_json::Value>) -> bool {
        if self.conditions.is_empty() {
            return true;
        }

        // All conditions must be satisfied
        for (key, expected_value) in &self.conditions {
            match context.get(key) {
                Some(actual_value) if actual_value == expected_value => continue,
                _ => return false,
            }
        }

        true
    }
}

/// User role assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRole {
    /// User ID
    pub user_id: String,

    /// Role ID
    pub role_id: String,

    /// When this assignment was created
    pub assigned_at: DateTime<Utc>,

    /// Optional expiration time for the role assignment
    pub expires_at: Option<DateTime<Utc>>,

    /// Who assigned this role
    pub assigned_by: Option<String>,

    /// Whether this assignment is active
    pub is_active: bool,
}

impl UserRole {
    /// Create a new user role assignment
    pub fn new(user_id: String, role_id: String) -> Self {
        Self {
            user_id,
            role_id,
            assigned_at: Utc::now(),
            expires_at: None,
            assigned_by: None,
            is_active: true,
        }
    }

    /// Check if this role assignment is currently valid
    pub fn is_valid(&self) -> bool {
        self.is_active && !self.is_expired()
    }

    /// Check if this role assignment has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| Utc::now() > exp)
    }
}

/// RBAC authorization provider trait
#[async_trait]
pub trait RbacProvider: Send + Sync {
    /// Get a role by ID
    async fn get_role(&self, role_id: &str) -> AuthResult<Option<Role>>;

    /// Get a permission by ID
    async fn get_permission(&self, permission_id: &str) -> AuthResult<Option<Permission>>;

    /// Get all roles for a user
    async fn get_user_roles(&self, user_id: &str) -> AuthResult<Vec<Role>>;

    /// Get all permissions for a user (direct and through roles)
    async fn get_user_permissions(&self, user_id: &str) -> AuthResult<Vec<Permission>>;

    /// Assign a role to a user
    async fn assign_role_to_user(&self, user_id: &str, role_id: &str) -> AuthResult<()>;

    /// Remove a role from a user
    async fn remove_role_from_user(&self, user_id: &str, role_id: &str) -> AuthResult<()>;

    /// Check if a user has a specific role (including inherited roles)
    async fn user_has_role(&self, user_id: &str, role_id: &str) -> AuthResult<bool>;

    /// Check if a user has a specific permission
    async fn user_has_permission(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
        context: Option<&HashMap<String, serde_json::Value>>,
    ) -> AuthResult<bool>;

    /// Get effective roles for a user (including inherited roles)
    async fn get_effective_user_roles(&self, user_id: &str) -> AuthResult<Vec<Role>>;
}

/// In-memory RBAC provider for testing and simple use cases
#[derive(Debug, Clone)]
pub struct InMemoryRbacProvider {
    roles: HashMap<String, Role>,
    permissions: HashMap<String, Permission>,
    user_roles: HashMap<String, Vec<UserRole>>,
}

impl InMemoryRbacProvider {
    /// Create a new in-memory RBAC provider
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
            permissions: HashMap::new(),
            user_roles: HashMap::new(),
        }
    }

    /// Add a role
    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.id.clone(), role);
    }

    /// Add a permission
    pub fn add_permission(&mut self, permission: Permission) {
        self.permissions.insert(permission.id.clone(), permission);
    }

    /// Assign a role to a user
    pub fn assign_role_to_user_mut(&mut self, user_id: &str, role_id: &str) -> AuthResult<()> {
        // Check if role exists
        if !self.roles.contains_key(role_id) {
            return Err(AuthError::access_denied(format!(
                "Role '{}' does not exist",
                role_id
            )));
        }

        let user_roles = self.user_roles.entry(user_id.to_string()).or_default();

        // Check if user already has this role
        if user_roles
            .iter()
            .any(|ur| ur.role_id == role_id && ur.is_valid())
        {
            return Ok(()); // Already assigned
        }

        // Add the role assignment
        let assignment = UserRole::new(user_id.to_string(), role_id.to_string());
        user_roles.push(assignment);

        Ok(())
    }

    /// Remove a role from a user
    pub fn remove_role_from_user_mut(&mut self, user_id: &str, role_id: &str) -> AuthResult<()> {
        if let Some(user_roles) = self.user_roles.get_mut(user_id) {
            user_roles.retain(|ur| !(ur.role_id == role_id && ur.is_valid()));
        }
        Ok(())
    }

    /// Get all users with a specific role
    pub fn get_users_with_role(&self, role_id: &str) -> Vec<String> {
        let mut users = Vec::new();

        for (user_id, user_roles) in &self.user_roles {
            if user_roles
                .iter()
                .any(|ur| ur.role_id == role_id && ur.is_valid())
            {
                users.push(user_id.clone());
            }
        }

        users
    }

    /// Get role assignment statistics
    pub fn get_role_stats(&self) -> HashMap<String, u32> {
        let mut stats = HashMap::new();

        for role_id in self.roles.keys() {
            let count = self.get_users_with_role(role_id).len() as u32;
            stats.insert(role_id.clone(), count);
        }

        stats
    }

    /// Get all roles transitively inherited by a role
    fn get_inherited_roles(&self, role_id: &str, visited: &mut HashSet<String>) -> Vec<Role> {
        let mut inherited = vec![];

        if visited.contains(role_id) {
            return inherited; // Prevent cycles
        }

        visited.insert(role_id.to_string());

        if let Some(role) = self.roles.get(role_id) {
            inherited.push(role.clone());

            // Add parent roles recursively
            for parent_id in &role.parent_roles {
                inherited.extend(self.get_inherited_roles(parent_id, visited));
            }
        }

        inherited
    }
}

impl Default for InMemoryRbacProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RbacProvider for InMemoryRbacProvider {
    async fn get_role(&self, role_id: &str) -> AuthResult<Option<Role>> {
        Ok(self.roles.get(role_id).cloned())
    }

    async fn get_permission(&self, permission_id: &str) -> AuthResult<Option<Permission>> {
        Ok(self.permissions.get(permission_id).cloned())
    }

    async fn get_user_roles(&self, user_id: &str) -> AuthResult<Vec<Role>> {
        let empty_vec = vec![];
        let user_role_assignments = self.user_roles.get(user_id).unwrap_or(&empty_vec);
        let mut roles = vec![];

        for assignment in user_role_assignments {
            if assignment.is_valid() {
                if let Some(role) = self.roles.get(&assignment.role_id) {
                    if role.is_active {
                        roles.push(role.clone());
                    }
                }
            }
        }

        Ok(roles)
    }

    async fn get_user_permissions(&self, user_id: &str) -> AuthResult<Vec<Permission>> {
        let roles = self.get_effective_user_roles(user_id).await?;
        let mut permissions = HashMap::new();

        // Collect permissions from all roles
        for role in roles {
            for permission_id in &role.permissions {
                if let Some(permission) = self.permissions.get(permission_id) {
                    if permission.is_active {
                        permissions.insert(permission.id.clone(), permission.clone());
                    }
                }
            }
        }

        Ok(permissions.into_values().collect())
    }

    async fn assign_role_to_user(&self, _user_id: &str, _role_id: &str) -> AuthResult<()> {
        // Note: This is a read-only method in the current trait design
        // In a real implementation with mutable storage, this would work
        // For now, we need to use the mutable methods on the provider directly
        Err(AuthError::access_denied(
            "Use assign_role_to_user_mut on the provider directly",
        ))
    }

    async fn remove_role_from_user(&self, _user_id: &str, _role_id: &str) -> AuthResult<()> {
        // Note: This is a read-only method in the current trait design
        // In a real implementation with mutable storage, this would work
        // For now, we need to use the mutable methods on the provider directly
        Err(AuthError::access_denied(
            "Use remove_role_from_user_mut on the provider directly",
        ))
    }

    async fn user_has_role(&self, user_id: &str, role_id: &str) -> AuthResult<bool> {
        let effective_roles = self.get_effective_user_roles(user_id).await?;
        Ok(effective_roles.iter().any(|role| role.id == role_id))
    }

    async fn user_has_permission(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
        context: Option<&HashMap<String, serde_json::Value>>,
    ) -> AuthResult<bool> {
        let permissions = self.get_user_permissions(user_id).await?;
        let empty_context = HashMap::new();
        let context = context.unwrap_or(&empty_context);

        for permission in permissions {
            if permission.matches(resource, action) && permission.check_conditions(context) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn get_effective_user_roles(&self, user_id: &str) -> AuthResult<Vec<Role>> {
        let direct_roles = self.get_user_roles(user_id).await?;
        let mut all_roles = HashMap::new();

        // Get all roles including inherited ones
        for role in direct_roles {
            let mut visited = HashSet::new();
            let inherited = self.get_inherited_roles(&role.id, &mut visited);

            for inherited_role in inherited {
                if inherited_role.is_active {
                    all_roles.insert(inherited_role.id.clone(), inherited_role);
                }
            }
        }

        Ok(all_roles.into_values().collect())
    }
}

/// Adapter that implements AuthorizationProvider using RbacProvider
pub struct RbacAuthorizationAdapter<R, U>
where
    R: RbacProvider,
    U: Authenticatable,
{
    rbac_provider: R,
    _phantom: std::marker::PhantomData<U>,
}

impl<R, U> RbacAuthorizationAdapter<R, U>
where
    R: RbacProvider,
    U: Authenticatable,
{
    /// Create a new RBAC authorization adapter
    pub fn new(rbac_provider: R) -> Self {
        Self {
            rbac_provider,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get the underlying RBAC provider
    pub fn rbac_provider(&self) -> &R {
        &self.rbac_provider
    }
}

#[async_trait]
impl<R, U> AuthorizationProvider for RbacAuthorizationAdapter<R, U>
where
    R: RbacProvider + Send + Sync,
    U: Authenticatable,
    U::Id: std::fmt::Display,
{
    type User = U;
    type Role = Role;
    type Permission = Permission;

    async fn has_role(&self, user: &Self::User, role: &str) -> AuthResult<bool> {
        let user_id = user.id().to_string();
        self.rbac_provider.user_has_role(&user_id, role).await
    }

    async fn has_permission(&self, user: &Self::User, permission: &str) -> AuthResult<bool> {
        // For backwards compatibility, parse permission as "resource.action"
        let parts: Vec<&str> = permission.split('.').collect();
        if parts.len() == 2 {
            let resource = parts[0];
            let action = parts[1];
            let user_id = user.id().to_string();
            self.rbac_provider
                .user_has_permission(&user_id, resource, action, None)
                .await
        } else {
            // If not in expected format, try direct permission lookup
            let user_id = user.id().to_string();
            self.rbac_provider
                .user_has_permission(&user_id, "*", permission, None)
                .await
        }
    }

    async fn has_permission_with_context(
        &self,
        user: &Self::User,
        resource: &str,
        action: &str,
        context: Option<&HashMap<String, serde_json::Value>>,
    ) -> AuthResult<bool> {
        let user_id = user.id().to_string();
        self.rbac_provider
            .user_has_permission(&user_id, resource, action, context)
            .await
    }

    async fn get_user_roles(&self, user: &Self::User) -> AuthResult<Vec<Self::Role>> {
        let user_id = user.id().to_string();
        self.rbac_provider.get_effective_user_roles(&user_id).await
    }

    async fn get_user_permissions(&self, user: &Self::User) -> AuthResult<Vec<Self::Permission>> {
        let user_id = user.id().to_string();
        self.rbac_provider.get_user_permissions(&user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_role_creation() {
        let role = Role::new("admin".to_string(), "Administrator".to_string());

        assert_eq!(role.id, "admin");
        assert_eq!(role.name, "Administrator");
        assert!(role.is_active);
        assert!(role.permissions.is_empty());
        assert!(role.parent_roles.is_empty());
    }

    #[test]
    fn test_role_permission_management() {
        let mut role = Role::new("editor".to_string(), "Content Editor".to_string());

        role.add_permission("articles.create".to_string());
        role.add_permission("articles.edit".to_string());

        assert!(role.has_direct_permission("articles.create"));
        assert!(role.has_direct_permission("articles.edit"));
        assert!(!role.has_direct_permission("articles.delete"));

        role.remove_permission("articles.edit");
        assert!(!role.has_direct_permission("articles.edit"));
    }

    #[test]
    fn test_permission_creation() {
        let permission = Permission::new(
            "articles.create".to_string(),
            "Create Articles".to_string(),
            "articles".to_string(),
            "create".to_string(),
        );

        assert_eq!(permission.id, "articles.create");
        assert_eq!(permission.resource, "articles");
        assert_eq!(permission.action, "create");
        assert!(permission.is_active);
    }

    #[test]
    fn test_permission_matching() {
        let mut permission = Permission::new(
            "articles.create".to_string(),
            "Create Articles".to_string(),
            "articles".to_string(),
            "create".to_string(),
        );

        assert!(permission.matches("articles", "create"));
        assert!(!permission.matches("articles", "delete"));
        assert!(!permission.matches("users", "create"));

        // Test wildcard resource
        permission.resource = "*".to_string();
        assert!(permission.matches("articles", "create"));
        assert!(permission.matches("users", "create"));

        // Test wildcard action
        permission.resource = "articles".to_string();
        permission.action = "*".to_string();
        assert!(permission.matches("articles", "create"));
        assert!(permission.matches("articles", "delete"));
        assert!(!permission.matches("users", "create"));
    }

    #[test]
    fn test_permission_conditions() {
        let mut permission = Permission::new(
            "articles.edit".to_string(),
            "Edit Own Articles".to_string(),
            "articles".to_string(),
            "edit".to_string(),
        );

        permission.add_condition("owner".to_string(), json!("self"));

        let mut context = HashMap::new();
        context.insert("owner".to_string(), json!("self"));

        assert!(permission.check_conditions(&context));

        context.insert("owner".to_string(), json!("other"));
        assert!(!permission.check_conditions(&context));
    }

    #[test]
    fn test_user_role_assignment() {
        let assignment = UserRole::new("user123".to_string(), "admin".to_string());

        assert_eq!(assignment.user_id, "user123");
        assert_eq!(assignment.role_id, "admin");
        assert!(assignment.is_valid());
        assert!(!assignment.is_expired());
    }

    #[tokio::test]
    async fn test_in_memory_rbac_provider() {
        let mut provider = InMemoryRbacProvider::new();

        // Create test role and permission
        let mut admin_role = Role::new("admin".to_string(), "Administrator".to_string());
        admin_role.add_permission("users.create".to_string());

        let permission = Permission::new(
            "users.create".to_string(),
            "Create Users".to_string(),
            "users".to_string(),
            "create".to_string(),
        );

        provider.add_role(admin_role.clone());
        provider.add_permission(permission);

        // Test role retrieval
        let retrieved_role = provider.get_role("admin").await.unwrap();
        assert_eq!(retrieved_role, Some(admin_role));

        // Test non-existent role
        let non_existent = provider.get_role("nonexistent").await.unwrap();
        assert_eq!(non_existent, None);
    }

    #[tokio::test]
    async fn test_hierarchical_roles() {
        let mut provider = InMemoryRbacProvider::new();

        // Create role hierarchy: admin -> manager -> employee
        let mut admin_role = Role::new("admin".to_string(), "Administrator".to_string());
        admin_role.add_permission("system.admin".to_string());

        let mut manager_role = Role::new("manager".to_string(), "Manager".to_string());
        manager_role.add_parent_role("admin".to_string());
        manager_role.add_permission("team.manage".to_string());

        let mut employee_role = Role::new("employee".to_string(), "Employee".to_string());
        employee_role.add_parent_role("manager".to_string());
        employee_role.add_permission("tasks.view".to_string());

        provider.add_role(admin_role);
        provider.add_role(manager_role);
        provider.add_role(employee_role);

        // Add permissions
        let admin_perm = Permission::new(
            "system.admin".to_string(),
            "System Administration".to_string(),
            "system".to_string(),
            "admin".to_string(),
        );
        let manager_perm = Permission::new(
            "team.manage".to_string(),
            "Team Management".to_string(),
            "team".to_string(),
            "manage".to_string(),
        );
        let employee_perm = Permission::new(
            "tasks.view".to_string(),
            "View Tasks".to_string(),
            "tasks".to_string(),
            "view".to_string(),
        );

        provider.add_permission(admin_perm);
        provider.add_permission(manager_perm);
        provider.add_permission(employee_perm);

        // Test that employee role inherits from manager and admin
        let mut visited = HashSet::new();
        let inherited = provider.get_inherited_roles("employee", &mut visited);

        let role_ids: Vec<&str> = inherited.iter().map(|r| r.id.as_str()).collect();
        assert!(role_ids.contains(&"employee"));
        assert!(role_ids.contains(&"manager"));
        assert!(role_ids.contains(&"admin"));
    }

    // Mock user for testing
    #[derive(Debug, Clone)]
    struct MockUser {
        id: String,
        username: String,
    }

    #[async_trait]
    impl Authenticatable for MockUser {
        type Id = String;
        type Credentials = String;

        fn id(&self) -> &Self::Id {
            &self.id
        }

        fn username(&self) -> &str {
            &self.username
        }

        async fn verify_credentials(&self, _credentials: &Self::Credentials) -> AuthResult<bool> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_rbac_authorization_adapter() {
        let mut rbac_provider = InMemoryRbacProvider::new();

        // Create test role and permission
        let mut admin_role = Role::new("admin".to_string(), "Administrator".to_string());
        admin_role.add_permission("users.create".to_string());

        let permission = Permission::new(
            "users.create".to_string(),
            "Create Users".to_string(),
            "users".to_string(),
            "create".to_string(),
        );

        rbac_provider.add_role(admin_role);
        rbac_provider.add_permission(permission);

        // Create adapter
        let adapter = RbacAuthorizationAdapter::<_, MockUser>::new(rbac_provider);

        // Create test user
        let user = MockUser {
            id: "user123".to_string(),
            username: "admin@example.com".to_string(),
        };

        // Test permission checking (backwards compatibility)
        let has_permission = adapter.has_permission(&user, "users.create").await.unwrap();
        // This will be false because we haven't assigned the role to the user yet
        assert!(!has_permission);

        // Test permission with context
        let has_permission_with_context = adapter
            .has_permission_with_context(&user, "users", "create", None)
            .await
            .unwrap();
        assert!(!has_permission_with_context);

        // Test role checking
        let has_role = adapter.has_role(&user, "admin").await.unwrap();
        assert!(!has_role);
    }

    #[tokio::test]
    async fn test_role_assignment_and_checking() {
        let mut provider = InMemoryRbacProvider::new();

        // Create test role and permissions
        let mut admin_role = Role::new("admin".to_string(), "Administrator".to_string());
        admin_role.add_permission("users.create".to_string());
        admin_role.add_permission("users.delete".to_string());

        let create_permission = Permission::new(
            "users.create".to_string(),
            "Create Users".to_string(),
            "users".to_string(),
            "create".to_string(),
        );

        let delete_permission = Permission::new(
            "users.delete".to_string(),
            "Delete Users".to_string(),
            "users".to_string(),
            "delete".to_string(),
        );

        provider.add_role(admin_role);
        provider.add_permission(create_permission);
        provider.add_permission(delete_permission);

        let user_id = "user123";

        // Initially user has no roles
        assert!(!provider.user_has_role(user_id, "admin").await.unwrap());
        assert!(!provider
            .user_has_permission(user_id, "users", "create", None)
            .await
            .unwrap());

        // Assign admin role to user
        provider.assign_role_to_user_mut(user_id, "admin").unwrap();

        // Now user should have admin role and permissions
        assert!(provider.user_has_role(user_id, "admin").await.unwrap());
        assert!(provider
            .user_has_permission(user_id, "users", "create", None)
            .await
            .unwrap());
        assert!(provider
            .user_has_permission(user_id, "users", "delete", None)
            .await
            .unwrap());
        assert!(!provider
            .user_has_permission(user_id, "posts", "create", None)
            .await
            .unwrap());

        // Test role assignment twice (should be idempotent)
        provider.assign_role_to_user_mut(user_id, "admin").unwrap();
        assert!(provider.user_has_role(user_id, "admin").await.unwrap());

        // Test role removal
        provider
            .remove_role_from_user_mut(user_id, "admin")
            .unwrap();
        assert!(!provider.user_has_role(user_id, "admin").await.unwrap());
        assert!(!provider
            .user_has_permission(user_id, "users", "create", None)
            .await
            .unwrap());

        // Test assigning non-existent role
        let result = provider.assign_role_to_user_mut(user_id, "nonexistent");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_permission_context_checking() {
        let mut provider = InMemoryRbacProvider::new();

        // Create a role with conditional permission
        let mut editor_role = Role::new("editor".to_string(), "Content Editor".to_string());
        editor_role.add_permission("articles.edit".to_string());

        let mut edit_permission = Permission::new(
            "articles.edit".to_string(),
            "Edit Articles".to_string(),
            "articles".to_string(),
            "edit".to_string(),
        );

        // Add condition: user can only edit their own articles
        edit_permission.add_condition("owner".to_string(), serde_json::json!("self"));

        provider.add_role(editor_role);
        provider.add_permission(edit_permission);

        let user_id = "user123";
        provider.assign_role_to_user_mut(user_id, "editor").unwrap();

        // Test without context (should fail because of condition)
        assert!(!provider
            .user_has_permission(user_id, "articles", "edit", None)
            .await
            .unwrap());

        // Test with wrong context
        let mut wrong_context = HashMap::new();
        wrong_context.insert("owner".to_string(), serde_json::json!("other"));
        assert!(!provider
            .user_has_permission(user_id, "articles", "edit", Some(&wrong_context))
            .await
            .unwrap());

        // Test with correct context
        let mut correct_context = HashMap::new();
        correct_context.insert("owner".to_string(), serde_json::json!("self"));
        assert!(provider
            .user_has_permission(user_id, "articles", "edit", Some(&correct_context))
            .await
            .unwrap());
    }

    #[test]
    fn test_role_statistics() {
        let mut provider = InMemoryRbacProvider::new();

        // Create roles
        let admin_role = Role::new("admin".to_string(), "Administrator".to_string());
        let editor_role = Role::new("editor".to_string(), "Editor".to_string());

        provider.add_role(admin_role);
        provider.add_role(editor_role);

        // Initially no users assigned
        let stats = provider.get_role_stats();
        assert_eq!(stats.get("admin"), Some(&0));
        assert_eq!(stats.get("editor"), Some(&0));

        // Assign roles to users
        provider.assign_role_to_user_mut("user1", "admin").unwrap();
        provider.assign_role_to_user_mut("user2", "admin").unwrap();
        provider.assign_role_to_user_mut("user3", "editor").unwrap();

        let stats = provider.get_role_stats();
        assert_eq!(stats.get("admin"), Some(&2));
        assert_eq!(stats.get("editor"), Some(&1));

        // Test getting users with role
        let admin_users = provider.get_users_with_role("admin");
        assert_eq!(admin_users.len(), 2);
        assert!(admin_users.contains(&"user1".to_string()));
        assert!(admin_users.contains(&"user2".to_string()));

        let editor_users = provider.get_users_with_role("editor");
        assert_eq!(editor_users.len(), 1);
        assert!(editor_users.contains(&"user3".to_string()));
    }
}
