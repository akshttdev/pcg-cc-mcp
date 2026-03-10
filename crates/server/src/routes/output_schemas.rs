use axum::{
    Router,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct FieldDef {
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: bool,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TargetSchema {
    pub target_type: String,
    pub description: String,
    pub fields: std::collections::BTreeMap<String, FieldDef>,
}

pub fn crm_contact_schema() -> TargetSchema {
    let mut fields = std::collections::BTreeMap::new();

    fields.insert("first_name".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Contact's first name".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("last_name".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Contact's last name".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("email".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Email address".to_string(),
        enum_values: None,
        format: Some("email".to_string()),
    });
    fields.insert("phone".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Phone number".to_string(),
        enum_values: None,
        format: Some("phone".to_string()),
    });
    fields.insert("mobile".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Mobile phone number".to_string(),
        enum_values: None,
        format: Some("phone".to_string()),
    });
    fields.insert("company_name".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Company or organization the contact belongs to".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("job_title".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Job title or role".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("department".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Department within the company".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("linkedin_url".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "LinkedIn profile URL".to_string(),
        enum_values: None,
        format: Some("url".to_string()),
    });
    fields.insert("twitter_handle".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Twitter/X handle".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("website".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Personal or company website URL".to_string(),
        enum_values: None,
        format: Some("url".to_string()),
    });
    fields.insert("avatar_url".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "URL to contact's avatar image".to_string(),
        enum_values: None,
        format: Some("url".to_string()),
    });
    fields.insert("source".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "How the contact was acquired".to_string(),
        enum_values: Some(vec![
            "manual".to_string(), "email".to_string(), "social".to_string(),
            "website".to_string(), "referral".to_string(), "import".to_string(),
            "api".to_string(), "zoho_sync".to_string(), "gmail_sync".to_string(),
        ]),
        format: None,
    });
    fields.insert("lifecycle_stage".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Current lifecycle stage of the contact".to_string(),
        enum_values: Some(vec![
            "subscriber".to_string(), "lead".to_string(), "mql".to_string(),
            "sql".to_string(), "opportunity".to_string(), "customer".to_string(),
            "evangelist".to_string(), "churned".to_string(),
        ]),
        format: None,
    });
    fields.insert("tags".to_string(), FieldDef {
        field_type: "array".to_string(),
        required: false,
        description: "Tags for categorizing the contact".to_string(),
        enum_values: None,
        format: Some("string[]".to_string()),
    });
    fields.insert("custom_fields".to_string(), FieldDef {
        field_type: "object".to_string(),
        required: false,
        description: "Arbitrary custom fields as JSON object".to_string(),
        enum_values: None,
        format: None,
    });

    TargetSchema {
        target_type: "crm_contact".to_string(),
        description: "CRM Contact record".to_string(),
        fields,
    }
}

pub fn company_schema() -> TargetSchema {
    let mut fields = std::collections::BTreeMap::new();

    fields.insert("name".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: true,
        description: "Company name".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("slug".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "URL-friendly slug (auto-generated from name if not provided)".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("website".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Company website URL".to_string(),
        enum_values: None,
        format: Some("url".to_string()),
    });
    fields.insert("industry".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Industry or sector".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("description".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Company description".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("logo_url".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "URL to company logo image".to_string(),
        enum_values: None,
        format: Some("url".to_string()),
    });
    fields.insert("headquarters".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Company headquarters location".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("relationship".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Relationship type with this company".to_string(),
        enum_values: Some(vec![
            "potential_client".into(), "existing_client".into(), "partner".into(),
            "competitor".into(), "vendor".into(), "other".into(),
        ]),
        format: None,
    });
    fields.insert("context".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Context or notes about how this company was identified".to_string(),
        enum_values: None,
        format: None,
    });

    TargetSchema {
        target_type: "company".to_string(),
        description: "Company/Organization record".to_string(),
        fields,
    }
}

