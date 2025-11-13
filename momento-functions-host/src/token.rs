//! Host interface for working with Momento Token apis
use momento_functions_wit::host::momento::functions::permission_messages;
use momento_functions_wit::host::momento::functions::token;
use momento_functions_wit::host::momento::functions::token::Expires;

/// An error occurred when generating a disposable token
#[derive(thiserror::Error, Debug)]
pub enum GenerateDisposableTokenError {
    /// An error occurred when calling the host generate function
    #[error(transparent)]
    TokenError(#[from] token::TokenError),
}

/// Generate a disposable token with specific permissions.
pub fn generate_disposable_token(
    valid_for_seconds: u32,
    permissions: PermissionsBuilder,
    token_id: Option<String>,
) -> Result<FunctionHostGenerateDisposableTokenResponse, GenerateDisposableTokenError> {
    let permissions: momento_functions_wit::host::momento::functions::permission_messages::Permissions = permissions.into();
    token::generate_disposable_token(
        Expires { valid_for_seconds },
        &permissions,
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

/// Builds a single instance of restricted Cache permissions
#[derive(Clone, Debug)]
pub struct CachePermissionsBuilder {
    role: permission_messages::CacheRole,
    cache: permission_messages::CachePermissionsCache,
    cache_item: permission_messages::CachePermissionsItem,
}

impl Default for CachePermissionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CachePermissionsBuilder {
    /// Instantiates the builder with default values that will not allow
    /// any access to any caches nor the items within them.
    pub fn new() -> Self {
        Self {
            role: permission_messages::CacheRole::CachePermitNone,
            cache: permission_messages::CachePermissionsCache::AllCaches,
            cache_item: permission_messages::CachePermissionsItem::AllItems,
        }
    }

    /// Explicitly permits no caches in the role
    pub fn none(mut self) -> Self {
        self.role = permission_messages::CacheRole::CachePermitNone;
        self
    }
    /// Explicitly permits read/write access for caches in the role
    pub fn read_write(mut self) -> Self {
        self.role = permission_messages::CacheRole::CacheReadWrite;
        self
    }
    /// Explicitly permits read-only access for caches in the role
    pub fn read_only(mut self) -> Self {
        self.role = permission_messages::CacheRole::CacheReadOnly;
        self
    }
    /// Explicitly permits write-only access for caches in the role
    pub fn write_only(mut self) -> Self {
        self.role = permission_messages::CacheRole::CacheWriteOnly;
        self
    }

    /// Explicitly permits all caches can be accessed
    pub fn all_caches(mut self) -> Self {
        self.cache = permission_messages::CachePermissionsCache::AllCaches;
        self
    }
    /// Explicitly permits only a specific Cache
    pub fn cache_name(mut self, name: impl Into<String>) -> Self {
        self.cache = permission_messages::CachePermissionsCache::CacheSelector(
            permission_messages::CacheSelector::CacheName(name.into()),
        );
        self
    }

    /// Explicitly permits all items within the specified cache(s)
    pub fn all_items(mut self) -> Self {
        self.cache_item = permission_messages::CachePermissionsItem::AllItems;
        self
    }
    /// Explicitly permits access to this specific key within the cache
    pub fn item_key(mut self, key: Vec<u8>) -> Self {
        self.cache_item = permission_messages::CachePermissionsItem::ItemSelector(
            permission_messages::CacheItemSelector::Key(key),
        );
        self
    }
    /// Explicitly permits access to keys that begin with this prefix
    pub fn item_key_prefix(mut self, prefix: Vec<u8>) -> Self {
        self.cache_item = permission_messages::CachePermissionsItem::ItemSelector(
            permission_messages::CacheItemSelector::KeyPrefix(prefix),
        );
        self
    }

    /// Finalize into the generated WIT type
    pub fn build(self) -> permission_messages::CachePermissions {
        permission_messages::CachePermissions {
            role: self.role,
            cache: self.cache,
            cache_item: self.cache_item,
        }
    }
}

/// Builds a single instance of restricted Topic permissions
#[derive(Clone, Debug)]
pub struct TopicPermissionsBuilder {
    role: permission_messages::TopicRole,
    cache: permission_messages::TopicPermissionsCache,
    topic: permission_messages::TopicPermissionsTopic,
}

impl Default for TopicPermissionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TopicPermissionsBuilder {
    /// Instantiates the builder with default values that will not allow
    /// any access to any topics belonging to any caches
    pub fn new() -> Self {
        Self {
            role: permission_messages::TopicRole::TopicPermitNone,
            cache: permission_messages::TopicPermissionsCache::AllCaches,
            topic: permission_messages::TopicPermissionsTopic::AllTopics,
        }
    }

    /// Explicitly permits no topics in the role
    pub fn none(mut self) -> Self {
        self.role = permission_messages::TopicRole::TopicPermitNone;
        self
    }
    /// Explicitly permits publish/subscribe access to topics in the role
    pub fn read_write(mut self) -> Self {
        self.role = permission_messages::TopicRole::TopicReadWrite;
        self
    }
    /// Explicitly permits subscribe access to topics in the role
    pub fn read_only(mut self) -> Self {
        self.role = permission_messages::TopicRole::TopicReadOnly;
        self
    }
    /// Explicitly permits publish access to topics in the role
    pub fn write_only(mut self) -> Self {
        self.role = permission_messages::TopicRole::TopicWriteOnly;
        self
    }

    /// Explicitly permits topics access to all caches
    pub fn all_caches(mut self) -> Self {
        self.cache = permission_messages::TopicPermissionsCache::AllCaches;
        self
    }
    /// Explicitly permits topics access to a specific cache
    pub fn cache_name(mut self, name: impl Into<String>) -> Self {
        self.cache = permission_messages::TopicPermissionsCache::CacheSelector(
            permission_messages::CacheSelector::CacheName(name.into()),
        );
        self
    }

    /// Explicitly permits topics access to all topics
    pub fn all_topics(mut self) -> Self {
        self.topic = permission_messages::TopicPermissionsTopic::AllTopics;
        self
    }
    /// Explicitly permits topics access to a specific topic
    pub fn topic_name(mut self, name: impl Into<String>) -> Self {
        self.topic = permission_messages::TopicPermissionsTopic::TopicSelector(
            permission_messages::TopicSelector::TopicName(name.into()),
        );
        self
    }
    /// Explicitly permits topics access to topics beginning with this prefix
    pub fn topic_name_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.topic = permission_messages::TopicPermissionsTopic::TopicSelector(
            permission_messages::TopicSelector::TopicNamePrefix(prefix.into()),
        );
        self
    }

    /// Finalized into the generated WIT type
    pub fn build(self) -> permission_messages::TopicPermissions {
        permission_messages::TopicPermissions {
            role: self.role,
            cache: self.cache,
            topic: self.topic,
        }
    }
}

/// Builds a single instance of restricted Function permissions
#[derive(Clone, Debug)]
pub struct FunctionPermissionsBuilder {
    role: permission_messages::FunctionRole,
    cache: permission_messages::FunctionPermissionsCache,
    function: permission_messages::FunctionPermissionsFunction,
}

impl Default for FunctionPermissionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionPermissionsBuilder {
    /// Instantiates the builder with default values that will not allow
    /// any access to any functions belonging to any caches
    pub fn new() -> Self {
        Self {
            role: permission_messages::FunctionRole::FunctionPermitNone,
            cache: permission_messages::FunctionPermissionsCache::AllCaches,
            function: permission_messages::FunctionPermissionsFunction::AllFunctions,
        }
    }

    /// Allow no access to any functions
    pub fn none(mut self) -> Self {
        self.role = permission_messages::FunctionRole::FunctionPermitNone;
        self
    }
    /// Allow access to invoke functions
    pub fn invoke(mut self) -> Self {
        self.role = permission_messages::FunctionRole::FunctionInvoke;
        self
    }

    /// Restricts access to functions within all caches
    pub fn all_caches(mut self) -> Self {
        self.cache = permission_messages::FunctionPermissionsCache::AllCaches;
        self
    }
    /// Restricts access to functions within a specifi cache
    pub fn cache_name(mut self, name: impl Into<String>) -> Self {
        self.cache = permission_messages::FunctionPermissionsCache::CacheSelector(
            permission_messages::CacheSelector::CacheName(name.into()),
        );
        self
    }

    /// Restricts access to all functions
    pub fn all_functions(mut self) -> Self {
        self.function = permission_messages::FunctionPermissionsFunction::AllFunctions;
        self
    }
    /// Restricts access to a specific function
    pub fn function_name(mut self, name: impl Into<String>) -> Self {
        self.function = permission_messages::FunctionPermissionsFunction::FunctionSelector(
            permission_messages::FunctionSelector::FunctionName(name.into()),
        );
        self
    }
    /// Restricts access to functions beginning with this prefix
    pub fn function_name_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.function = permission_messages::FunctionPermissionsFunction::FunctionSelector(
            permission_messages::FunctionSelector::FunctionNamePrefix(prefix.into()),
        );
        self
    }

    /// Finalized into the generated WIT type
    pub fn build(self) -> permission_messages::FunctionPermissions {
        permission_messages::FunctionPermissions {
            role: self.role,
            cache: self.cache,
            function: self.function,
        }
    }
}

/// Top-level builder for building the desired permissions of your scoped token
#[derive(Clone, Debug)]
pub struct PermissionsBuilder {
    superuser: bool,
    explicit_permissions: Vec<permission_messages::PermissionsType>,
}

impl Default for PermissionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionsBuilder {
    /// Creates a default Permissions builder that isn't a superuser and has no permissions
    pub fn new() -> Self {
        Self {
            superuser: false,
            explicit_permissions: Vec::new(),
        }
    }

