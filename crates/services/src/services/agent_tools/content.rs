//! Content Tools Implementation
//!
//! Tools for copywriting, SEO analysis, and content calendar generation.
//! Used primarily by Scribe agent.

use crate::services::agent_tools::ToolResult;
use serde::{Deserialize, Serialize};
use std::time::Instant;

// ============================================================================
// Copy Writer
// ============================================================================

/// Copy generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyRequest {
    pub copy_type: CopyType,
    pub target_audience: String,
    pub key_messages: Vec<String>,
    pub tone: Option<String>,
    pub brand_voice: Option<BrandVoice>,
    pub seo_keywords: Option<Vec<String>>,
    pub length: Option<CopyLength>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CopyType {
    Headline,
    Tagline,
    Homepage,
    AboutPage,
    ProductDescription,
    Email,
    SocialPost,
    AdCopy,
    CTA,
    MetaDescription,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandVoice {
    pub personality_traits: Vec<String>,
    pub tone_keywords: Vec<String>,
    pub do_use: Vec<String>,
    pub dont_use: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CopyLength {
    Short,  // < 50 words
    Medium, // 50-150 words
    Long,   // > 150 words
}

/// Generated copy result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedCopy {
    pub copy: String,
    pub variations: Vec<String>,
    pub word_count: u32,
    pub readability_score: f32,
    pub seo_score: Option<f32>,
}

/// Copywriter service
pub struct CopyWriter;

impl CopyWriter {
    /// Generate copy (placeholder - would integrate with LLM)
    pub fn generate_copy_prompt(request: &CopyRequest) -> String {
        let mut prompt = format!(
            "Write {} copy for {} audience.\n\n",
            Self::copy_type_name(&request.copy_type),
            request.target_audience
        );

        prompt.push_str("Key messages to convey:\n");
        for msg in &request.key_messages {
            prompt.push_str(&format!("- {}\n", msg));
        }

        if let Some(tone) = &request.tone {
            prompt.push_str(&format!("\nTone: {}\n", tone));
        }

        if let Some(voice) = &request.brand_voice {
            prompt.push_str("\nBrand voice guidelines:\n");
            prompt.push_str(&format!("Personality: {}\n", voice.personality_traits.join(", ")));
            if !voice.do_use.is_empty() {
                prompt.push_str(&format!("Do use: {}\n", voice.do_use.join(", ")));
            }
            if !voice.dont_use.is_empty() {
                prompt.push_str(&format!("Don't use: {}\n", voice.dont_use.join(", ")));
            }
        }

        if let Some(keywords) = &request.seo_keywords {
            prompt.push_str(&format!("\nInclude these keywords naturally: {}\n", keywords.join(", ")));
        }

        if let Some(length) = &request.length {
            prompt.push_str(&format!("\nLength: {}\n", Self::length_name(length)));
        }

        prompt
    }

    fn copy_type_name(copy_type: &CopyType) -> &'static str {
        match copy_type {
            CopyType::Headline => "headline",
            CopyType::Tagline => "tagline",
            CopyType::Homepage => "homepage",
            CopyType::AboutPage => "about page",
            CopyType::ProductDescription => "product description",
            CopyType::Email => "email",
            CopyType::SocialPost => "social media post",
            CopyType::AdCopy => "advertising",
            CopyType::CTA => "call-to-action",
            CopyType::MetaDescription => "meta description",
        }
    }

    fn length_name(length: &CopyLength) -> &'static str {
        match length {
            CopyLength::Short => "short (under 50 words)",
            CopyLength::Medium => "medium (50-150 words)",
            CopyLength::Long => "long (150+ words)",
        }
    }

    /// Calculate readability score (Flesch-Kincaid)
    pub fn calculate_readability(text: &str) -> f32 {
        let words: Vec<&str> = text.split_whitespace().collect();
        let sentences: Vec<&str> = text.split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .collect();

        let word_count = words.len() as f32;
        let sentence_count = sentences.len().max(1) as f32;

        // Simple syllable estimation
        let syllable_count: f32 = words.iter()
            .map(|w| Self::estimate_syllables(w) as f32)
            .sum();

        // Flesch Reading Ease formula
        206.835 - 1.015 * (word_count / sentence_count) - 84.6 * (syllable_count / word_count)
    }

    fn estimate_syllables(word: &str) -> u32 {
        let word = word.to_lowercase();
        let vowels = ['a', 'e', 'i', 'o', 'u'];

        let mut count = 0;
        let mut prev_was_vowel = false;

        for c in word.chars() {
            let is_vowel = vowels.contains(&c);
            if is_vowel && !prev_was_vowel {
                count += 1;
            }
            prev_was_vowel = is_vowel;
        }

        // Silent 'e' at end
        if word.ends_with('e') && count > 1 {
            count -= 1;
        }

        count.max(1)
    }
}