pub fn crm_deal_schema() -> TargetSchema {
    let mut fields = std::collections::BTreeMap::new();

    fields.insert("name".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: true,
        description: "Deal name or title".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("description".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Deal description".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("amount".to_string(), FieldDef {
        field_type: "number".to_string(),
        required: false,
        description: "Deal monetary value".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("currency".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Currency code (default: USD)".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("expected_close_date".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Expected close date".to_string(),
        enum_values: None,
        format: Some("date".to_string()),
    });
    fields.insert("tags".to_string(), FieldDef {
        field_type: "array".to_string(),
        required: false,
        description: "Tags for categorizing the deal".to_string(),
        enum_values: None,
        format: Some("string[]".to_string()),
    });
    fields.insert("contact_name".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Contact person name for the deal".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("contact_email".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Contact email for the deal (used to auto-link CRM contact)".to_string(),
        enum_values: None,
        format: Some("email".to_string()),
    });
    fields.insert("deal_type".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Type of deal or opportunity".to_string(),
        enum_values: Some(vec![
            "project".into(), "service".into(), "product".into(),
            "partnership".into(), "other".into(),
        ]),
        format: None,
    });
    fields.insert("next_steps".to_string(), FieldDef {
        field_type: "array".to_string(),
        required: false,
        description: "Action items or next steps for the deal".to_string(),
        enum_values: None,
        format: Some("string[]".to_string()),
    });
    fields.insert("estimated_value".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Human-readable estimated value (e.g. $150K)".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("custom_fields".to_string(), FieldDef {
        field_type: "object".to_string(),
        required: false,
        description: "Arbitrary custom fields as JSON object".to_string(),
        enum_values: None,
        format: None,
    });

    TargetSchema {
        target_type: "crm_deal".to_string(),
        description: "CRM Deal/Opportunity record".to_string(),
        fields,
    }
}

pub fn task_schema() -> TargetSchema {
    let mut fields = std::collections::BTreeMap::new();

    fields.insert("title".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: true,
        description: "Task title".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("description".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Task description".to_string(),
        enum_values: None,
        format: None,
    });
    fields.insert("priority".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Task priority level (default: medium)".to_string(),
        enum_values: Some(vec![
            "critical".to_string(), "high".to_string(),
            "medium".to_string(), "low".to_string(),
        ]),
        format: None,
    });
    fields.insert("tags".to_string(), FieldDef {
        field_type: "array".to_string(),
        required: false,
        description: "Tags for categorizing the task".to_string(),
        enum_values: None,
        format: Some("string[]".to_string()),
    });
    fields.insert("due_date".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "Due date for the task".to_string(),
        enum_values: None,
        format: Some("date-time".to_string()),
    });
    fields.insert("assignee_id".to_string(), FieldDef {
        field_type: "string".to_string(),
        required: false,
        description: "User ID of the assignee".to_string(),
        enum_values: None,
        format: Some("uuid".to_string()),
    });

    TargetSchema {
        target_type: "task".to_string(),
        description: "Task/Action Item".to_string(),
        fields,
    }
}

/// Look up the schema for a given target_type string.
/// Returns None if the target_type is unknown.
pub fn get_schema_for_target(target_type: &str) -> Option<TargetSchema> {
    match target_type {
        "crm_contact" => Some(crm_contact_schema()),
        "company" => Some(company_schema()),
        "crm_deal" => Some(crm_deal_schema()),
        "task" => Some(task_schema()),
        _ => None,
    }
}

async fn get_schema(Path(target_type): Path<String>) -> Result<impl IntoResponse, (StatusCode, String)> {
    let schema = match target_type.as_str() {
        "crm_contact" => crm_contact_schema(),
        "company" => company_schema(),
        "crm_deal" => crm_deal_schema(),
        "task" => task_schema(),
        _ => return Err((StatusCode::NOT_FOUND, format!("Unknown target type: {}", target_type))),
    };
    Ok(Json(schema))
}

async fn list_schemas() -> impl IntoResponse {
    Json(json!([
        { "target_type": "crm_contact", "description": "CRM Contact record", "icon": "user" },
        { "target_type": "company", "description": "Company/Organization record", "icon": "building" },
        { "target_type": "crm_deal", "description": "CRM Deal/Opportunity record", "icon": "handshake" },
        { "target_type": "task", "description": "Task/Action Item", "icon": "check-square" },
    ]))
}

pub fn router() -> Router<crate::DeploymentImpl> {
    Router::new()
        .route("/schemas", get(list_schemas))
        .route("/schemas/{target_type}", get(get_schema))
}
