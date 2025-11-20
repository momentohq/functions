//! Host interface for working with Momento Token apis
use momento_functions_wit::host::momento::functions::token;
use momento_functions_wit::host::momento::functions::token::Expires;
use serde::{Deserialize, Serialize};

/// An error occurred when generating a disposable token
#[derive(thiserror::Error, Debug)]
pub enum GenerateDisposableTokenError {
    /// An error occurred when calling the host generate function
    #[error(transparent)]
    TokenError(#[from] token::TokenError),
}

/// Generate a disposable token with scoped permissions.
pub fn generate_disposable_token(
    valid_for_seconds: u32,
    permissions: Permissions,
    token_id: Option<String>,
) -> Result<FunctionHostGenerateDisposableTokenResponse, GenerateDisposableTokenError> {
    let stringified_permissions = serde_json::to_string(&permissions).map_err(|e| {
        GenerateDisposableTokenError::TokenError(token::TokenError::InvalidArgument(format!(
            "Invalid permissions object passed in: {e:?}"
        )))
    })?;
    log::debug!("generated permissions JSON string: {stringified_permissions}");
    token::generate_disposable_token(
        Expires { valid_for_seconds },
        &stringified_permissions,
        token_id.as_deref(),
    )
    .map_err(Into::into)
    .map(Into::into)
}

/// The translated response from the .WIT interface to a Rust-native one
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionHostGenerateDisposableTokenResponse {
    /// The new, disposable token with scoped permissions
    pub api_key: String,
    /// The endpoint to be used against when calling Momento with this token
    pub endpoint: String,
    /// How long the token is valid for, in epoch seconds
    pub valid_until: u64,
}

impl From<momento_functions_wit::host::momento::functions::token::GenerateDisposableTokenResponse>
    for FunctionHostGenerateDisposableTokenResponse
{
    fn from(
        response: momento_functions_wit::host::momento::functions::token::GenerateDisposableTokenResponse,
    ) -> Self {
        FunctionHostGenerateDisposableTokenResponse {
            api_key: response.api_key,
            endpoint: response.endpoint,
            valid_until: response.valid_until,
        }
    }
}

/// Role defining the level of access to cache operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CacheRole {
    /// No cache access permitted.
    CachePermitNone = 0,
    /// Full read and write access.
    CacheReadWrite = 1,
    /// Read-only access.
    CacheReadOnly = 2,
    /// Write-only access to cache data.
    /// Does not allow conditional write APIs (SetIfNotExists, IncreaseTTL, etc.).
    CacheWriteOnly = 3,
}

/// Role defining the level of access to topic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TopicRole {
    /// No topic access permitted.
    TopicPermitNone = 0,
    /// Full read and write access.
    TopicReadWrite = 1,
    /// Allows subscribing to topics only.
    TopicReadOnly = 2,
    /// Allows publishing to topics only.
    TopicWriteOnly = 3,
}

/// Role defining the level of access to function operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FunctionRole {
    /// No function access permitted.
    FunctionPermitNone = 0,
    /// Permission to invoke functions.
    FunctionInvoke = 1,
}

/// Top-level permissions structure. Only scoped permissions can be added,
/// as diposable tokens do not allow the generation of super user tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Permissions {
    /// Scoped permissions for your generated token. We wrap this in a top-level
    /// struct so the generated JSON is compatible with host interpolation.
    explicit: ExplicitPermissions,
}

impl Permissions {
    /// Create new explicit permissions with no access by default.
    ///
    /// This creates an empty permission set. Use `with_cache`, `with_topic`,
    /// and `with_function` to add specific permissions.
    ///
    /// # Example
    ///
    /// ```
    /// let perms = Permissions::new()
    ///     .with_cache(CachePermissions::read_write())
    ///     .with_topic(TopicPermissions::read_only());
    /// ```
    pub fn new() -> Self {
        Self {
            explicit: ExplicitPermissions {
                permissions: Vec::new(),
            },
        }
    }

    /// Add a cache permission.
    pub fn with_cache(mut self, permission: CachePermissions) -> Self {
        self.explicit
            .permissions
            .push(PermissionsType::CachePermissions(permission));
        self
    }

