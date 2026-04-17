use momento_functions_bytes::Data;

/// A complete Valkey command, ready to send.
///
/// Use the convenience constructors for common commands, or [`Command::builder`]
/// for anything custom.
///
/// # Examples
/// ________
/// Common commands:
/// ```rust,no_run
/// use momento_functions_valkey::Command;
///
/// let get = Command::get("my_key");
/// let set = Command::set("my_key", "my_value");
/// let del = Command::del("my_key");
/// ```
/// ________
/// Custom command via builder:
/// ```rust,no_run
/// use momento_functions_valkey::Command;
///
/// let mut builder = Command::builder("ZADD");
/// builder.argument("my_sorted_set").argument("1.0").argument("member");
/// let cmd = Command::from(builder);
/// ```
pub struct Command {
    pub(crate) command: String,
    pub(crate) arguments: Vec<Data>,
}

impl Command {
    /// Create a builder for a fully custom command.
    pub fn builder(command_name: impl Into<String>) -> CommandBuilder {
        CommandBuilder {
            command: command_name.into(),
            arguments: Vec::new(),
        }
    }

    /// Create a `GET` command.
    pub fn get(key: impl Into<Data>) -> Self {
        Self {
            command: "GET".to_string(),
            arguments: vec![key.into()],
        }
    }

    /// Create a `SET` command.
    pub fn set(key: impl Into<Data>, value: impl Into<Data>) -> Self {
        Self {
            command: "SET".to_string(),
            arguments: vec![key.into(), value.into()],
        }
    }

    /// Create a `DEL` command.
    pub fn del(key: impl Into<Data>) -> Self {
        Self {
            command: "DEL".to_string(),
            arguments: vec![key.into()],
        }
    }
}

/// Allow bare strings for no-argument commands like `"PING"` or `"FLUSHDB"`.
impl From<String> for Command {
    fn from(s: String) -> Self {
        Self {
            command: s,
            arguments: Vec::new(),
        }
    }
}

impl From<&str> for Command {
    fn from(s: &str) -> Self {
        Self {
            command: s.to_string(),
            arguments: Vec::new(),
        }
    }
}

impl From<CommandBuilder> for Command {
    fn from(b: CommandBuilder) -> Self {
        Self {
            command: b.command,
            arguments: b.arguments,
        }
    }
}

/// A builder for fully custom Valkey commands.
///
/// Obtain one via [`Command::builder`].
pub struct CommandBuilder {
    command: String,
    arguments: Vec<Data>,
}

impl CommandBuilder {
    /// Append an argument to the command.
    ///
    /// Your contract for argument types and ordering is between you and Valkey —
    /// nothing here is validated.
    pub fn argument(&mut self, argument: impl Into<Data>) -> &mut Self {
        self.arguments.push(argument.into());
        self
    }
}
