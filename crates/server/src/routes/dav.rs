//! APN Drive — WebDAV server
//!
//! Admin access (full filesystem):
//!   Username: pcg   Password: <ADMIN_API_KEY>
//!
//! User access (org-scoped virtual tree):
//!   Username: <email>   Password: <session_token>
//!
//! The virtual tree maps:
//!   /                              → user's org folders
//!   /<org-slug>/                   → org's project folders
//!   /<org-slug>/<proj-slug>/       → source, editron_output, renders, artifacts
//!   /<org-slug>/<proj-slug>/source/<rest>  → media_pipeline2/source/<proj-slug>/<rest>

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{StatusCode, header},
    response::Response,
    routing::any,
};
use base64::Engine as _;
use deployment::Deployment;
use sqlx::SqlitePool;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
};
use tokio_util::io::ReaderStream;
use utils::assets::asset_dir;

use crate::{DeploymentImpl, middleware::access_control::get_current_user};

/// Volume name shown in macOS Finder
pub const APN_VOLUME_NAME: &str = "PCG APN";
/// Mount path on operator's Mac after connecting
pub const APN_MOUNT_PATH: &str = "/Volumes/PCG APN";

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/dav", any(dav_handler))
        .route("/dav/", any(dav_handler))
        .route("/dav/{*path}", any(dav_handler))
        .with_state(deployment.clone())
}

// ── Auth ──────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct OrgEntry {
    id: String,
    name: String,
    slug: String,
}

#[derive(Debug)]
struct ProjEntry {
    slug: String,
}

enum DavAuth {
    Admin,
    User { orgs: Vec<OrgEntry> },
    Denied,
}

/// Extract the raw auth header value from a request (sync, no borrows held across await)
fn extract_auth_header(req: &Request) -> Option<String> {
    req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

async fn resolve_auth(auth_header_str: Option<String>, deployment: &DeploymentImpl) -> DavAuth {
    let expected_admin = std::env::var("ADMIN_API_KEY").unwrap_or_default();

    if let Some(s) = auth_header_str {
        let s = s.as_str();

        // Extract raw token from Bearer or Basic
        let token = if let Some(t) = s.strip_prefix("Bearer ") {
            t.to_string()
        } else if let Some(encoded) = s.strip_prefix("Basic ") {
            // Base64(username:password) — password is the token
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .and_then(|creds| creds.splitn(2, ':').nth(1).map(|p| p.to_string()))
                .unwrap_or_default()
        } else {
            return DavAuth::Denied;
        };

        // Admin check first
        if !expected_admin.is_empty() && token == expected_admin {
            return DavAuth::Admin;
        }

        // Try session token
        let auth_header = Some(format!("Bearer {}", token));
        let auth_ref = auth_header.as_deref();
        if let Ok(ctx) = get_current_user(deployment, auth_ref, None).await {
            let pool = &deployment.db().pool;
            let user_id_str = ctx.user_id.to_string();
            let orgs = orgs_for_user(pool, &user_id_str).await;
            if ctx.is_admin {
                return DavAuth::Admin;
            }
            return DavAuth::User { orgs };
        }
    }

    // No auth header — deny (triggers 401 + WWW-Authenticate)
    DavAuth::Denied
}

async fn orgs_for_user(pool: &SqlitePool, user_id: &str) -> Vec<OrgEntry> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
        name: String,
        slug: String,
    }

    sqlx::query_as::<_, Row>(
        "SELECT o.id, o.name, o.slug FROM organizations o
         INNER JOIN organization_members om ON om.organization_id = o.id
         WHERE om.user_id = ? AND o.is_active = 1
         ORDER BY o.name ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| OrgEntry {
        id: r.id,
        name: r.name,
        slug: r.slug,
    })
    .collect()
}

async fn projects_for_org(pool: &SqlitePool, org_id: &str) -> Vec<ProjEntry> {
    #[derive(sqlx::FromRow)]
    struct Row {
        slug: Option<String>,
        name: String,
    }

    sqlx::query_as::<_, Row>(
        "SELECT COALESCE(slug, lower(replace(name,' ','-'))) as slug, name
         FROM projects WHERE organization_id = ? AND deleted_at IS NULL ORDER BY name ASC",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| ProjEntry {
        slug: r
            .slug
            .unwrap_or_else(|| r.name.to_lowercase().replace(' ', "-")),
    })
    .collect()
}