    /// Add a topic permission.
    pub fn with_topic(mut self, permission: TopicPermissions) -> Self {
        self.explicit
            .permissions
            .push(PermissionsType::TopicPermissions(permission));
        self
    }

    /// Add a function permission.
    pub fn with_function(mut self, permission: FunctionPermissions) -> Self {
        self.explicit
            .permissions
            .push(PermissionsType::FunctionPermissions(permission));
        self
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper struct for explicit permissions, exists mainly for JSON interpolation within the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplicitPermissions {
    /// List of individual permission types.
    pub permissions: Vec<PermissionsType>,
}

/// Captures the kinds of permissions you can scope your tokens to.
///
/// A single permission can be for caches, topics, or functions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionsType {
    /// Cache-related permissions.
    CachePermissions(CachePermissions),
    /// Topic-related permissions.
    TopicPermissions(TopicPermissions),
    /// Function-related permissions.
    FunctionPermissions(FunctionPermissions),
}

/// Selector for targeting specific caches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CacheSelector {
    /// Select a cache by its exact name.
    CacheName(String),
}

/// Selector for targeting specific cache items.
///
/// Used to limit permissions to particular keys or key prefixes within a cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheItemSelector {
    /// Select a specific cache key.
    Key(Vec<u8>),
    /// Select all keys with a specific prefix.
    KeyPrefix(Vec<u8>),
}

/// Permissions for a cache(s).
///
/// Defines the role (read, write, or both), which caches are accessible,
/// and which items within those caches can be accessed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachePermissions {
    /// The level of access granted (read, write, or both).
    role: CacheRole,
    /// Has permissions for all caches
    #[serde(skip_serializing_if = "Option::is_none")]
    all_caches: Option<()>,
    /// Which cache(s) this permission applies to.
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_selector: Option<CacheSelector>,
    /// Has permissions for all items
    #[serde(skip_serializing_if = "Option::is_none")]
    all_items: Option<()>,
    /// Which items within the cache(s) this permission applies to.
    #[serde(skip_serializing_if = "Option::is_none")]
    item_selector: Option<CacheItemSelector>,
}

impl CachePermissions {
    /// Create cache permissions with no access by default.
    ///
    /// Defaults to no access on all caches and all items. Use `with_role`,
    /// `with_cache`, and `with_items` to configure access.
    ///
    /// # Example
    ///
    /// ```
    /// let perms = CachePermissions::new()
    ///     .with_role(CacheRole::CacheReadWrite)
    ///     .with_cache("my-cache")
    ///     .with_all_items();
    /// ```
    pub fn new() -> Self {
        Self {
            role: CacheRole::CachePermitNone,
            all_caches: None,
            cache_selector: None,
            all_items: None,
            item_selector: None,
        }
    }

    /// Create read-write cache permissions.
    ///
    /// Defaults to all caches and all items.
    ///
    /// # Example
    ///
    /// ```
    /// let perms = CachePermissions::read_write()
    ///     .with_cache("my-cache");
    /// ```
    pub fn read_write() -> Self {
        Self {
            role: CacheRole::CacheReadWrite,
            all_caches: Some(()),
            cache_selector: None,
            all_items: Some(()),
            item_selector: None,
        }
    }

    /// Create read-only cache permissions.
    ///
    /// Defaults to all caches and all items.
    ///
    /// # Example
    ///
    /// ```
    /// let perms = CachePermissions::read_only()
    ///     .with_cache("my-cache");
    /// ```
    pub fn read_only() -> Self {
        Self {
            role: CacheRole::CacheReadOnly,
            all_caches: Some(()),
            cache_selector: None,
            all_items: Some(()),
            item_selector: None,
        }
    }

    /// Create write-only cache permissions.
    ///
    /// Defaults to all caches and all items.
    ///
    /// # Example
    ///
    /// ```
    /// let perms = CachePermissions::write_only()
    ///     .with_cache("my-cache");
    /// ```
    pub fn write_only() -> Self {
        Self {
            role: CacheRole::CacheWriteOnly,
            all_caches: Some(()),
            cache_selector: None,
            all_items: Some(()),
            item_selector: None,
        }
    }

    /// Set the access role, such as `CacheReadWrite` or `CacheReadOnly`.
    pub fn with_role(mut self, role: CacheRole) -> Self {
        self.role = role;
        self
    }

