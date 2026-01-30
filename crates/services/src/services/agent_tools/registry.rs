//! Agent Tool Registry
//!
//! Central registry for managing agent tool access, permissions, and execution.

use super::{AgentId, ToolCategory, ToolConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool access permission
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolAccess {
    /// Full access to execute and configure
    Full,
    /// Can execute but not configure
    Execute,
    /// Read-only access to results
    ReadOnly,
    /// No access
    Denied,
}

/// A set of tools available to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolSet {
    pub agent_id: AgentId,
    pub tools: HashMap<String, ToolAccess>,
    pub category_access: HashMap<ToolCategory, ToolAccess>,
}

impl AgentToolSet {
    pub fn new(agent_id: AgentId) -> Self {
        let mut category_access = HashMap::new();

        // Set up default category access based on agent
        for category in agent_id.allowed_categories() {
            category_access.insert(category, ToolAccess::Full);
        }

        Self {
            agent_id,
            tools: HashMap::new(),
            category_access,
        }
    }

    pub fn can_execute(&self, tool_name: &str, category: ToolCategory) -> bool {
        // Check specific tool override first
        if let Some(access) = self.tools.get(tool_name) {
            return matches!(access, ToolAccess::Full | ToolAccess::Execute);
        }

        // Fall back to category access
        if let Some(access) = self.category_access.get(&category) {
            return matches!(access, ToolAccess::Full | ToolAccess::Execute);
        }

        false
    }
}

/// Central registry for all agent tools
pub struct AgentToolRegistry {
    /// Tool configurations by name
    tool_configs: Arc<RwLock<HashMap<String, ToolConfig>>>,

    /// Agent tool sets
    agent_tools: Arc<RwLock<HashMap<AgentId, AgentToolSet>>>,

    /// Tool definitions with schemas
    tool_definitions: Arc<RwLock<HashMap<String, ToolDefinition>>>,
}

/// Definition of a tool for LLM function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub agent_owner: AgentId,
    pub parameters: serde_json::Value, // JSON Schema
    pub returns: serde_json::Value,    // JSON Schema for return type
    pub examples: Vec<ToolExample>,
    pub estimated_duration_ms: Option<u64>,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub requests_per_hour: Option<u32>,
    pub requests_per_day: Option<u32>,
}

impl AgentToolRegistry {
    pub fn new() -> Self {
        let registry = Self {
            tool_configs: Arc::new(RwLock::new(HashMap::new())),
            agent_tools: Arc::new(RwLock::new(HashMap::new())),
            tool_definitions: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize in a blocking context if needed
        registry
    }

    /// Initialize default agent tool sets
    pub async fn initialize_defaults(&self) {
        let mut agent_tools = self.agent_tools.write().await;

        // Initialize tool sets for all agents
        for agent_id in [
            AgentId::Nora,
            AgentId::Topsi,
            AgentId::Astra,
            AgentId::Scout,
            AgentId::Genesis,
            AgentId::Maci,
            AgentId::Scribe,
            AgentId::Flux,
            AgentId::Auri,
            AgentId::Launch,
            AgentId::Growth,
            AgentId::Sentinel,
            AgentId::Editron,
            AgentId::Compass,
            AgentId::Bowser,
        ] {
            agent_tools.insert(agent_id, AgentToolSet::new(agent_id));
        }
    }

    /// Register a tool definition
    pub async fn register_tool(&self, definition: ToolDefinition) {
        let mut definitions = self.tool_definitions.write().await;
        definitions.insert(definition.name.clone(), definition);
    }

    /// Get tools available to an agent
    pub async fn get_agent_tools(&self, agent_id: AgentId) -> Vec<ToolDefinition> {
        let definitions = self.tool_definitions.read().await;
        let agent_tools = self.agent_tools.read().await;

        let tool_set = match agent_tools.get(&agent_id) {
            Some(ts) => ts,
            None => return vec![],
        };

        definitions
            .values()
            .filter(|def| tool_set.can_execute(&def.name, def.category))
            .cloned()
            .collect()
    }

    /// Get tool definitions in OpenAI function format
    pub async fn get_openai_tools(&self, agent_id: AgentId) -> Vec<serde_json::Value> {
        let tools = self.get_agent_tools(agent_id).await;

        tools
            .into_iter()
            .map(|def| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": def.name,
                        "description": def.description,
                        "parameters": def.parameters,
                    }
                })
            })
            .collect()
    }

    /// Get tool definitions in Anthropic format
    pub async fn get_anthropic_tools(&self, agent_id: AgentId) -> Vec<serde_json::Value> {
        let tools = self.get_agent_tools(agent_id).await;

        tools
            .into_iter()
            .map(|def| {
                serde_json::json!({
                    "name": def.name,
                    "description": def.description,
                    "input_schema": def.parameters,
                })
            })
            .collect()
    }

    /// Check if an agent can execute a tool
    pub async fn can_execute(&self, agent_id: AgentId, tool_name: &str) -> bool {
        let definitions = self.tool_definitions.read().await;
        let agent_tools = self.agent_tools.read().await;

        let tool_set = match agent_tools.get(&agent_id) {
            Some(ts) => ts,
            None => return false,
        };

        let definition = match definitions.get(tool_name) {
            Some(def) => def,
            None => return false,
        };

        tool_set.can_execute(tool_name, definition.category)
    }

    /// Set tool configuration
    pub async fn configure_tool(&self, tool_name: &str, config: ToolConfig) {
        let mut configs = self.tool_configs.write().await;
        configs.insert(tool_name.to_string(), config);
    }

    /// Get tool configuration
    pub async fn get_tool_config(&self, tool_name: &str) -> Option<ToolConfig> {
        let configs = self.tool_configs.read().await;
        configs.get(tool_name).cloned()
    }
}