fn unauthorized_response() -> Response {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header("WWW-Authenticate", r#"Basic realm="PCG APN Drive""#)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from("Authentication required"))
        .unwrap()
}

// ── Virtual path ──────────────────────────────────────────────────────────────

/// Subfolders exposed under each project
const PROJECT_SUBDIRS: &[&str] = &["source", "editron_output", "renders", "artifacts"];

#[derive(Debug)]
enum VPath<'a> {
    Root,
    OrgRoot {
        org_slug: &'a str,
    },
    ProjRoot {
        org_slug: &'a str,
        proj_slug: &'a str,
    },
    SubDir {
        org_slug: &'a str,
        proj_slug: &'a str,
        sub: &'a str,
    },
    SubFile {
        org_slug: &'a str,
        proj_slug: &'a str,
        sub: &'a str,
        rest: &'a str,
    },
}

fn parse_vpath(raw: &str) -> VPath<'_> {
    // raw is the decoded path after stripping /api/dav or /dav — starts with /
    let trimmed = raw.trim_matches('/');
    if trimmed.is_empty() {
        return VPath::Root;
    }
    let parts: Vec<&str> = trimmed.splitn(4, '/').collect();
    match parts.as_slice() {
        [org] => VPath::OrgRoot { org_slug: org },
        [org, proj] => VPath::ProjRoot {
            org_slug: org,
            proj_slug: proj,
        },
        [org, proj, sub] => VPath::SubDir {
            org_slug: org,
            proj_slug: proj,
            sub,
        },
        [org, proj, sub, rest] => VPath::SubFile {
            org_slug: org,
            proj_slug: proj,
            sub,
            rest,
        },
        _ => VPath::Root,
    }
}

/// Map a virtual project subfolder + rest to a real filesystem path
fn real_path(proj_slug: &str, sub: &str, rest: &str) -> std::path::PathBuf {
    let base = asset_dir();
    match sub {
        "source" => base
            .join("media_pipeline2")
            .join("source")
            .join(proj_slug)
            .join(rest),
        "editron_output" => base
            .join("media_pipeline2")
            .join("editron_output")
            .join(proj_slug)
            .join(rest),
        "renders" => base
            .join("media_pipeline2")
            .join("renders")
            .join(proj_slug)
            .join(rest),
        "artifacts" => base.join("artifacts").join(rest),
        _ => base.join(sub).join(rest),
    }
}

// ── Main handler ──────────────────────────────────────────────────────────────

async fn dav_handler(State(deployment): State<DeploymentImpl>, req: Request) -> Response {
    let method = req.method().as_str().to_uppercase();
    let uri_path = req.uri().path().to_string();

    // OPTIONS — no auth needed (macOS probes before mount)
    if method == "OPTIONS" {
        return handle_options();
    }

    // Extract auth header synchronously before any await (Request<Body> is not Sync)
    let auth_header_str = extract_auth_header(&req);
    let auth = resolve_auth(auth_header_str, &deployment).await;
    if matches!(auth, DavAuth::Denied) {
        return unauthorized_response();
    }

    // Strip /api/dav prefix and URL-decode
    let raw = uri_path
        .strip_prefix("/api/dav")
        .or_else(|| uri_path.strip_prefix("/dav"))
        .unwrap_or("/");
    let fs_rel = urlencoding::decode(raw).unwrap_or_default().into_owned();

    // Security: reject traversal
    if fs_rel.contains("..") {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::empty())
            .unwrap();
    }

    let depth = req
        .headers()
        .get("Depth")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("1")
        .to_string();

    match auth {
        DavAuth::Admin => {
            // Admin: serve scoped raw filesystem (existing behaviour)
            const ALLOWED: &[&str] = &["media_pipeline2", "media_pipeline", "artifacts", "media"];
            let rel = fs_rel.trim_start_matches('/');
            if !rel.is_empty() {
                let top = rel.splitn(2, '/').next().unwrap_or("");
                if !ALLOWED.contains(&top) {
                    return Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::empty())
                        .unwrap();
                }
            }
            let local_path = asset_dir().join(rel);
            match method.as_str() {
                "PROPFIND" => {
                    let asset_root = asset_dir();
                    let is_root = local_path == asset_root;
                    handle_propfind_fs(&local_path, &uri_path, &depth, is_root, ALLOWED).await
                }
                "GET" => handle_get(req, &local_path, false).await,
                "HEAD" => handle_get(req, &local_path, true).await,
                _ => method_not_allowed(),
            }
        }
        DavAuth::User { orgs } => {
            let pool = deployment.db().pool.clone();
            let vpath = parse_vpath(&fs_rel);
            match method.as_str() {
                "PROPFIND" => handle_propfind_virtual(vpath, &orgs, &pool, &uri_path, &depth).await,
                "GET" => handle_get_virtual(vpath, &orgs, &pool, req).await,
                "HEAD" => handle_get_virtual_head(vpath, &orgs, &pool).await,
                _ => method_not_allowed(),
            }
        }
        DavAuth::Denied => unauthorized_response(),
    }
}

