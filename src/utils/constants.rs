/// Application constants

/// Maximum length for indexed string properties in Dash Platform
pub const MAX_INDEXED_STRING_LENGTH: u32 = 63;

/// Maximum items for indexed array properties in Dash Platform
pub const MAX_INDEXED_ARRAY_ITEMS: u32 = 255;

/// Default formats for string properties
pub const STRING_FORMATS: &[&str] = &[
    "uri",
    "email",
    "date",
    "date-time",
    "time",
    "hostname",
    "ipv4",
    "ipv6",
    "uuid",
];

/// Available sort orders (Dash Platform only supports ascending)
pub const SORT_ORDERS: &[&str] = &["asc"];

/// System properties that can be automatically added
pub const SYSTEM_PROPERTIES: &[&str] = &["$createdAt", "$updatedAt"];

/// OpenAI model to use
pub const OPENAI_MODEL: &str = "gpt-4o";

/// Maximum tokens for OpenAI responses
pub const OPENAI_MAX_TOKENS: u32 = 4096;

/// Temperature for OpenAI requests (lower = more deterministic)
pub const OPENAI_TEMPERATURE: f32 = 0.2;