impl Default for AgentToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tool Registration Macros
// ============================================================================

/// Macro to simplify tool registration
#[macro_export]
macro_rules! register_agent_tool {
    (
        name: $name:expr,
        description: $desc:expr,
        category: $category:expr,
        owner: $owner:expr,
        parameters: { $($param_name:expr => $param_type:expr),* $(,)? },
        required: [$($required:expr),* $(,)?]
    ) => {
        ToolDefinition {
            name: $name.to_string(),
            description: $desc.to_string(),
            category: $category,
            agent_owner: $owner,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    $($param_name: $param_type),*
                },
                "required": [$($required),*]
            }),
            returns: serde_json::json!({"type": "object"}),
            examples: vec![],
            estimated_duration_ms: None,
            rate_limit: None,
        }
    };
}

// ============================================================================
// Pre-built Tool Definitions
// ============================================================================

impl AgentToolRegistry {
    /// Register all research tools
    pub async fn register_research_tools(&self) {
        // Web Search
        self.register_tool(ToolDefinition {
            name: "web_search".to_string(),
            description: "Search the web for information. Returns relevant results with snippets.".to_string(),
            category: ToolCategory::WebSearch,
            agent_owner: AgentId::Astra,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    },
                    "num_results": {
                        "type": "integer",
                        "description": "Number of results to return (default: 10)",
                        "default": 10
                    },
                    "search_type": {
                        "type": "string",
                        "enum": ["general", "news", "images", "academic"],
                        "default": "general"
                    }
                },
                "required": ["query"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "results": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": {"type": "string"},
                                "url": {"type": "string"},
                                "snippet": {"type": "string"}
                            }
                        }
                    }
                }
            }),
            examples: vec![
                ToolExample {
                    description: "Search for market size data".to_string(),
                    input: serde_json::json!({"query": "SaaS market size 2024", "num_results": 5}),
                    output: serde_json::json!({"results": []}),
                }
            ],
            estimated_duration_ms: Some(2000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 30,
                requests_per_hour: Some(500),
                requests_per_day: Some(5000),
            }),
        }).await;

        // Web Fetch
        self.register_tool(ToolDefinition {
            name: "web_fetch".to_string(),
            description: "Fetch and extract content from a specific URL.".to_string(),
            category: ToolCategory::WebSearch,
            agent_owner: AgentId::Astra,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to fetch"
                    },
                    "extract_type": {
                        "type": "string",
                        "enum": ["text", "html", "markdown", "structured"],
                        "default": "markdown"
                    },
                    "selectors": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "CSS selectors to extract specific elements"
                    }
                },
                "required": ["url"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": {"type": "string"},
                    "title": {"type": "string"},
                    "metadata": {"type": "object"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(5000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 20,
                requests_per_hour: Some(200),
                requests_per_day: None,
            }),
        }).await;

        // Competitor Analysis
        self.register_tool(ToolDefinition {
            name: "analyze_competitor".to_string(),
            description: "Analyze a competitor's web presence, social media, and positioning.".to_string(),
            category: ToolCategory::CompetitorAnalysis,
            agent_owner: AgentId::Scout,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "company_name": {
                        "type": "string",
                        "description": "Company name to analyze"
                    },
                    "website_url": {
                        "type": "string",
                        "description": "Company website URL"
                    },
                    "analysis_depth": {
                        "type": "string",
                        "enum": ["quick", "standard", "deep"],
                        "default": "standard"
                    },
                    "include_social": {
                        "type": "boolean",
                        "default": true
                    }
                },
                "required": ["company_name"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "company_profile": {"type": "object"},
                    "website_analysis": {"type": "object"},
                    "social_presence": {"type": "object"},
                    "positioning": {"type": "object"},
                    "strengths": {"type": "array"},
                    "weaknesses": {"type": "array"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(30000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 5,
                requests_per_hour: Some(50),
                requests_per_day: None,
            }),
        }).await;

        // Domain Availability Check
        self.register_tool(ToolDefinition {
            name: "check_domain".to_string(),
            description: "Check domain name availability and suggest alternatives.".to_string(),
            category: ToolCategory::MarketResearch,
            agent_owner: AgentId::Astra,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "domain": {
                        "type": "string",
                        "description": "Domain name to check (without TLD)"
                    },
                    "tlds": {
                        "type": "array",
                        "items": {"type": "string"},
                        "default": [".com", ".io", ".co", ".ai"]
                    },
                    "suggest_alternatives": {
                        "type": "boolean",
                        "default": true
                    }
                },
                "required": ["domain"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "availability": {"type": "object"},
                    "alternatives": {"type": "array"},
                    "pricing": {"type": "object"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(3000),
            rate_limit: None,
        }).await;
    }

    /// Register all brand/creative tools
    pub async fn register_brand_tools(&self) {
        // Generate Image (ComfyUI)
        self.register_tool(ToolDefinition {
            name: "generate_image".to_string(),
            description: "Generate an image using AI (Flux/SDXL via ComfyUI).".to_string(),
            category: ToolCategory::ImageGeneration,
            agent_owner: AgentId::Maci,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "prompt": {
                        "type": "string",
                        "description": "Detailed description of the image to generate"
                    },
                    "negative_prompt": {
                        "type": "string",
                        "description": "What to avoid in the image"
                    },
                    "style": {
                        "type": "string",
                        "enum": ["cinematic", "commercial", "editorial", "abstract", "minimal", "bold"],
                        "default": "commercial"
                    },
                    "aspect_ratio": {
                        "type": "string",
                        "enum": ["1:1", "16:9", "9:16", "4:3", "3:4", "21:9"],
                        "default": "16:9"
                    },
                    "model": {
                        "type": "string",
                        "enum": ["flux-pro", "flux-dev", "sdxl"],
                        "default": "flux-pro"
                    },
                    "steps": {
                        "type": "integer",
                        "default": 30,
                        "minimum": 10,
                        "maximum": 50
                    },
                    "cfg_scale": {
                        "type": "number",
                        "default": 7.5,
                        "minimum": 1,
                        "maximum": 20
                    }
                },
                "required": ["prompt"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "image_url": {"type": "string"},
                    "image_path": {"type": "string"},
                    "seed": {"type": "integer"},
                    "parameters_used": {"type": "object"}
                }
            }),
            examples: vec![
                ToolExample {
                    description: "Generate a hero image for a tech startup".to_string(),
                    input: serde_json::json!({
                        "prompt": "Modern tech startup office, diverse team collaborating, natural lighting, clean aesthetic, professional photography",
                        "style": "commercial",
                        "aspect_ratio": "16:9"
                    }),
                    output: serde_json::json!({
                        "image_url": "https://...",
                        "seed": 12345
                    }),
                }
            ],
            estimated_duration_ms: Some(45000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 10,
                requests_per_hour: Some(100),
                requests_per_day: None,
            }),
        }).await;

        // Generate Logo Concepts
        self.register_tool(ToolDefinition {
            name: "generate_logo_concepts".to_string(),
            description: "Generate multiple logo concept variations for a brand.".to_string(),
            category: ToolCategory::LogoDesign,
            agent_owner: AgentId::Genesis,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "brand_name": {
                        "type": "string",
                        "description": "The brand/company name"
                    },
                    "industry": {
                        "type": "string",
                        "description": "Industry or sector"
                    },
                    "style_keywords": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Style keywords (e.g., modern, minimal, bold)"
                    },
                    "logo_types": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["wordmark", "lettermark", "symbol", "combination", "emblem"]
                        },
                        "default": ["wordmark", "symbol", "combination"]
                    },
                    "color_preferences": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Preferred colors or color families"
                    },
                    "num_concepts": {
                        "type": "integer",
                        "default": 5,
                        "minimum": 3,
                        "maximum": 10
                    }
                },
                "required": ["brand_name", "industry"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "concepts": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "concept_id": {"type": "string"},
                                "logo_type": {"type": "string"},
                                "image_url": {"type": "string"},
                                "rationale": {"type": "string"},
                                "color_palette": {"type": "array"}
                            }
                        }
                    }
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(120000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 3,
                requests_per_hour: Some(20),
                requests_per_day: None,
            }),
        }).await;

        // Generate Color Palette
        self.register_tool(ToolDefinition {
            name: "generate_color_palette".to_string(),
            description: "Generate a brand color palette with accessibility compliance.".to_string(),
            category: ToolCategory::ColorPalette,
            agent_owner: AgentId::Genesis,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "brand_personality": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Brand personality traits"
                    },
                    "industry": {
                        "type": "string"
                    },
                    "base_color": {
                        "type": "string",
                        "description": "Optional starting color (hex)"
                    },
                    "palette_type": {
                        "type": "string",
                        "enum": ["monochromatic", "complementary", "analogous", "triadic", "custom"],
                        "default": "custom"
                    },
                    "ensure_accessibility": {
                        "type": "boolean",
                        "default": true,
                        "description": "Ensure WCAG AA compliance"
                    }
                },
                "required": ["brand_personality", "industry"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "primary": {
                        "type": "object",
                        "properties": {
                            "hex": {"type": "string"},
                            "rgb": {"type": "object"},
                            "usage": {"type": "string"}
                        }
                    },
                    "secondary": {"type": "object"},
                    "accent": {"type": "object"},
                    "neutrals": {"type": "array"},
                    "accessibility_report": {"type": "object"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(5000),
            rate_limit: None,
        }).await;

        // Select Typography
        self.register_tool(ToolDefinition {
            name: "select_typography".to_string(),
            description: "Select and pair typefaces for a brand identity.".to_string(),
            category: ToolCategory::Typography,
            agent_owner: AgentId::Genesis,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "brand_personality": {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    "use_cases": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["web", "print", "mobile", "presentation"]
                        },
                        "default": ["web"]
                    },
                    "style_preference": {
                        "type": "string",
                        "enum": ["serif", "sans-serif", "mixed", "display"],
                        "default": "sans-serif"
                    },
                    "require_free_fonts": {
                        "type": "boolean",
                        "default": true
                    }
                },
                "required": ["brand_personality"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "heading_font": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "weights": {"type": "array"},
                            "source": {"type": "string"},
                            "css_import": {"type": "string"}
                        }
                    },
                    "body_font": {"type": "object"},
                    "accent_font": {"type": "object"},
                    "type_scale": {"type": "object"},
                    "pairing_rationale": {"type": "string"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(3000),
            rate_limit: None,
        }).await;
    }

    /// Register all content tools
    pub async fn register_content_tools(&self) {
        // Write Copy
        self.register_tool(ToolDefinition {
            name: "write_copy".to_string(),
            description: "Write marketing copy for various purposes.".to_string(),
            category: ToolCategory::Copywriting,
            agent_owner: AgentId::Scribe,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "copy_type": {
                        "type": "string",
                        "enum": [
                            "headline", "tagline", "homepage", "about_page",
                            "product_description", "email", "social_post",
                            "ad_copy", "cta", "meta_description"
                        ]
                    },
                    "brand_voice": {
                        "type": "object",
                        "description": "Brand voice guidelines"
                    },
                    "target_audience": {
                        "type": "string"
                    },
                    "key_messages": {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    "tone": {
                        "type": "string",
                        "enum": ["professional", "casual", "bold", "friendly", "authoritative"]
                    },
                    "length": {
                        "type": "string",
                        "enum": ["short", "medium", "long"]
                    },
                    "seo_keywords": {
                        "type": "array",
                        "items": {"type": "string"}
                    }
                },
                "required": ["copy_type", "target_audience"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "copy": {"type": "string"},
                    "variations": {"type": "array"},
                    "word_count": {"type": "integer"},
                    "readability_score": {"type": "number"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(10000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 20,
                requests_per_hour: None,
                requests_per_day: None,
            }),
        }).await;

        // Analyze SEO
        self.register_tool(ToolDefinition {
            name: "analyze_seo".to_string(),
            description: "Analyze content or URL for SEO optimization.".to_string(),
            category: ToolCategory::SEOOptimization,
            agent_owner: AgentId::Scribe,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "Content to analyze"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL to analyze (alternative to content)"
                    },
                    "target_keywords": {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    "analysis_type": {
                        "type": "string",
                        "enum": ["content", "technical", "full"],
                        "default": "full"
                    }
                },
                "required": []
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "score": {"type": "integer"},
                    "keyword_analysis": {"type": "object"},
                    "content_suggestions": {"type": "array"},
                    "technical_issues": {"type": "array"},
                    "meta_recommendations": {"type": "object"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(15000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 10,
                requests_per_hour: Some(100),
                requests_per_day: None,
            }),
        }).await;

        // Generate Content Calendar
        self.register_tool(ToolDefinition {
            name: "generate_content_calendar".to_string(),
            description: "Generate a social media content calendar.".to_string(),
            category: ToolCategory::ContentCalendar,
            agent_owner: AgentId::Scribe,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "brand_guide": {
                        "type": "object"
                    },
                    "platforms": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["instagram", "twitter", "linkedin", "facebook", "tiktok"]
                        }
                    },
                    "content_pillars": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Main content themes"
                    },
                    "duration_days": {
                        "type": "integer",
                        "default": 30
                    },
                    "posts_per_week": {
                        "type": "integer",
                        "default": 5
                    }
                },
                "required": ["platforms", "content_pillars"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "calendar": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "date": {"type": "string"},
                                "platform": {"type": "string"},
                                "content_pillar": {"type": "string"},
                                "post_type": {"type": "string"},
                                "copy": {"type": "string"},
                                "hashtags": {"type": "array"},
                                "visual_direction": {"type": "string"}
                            }
                        }
                    }
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(30000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 5,
                requests_per_hour: Some(30),
                requests_per_day: None,
            }),
        }).await;
    }

    /// Register all development tools
    pub async fn register_development_tools(&self) {
        // Build Site
        self.register_tool(ToolDefinition {
            name: "build_site".to_string(),
            description: "Build a website from template with brand customization.".to_string(),
            category: ToolCategory::SiteBuilding,
            agent_owner: AgentId::Flux,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "template_id": {
                        "type": "string",
                        "description": "Template to use"
                    },
                    "brand_assets": {
                        "type": "object",
                        "properties": {
                            "logo_url": {"type": "string"},
                            "colors": {"type": "object"},
                            "fonts": {"type": "object"}
                        }
                    },
                    "pages": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "slug": {"type": "string"},
                                "content": {"type": "object"}
                            }
                        }
                    },
                    "platform": {
                        "type": "string",
                        "enum": ["webflow", "framer", "nextjs", "astro"],
                        "default": "webflow"
                    }
                },
                "required": ["template_id", "brand_assets"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "site_id": {"type": "string"},
                    "staging_url": {"type": "string"},
                    "admin_url": {"type": "string"},
                    "build_log": {"type": "array"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(300000), // 5 minutes
            rate_limit: Some(RateLimit {
                requests_per_minute: 2,
                requests_per_hour: Some(10),
                requests_per_day: None,
            }),
        }).await;

        // Performance Audit
        self.register_tool(ToolDefinition {
            name: "audit_performance".to_string(),
            description: "Run performance audit on a website.".to_string(),
            category: ToolCategory::PerformanceOptimization,
            agent_owner: AgentId::Flux,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string"
                    },
                    "device": {
                        "type": "string",
                        "enum": ["mobile", "desktop", "both"],
                        "default": "both"
                    },
                    "include_recommendations": {
                        "type": "boolean",
                        "default": true
                    }
                },
                "required": ["url"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "scores": {
                        "type": "object",
                        "properties": {
                            "performance": {"type": "integer"},
                            "accessibility": {"type": "integer"},
                            "best_practices": {"type": "integer"},
                            "seo": {"type": "integer"}
                        }
                    },
                    "core_web_vitals": {"type": "object"},
                    "recommendations": {"type": "array"},
                    "full_report_url": {"type": "string"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(60000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 5,
                requests_per_hour: Some(50),
                requests_per_day: None,
            }),
        }).await;
    }

    /// Register all deployment tools
    pub async fn register_deployment_tools(&self) {
        // Configure Domain
        self.register_tool(ToolDefinition {
            name: "configure_domain".to_string(),
            description: "Configure domain DNS and SSL.".to_string(),
            category: ToolCategory::DomainManagement,
            agent_owner: AgentId::Launch,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "domain": {
                        "type": "string"
                    },
                    "target": {
                        "type": "string",
                        "description": "Target IP or CNAME"
                    },
                    "record_type": {
                        "type": "string",
                        "enum": ["A", "CNAME", "AAAA"],
                        "default": "CNAME"
                    },
                    "enable_ssl": {
                        "type": "boolean",
                        "default": true
                    },
                    "enable_cdn": {
                        "type": "boolean",
                        "default": true
                    }
                },
                "required": ["domain", "target"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "dns_configured": {"type": "boolean"},
                    "ssl_status": {"type": "string"},
                    "cdn_status": {"type": "string"},
                    "propagation_estimate": {"type": "string"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(30000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 10,
                requests_per_hour: None,
                requests_per_day: None,
            }),
        }).await;

        // Setup Monitoring
        self.register_tool(ToolDefinition {
            name: "setup_monitoring".to_string(),
            description: "Set up uptime and error monitoring.".to_string(),
            category: ToolCategory::Monitoring,
            agent_owner: AgentId::Launch,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string"
                    },
                    "check_interval_minutes": {
                        "type": "integer",
                        "default": 5
                    },
                    "alert_channels": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {"type": "string", "enum": ["email", "slack", "discord", "sms"]},
                                "target": {"type": "string"}
                            }
                        }
                    },
                    "error_tracking": {
                        "type": "boolean",
                        "default": true
                    }
                },
                "required": ["url"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "monitor_id": {"type": "string"},
                    "status_page_url": {"type": "string"},
                    "error_tracking_dsn": {"type": "string"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(15000),
            rate_limit: None,
        }).await;
    }

    /// Register all quality tools
    pub async fn register_quality_tools(&self) {
        // Full QA Audit
        self.register_tool(ToolDefinition {
            name: "full_qa_audit".to_string(),
            description: "Run comprehensive quality assurance audit.".to_string(),
            category: ToolCategory::TechnicalAudit,
            agent_owner: AgentId::Sentinel,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string"
                    },
                    "brand_guide": {
                        "type": "object",
                        "description": "Brand guidelines for consistency check"
                    },
                    "audit_types": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["performance", "accessibility", "security", "seo", "brand"]
                        },
                        "default": ["performance", "accessibility", "security", "seo"]
                    },
                    "wcag_level": {
                        "type": "string",
                        "enum": ["A", "AA", "AAA"],
                        "default": "AA"
                    }
                },
                "required": ["url"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "overall_score": {"type": "integer"},
                    "pass": {"type": "boolean"},
                    "audit_results": {"type": "object"},
                    "critical_issues": {"type": "array"},
                    "warnings": {"type": "array"},
                    "recommendations": {"type": "array"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(120000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 3,
                requests_per_hour: Some(20),
                requests_per_day: None,
            }),
        }).await;

        // Accessibility Check
        self.register_tool(ToolDefinition {
            name: "check_accessibility".to_string(),
            description: "Check website accessibility compliance.".to_string(),
            category: ToolCategory::AccessibilityCheck,
            agent_owner: AgentId::Sentinel,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string"
                    },
                    "wcag_level": {
                        "type": "string",
                        "enum": ["A", "AA", "AAA"],
                        "default": "AA"
                    },
                    "include_best_practices": {
                        "type": "boolean",
                        "default": true
                    }
                },
                "required": ["url"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "compliant": {"type": "boolean"},
                    "violations": {"type": "array"},
                    "passes": {"type": "array"},
                    "incomplete": {"type": "array"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(30000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 10,
                requests_per_hour: None,
                requests_per_day: None,
            }),
        }).await;

        // Security Scan
        self.register_tool(ToolDefinition {
            name: "security_scan".to_string(),
            description: "Scan website for security issues.".to_string(),
            category: ToolCategory::SecurityScan,
            agent_owner: AgentId::Sentinel,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string"
                    },
                    "scan_depth": {
                        "type": "string",
                        "enum": ["quick", "standard", "deep"],
                        "default": "standard"
                    },
                    "check_ssl": {
                        "type": "boolean",
                        "default": true
                    },
                    "check_headers": {
                        "type": "boolean",
                        "default": true
                    }
                },
                "required": ["url"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "secure": {"type": "boolean"},
                    "ssl_grade": {"type": "string"},
                    "security_headers": {"type": "object"},
                    "vulnerabilities": {"type": "array"},
                    "recommendations": {"type": "array"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(45000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 5,
                requests_per_hour: Some(30),
                requests_per_day: None,
            }),
        }).await;
    }

    /// Register all marketing tools
    pub async fn register_marketing_tools(&self) {
        // Setup Analytics
        self.register_tool(ToolDefinition {
            name: "setup_analytics".to_string(),
            description: "Configure analytics tracking for a website.".to_string(),
            category: ToolCategory::Analytics,
            agent_owner: AgentId::Growth,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "site_url": {
                        "type": "string"
                    },
                    "provider": {
                        "type": "string",
                        "enum": ["google_analytics", "plausible", "posthog", "mixpanel"],
                        "default": "google_analytics"
                    },
                    "goals": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "type": {"type": "string"},
                                "target": {"type": "string"}
                            }
                        }
                    },
                    "ecommerce": {
                        "type": "boolean",
                        "default": false
                    }
                },
                "required": ["site_url"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "tracking_id": {"type": "string"},
                    "tracking_code": {"type": "string"},
                    "dashboard_url": {"type": "string"},
                    "goals_configured": {"type": "array"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(20000),
            rate_limit: None,
        }).await;

        // Keyword Research
        self.register_tool(ToolDefinition {
            name: "keyword_research".to_string(),
            description: "Research keywords for SEO and content strategy.".to_string(),
            category: ToolCategory::SEOOptimization,
            agent_owner: AgentId::Growth,
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "seed_keywords": {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    "industry": {
                        "type": "string"
                    },
                    "location": {
                        "type": "string",
                        "default": "US"
                    },
                    "include_questions": {
                        "type": "boolean",
                        "default": true
                    }
                },
                "required": ["seed_keywords"]
            }),
            returns: serde_json::json!({
                "type": "object",
                "properties": {
                    "keywords": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "keyword": {"type": "string"},
                                "volume": {"type": "integer"},
                                "difficulty": {"type": "integer"},
                                "cpc": {"type": "number"},
                                "trend": {"type": "string"}
                            }
                        }
                    },
                    "questions": {"type": "array"},
                    "related_topics": {"type": "array"}
                }
            }),
            examples: vec![],
            estimated_duration_ms: Some(15000),
            rate_limit: Some(RateLimit {
                requests_per_minute: 10,
                requests_per_hour: Some(100),
                requests_per_day: None,
            }),
        }).await;
    }

    /// Register all tools for the ecosystem
    pub async fn register_all_tools(&self) {
        self.initialize_defaults().await;
        self.register_research_tools().await;
        self.register_brand_tools().await;
        self.register_content_tools().await;
        self.register_development_tools().await;
        self.register_deployment_tools().await;
        self.register_quality_tools().await;
        self.register_marketing_tools().await;
    }
}