    /// Grant superuser permissions (ignores explicit permissions).
    /// ***Only a superuser can grant superuser permissions***.
    pub fn super_user(mut self) -> Self {
        self.superuser = true;
        self
    }

    /// Add an explicit cache permission
    pub fn add_cache(mut self, cache_perms: CachePermissionsBuilder) -> Self {
        self.explicit_permissions
            .push(permission_messages::PermissionsType::CachePermissions(
                cache_perms.build(),
            ));
        self
    }

    /// Add an explicit topic permission
    pub fn add_topic(mut self, topic_perms: TopicPermissionsBuilder) -> Self {
        self.explicit_permissions
            .push(permission_messages::PermissionsType::TopicPermissions(
                topic_perms.build(),
            ));
        self
    }

    /// Add an explicit function permission
    pub fn add_function(mut self, func_perms: FunctionPermissionsBuilder) -> Self {
        self.explicit_permissions
            .push(permission_messages::PermissionsType::FunctionPermissions(
                func_perms.build(),
            ));
        self
    }

    /// Finalize into the generated WIT Permissions
    pub fn build(self) -> permission_messages::Permissions {
        if self.superuser {
            permission_messages::Permissions::SuperUser(
                permission_messages::SuperUserPermissions::SuperUser,
            )
        } else {
            permission_messages::Permissions::Explicit(permission_messages::ExplicitPermissions {
                permissions: self.explicit_permissions,
            })
        }
    }
}

impl From<PermissionsBuilder> for permission_messages::Permissions {
    fn from(pb: PermissionsBuilder) -> Self {
        pb.build()
    }
}