    /// Grant access to all caches.
    pub fn with_all_caches(mut self) -> Self {
        self.all_caches = Some(());
        self
    }

    /// Grant access to a specific cache.
    pub fn with_cache(mut self, cache_name: impl Into<String>) -> Self {
        self.cache_selector = Some(CacheSelector::CacheName(cache_name.into()));
        self
    }

    /// Grant access to all items in the cache(s).
    pub fn with_all_items(mut self) -> Self {
        self.all_items = Some(());
        self
    }

    /// Grant access to a specific key within the cache.
    pub fn with_key(mut self, key: impl Into<Vec<u8>>) -> Self {
        self.item_selector = Some(CacheItemSelector::Key(key.into()));
        self
    }

    /// Grant access to all keys with a specific prefix.
    pub fn with_key_prefix(mut self, prefix: impl Into<Vec<u8>>) -> Self {
        self.item_selector = Some(CacheItemSelector::KeyPrefix(prefix.into()));
        self
    }
}

impl Default for CachePermissions {
    fn default() -> Self {
        Self::new()
    }
}

/// Selector for targeting specific topics.
///
/// Used to limit permissions to particular topics by name or name prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicSelector {
    /// Select a topic by its exact name.
    TopicName(String),
    /// Select all topics with a specific name prefix.
    TopicNamePrefix(String),
}

/// Permissions for a topic(s).
///
/// Defines the role (publish, subscribe, or both), which caches contain the topics,
/// and which specific topics are accessible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicPermissions {
    /// The level of access granted (publish, subscribe, or both).
    role: TopicRole,
    /// Has permissions for all caches
    #[serde(skip_serializing_if = "Option::is_none")]
    all_caches: Option<()>,
    /// Which cache(s) this permission applies to.
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_selector: Option<CacheSelector>,
    /// Has permissions for all topics
    #[serde(skip_serializing_if = "Option::is_none")]
    all_topics: Option<()>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Which topic(s) this permission applies to.
    topic_selector: Option<TopicSelector>,
}

impl TopicPermissions {
    /// Create topic permissions with no access by default.
    ///
    /// Defaults to no access for all caches and all topics. Use `with_role`,
    /// `with_cache`, and topic selectors to configure access.
    ///
    /// # Example
    ///
    /// ```
    /// let perms = TopicPermissions::new()
    ///     .with_role(TopicRole::TopicReadWrite)
    ///     .with_cache("my-cache")
    ///     .with_topic("notifications");
    /// ```
    pub fn new() -> Self {
        Self {
            role: TopicRole::TopicPermitNone,
            all_caches: None,
            cache_selector: None,
            all_topics: None,
            topic_selector: None,
        }
    }

    /// Create read-write topic permissions.
    ///
    /// Defaults to all caches and all topics.
    ///
    /// # Example
    ///
    /// ```
    /// let perms = TopicPermissions::read_write()
    ///     .with_topic("notifications");
    /// ```
    pub fn read_write() -> Self {
        Self {
            role: TopicRole::TopicReadWrite,
            all_caches: Some(()),
            cache_selector: None,
            all_topics: Some(()),
            topic_selector: None,
        }
    }

    /// Create read-only topic permissions.
    ///
    /// Defaults to all caches and all topics.
    ///
    /// # Example
    ///
    /// ```
    /// let perms = TopicPermissions::read_only()
    ///     .with_topic("notifications");
    /// ```
    pub fn read_only() -> Self {
        Self {
            role: TopicRole::TopicReadOnly,
            all_caches: Some(()),
            cache_selector: None,
            all_topics: Some(()),
            topic_selector: None,
        }
    }

    /// Create write-only topic permissions.
    ///
    /// Defaults to all caches and all topics.
    ///
    /// # Example
    ///
    /// ```
    /// let perms = TopicPermissions::write_only()
    ///     .with_topic("notifications");
    /// ```
    pub fn write_only() -> Self {
        Self {
            role: TopicRole::TopicWriteOnly,
            all_caches: None,
            cache_selector: None,
            all_topics: None,
            topic_selector: None,
        }
    }

    /// Set the access role, such as `TopicReadWrite`.
    pub fn with_role(mut self, role: TopicRole) -> Self {
        self.role = role;
        self
    }