// ============================================================================
// SEO Analyzer
// ============================================================================

/// SEO analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SEOAnalysis {
    pub score: u32,
    pub keyword_analysis: KeywordAnalysis,
    pub content_suggestions: Vec<String>,
    pub technical_issues: Vec<String>,
    pub meta_recommendations: MetaRecommendations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordAnalysis {
    pub target_keywords: Vec<String>,
    pub keyword_density: std::collections::HashMap<String, f32>,
    pub keyword_in_title: bool,
    pub keyword_in_h1: bool,
    pub keyword_in_first_paragraph: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaRecommendations {
    pub title_suggestion: Option<String>,
    pub description_suggestion: Option<String>,
    pub title_length_ok: bool,
    pub description_length_ok: bool,
}

/// SEO analyzer service
pub struct SEOAnalyzer;

impl SEOAnalyzer {
    /// Analyze content for SEO
    pub fn analyze(content: &str, target_keywords: &[String]) -> SEOAnalysis {
        let mut score = 50; // Base score
        let mut suggestions = vec![];
        let mut issues = vec![];

        // Calculate keyword density
        let word_count = content.split_whitespace().count();
        let content_lower = content.to_lowercase();

        let mut keyword_density = std::collections::HashMap::new();
        let mut keyword_in_first_para = false;

        for keyword in target_keywords {
            let keyword_lower = keyword.to_lowercase();
            let count = content_lower.matches(&keyword_lower).count();
            let density = (count as f32 / word_count as f32) * 100.0;
            keyword_density.insert(keyword.clone(), density);

            // Check first paragraph
            let first_para = content.split('\n').next().unwrap_or("");
            if first_para.to_lowercase().contains(&keyword_lower) {
                keyword_in_first_para = true;
                score += 5;
            }

            // Check density
            if density < 0.5 {
                suggestions.push(format!("Increase usage of keyword '{}'", keyword));
            } else if density > 3.0 {
                issues.push(format!("Keyword '{}' may be overused ({}%)", keyword, density));
                score -= 5;
            } else {
                score += 10;
            }
        }

        // Content length check
        if word_count < 300 {
            suggestions.push("Consider adding more content (aim for 300+ words)".to_string());
            score -= 10;
        } else if word_count > 500 {
            score += 10;
        }

        // Heading check (simplified)
        if !content.contains('#') && !content.contains("<h1") {
            suggestions.push("Add headings to structure your content".to_string());
        } else {
            score += 5;
        }

        SEOAnalysis {
            score: score.clamp(0, 100) as u32,
            keyword_analysis: KeywordAnalysis {
                target_keywords: target_keywords.to_vec(),
                keyword_density,
                keyword_in_title: false, // Would need title
                keyword_in_h1: false,    // Would need HTML parsing
                keyword_in_first_paragraph: keyword_in_first_para,
            },
            content_suggestions: suggestions,
            technical_issues: issues,
            meta_recommendations: MetaRecommendations {
                title_suggestion: None,
                description_suggestion: None,
                title_length_ok: true,
                description_length_ok: true,
            },
        }
    }
}

// ============================================================================
// Content Calendar Generator
// ============================================================================

/// Content calendar entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentCalendarEntry {
    pub date: String,
    pub platform: String,
    pub content_pillar: String,
    pub post_type: String,
    pub copy: String,
    pub hashtags: Vec<String>,
    pub visual_direction: String,
    pub status: String,
}

/// Content calendar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentCalendar {
    pub entries: Vec<ContentCalendarEntry>,
    pub start_date: String,
    pub end_date: String,
    pub platforms: Vec<String>,
    pub content_pillars: Vec<String>,
}

/// Content calendar generator
pub struct ContentCalendarGenerator;

impl ContentCalendarGenerator {
    /// Generate content calendar structure
    pub fn generate_calendar_structure(
        platforms: &[String],
        content_pillars: &[String],
        duration_days: u32,
        posts_per_week: u32,
    ) -> ContentCalendar {
        let mut entries = vec![];

        let posts_total = (duration_days / 7) * posts_per_week;

        for i in 0..posts_total {
            let day_offset = (i * 7 / posts_per_week) as i64;
            let platform = &platforms[i as usize % platforms.len()];
            let pillar = &content_pillars[i as usize % content_pillars.len()];

            entries.push(ContentCalendarEntry {
                date: format!("Day {}", day_offset + 1),
                platform: platform.clone(),
                content_pillar: pillar.clone(),
                post_type: Self::suggest_post_type(platform, pillar),
                copy: String::new(), // To be filled by LLM
                hashtags: vec![],
                visual_direction: Self::suggest_visual_direction(pillar),
                status: "draft".to_string(),
            });
        }

        ContentCalendar {
            entries,
            start_date: "TBD".to_string(),
            end_date: "TBD".to_string(),
            platforms: platforms.to_vec(),
            content_pillars: content_pillars.to_vec(),
        }
    }