// ── OPTIONS ───────────────────────────────────────────────────────────────────

fn handle_options() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("DAV", "1, 2")
        .header("MS-Author-Via", "DAV")
        .header(header::ALLOW, "OPTIONS, GET, HEAD, PROPFIND")
        .header(header::CONTENT_LENGTH, "0")
        .body(Body::empty())
        .unwrap()
}

fn method_not_allowed() -> Response {
    Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .header(header::ALLOW, "OPTIONS, GET, HEAD, PROPFIND")
        .body(Body::empty())
        .unwrap()
}

// ── PROPFIND (filesystem — admin) ─────────────────────────────────────────────

async fn handle_propfind_fs(
    local_path: &std::path::Path,
    href_path: &str,
    depth: &str,
    is_root: bool,
    allowed: &[&str],
) -> Response {
    let mut xml =
        String::from(r#"<?xml version="1.0" encoding="utf-8"?><D:multistatus xmlns:D="DAV:">"#);

    match tokio::fs::metadata(local_path).await {
        Ok(meta) => xml.push_str(&prop_response(href_path, &meta, local_path, is_root)),
        Err(_) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap();
        }
    }

    if depth != "0" {
        if let Ok(mut entries) = tokio::fs::read_dir(local_path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                if is_root && !allowed.contains(&name.as_str()) {
                    continue;
                }
                if let Ok(meta) = entry.metadata().await {
                    let child_href = if href_path.ends_with('/') {
                        format!("{}{}", href_path, urlencoding::encode(&name))
                    } else {
                        format!("{}/{}", href_path, urlencoding::encode(&name))
                    };
                    xml.push_str(&prop_response(&child_href, &meta, &entry.path(), false));
                }
            }
        }
    }

    xml.push_str("</D:multistatus>");
    multistatus_response(xml)
}

// ── PROPFIND (virtual — user) ─────────────────────────────────────────────────