    /// Grant access to topics in all caches.
    pub fn with_all_caches(mut self) -> Self {
        self.all_caches = Some(());
        self
    }

    /// Grant access to topics in a specific cache by cache name.
    pub fn with_cache(mut self, cache_name: impl Into<String>) -> Self {
        self.cache_selector = Some(CacheSelector::CacheName(cache_name.into()));
        self
    }

    /// Grant access to all topics.
    pub fn with_all_topics(mut self) -> Self {
        self.all_topics = Some(());
        self
    }

    /// Grant access to a specific topic by topic name.
    pub fn with_topic(mut self, topic_name: impl Into<String>) -> Self {
        self.topic_selector = Some(TopicSelector::TopicName(topic_name.into()));
        self
    }

    /// Grant access to all topics with a specific topic name prefix.
    pub fn with_topic_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.topic_selector = Some(TopicSelector::TopicNamePrefix(prefix.into()));
        self
    }
}

impl Default for TopicPermissions {
    fn default() -> Self {
        Self::new()
    }
}

/// Selector for targeting specific functions.
///
/// Used to limit permissions to particular functions by name or name prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionSelector {
    /// Select a function by its exact name.
    FunctionName(String),
    /// Select all functions with a specific name prefix.
    FunctionNamePrefix(String),
}

/// Permissions for a function(s).
///
/// Defines the role, the caches the function(s) belong to, and the function(s) that
/// are accessible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionPermissions {
    /// The level of access to your function(s).
    pub role: FunctionRole,
    /// Grants permissions for functions within all caches
    #[serde(skip_serializing_if = "Option::is_none")]
    all_caches: Option<()>,
    /// Which cache(s) this permission applies to.
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_selector: Option<CacheSelector>,
    /// Has permissions for all functions
    #[serde(skip_serializing_if = "Option::is_none")]
    all_functions: Option<()>,
    /// Which function(s) this permission applies to.
    #[serde(skip_serializing_if = "Option::is_none")]
    function_selector: Option<FunctionSelector>,
}

impl FunctionPermissions {
    /// Create function permissions with no access by default.
    ///
    /// Defaults to no access on all caches and all functions. Use `with_role`,
    /// `with_cache`, and function selectors to configure access.
    ///
    /// # Example
    ///
    /// ```
    /// let perms = FunctionPermissions::new()
    ///     .with_role(FunctionRole::FunctionInvoke)
    ///     .with_cache("my-cache")
    ///     .with_function("my-function");
    /// ```
    pub fn new() -> Self {
        Self {
            role: FunctionRole::FunctionPermitNone,
            all_caches: None,
            cache_selector: None,
            all_functions: None,
            function_selector: None,
        }
    }

    /// Create invoke function permissions.
    ///
    /// Defaults to all caches and all functions.
    ///
    /// # Example
    ///
    /// ```
    /// let perms = FunctionPermissions::invoke()
    ///     .with_function("my-function");
    /// ```
    pub fn invoke() -> Self {
        Self {
            role: FunctionRole::FunctionInvoke,
            all_caches: Some(()),
            cache_selector: None,
            all_functions: Some(()),
            function_selector: None,
        }
    }

    /// Set the access role.
    pub fn with_role(mut self, role: FunctionRole) -> Self {
        self.role = role;
        self
    }

    /// Grant access to functions in all caches.
    pub fn with_all_caches(mut self) -> Self {
        self.all_caches = Some(());
        self
    }

    /// Grant access to functions in a specific cache by name.
    pub fn with_cache(mut self, cache_name: impl Into<String>) -> Self {
        self.cache_selector = Some(CacheSelector::CacheName(cache_name.into()));
        self
    }

    /// Grant access to all functions.
    pub fn with_all_functions(mut self) -> Self {
        self.all_functions = Some(());
        self
    }

    /// Grant access to a specific function by name.
    pub fn with_function(mut self, function_name: impl Into<String>) -> Self {
        self.function_selector = Some(FunctionSelector::FunctionName(function_name.into()));
        self
    }

    /// Grant access to all functions with a specific name prefix.
    pub fn with_function_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.function_selector = Some(FunctionSelector::FunctionNamePrefix(prefix.into()));
        self
    }
}

impl Default for FunctionPermissions {
    fn default() -> Self {
        Self::new()
    }
}