    fn suggest_post_type(platform: &str, _pillar: &str) -> String {
        match platform.to_lowercase().as_str() {
            "instagram" => "carousel".to_string(),
            "twitter" | "x" => "thread".to_string(),
            "linkedin" => "article".to_string(),
            "tiktok" => "video".to_string(),
            "facebook" => "post".to_string(),
            _ => "post".to_string(),
        }
    }

    fn suggest_visual_direction(pillar: &str) -> String {
        format!("Visual content aligned with '{}' theme", pillar)
    }
}

// ============================================================================
// Content Tools (High-level interface)
// ============================================================================

/// High-level content tools interface
pub struct ContentTools {
    pub copy_writer: CopyWriter,
    pub seo_analyzer: SEOAnalyzer,
    pub calendar_generator: ContentCalendarGenerator,
}

impl ContentTools {
    pub fn new() -> Self {
        Self {
            copy_writer: CopyWriter,
            seo_analyzer: SEOAnalyzer,
            calendar_generator: ContentCalendarGenerator,
        }
    }

    /// Execute a content tool by name
    pub async fn execute(&self, tool_name: &str, params: serde_json::Value) -> ToolResult {
        let start = Instant::now();

        let result = match tool_name {
            "write_copy" => self.execute_write_copy(params).await,
            "analyze_seo" => self.execute_analyze_seo(params).await,
            "generate_content_calendar" => self.execute_generate_calendar(params).await,
            _ => Err(format!("Unknown tool: {}", tool_name)),
        };

        match result {
            Ok(data) => ToolResult {
                success: true,
                tool_name: tool_name.to_string(),
                agent_name: "content".to_string(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                data,
                error: None,
            },
            Err(error) => ToolResult {
                success: false,
                tool_name: tool_name.to_string(),
                agent_name: "content".to_string(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                data: serde_json::json!({}),
                error: Some(error),
            },
        }
    }

    async fn execute_write_copy(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        // Generate the prompt for LLM-based copy generation
        let copy_type_str = params.get("copy_type")
            .and_then(|v| v.as_str())
            .unwrap_or("headline");

        let copy_type = match copy_type_str {
            "headline" => CopyType::Headline,
            "tagline" => CopyType::Tagline,
            "homepage" => CopyType::Homepage,
            "about_page" => CopyType::AboutPage,
            "product_description" => CopyType::ProductDescription,
            "email" => CopyType::Email,
            "social_post" => CopyType::SocialPost,
            "ad_copy" => CopyType::AdCopy,
            "cta" => CopyType::CTA,
            "meta_description" => CopyType::MetaDescription,
            _ => CopyType::Headline,
        };

        let target_audience = params.get("target_audience")
            .and_then(|v| v.as_str())
            .unwrap_or("general audience")
            .to_string();

        let key_messages: Vec<String> = params.get("key_messages")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let request = CopyRequest {
            copy_type,
            target_audience,
            key_messages,
            tone: params.get("tone").and_then(|v| v.as_str()).map(|s| s.to_string()),
            brand_voice: None,
            seo_keywords: None,
            length: None,
        };

        let prompt = CopyWriter::generate_copy_prompt(&request);

        Ok(serde_json::json!({
            "prompt_generated": prompt,
            "instructions": "Use this prompt with an LLM to generate the copy",
            "copy_type": copy_type_str,
        }))
    }

    async fn execute_analyze_seo(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let content = params.get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing content parameter")?;

        let keywords: Vec<String> = params.get("target_keywords")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let analysis = SEOAnalyzer::analyze(content, &keywords);

        Ok(serde_json::to_value(analysis).unwrap())
    }

    async fn execute_generate_calendar(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let platforms: Vec<String> = params.get("platforms")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec!["instagram".to_string(), "twitter".to_string()]);

        let content_pillars: Vec<String> = params.get("content_pillars")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec!["educational".to_string(), "promotional".to_string(), "engagement".to_string()]);

        let duration_days = params.get("duration_days")
            .and_then(|v| v.as_u64())
            .unwrap_or(30) as u32;

        let posts_per_week = params.get("posts_per_week")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as u32;

        let calendar = ContentCalendarGenerator::generate_calendar_structure(
            &platforms,
            &content_pillars,
            duration_days,
            posts_per_week,
        );

        Ok(serde_json::to_value(calendar).unwrap())
    }
}

impl Default for ContentTools {
    fn default() -> Self {
        Self::new()
    }
}