async fn handle_propfind_virtual(
    vpath: VPath<'_>,
    orgs: &[OrgEntry],
    pool: &SqlitePool,
    href_path: &str,
    depth: &str,
) -> Response {
    let mut xml =
        String::from(r#"<?xml version="1.0" encoding="utf-8"?><D:multistatus xmlns:D="DAV:">"#);

    // Self entry
    match &vpath {
        VPath::Root => {
            xml.push_str(&virt_collection_response(href_path, APN_VOLUME_NAME));
        }
        VPath::OrgRoot { org_slug } => {
            let name = orgs
                .iter()
                .find(|o| o.slug == *org_slug)
                .map(|o| o.name.as_str())
                .unwrap_or(org_slug);
            if !orgs.iter().any(|o| o.slug == *org_slug) {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap();
            }
            xml.push_str(&virt_collection_response(href_path, name));
        }
        VPath::ProjRoot {
            org_slug,
            proj_slug,
        } => {
            if !orgs.iter().any(|o| o.slug == *org_slug) {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap();
            }
            xml.push_str(&virt_collection_response(href_path, proj_slug));
        }
        VPath::SubDir {
            org_slug,
            proj_slug,
            sub,
        } => {
            if !orgs.iter().any(|o| o.slug == *org_slug) {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap();
            }
            let rp = real_path(proj_slug, sub, "");
            if !rp.exists() {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap();
            }
            xml.push_str(&virt_collection_response(href_path, sub));
        }
        VPath::SubFile {
            org_slug,
            proj_slug,
            sub,
            rest,
        } => {
            if !orgs.iter().any(|o| o.slug == *org_slug) {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap();
            }
            let rp = real_path(proj_slug, sub, rest);
            match tokio::fs::metadata(&rp).await {
                Ok(meta) => xml.push_str(&prop_response(href_path, &meta, &rp, false)),
                Err(_) => {
                    return Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::empty())
                        .unwrap();
                }
            }
            xml.push_str("</D:multistatus>");
            return multistatus_response(xml);
        }
    }

    // Children (depth > 0)
    if depth != "0" {
        match &vpath {
            VPath::Root => {
                for org in orgs {
                    let child_href = format!(
                        "{}{}/",
                        href_path.trim_end_matches('/'),
                        urlencoding::encode(&org.slug)
                    );
                    xml.push_str(&virt_collection_response(&child_href, &org.name));
                }
            }
            VPath::OrgRoot { org_slug } => {
                if let Some(org) = orgs.iter().find(|o| o.slug == *org_slug) {
                    let projects = projects_for_org(pool, &org.id).await;
                    for proj in &projects {
                        let child_href = format!(
                            "{}{}/",
                            href_path.trim_end_matches('/'),
                            urlencoding::encode(&proj.slug)
                        );
                        xml.push_str(&virt_collection_response(&child_href, &proj.slug));
                    }
                }
            }
            VPath::ProjRoot { proj_slug, .. } => {
                for sub in PROJECT_SUBDIRS {
                    let rp = real_path(proj_slug, sub, "");
                    if rp.exists() {
                        let child_href = format!("{}{}/", href_path.trim_end_matches('/'), sub);
                        xml.push_str(&virt_collection_response(&child_href, sub));
                    }
                }
            }
            VPath::SubDir { proj_slug, sub, .. } => {
                let rp = real_path(proj_slug, sub, "");
                if let Ok(mut entries) = tokio::fs::read_dir(&rp).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') {
                            continue;
                        }
                        if let Ok(meta) = entry.metadata().await {
                            let child_href = if href_path.ends_with('/') {
                                format!("{}{}", href_path, urlencoding::encode(&name))
                            } else {
                                format!("{}/{}", href_path, urlencoding::encode(&name))
                            };
                            xml.push_str(&prop_response(&child_href, &meta, &entry.path(), false));
                        }
                    }
                }
            }
            VPath::SubFile { .. } => {} // handled above
        }
    }

    xml.push_str("</D:multistatus>");
    multistatus_response(xml)
}

// ── GET / HEAD (virtual — user) ───────────────────────────────────────────────

async fn handle_get_virtual(
    vpath: VPath<'_>,
    orgs: &[OrgEntry],
    pool: &SqlitePool,
    req: Request,
) -> Response {
    match vpath {
        VPath::SubFile {
            org_slug,
            proj_slug,
            sub,
            rest,
        } => {
            if !orgs.iter().any(|o| o.slug == org_slug) {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap();
            }
            let _ = pool; // pool available for future artifact ownership checks
            let rp = real_path(proj_slug, sub, rest);
            handle_get(req, &rp, false).await
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    }
}

async fn handle_get_virtual_head(
    vpath: VPath<'_>,
    orgs: &[OrgEntry],
    pool: &SqlitePool,
) -> Response {
    match vpath {
        VPath::SubFile {
            org_slug,
            proj_slug,
            sub,
            rest,
        } => {
            if !orgs.iter().any(|o| o.slug == org_slug) {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap();
            }
            let _ = pool;
            let rp = real_path(proj_slug, sub, rest);
            // Minimal HEAD response
            match tokio::fs::metadata(&rp).await {
                Ok(m) => {
                    let ct = mime_guess::from_path(&rp)
                        .first_or_octet_stream()
                        .to_string();
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, ct)
                        .header(header::CONTENT_LENGTH, m.len())
                        .header(header::ACCEPT_RANGES, "bytes")
                        .body(Body::empty())
                        .unwrap()
                }
                Err(_) => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap(),
            }
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    }
}

// ── GET / HEAD (filesystem — admin) ──────────────────────────────────────────

async fn handle_get(req: Request, local_path: &std::path::Path, head_only: bool) -> Response {
    if !local_path.is_file() {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap();
    }

    let file_size = match tokio::fs::metadata(local_path).await {
        Ok(m) => m.len(),
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
        }
    };

    let content_type = mime_guess::from_path(local_path)
        .first_or_octet_stream()
        .to_string();

    let range = req
        .headers()
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("bytes="))
        .and_then(|s| {
            let mut parts = s.splitn(2, '-');
            let start: u64 = parts.next()?.parse().ok()?;
            let end: u64 = parts
                .next()
                .and_then(|e| if e.is_empty() { None } else { e.parse().ok() })
                .unwrap_or(file_size.saturating_sub(1));
            Some((start, end))
        });

    if head_only {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, &content_type)
            .header(header::CONTENT_LENGTH, file_size)
            .header(header::ACCEPT_RANGES, "bytes")
            .body(Body::empty())
            .unwrap();
    }

    if let Some((start, end)) = range {
        let end = end.min(file_size.saturating_sub(1));
        if start > end || start >= file_size {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{}", file_size))
                .body(Body::empty())
                .unwrap();
        }
        let length = end - start + 1;
        match File::open(local_path).await {
            Ok(mut file) => {
                let _ = file.seek(std::io::SeekFrom::Start(start)).await;
                let stream = ReaderStream::new(file.take(length));
                Response::builder()
                    .status(StatusCode::PARTIAL_CONTENT)
                    .header(header::CONTENT_TYPE, &content_type)
                    .header(header::CONTENT_LENGTH, length)
                    .header(
                        header::CONTENT_RANGE,
                        format!("bytes {}-{}/{}", start, end, file_size),
                    )
                    .header(header::ACCEPT_RANGES, "bytes")
                    .body(Body::from_stream(stream))
                    .unwrap()
            }
            Err(_) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap(),
        }
    } else {
        match File::open(local_path).await {
            Ok(file) => {
                let stream = ReaderStream::new(file);
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, &content_type)
                    .header(header::CONTENT_LENGTH, file_size)
                    .header(header::ACCEPT_RANGES, "bytes")
                    .body(Body::from_stream(stream))
                    .unwrap()
            }
            Err(_) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap(),
        }
    }
}

