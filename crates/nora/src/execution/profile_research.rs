//! Direct Profile Research
//!
//! Researches speakers and brands using extracted URLs directly (no web search fabrication).
//! Uses actual data from LinkedIn, Twitter, personal websites, and conference pages.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::time::Duration;

use super::crawler::{CrawledPage, WebsiteCrawler, CrawlConfig};

/// Social links extracted from conference page
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SocialLinks {
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub website: Option<String>,
    pub github_url: Option<String>,
}

/// Source of data for tracking provenance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataSource {
    ConferencePage,
    LinkedIn,
    Twitter,
    PersonalWebsite,
    CompanyWebsite,
    CrunchBase,
}

/// Result of speaker research with provenance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerResearchResult {
    pub name: String,
    pub title: Option<String>,
    pub company: Option<String>,
    pub bio: Option<String>,
    pub photo_url: Option<String>,
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub website: Option<String>,
    pub talk_title: Option<String>,
    pub talk_description: Option<String>,
    pub expertise: Vec<String>,
    /// Where data actually came from
    pub data_sources: Vec<DataSource>,
    /// Calculated completeness score (0.0-1.0)
    pub data_completeness: f64,
}

impl Default for SpeakerResearchResult {
    fn default() -> Self {
        Self {
            name: String::new(),
            title: None,
            company: None,
            bio: None,
            photo_url: None,
            linkedin_url: None,
            twitter_handle: None,
            website: None,
            talk_title: None,
            talk_description: None,
            expertise: Vec::new(),
            data_sources: Vec::new(),
            data_completeness: 0.0,
        }
    }
}

/// Result of brand/sponsor research with provenance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandResearchResult {
    pub name: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub logo_url: Option<String>,
    pub industry: Option<String>,
    pub headquarters: Option<String>,
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub sponsorship_level: Option<String>,
    /// Where data actually came from
    pub data_sources: Vec<DataSource>,
    /// Calculated completeness score (0.0-1.0)
    pub data_completeness: f64,
}

impl Default for BrandResearchResult {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            website: None,
            logo_url: None,
            industry: None,
            headquarters: None,
            linkedin_url: None,
            twitter_handle: None,
            sponsorship_level: None,
            data_sources: Vec::new(),
            data_completeness: 0.0,
        }
    }
}

/// Direct profile researcher - uses real URLs, no fabrication
pub struct ProfileResearcher {
    http_client: Client,
    pool: SqlitePool,
}

impl ProfileResearcher {
    pub fn new(pool: SqlitePool) -> Self {
        let http_client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; Scout/1.0; Research Agent)")
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_default();

