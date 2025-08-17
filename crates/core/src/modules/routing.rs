/// HTTP method enumeration for route definitions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    OPTIONS,
    HEAD,
    CONNECT,
    TRACE,
}

impl HttpMethod {
    /// Get the method as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::OPTIONS => "OPTIONS",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::CONNECT => "CONNECT",
            HttpMethod::TRACE => "TRACE",
        }
    }
    
    /// Check if method is safe (no side effects)
    pub fn is_safe(&self) -> bool {
        matches!(self, HttpMethod::GET | HttpMethod::HEAD | HttpMethod::OPTIONS | HttpMethod::TRACE)
    }
    
    /// Check if method is idempotent
    pub fn is_idempotent(&self) -> bool {
        matches!(
            self,
            HttpMethod::GET
                | HttpMethod::HEAD
                | HttpMethod::PUT
                | HttpMethod::DELETE
                | HttpMethod::OPTIONS
                | HttpMethod::TRACE
        )
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for HttpMethod {
    type Err = crate::errors::CoreError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            "PUT" => Ok(HttpMethod::PUT),
            "PATCH" => Ok(HttpMethod::PATCH),
            "DELETE" => Ok(HttpMethod::DELETE),
            "OPTIONS" => Ok(HttpMethod::OPTIONS),
            "HEAD" => Ok(HttpMethod::HEAD),
            "CONNECT" => Ok(HttpMethod::CONNECT),
            "TRACE" => Ok(HttpMethod::TRACE),
            _ => Err(crate::errors::CoreError::validation(
                format!("Invalid HTTP method: {}", s)
            )),
        }
    }
}

/// Route definition for module routing
#[derive(Debug, Clone)]
pub struct RouteDefinition {
    pub method: HttpMethod,
    pub path: String,
    pub handler: String,
    pub middleware: Vec<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub parameters: Vec<RouteParameter>,
}

impl RouteDefinition {
    /// Create a new route definition
    pub fn new(method: HttpMethod, path: impl Into<String>, handler: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            handler: handler.into(),
            middleware: Vec::new(),
            description: None,
            tags: Vec::new(),
            parameters: Vec::new(),
        }
    }
    
    /// Add middleware to the route
    pub fn with_middleware(mut self, middleware: Vec<String>) -> Self {
        self.middleware = middleware;
        self
    }
    
    /// Add a single middleware
    pub fn add_middleware(mut self, middleware: impl Into<String>) -> Self {
        self.middleware.push(middleware.into());
        self
    }
    
    /// Set route description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Add tags to the route
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    
    /// Add a single tag
    pub fn add_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
    
    /// Add parameters to the route
    pub fn with_parameters(mut self, parameters: Vec<RouteParameter>) -> Self {
        self.parameters = parameters;
        self
    }
    
    /// Add a single parameter
    pub fn add_parameter(mut self, parameter: RouteParameter) -> Self {
        self.parameters.push(parameter);
        self
    }
    
    /// Get route path with parameter placeholders
    pub fn path_pattern(&self) -> String {
        self.path.clone()
    }
    
    /// Check if route matches a given path and method
    pub fn matches(&self, method: &HttpMethod, path: &str) -> bool {
        self.method == *method && self.matches_path(path)
    }
    
    /// Check if route path matches (basic implementation)
    fn matches_path(&self, path: &str) -> bool {
        // This is a simplified implementation
        // In a real router, you would implement parameter matching
        self.path == path
    }
}

/// Route parameter definition
#[derive(Debug, Clone)]
pub struct RouteParameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub required: bool,
    pub description: Option<String>,
    pub default_value: Option<String>,
}

impl RouteParameter {
    /// Create a new route parameter
    pub fn new(name: impl Into<String>, parameter_type: ParameterType) -> Self {
        Self {
            name: name.into(),
            parameter_type,
            required: true,
            description: None,
            default_value: None,
        }
    }
    
    /// Make parameter optional
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
    
    /// Set parameter description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Set default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default_value = Some(default.into());
        self.required = false; // Default implies optional
        self
    }
}

/// Parameter type enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Uuid,
    Path,
    Query,
    Header,
    Body,
}

impl ParameterType {
    /// Get parameter type as string
    pub fn as_str(&self) -> &'static str {
        match self {
            ParameterType::String => "string",
            ParameterType::Integer => "integer",
            ParameterType::Float => "float",
            ParameterType::Boolean => "boolean",
            ParameterType::Uuid => "uuid",
            ParameterType::Path => "path",
            ParameterType::Query => "query",
            ParameterType::Header => "header",
            ParameterType::Body => "body",
        }
    }
}