// ── XML helpers ───────────────────────────────────────────────────────────────

fn multistatus_response(xml: String) -> Response {
    Response::builder()
        .status(207)
        .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
        .header("DAV", "1, 2")
        .body(Body::from(xml))
        .unwrap()
}

/// Synthetic collection response for virtual directories
fn virt_collection_response(href: &str, display_name: &str) -> String {
    let href_display = if href.ends_with('/') {
        href.to_string()
    } else {
        format!("{}/", href)
    };
    format!(
        r#"<D:response><D:href>{href}</D:href><D:propstat><D:prop><D:displayname>{name}</D:displayname><D:resourcetype><D:collection/></D:resourcetype><D:getcontentlength>0</D:getcontentlength><D:getcontenttype>httpd/unix-directory</D:getcontenttype><D:getlastmodified>Thu, 01 Jan 2026 00:00:00 GMT</D:getlastmodified></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>"#,
        href = escape_xml(&href_display),
        name = escape_xml(display_name),
    )
}

fn prop_response(
    href: &str,
    meta: &std::fs::Metadata,
    path: &std::path::Path,
    is_root: bool,
) -> String {
    let is_dir = meta.is_dir();
    let size = meta.len();
    let name = if is_root {
        APN_VOLUME_NAME.to_string()
    } else {
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| APN_VOLUME_NAME.to_string())
    };

    let modified = meta
        .modified()
        .ok()
        .and_then(|t| {
            let secs = t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
            chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0)
                .map(|dt| dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
        })
        .unwrap_or_else(|| "Thu, 01 Jan 1970 00:00:00 GMT".to_string());

    let content_type = if is_dir {
        "httpd/unix-directory".to_string()
    } else {
        mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string()
    };

    let resource_type = if is_dir {
        "<D:resourcetype><D:collection/></D:resourcetype>"
    } else {
        "<D:resourcetype/>"
    };

    let href_display = if is_dir && !href.ends_with('/') {
        format!("{}/", href)
    } else {
        href.to_string()
    };

    format!(
        r#"<D:response><D:href>{href}</D:href><D:propstat><D:prop><D:displayname>{name}</D:displayname>{resource_type}<D:getcontentlength>{size}</D:getcontentlength><D:getcontenttype>{ct}</D:getcontenttype><D:getlastmodified>{modified}</D:getlastmodified></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>"#,
        href = escape_xml(&href_display),
        name = escape_xml(&name),
        size = size,
        ct = escape_xml(&content_type),
        modified = modified,
    )
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