        Self { http_client, pool }
    }

    /// Research speaker using extracted URLs (no web search fabrication)
    pub async fn research_speaker(
        &self,
        name: &str,
        social_links: &SocialLinks,
        conference_html: Option<&str>,
    ) -> SpeakerResearchResult {
        let mut result = SpeakerResearchResult {
            name: name.to_string(),
            ..Default::default()
        };

        tracing::info!("[PROFILE_RESEARCH] Researching speaker: {} (has_linkedin={}, has_twitter={}, has_website={})",
            name,
            social_links.linkedin_url.is_some(),
            social_links.twitter_handle.is_some(),
            social_links.website.is_some()
        );

        // Step 1: Extract from conference page HTML if available
        if let Some(html) = conference_html {
            self.extract_speaker_from_conference_html(&mut result, name, html);
            if result.title.is_some() || result.bio.is_some() || result.photo_url.is_some() {
                if !result.data_sources.contains(&DataSource::ConferencePage) {
                    result.data_sources.push(DataSource::ConferencePage);
                }
            }
        }

        // Step 2: Fetch LinkedIn profile if URL provided
        if let Some(ref linkedin_url) = social_links.linkedin_url {
            result.linkedin_url = Some(linkedin_url.clone());
            // Note: LinkedIn profiles typically require authentication to scrape
            // We store the URL but don't fetch (would need official API)
            tracing::debug!("[PROFILE_RESEARCH] LinkedIn URL stored for {}: {}", name, linkedin_url);
        }

        // Step 3: Process Twitter handle
        if let Some(ref twitter) = social_links.twitter_handle {
            result.twitter_handle = Some(twitter.clone());
            // Note: Twitter/X API access would be needed for profile data
            tracing::debug!("[PROFILE_RESEARCH] Twitter handle stored for {}: {}", name, twitter);
        }

        // Step 4: Fetch personal website if URL provided
        if let Some(ref website_url) = social_links.website {
            result.website = Some(website_url.clone());
            match self.fetch_personal_website(website_url).await {
                Ok(info) => {
                    // Merge info from personal website (don't overwrite existing data)
                    if result.bio.is_none() {
                        result.bio = info.bio;
                    }
                    if result.photo_url.is_none() {
                        result.photo_url = info.photo_url;
                    }
                    if result.title.is_none() {
                        result.title = info.title;
                    }
                    if !result.data_sources.contains(&DataSource::PersonalWebsite) {
                        result.data_sources.push(DataSource::PersonalWebsite);
                    }
                    tracing::info!("[PROFILE_RESEARCH] Extracted data from personal website for {}", name);
                }
                Err(e) => {
                    tracing::warn!("[PROFILE_RESEARCH] Failed to fetch personal website for {}: {}", name, e);
                }
            }
        }

        // Calculate data completeness
        result.data_completeness = calculate_speaker_completeness(&result);

        tracing::info!(
            "[PROFILE_RESEARCH] {} - completeness={:.0}%, sources={:?}",
            name,
            result.data_completeness * 100.0,
            result.data_sources
        );

        // Log warnings for missing important fields
        if result.bio.is_none() {
            tracing::warn!("[PROFILE_RESEARCH] No bio found for {} - field left empty", name);
        }
        if result.photo_url.is_none() {
            tracing::warn!("[PROFILE_RESEARCH] No photo found for {} - field left empty", name);
        }

        result
    }

    /// Research brand/sponsor using direct website fetch (no web search fabrication)
    pub async fn research_brand(
        &self,
        name: &str,
        website_url: Option<&str>,
        conference_html: Option<&str>,
    ) -> BrandResearchResult {
        let mut result = BrandResearchResult {
            name: name.to_string(),
            ..Default::default()
        };

        tracing::info!("[PROFILE_RESEARCH] Researching brand: {} (has_website={})",
            name,
            website_url.is_some()
        );

        // Step 1: Extract from conference page HTML if available
        if let Some(html) = conference_html {
            self.extract_brand_from_conference_html(&mut result, name, html);
            if result.logo_url.is_some() || result.sponsorship_level.is_some() {
                if !result.data_sources.contains(&DataSource::ConferencePage) {
                    result.data_sources.push(DataSource::ConferencePage);
                }
            }
        }

        // Step 2: Fetch company website if URL provided
        if let Some(url) = website_url {
            result.website = Some(url.to_string());
            match self.fetch_company_website(url).await {
                Ok(info) => {
                    // Merge info from company website
                    if result.description.is_none() {
                        result.description = info.description;
                    }
                    if result.logo_url.is_none() {
                        result.logo_url = info.logo_url;
                    }
                    if result.industry.is_none() {
                        result.industry = info.industry;
                    }
                    result.linkedin_url = info.linkedin_url.or(result.linkedin_url);
                    result.twitter_handle = info.twitter_handle.or(result.twitter_handle);
                    if !result.data_sources.contains(&DataSource::CompanyWebsite) {
                        result.data_sources.push(DataSource::CompanyWebsite);
                    }
                    tracing::info!("[PROFILE_RESEARCH] Extracted data from company website for {}", name);
                }
                Err(e) => {
                    tracing::warn!("[PROFILE_RESEARCH] Failed to fetch company website for {}: {}", name, e);
                }
            }
        }

        // Calculate data completeness
        result.data_completeness = calculate_brand_completeness(&result);

        tracing::info!(
            "[PROFILE_RESEARCH] {} - completeness={:.0}%, sources={:?}",
            name,
            result.data_completeness * 100.0,
            result.data_sources
        );

        result
    }

    /// Extract speaker info from conference HTML near their name
    fn extract_speaker_from_conference_html(&self, result: &mut SpeakerResearchResult, name: &str, html: &str) {
        let name_lower = name.to_lowercase();

        // Look for speaker cards/sections containing the name
        // Pattern: Find a section containing the name and extract nearby data

        // Try to find photo URL near the name
        if let Some(photo_url) = self.find_image_near_name(html, name) {
            result.photo_url = Some(photo_url);
        }

        // Try to extract title/role
        if let Some(title) = self.extract_title_near_name(html, name) {
            result.title = Some(title);
        }

        // Try to extract bio/description
        if let Some(bio) = self.extract_bio_near_name(html, name) {
            result.bio = Some(bio);
        }

        // Extract social links near the name
        let social = self.extract_social_links_near_name(html, name);
        if result.linkedin_url.is_none() {
            result.linkedin_url = social.linkedin_url;
        }
        if result.twitter_handle.is_none() {
            result.twitter_handle = social.twitter_handle;
        }
        if result.website.is_none() {
            result.website = social.website;
        }
    }

    /// Extract brand info from conference HTML
    fn extract_brand_from_conference_html(&self, result: &mut BrandResearchResult, name: &str, html: &str) {
        // Look for sponsor sections containing the brand name

        // Try to find logo URL near the name
        if let Some(logo_url) = self.find_image_near_name(html, name) {
            result.logo_url = Some(logo_url);
        }

        // Try to extract sponsorship level (platinum, gold, silver, etc.)
        if let Some(level) = self.extract_sponsorship_level(html, name) {
            result.sponsorship_level = Some(level);
        }
    }

    /// Find an image URL near a person/brand name in HTML
    fn find_image_near_name(&self, html: &str, name: &str) -> Option<String> {
        let name_escaped = regex::escape(name);

        // Pattern 1: Image with alt text containing the name
        if let Ok(re) = regex::Regex::new(&format!(r#"<img[^>]+alt="[^"]*{}[^"]*"[^>]+src="([^"]+)""#, name_escaped)) {
            if let Some(cap) = re.captures(html) {
                if let Some(src) = cap.get(1) {
                    return Some(src.as_str().to_string());
                }
            }
        }

        // Pattern 2: Image with src, then alt containing name
        if let Ok(re) = regex::Regex::new(&format!(r#"<img[^>]+src="([^"]+)"[^>]+alt="[^"]*{}[^"]*""#, name_escaped)) {
            if let Some(cap) = re.captures(html) {
                if let Some(src) = cap.get(1) {
                    return Some(src.as_str().to_string());
                }
            }
        }

        None
    }

    /// Extract title/role near a person's name
    fn extract_title_near_name(&self, html: &str, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();

        // Find position of name in HTML
        if let Some(name_pos) = html.to_lowercase().find(&name_lower) {
            // Look in the surrounding 500 characters
            let start = name_pos.saturating_sub(200);
            let end = (name_pos + 500).min(html.len());
            let context = &html[start..end];

            // Look for common title patterns
            // Pattern: <p class="title">CEO at Company</p>
            if let Ok(re) = regex::Regex::new(r#"<(?:p|span|div)[^>]*class="[^"]*(?:title|role|position)[^"]*"[^>]*>([^<]+)</(?:p|span|div)>"#) {
                if let Some(cap) = re.captures(context) {
                    if let Some(title) = cap.get(1) {
                        return Some(title.as_str().trim().to_string());
                    }
                }
            }

            // Pattern: "Title at Company" or "Title, Company"
            if let Ok(re) = regex::Regex::new(r">([A-Z][^<]{3,50}(?:at|@|,)\s+[^<]{2,30})<") {
                if let Some(cap) = re.captures(context) {
                    if let Some(title) = cap.get(1) {
                        let t = title.as_str().trim();
                        if t.len() < 80 && !t.contains("http") {
                            return Some(t.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract bio/description near a person's name
    fn extract_bio_near_name(&self, html: &str, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();

        if let Some(name_pos) = html.to_lowercase().find(&name_lower) {
            let start = name_pos.saturating_sub(100);
            let end = (name_pos + 1500).min(html.len());
            let context = &html[start..end];

            // Look for bio patterns
            // Pattern: <p class="bio">Bio text...</p>
            if let Ok(re) = regex::Regex::new(r#"<(?:p|div)[^>]*class="[^"]*(?:bio|description|about)[^"]*"[^>]*>([^<]{50,500})</(?:p|div)>"#) {
                if let Some(cap) = re.captures(context) {
                    if let Some(bio) = cap.get(1) {
                        return Some(bio.as_str().trim().to_string());
                    }
                }
            }

            // Pattern: Longer paragraph after name
            if let Ok(re) = regex::Regex::new(r"<p[^>]*>([^<]{100,500})</p>") {
                for cap in re.captures_iter(context) {
                    if let Some(text) = cap.get(1) {
                        let t = text.as_str().trim();
                        // Check if it looks like a bio (contains common bio words)
                        let t_lower = t.to_lowercase();
                        if (t_lower.contains("is a") || t_lower.contains("works") ||
                            t_lower.contains("leads") || t_lower.contains("founded") ||
                            t_lower.contains("expert") || t_lower.contains("experience")) {
                            return Some(t.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract social links near a person's name
    fn extract_social_links_near_name(&self, html: &str, name: &str) -> SocialLinks {
        let mut links = SocialLinks::default();
        let name_lower = name.to_lowercase();

        if let Some(name_pos) = html.to_lowercase().find(&name_lower) {
            let start = name_pos.saturating_sub(200);
            let end = (name_pos + 800).min(html.len());
            let context = &html[start..end];

            // LinkedIn
            if let Ok(re) = regex::Regex::new(r#"href="(https?://(?:www\.)?linkedin\.com/in/[^"]+)""#) {
                if let Some(cap) = re.captures(context) {
                    if let Some(url) = cap.get(1) {
                        links.linkedin_url = Some(url.as_str().to_string());
                    }
                }
            }

            // Twitter/X
            if let Ok(re) = regex::Regex::new(r#"href="https?://(?:www\.)?(?:twitter|x)\.com/([^"/?]+)"#) {
                if let Some(cap) = re.captures(context) {
                    if let Some(handle) = cap.get(1) {
                        let h = handle.as_str();
                        if h != "share" && h != "intent" {
                            links.twitter_handle = Some(format!("@{}", h));
                        }
                    }
                }
            }

            // Personal website (excluding social platforms)
            if let Ok(re) = regex::Regex::new(r#"href="(https?://[^"]+)"[^>]*>(?:website|personal|homepage|site)"#) {
                if let Some(cap) = re.captures(&context.to_lowercase()) {
                    if let Some(url) = cap.get(1) {
                        let u = url.as_str();
                        if !u.contains("linkedin") && !u.contains("twitter") &&
                           !u.contains("facebook") && !u.contains("instagram") {
                            links.website = Some(u.to_string());
                        }
                    }
                }
            }
        }

        links
    }

    /// Extract sponsorship level from HTML
    fn extract_sponsorship_level(&self, html: &str, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();
        let html_lower = html.to_lowercase();

        if let Some(name_pos) = html_lower.find(&name_lower) {
            // Look in surrounding context for tier/level indicators
            let start = name_pos.saturating_sub(500);
            let end = (name_pos + 200).min(html_lower.len());
            let context = &html_lower[start..end];

            let levels = ["platinum", "gold", "silver", "bronze", "diamond", "premier", "principal", "founding"];
            for level in levels {
                if context.contains(level) {
                    return Some(level.to_string().to_uppercase().chars().next().unwrap().to_string() +
                               &level[1..]);
                }
            }
        }

        None
    }

    /// Fetch and extract info from a personal website
    async fn fetch_personal_website(&self, url: &str) -> Result<PersonalWebsiteInfo, String> {
        let response = self.http_client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let html = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        Ok(extract_personal_website_info(&html))
    }

    /// Fetch and extract info from a company website
    async fn fetch_company_website(&self, url: &str) -> Result<CompanyWebsiteInfo, String> {
        let response = self.http_client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let html = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        Ok(extract_company_website_info(&html, url))
    }
}

/// Info extracted from a personal website
struct PersonalWebsiteInfo {
    bio: Option<String>,
    title: Option<String>,
    photo_url: Option<String>,
}

/// Info extracted from a company website
struct CompanyWebsiteInfo {
    description: Option<String>,
    logo_url: Option<String>,
    industry: Option<String>,
    linkedin_url: Option<String>,
    twitter_handle: Option<String>,
}

/// Extract info from a personal website HTML
fn extract_personal_website_info(html: &str) -> PersonalWebsiteInfo {
    let mut info = PersonalWebsiteInfo {
        bio: None,
        title: None,
        photo_url: None,
    };

    // Extract meta description as bio
    if let Ok(re) = regex::Regex::new(r#"<meta[^>]+name="description"[^>]+content="([^"]+)""#) {
        if let Some(cap) = re.captures(html) {
            if let Some(desc) = cap.get(1) {
                let d = desc.as_str();
                if d.len() > 50 && d.len() < 500 {
                    info.bio = Some(d.to_string());
                }
            }
        }
    }

    // Extract OpenGraph title
    if let Ok(re) = regex::Regex::new(r#"<meta[^>]+property="og:title"[^>]+content="([^"]+)""#) {
        if let Some(cap) = re.captures(html) {
            if let Some(title) = cap.get(1) {
                info.title = Some(title.as_str().to_string());
            }
        }
    }

    // Extract OpenGraph image as photo
    if let Ok(re) = regex::Regex::new(r#"<meta[^>]+property="og:image"[^>]+content="([^"]+)""#) {
        if let Some(cap) = re.captures(html) {
            if let Some(img) = cap.get(1) {
                info.photo_url = Some(img.as_str().to_string());
            }
        }
    }

    info
}

/// Extract info from a company website HTML
fn extract_company_website_info(html: &str, base_url: &str) -> CompanyWebsiteInfo {
    let mut info = CompanyWebsiteInfo {
        description: None,
        logo_url: None,
        industry: None,
        linkedin_url: None,
        twitter_handle: None,
    };

    // Extract meta description
    if let Ok(re) = regex::Regex::new(r#"<meta[^>]+name="description"[^>]+content="([^"]+)""#) {
        if let Some(cap) = re.captures(html) {
            if let Some(desc) = cap.get(1) {
                let d = desc.as_str();
                if d.len() > 30 && d.len() < 500 {
                    info.description = Some(d.to_string());
                }
            }
        }
    }

    // Extract OpenGraph image as logo
    if let Ok(re) = regex::Regex::new(r#"<meta[^>]+property="og:image"[^>]+content="([^"]+)""#) {
        if let Some(cap) = re.captures(html) {
            if let Some(img) = cap.get(1) {
                info.logo_url = Some(img.as_str().to_string());
            }
        }
    }

    // Look for LinkedIn company page
    if let Ok(re) = regex::Regex::new(r#"href="(https?://(?:www\.)?linkedin\.com/company/[^"]+)""#) {
        if let Some(cap) = re.captures(html) {
            if let Some(url) = cap.get(1) {
                info.linkedin_url = Some(url.as_str().to_string());
            }
        }
    }

    // Look for Twitter handle
    if let Ok(re) = regex::Regex::new(r#"href="https?://(?:www\.)?(?:twitter|x)\.com/([^"/?]+)"#) {
        if let Some(cap) = re.captures(html) {
            if let Some(handle) = cap.get(1) {
                let h = handle.as_str();
                if h != "share" && h != "intent" {
                    info.twitter_handle = Some(format!("@{}", h));
                }
            }
        }
    }

    // Try to extract industry from JSON-LD
    if let Ok(re) = regex::Regex::new(r#"<script[^>]*type="application/ld\+json"[^>]*>([\s\S]*?)</script>"#) {
        for cap in re.captures_iter(html) {
            if let Some(json_str) = cap.get(1) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str.as_str()) {
                    if let Some(industry) = json.get("industry").and_then(|v| v.as_str()) {
                        info.industry = Some(industry.to_string());
                    }
                }
            }
        }
    }

    info
}

/// Calculate completeness score for speaker research
pub fn calculate_speaker_completeness(result: &SpeakerResearchResult) -> f64 {
    let fields = [
        (!result.name.is_empty(), 0.10),
        (result.bio.is_some(), 0.20),
        (result.title.is_some(), 0.15),
        (result.company.is_some(), 0.15),
        (result.photo_url.is_some(), 0.15),
        (result.linkedin_url.is_some(), 0.10),
        (result.twitter_handle.is_some(), 0.05),
        (result.website.is_some(), 0.10),
    ];

    fields.iter()
        .filter(|(has, _)| *has)
        .map(|(_, w)| w)
        .sum()
}

/// Calculate completeness score for brand research
pub fn calculate_brand_completeness(result: &BrandResearchResult) -> f64 {
    let fields = [
        (!result.name.is_empty(), 0.10),
        (result.description.is_some(), 0.20),
        (result.website.is_some(), 0.15),
        (result.logo_url.is_some(), 0.15),
        (result.industry.is_some(), 0.10),
        (result.linkedin_url.is_some(), 0.15),
        (result.twitter_handle.is_some(), 0.10),
        (result.sponsorship_level.is_some(), 0.05),
    ];

    fields.iter()
        .filter(|(has, _)| *has)
        .map(|(_, w)| w)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speaker_completeness_full() {
        let result = SpeakerResearchResult {
            name: "John Doe".to_string(),
            bio: Some("A professional".to_string()),
            title: Some("CEO".to_string()),
            company: Some("Acme".to_string()),
            photo_url: Some("http://example.com/photo.jpg".to_string()),
            linkedin_url: Some("http://linkedin.com/in/john".to_string()),
            twitter_handle: Some("@johndoe".to_string()),
            website: Some("http://johndoe.com".to_string()),
            ..Default::default()
        };
        let completeness = calculate_speaker_completeness(&result);
        assert!((completeness - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_speaker_completeness_partial() {
        let result = SpeakerResearchResult {
            name: "John Doe".to_string(),
            title: Some("CEO".to_string()),
            ..Default::default()
        };
        let completeness = calculate_speaker_completeness(&result);
        assert!(completeness > 0.2 && completeness < 0.3);
    }
}