/// Middleware definition for module middleware
#[derive(Debug, Clone)]
pub struct MiddlewareDefinition {
    pub name: String,
    pub priority: i32, // Lower numbers = higher priority (executed first)
    pub description: Option<String>,
    pub enabled: bool,
    pub config: std::collections::HashMap<String, String>,
}

impl MiddlewareDefinition {
    /// Create a new middleware definition
    pub fn new(name: impl Into<String>, priority: i32) -> Self {
        Self {
            name: name.into(),
            priority,
            description: None,
            enabled: true,
            config: std::collections::HashMap::new(),
        }
    }
    
    /// Set middleware description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Set middleware enabled status
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    /// Add configuration to middleware
    pub fn with_config(mut self, config: std::collections::HashMap<String, String>) -> Self {
        self.config = config;
        self
    }
    
    /// Add a single configuration value
    pub fn add_config(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.insert(key.into(), value.into());
        self
    }
}

impl PartialOrd for MiddlewareDefinition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MiddlewareDefinition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialEq for MiddlewareDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.priority == other.priority
    }
}

impl Eq for MiddlewareDefinition {}

/// Route group for organizing related routes
#[derive(Debug, Clone)]
pub struct RouteGroup {
    pub prefix: String,
    pub middleware: Vec<String>,
    pub routes: Vec<RouteDefinition>,
    pub name: Option<String>,
    pub description: Option<String>,
}

impl RouteGroup {
    /// Create a new route group
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            middleware: Vec::new(),
            routes: Vec::new(),
            name: None,
            description: None,
        }
    }
    
    /// Set group name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    
    /// Set group description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Add middleware to the group
    pub fn with_middleware(mut self, middleware: Vec<String>) -> Self {
        self.middleware = middleware;
        self
    }
    
    /// Add routes to the group
    pub fn with_routes(mut self, routes: Vec<RouteDefinition>) -> Self {
        self.routes = routes;
        self
    }
    
    /// Add a single route
    pub fn add_route(mut self, route: RouteDefinition) -> Self {
        self.routes.push(route);
        self
    }
    
    /// Get all routes with prefix applied
    pub fn prefixed_routes(&self) -> Vec<RouteDefinition> {
        self.routes
            .iter()
            .map(|route| {
                let mut prefixed_route = route.clone();
                prefixed_route.path = format!("{}{}", self.prefix, route.path);
                // Add group middleware to route middleware
                let mut middleware = self.middleware.clone();
                middleware.extend(route.middleware.clone());
                prefixed_route.middleware = middleware;
                prefixed_route
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_http_method() {
        assert_eq!(HttpMethod::GET.as_str(), "GET");
        assert!(HttpMethod::GET.is_safe());
        assert!(HttpMethod::GET.is_idempotent());
        assert!(!HttpMethod::POST.is_safe());
        assert!(!HttpMethod::POST.is_idempotent());
        
        assert_eq!("GET".parse::<HttpMethod>().unwrap(), HttpMethod::GET);
        assert!("INVALID".parse::<HttpMethod>().is_err());
    }
    
    #[test]
    fn test_route_definition() {
        let route = RouteDefinition::new(HttpMethod::GET, "/users/{id}", "get_user")
            .with_description("Get user by ID")
            .add_middleware("auth")
            .add_tag("users");
        
        assert_eq!(route.method, HttpMethod::GET);
        assert_eq!(route.path, "/users/{id}");
        assert_eq!(route.handler, "get_user");
        assert_eq!(route.middleware, vec!["auth"]);
        assert_eq!(route.tags, vec!["users"]);
        assert!(route.matches(&HttpMethod::GET, "/users/{id}"));
        assert!(!route.matches(&HttpMethod::POST, "/users/{id}"));
    }
    
    #[test]
    fn test_route_group() {
        let group = RouteGroup::new("/api/v1")
            .with_name("API v1")
            .add_route(RouteDefinition::new(HttpMethod::GET, "/users", "list_users"))
            .add_route(RouteDefinition::new(HttpMethod::POST, "/users", "create_user"));
        
        let prefixed_routes = group.prefixed_routes();
        assert_eq!(prefixed_routes.len(), 2);
        assert_eq!(prefixed_routes[0].path, "/api/v1/users");
        assert_eq!(prefixed_routes[1].path, "/api/v1/users");
    }
}