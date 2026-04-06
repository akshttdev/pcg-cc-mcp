use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{types::Json, FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "custom_field_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CustomFieldType {
    Text,
    Number,
    Date,
    Url,
    Checkbox,
    Select,
    MultiSelect,
    Formula,
    Relationship,
    User,
    File,
    AutoIncrement,
}

impl CustomFieldType {
    pub fn parse_type(value: &str) -> Option<Self> {
        match value {
            "text" => Some(Self::Text),
            "number" => Some(Self::Number),
            "date" => Some(Self::Date),
            "url" => Some(Self::Url),
            "checkbox" => Some(Self::Checkbox),
            "select" => Some(Self::Select),
            "multi_select" => Some(Self::MultiSelect),
            "formula" => Some(Self::Formula),
            "relationship" => Some(Self::Relationship),
            "user" => Some(Self::User),
            "file" => Some(Self::File),
            "auto_increment" => Some(Self::AutoIncrement),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CustomFieldDefinition {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub field_type: CustomFieldType,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "Record<string, unknown> | null")]
    pub options: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "unknown")]
    pub default_value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(type = "Record<string, unknown> | null")]
    pub metadata: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct CreateCustomFieldDefinition {
    pub project_id: Uuid,
    pub name: String,
    pub field_type: CustomFieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    #[ts(type = "Record<string, unknown> | null")]
    pub options: Option<Value>,
    #[serde(default)]
    #[ts(type = "unknown")]
    pub default_value: Option<Value>,
    #[serde(default)]
    #[ts(type = "Record<string, unknown> | null")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCustomFieldDefinition {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub field_type: Option<CustomFieldType>,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    #[ts(type = "Record<string, unknown> | null")]
    pub options: Option<Option<Value>>,
    #[serde(default)]
    #[ts(type = "unknown")]
    pub default_value: Option<Option<Value>>,
    #[serde(default)]
    #[ts(type = "Record<string, unknown> | null")]
    pub metadata: Option<Option<Value>>,
}

impl CustomFieldDefinition {
    fn map_json(mut record: CustomFieldDefinitionInternal) -> CustomFieldDefinition {
        CustomFieldDefinition {
            id: record.id,
            project_id: record.project_id,
            name: record.name,
            field_type: record.field_type,
            required: record.required,
            options: record.options.take().map(|Json(value)| value),
            default_value: record.default_value.take().map(|Json(value)| value),
            metadata: record.metadata.take().map(|Json(value)| value),
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }

    pub async fn list_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            CustomFieldDefinitionInternal,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                field_type as "field_type!: CustomFieldType",
                required as "required!: bool",
                options as "options: Json<Value>",
                default_value as "default_value: Json<Value>",
                metadata as "metadata: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
              FROM custom_field_definitions
              WHERE project_id = $1
              ORDER BY created_at"#,
            project_id
        )
        .map(Self::map_json)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            CustomFieldDefinitionInternal,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                field_type as "field_type!: CustomFieldType",
                required as "required!: bool",
                options as "options: Json<Value>",
                default_value as "default_value: Json<Value>",
                metadata as "metadata: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
              FROM custom_field_definitions
              WHERE id = $1"#,
            id
        )
        .map(Self::map_json)
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &SqlitePool,
        payload: &CreateCustomFieldDefinition,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let field_type = payload.field_type.clone();
        let options_json = payload
            .options
            .as_ref()
            .map(|value| serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()));
        let default_value_json = payload
            .default_value
            .as_ref()
            .map(|value| serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()));
        let metadata_json = payload
            .metadata
            .as_ref()
            .map(|value| serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()));
        sqlx::query_as!(
            CustomFieldDefinitionInternal,
            r#"INSERT INTO custom_field_definitions
                (id, project_id, name, field_type, required, options, default_value, metadata)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                field_type as "field_type!: CustomFieldType",
                required as "required!: bool",
                options as "options: Json<Value>",
                default_value as "default_value: Json<Value>",
                metadata as "metadata: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            payload.project_id,
            payload.name,
            field_type,
            payload.required,
            options_json,
            default_value_json,
            metadata_json
        )
        .map(Self::map_json)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        payload: &UpdateCustomFieldDefinition,
    ) -> Result<Option<Self>, sqlx::Error> {
        let existing = match Self::find_by_id(pool, id).await? {
            Some(field) => field,
            None => return Ok(None),
        };

        let name = payload.name.clone().unwrap_or(existing.name);
        let field_type = payload
            .field_type
            .clone()
            .unwrap_or(existing.field_type.clone());
        let required = payload.required.unwrap_or(existing.required);
        let options = payload.options.clone().unwrap_or(existing.options.clone());
        let default_value = payload
            .default_value
            .clone()
            .unwrap_or(existing.default_value.clone());
        let metadata = payload
            .metadata
            .clone()
            .unwrap_or(existing.metadata.clone());

        let options_json = options
            .as_ref()
            .map(|value| serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()));
        let default_value_json = default_value
            .as_ref()
            .map(|value| serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()));
        let metadata_json = metadata
            .as_ref()
            .map(|value| serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()));

        sqlx::query_as!(
            CustomFieldDefinitionInternal,
            r#"UPDATE custom_field_definitions
                 SET name = $2,
                     field_type = $3,
                     required = $4,
                     options = $5,
                     default_value = $6,
                     metadata = $7,
                     updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                field_type as "field_type!: CustomFieldType",
                required as "required!: bool",
                options as "options: Json<Value>",
                default_value as "default_value: Json<Value>",
                metadata as "metadata: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            name,
            field_type,
            required,
            options_json,
            default_value_json,
            metadata_json
        )
        .map(Self::map_json)
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM custom_field_definitions WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

#[derive(Debug, Clone, FromRow)]
struct CustomFieldDefinitionInternal {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub field_type: CustomFieldType,
    pub required: bool,
    pub options: Option<Json<Value>>,
    pub default_value: Option<Json<Value>>,
    pub metadata: Option<Json<Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
