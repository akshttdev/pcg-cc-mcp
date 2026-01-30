//! Agent Tools Module
//!
//! This module provides specialized tool implementations for each agent in the PCG ecosystem.
//! Tools are organized by agent and category, with clear permission boundaries.

pub mod registry;
pub mod research;
pub mod brand;
pub mod content;
pub mod development;
pub mod deployment;
pub mod quality;
pub mod marketing;

// Re-exports
pub use registry::{AgentToolRegistry, ToolAccess, AgentToolSet};
pub use research::{ResearchTools, WebSearchProvider, CompetitorAnalyzer};
pub use brand::{BrandTools, ComfyUIClient, ColorSystem, TypographyTools};
pub use content::{ContentTools, CopyWriter, SEOAnalyzer};
pub use development::{DevelopmentTools, WebflowClient, VercelClient};
pub use deployment::{DeploymentTools, CloudflareClient, SslManager, DomainRegistrar};
pub use quality::{QualityTools, AccessibilityChecker, LinkChecker, BrandConsistencyChecker};
pub use marketing::{MarketingTools, GoogleAnalyticsClient, GoogleAdsClient, UtmBuilder};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub tool_name: String,
    pub agent_name: String,
    pub execution_time_ms: u64,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

/// Tool category for organization and permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCategory {
    // Research & Analysis
    WebSearch,
    CompetitorAnalysis,
    MarketResearch,
    DataSynthesis,

    // Creative & Brand
    ImageGeneration,
    LogoDesign,
    ColorPalette,
    Typography,
    BrandGuidelines,

    // Content
    Copywriting,
    SEOOptimization,
    ContentCalendar,
    EmailSequences,

    // Development
    SiteBuilding,
    TemplateCustomization,
    CMSSetup,
    PerformanceOptimization,

    // Deployment
    DomainManagement,
    SSLConfiguration,
    CDNSetup,
    Monitoring,

    // Quality
    TechnicalAudit,
    AccessibilityCheck,
    SecurityScan,
    BrandConsistency,

    // Marketing
    Analytics,
    PaidAds,
    SocialMedia,
    ABTesting,

    // Utility
    BrowserAutomation,
    FileManagement,
    Communication,
}

/// Agent identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentId {
    // Tier 0 - Internal
    Nora,

    // Tier 1 - Platform
    Topsi,

    // Tier 2 - Specialized
    Astra,
    Genesis,
    Scribe,
    Maci,
    Flux,
    Auri,
    Launch,
    Growth,
    Sentinel,
    Scout,
    Editron,
    Compass,

    // Tier 3 - Utility
    Bowser,
}

impl AgentId {
    /// Get the agent's access tier
    pub fn tier(&self) -> u8 {
        match self {
            AgentId::Nora => 0,
            AgentId::Topsi => 1,
            AgentId::Bowser => 3,
            _ => 2,
        }
    }

    /// Get the agent's team
    pub fn team(&self) -> &'static str {
        match self {
            AgentId::Nora => "executive",
            AgentId::Topsi => "platform",
            AgentId::Astra | AgentId::Scout => "research",
            AgentId::Genesis | AgentId::Maci | AgentId::Editron => "creative",
            AgentId::Scribe => "content",
            AgentId::Flux | AgentId::Auri | AgentId::Launch => "engineering",
            AgentId::Growth => "marketing",
            AgentId::Sentinel => "quality",
            AgentId::Compass => "operations",
            AgentId::Bowser => "utility",
        }
    }

    /// Get allowed tool categories for this agent
    pub fn allowed_categories(&self) -> Vec<ToolCategory> {
        match self {
            AgentId::Nora => vec![
                // Nora has access to all categories for orchestration
                ToolCategory::WebSearch,
                ToolCategory::CompetitorAnalysis,
                ToolCategory::MarketResearch,
                ToolCategory::DataSynthesis,
                ToolCategory::Communication,
            ],

            AgentId::Topsi => vec![
                ToolCategory::Communication,
                ToolCategory::FileManagement,
            ],

            AgentId::Astra => vec![
                ToolCategory::WebSearch,
                ToolCategory::CompetitorAnalysis,
                ToolCategory::MarketResearch,
                ToolCategory::DataSynthesis,
            ],

            AgentId::Scout => vec![
                ToolCategory::WebSearch,
                ToolCategory::CompetitorAnalysis,
                ToolCategory::SocialMedia,
                ToolCategory::BrowserAutomation,
            ],

            AgentId::Genesis => vec![
                ToolCategory::ImageGeneration,
                ToolCategory::LogoDesign,
                ToolCategory::ColorPalette,
                ToolCategory::Typography,
                ToolCategory::BrandGuidelines,
            ],

            AgentId::Maci => vec![
                ToolCategory::ImageGeneration,
                ToolCategory::ColorPalette,
            ],

            AgentId::Scribe => vec![
                ToolCategory::Copywriting,
                ToolCategory::SEOOptimization,
                ToolCategory::ContentCalendar,
                ToolCategory::EmailSequences,
            ],

            AgentId::Flux => vec![
                ToolCategory::SiteBuilding,
                ToolCategory::TemplateCustomization,
                ToolCategory::CMSSetup,
                ToolCategory::PerformanceOptimization,
            ],

            AgentId::Auri => vec![
                ToolCategory::SiteBuilding,
                ToolCategory::PerformanceOptimization,
                ToolCategory::FileManagement,
            ],

            AgentId::Launch => vec![
                ToolCategory::DomainManagement,
                ToolCategory::SSLConfiguration,
                ToolCategory::CDNSetup,
                ToolCategory::Monitoring,
            ],

            AgentId::Growth => vec![
                ToolCategory::Analytics,
                ToolCategory::PaidAds,
                ToolCategory::SocialMedia,
                ToolCategory::ABTesting,
                ToolCategory::SEOOptimization,
            ],

            AgentId::Sentinel => vec![
                ToolCategory::TechnicalAudit,
                ToolCategory::AccessibilityCheck,
                ToolCategory::SecurityScan,
                ToolCategory::BrandConsistency,
            ],

            AgentId::Editron => vec![
                ToolCategory::FileManagement,
            ],

            AgentId::Compass => vec![
                ToolCategory::Communication,
                ToolCategory::FileManagement,
            ],

            AgentId::Bowser => vec![
                ToolCategory::BrowserAutomation,
            ],
        }
    }
}

/// Configuration for external service credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCredentials {
    pub service_name: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub base_url: Option<String>,
    pub additional_config: HashMap<String, String>,
}

/// Tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub enabled: bool,
    pub rate_limit_per_minute: Option<u32>,
    pub timeout_seconds: Option<u32>,
    pub credentials: Option<ServiceCredentials>,
}
