use crate::database::{lock_conn, Database};
use crate::error::AppError;
use chrono::{Duration, Local, LocalResult, TimeZone};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardRange {
    Today,
    SevenDays,
    ThirtyDays,
}

impl LeaderboardRange {
    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "today" => Ok(Self::Today),
            "7d" => Ok(Self::SevenDays),
            "30d" => Ok(Self::ThirtyDays),
            other => Err(AppError::InvalidInput(format!("不支持的排行周期: {other}"))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::SevenDays => "7d",
            Self::ThirtyDays => "30d",
        }
    }

    fn window(self) -> (i64, i64) {
        let now = Local::now();
        let end = now.timestamp();
        let start_of_day = |days_ago: i64| {
            let base = now - Duration::days(days_ago);
            let Some(start_naive) = base.date_naive().and_hms_opt(0, 0, 0) else {
                return end;
            };
            match Local.from_local_datetime(&start_naive) {
                LocalResult::Single(dt) => dt.timestamp(),
                LocalResult::Ambiguous(dt, _) => dt.timestamp(),
                LocalResult::None => (base - Duration::hours(24)).timestamp(),
            }
        };
        match self {
            Self::Today => (start_of_day(0), end),
            Self::SevenDays => (start_of_day(6), end),
            Self::ThirtyDays => (start_of_day(29), end),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardProfile {
    pub user_id: String,
    pub github_login: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub opted_in: bool,
    pub joined_at: i64,
    #[serde(skip_serializing, skip_deserializing)]
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub user_id: String,
    pub github_login: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub request_count: i64,
    pub total_cost_usd: String,
    pub is_current_user: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntriesResult {
    pub range: String,
    pub updated_at: Option<i64>,
    pub entries: Vec<LeaderboardEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubDeviceLoginStart {
    pub session_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_at: i64,
    pub interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubDeviceLoginStatus {
    pub status: String,
    pub profile: Option<LeaderboardProfile>,
    pub retry_after_seconds: Option<u64>,
    pub message: Option<String>,
    #[serde(skip_serializing, skip_deserializing)]
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GithubAuthSession {
    pub id: String,
    pub device_code: String,
    pub expires_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct GithubDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: i64,
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GithubTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GithubUserResponse {
    id: u64,
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_URL: &str = "https://api.github.com/user";
const GITHUB_DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const DEFAULT_GITHUB_OAUTH_CLIENT_ID: &str = "Ov23li3Cs7WS1Fpu523D";
const DEFAULT_BACKEND_BASE_URL: &str = "https://token-use.lucsun.cn";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackendSnapshotPayload {
    range: String,
    window_start: i64,
    window_end: i64,
    total_tokens: i64,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
    request_count: i64,
    total_cost_usd: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackendLoginStatus {
    status: String,
    profile: Option<LeaderboardProfile>,
    auth_token: Option<String>,
    retry_after_seconds: Option<u64>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackendProfileResponse {
    profile: LeaderboardProfile,
}

fn backend_base_url() -> String {
    std::env::var("TOKEN_USE_BACKEND_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_BACKEND_BASE_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn github_oauth_client_id() -> Result<String, AppError> {
    std::env::var("TOKEN_USE_GITHUB_CLIENT_ID")
        .or_else(|_| std::env::var("TOKEN_MONITOR_GITHUB_CLIENT_ID"))
        .or_else(|_| {
            option_env!("TOKEN_USE_GITHUB_CLIENT_ID")
                .map(str::to_string)
                .ok_or(std::env::VarError::NotPresent)
        })
        .ok()
        .or_else(|| Some(DEFAULT_GITHUB_OAUTH_CLIENT_ID.to_string()))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            AppError::localized(
                "leaderboard.githubClientIdNotConfigured",
                "GitHub 登录尚未配置 Client ID。请先配置 TOKEN_USE_GITHUB_CLIENT_ID。",
                "GitHub login is missing a Client ID. Configure TOKEN_USE_GITHUB_CLIENT_ID first.",
            )
        })
}

fn github_http_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .user_agent("Token Use")
        .build()
        .map_err(|e| AppError::Message(format!("创建 GitHub HTTP 客户端失败: {e}")))
}

async fn parse_json_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    context: &str,
) -> Result<T, AppError> {
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<unreadable body>".to_string());
    if !status.is_success() {
        return Err(AppError::HttpStatus {
            status: status.as_u16(),
            body,
        });
    }

    serde_json::from_str(&body)
        .map_err(|e| AppError::Message(format!("{context}: {e}; body={body}")))
}

fn backend_auth_token(profile: &LeaderboardProfile) -> Result<&str, AppError> {
    profile
        .auth_token
        .as_deref()
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| {
            AppError::InvalidInput("排行登录已失效，请重新使用 GitHub 登录。".to_string())
        })
}

pub async fn start_github_device_login(db: &Database) -> Result<GithubDeviceLoginStart, AppError> {
    let backend = backend_base_url();
    let client = github_http_client()?;
    let response = client
        .post(format!("{backend}/api/github/device/start"))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Message(format!("请求 Token Use 后端登录会话失败: {e}")))?;
    let remote_login: GithubDeviceLoginStart =
        parse_json_response(response, "解析 Token Use 后端登录会话失败").await?;
    let local_session_id = db.create_github_auth_session(
        &format!("backend:{}", remote_login.session_id),
        remote_login.expires_at,
    )?;
    return Ok(GithubDeviceLoginStart {
        session_id: local_session_id,
        ..remote_login
    });

    #[allow(unreachable_code)]
    {
        let client_id = github_oauth_client_id()?;
        let client = github_http_client()?;
        let response = client
            .post(GITHUB_DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .form(&[("client_id", client_id.as_str()), ("scope", "read:user")])
            .send()
            .await
            .map_err(|e| AppError::Message(format!("请求 GitHub 设备验证码失败: {e}")))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<unreadable body>".to_string());
        if !status.is_success() {
            return Err(AppError::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }

        let device: GithubDeviceCodeResponse = serde_json::from_str(&body).map_err(|e| {
            AppError::Message(format!("解析 GitHub 设备验证码响应失败: {e}; body={body}"))
        })?;
        let now = Local::now().timestamp();
        let expires_at = now + device.expires_in;
        let session_id = db.create_github_auth_session(&device.device_code, expires_at)?;

        Ok(GithubDeviceLoginStart {
            session_id,
            user_code: device.user_code,
            verification_uri: device.verification_uri,
            expires_at,
            interval_seconds: device.interval.unwrap_or(5),
        })
    }
}

pub async fn poll_github_device_login(
    db: &Database,
    session_id: &str,
) -> Result<GithubDeviceLoginStatus, AppError> {
    let session = db.get_github_auth_session(session_id)?;
    if session.completed_at.is_some() {
        return Ok(GithubDeviceLoginStatus {
            status: "completed".to_string(),
            profile: db.get_leaderboard_profile()?,
            retry_after_seconds: None,
            message: None,
            auth_token: None,
        });
    }

    let now = Local::now().timestamp();
    if session.expires_at <= now {
        db.fail_github_auth_session(&session.id, "expired")?;
        return Ok(GithubDeviceLoginStatus {
            status: "expired".to_string(),
            profile: None,
            retry_after_seconds: None,
            message: Some("GitHub 验证码已过期，请重新登录。".to_string()),
            auth_token: None,
        });
    }

    if let Some(remote_session_id) = session.device_code.strip_prefix("backend:") {
        let backend = backend_base_url();
        let client = github_http_client()?;
        let response = client
            .post(format!("{backend}/api/github/device/poll"))
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "sessionId": remote_session_id }))
            .send()
            .await
            .map_err(|e| AppError::Message(format!("轮询 Token Use 后端登录状态失败: {e}")))?;
        let backend_status: BackendLoginStatus =
            parse_json_response(response, "解析 Token Use 后端登录状态失败").await?;

        if matches!(backend_status.status.as_str(), "authorized" | "completed") {
            let Some(remote_profile) = backend_status.profile else {
                return Err(AppError::Message(
                    "Token Use 后端登录成功但缺少用户资料".to_string(),
                ));
            };
            let auth_token = backend_status.auth_token.as_deref();
            let profile = db.upsert_leaderboard_profile(
                &remote_profile.user_id,
                &remote_profile.github_login,
                &remote_profile.display_name,
                remote_profile.avatar_url.as_deref(),
                remote_profile.opted_in,
                auth_token,
            )?;
            db.complete_github_auth_session(&session.id)?;
            return Ok(GithubDeviceLoginStatus {
                status: backend_status.status,
                profile: Some(profile),
                retry_after_seconds: backend_status.retry_after_seconds,
                message: backend_status.message,
                auth_token: None,
            });
        }

        if matches!(backend_status.status.as_str(), "expired" | "denied") {
            db.fail_github_auth_session(&session.id, &backend_status.status)?;
        }

        return Ok(GithubDeviceLoginStatus {
            status: backend_status.status,
            profile: None,
            retry_after_seconds: backend_status.retry_after_seconds,
            message: backend_status.message,
            auth_token: None,
        });
    }

    let client_id = github_oauth_client_id()?;
    let client = github_http_client()?;
    let response = client
        .post(GITHUB_ACCESS_TOKEN_URL)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("device_code", session.device_code.as_str()),
            ("grant_type", GITHUB_DEVICE_GRANT_TYPE),
        ])
        .send()
        .await
        .map_err(|e| AppError::Message(format!("轮询 GitHub 授权状态失败: {e}")))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<unreadable body>".to_string());
    if !status.is_success() {
        return Err(AppError::HttpStatus {
            status: status.as_u16(),
            body,
        });
    }

    let token: GithubTokenResponse = serde_json::from_str(&body).map_err(|e| {
        AppError::Message(format!("解析 GitHub 授权状态响应失败: {e}; body={body}"))
    })?;

    if let Some(error) = token.error.as_deref() {
        return match error {
            "authorization_pending" => Ok(GithubDeviceLoginStatus {
                status: "pending".to_string(),
                profile: None,
                retry_after_seconds: token.interval.or(Some(5)),
                message: None,
                auth_token: None,
            }),
            "slow_down" => Ok(GithubDeviceLoginStatus {
                status: "pending".to_string(),
                profile: None,
                retry_after_seconds: token.interval.or(Some(10)),
                message: token.error_description,
                auth_token: None,
            }),
            "expired_token" | "token_expired" => {
                db.fail_github_auth_session(&session.id, error)?;
                Ok(GithubDeviceLoginStatus {
                    status: "expired".to_string(),
                    profile: None,
                    retry_after_seconds: None,
                    message: Some("GitHub 验证码已过期，请重新登录。".to_string()),
                    auth_token: None,
                })
            }
            "access_denied" => {
                db.fail_github_auth_session(&session.id, error)?;
                Ok(GithubDeviceLoginStatus {
                    status: "denied".to_string(),
                    profile: None,
                    retry_after_seconds: None,
                    message: Some("GitHub 授权已取消。".to_string()),
                    auth_token: None,
                })
            }
            other => {
                db.fail_github_auth_session(&session.id, other)?;
                Err(AppError::Message(format!(
                    "GitHub 授权失败: {}",
                    token.error_description.unwrap_or_else(|| other.to_string())
                )))
            }
        };
    }

    let access_token = token
        .access_token
        .ok_or_else(|| AppError::Message("GitHub 响应缺少 access_token".to_string()))?;
    let github_user = fetch_github_user(&client, &access_token).await?;
    let user_id = format!("github:{}", github_user.id);
    let display_name = github_user
        .name
        .unwrap_or_else(|| github_user.login.clone());
    let previous_opt_in = db
        .get_leaderboard_profile()?
        .filter(|profile| profile.user_id == user_id)
        .map(|profile| profile.opted_in)
        .unwrap_or(false);

    let profile = db.upsert_leaderboard_profile(
        &user_id,
        &github_user.login,
        &display_name,
        github_user.avatar_url.as_deref(),
        previous_opt_in,
        None,
    )?;
    db.complete_github_auth_session(&session.id)?;

    Ok(GithubDeviceLoginStatus {
        status: "authorized".to_string(),
        profile: Some(profile),
        retry_after_seconds: None,
        message: None,
        auth_token: None,
    })
}

async fn fetch_github_user(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<GithubUserResponse, AppError> {
    let response = client
        .get(GITHUB_USER_URL)
        .bearer_auth(access_token)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| AppError::Message(format!("读取 GitHub 用户资料失败: {e}")))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<unreadable body>".to_string());
    if !status.is_success() {
        return Err(AppError::HttpStatus {
            status: status.as_u16(),
            body,
        });
    }

    serde_json::from_str(&body)
        .map_err(|e| AppError::Message(format!("解析 GitHub 用户资料失败: {e}; body={body}")))
}

pub async fn set_remote_leaderboard_opt_in(
    db: &Database,
    opted_in: bool,
) -> Result<LeaderboardProfile, AppError> {
    let current = db
        .get_leaderboard_profile()?
        .ok_or_else(|| AppError::InvalidInput("请先使用 GitHub 登录".to_string()))?;
    let auth_token = backend_auth_token(&current)?;
    let backend = backend_base_url();
    let client = github_http_client()?;
    let response = client
        .post(format!("{backend}/api/me/opt-in"))
        .bearer_auth(auth_token)
        .header("Accept", "application/json")
        .json(&serde_json::json!({ "optedIn": opted_in }))
        .send()
        .await
        .map_err(|e| AppError::Message(format!("更新 Token Use 后端排行加入状态失败: {e}")))?;
    let remote: BackendProfileResponse =
        parse_json_response(response, "解析 Token Use 后端用户状态失败").await?;

    db.upsert_leaderboard_profile(
        &remote.profile.user_id,
        &remote.profile.github_login,
        &remote.profile.display_name,
        remote.profile.avatar_url.as_deref(),
        remote.profile.opted_in,
        Some(auth_token),
    )
}

pub async fn get_remote_leaderboard_entries(
    db: &Database,
    range: LeaderboardRange,
) -> Result<LeaderboardEntriesResult, AppError> {
    let Some(profile) = db.get_leaderboard_profile()? else {
        return Ok(LeaderboardEntriesResult {
            range: range.as_str().to_string(),
            updated_at: None,
            entries: Vec::new(),
        });
    };
    if !profile.opted_in {
        return Ok(LeaderboardEntriesResult {
            range: range.as_str().to_string(),
            updated_at: None,
            entries: Vec::new(),
        });
    }

    let auth_token = backend_auth_token(&profile)?;
    let (window_start, window_end) = range.window();
    let snapshot = db.calculate_local_snapshot(window_start, window_end)?;
    let payload = BackendSnapshotPayload {
        range: range.as_str().to_string(),
        window_start,
        window_end,
        total_tokens: snapshot.total_tokens(),
        input_tokens: snapshot.input_tokens,
        output_tokens: snapshot.output_tokens,
        cache_read_tokens: snapshot.cache_read_tokens,
        cache_creation_tokens: snapshot.cache_creation_tokens,
        request_count: snapshot.request_count,
        total_cost_usd: snapshot.total_cost_usd,
    };

    let backend = backend_base_url();
    let client = github_http_client()?;
    let upload = client
        .post(format!("{backend}/api/leaderboard/snapshot"))
        .bearer_auth(auth_token)
        .header("Accept", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::Message(format!("上报 Token Use 排行快照失败: {e}")))?;
    let _: serde_json::Value =
        parse_json_response(upload, "解析 Token Use 排行快照响应失败").await?;

    let response = client
        .get(format!("{backend}/api/leaderboard"))
        .bearer_auth(auth_token)
        .query(&[("range", range.as_str())])
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Message(format!("读取 Token Use 远端排行榜失败: {e}")))?;

    parse_json_response(response, "解析 Token Use 远端排行榜失败").await
}

#[derive(Debug)]
struct LocalSnapshot {
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
    request_count: i64,
    total_cost_usd: String,
}

impl LocalSnapshot {
    fn total_tokens(&self) -> i64 {
        self.input_tokens + self.output_tokens + self.cache_read_tokens + self.cache_creation_tokens
    }
}

impl Database {
    pub fn get_leaderboard_profile(&self) -> Result<Option<LeaderboardProfile>, AppError> {
        let conn = lock_conn!(self.conn);
        conn.query_row(
            "SELECT user_id, github_login, display_name, avatar_url, opted_in, joined_at, auth_token
             FROM ranking_profile
             WHERE is_current = 1
             ORDER BY updated_at DESC
             LIMIT 1",
            [],
            |row| {
                Ok(LeaderboardProfile {
                    user_id: row.get(0)?,
                    github_login: row.get(1)?,
                    display_name: row.get(2)?,
                    avatar_url: row.get(3)?,
                    opted_in: row.get::<_, i64>(4)? != 0,
                    joined_at: row.get(5)?,
                    auth_token: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(AppError::from)
    }

    pub fn upsert_leaderboard_profile(
        &self,
        user_id: &str,
        github_login: &str,
        display_name: &str,
        avatar_url: Option<&str>,
        opted_in: bool,
        auth_token: Option<&str>,
    ) -> Result<LeaderboardProfile, AppError> {
        let now = Local::now().timestamp();
        let conn = lock_conn!(self.conn);
        conn.execute("UPDATE ranking_profile SET is_current = 0", [])?;
        conn.execute(
            "INSERT INTO ranking_profile (
                user_id, github_login, display_name, avatar_url, auth_token, opted_in, is_current, joined_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, COALESCE(?5, (SELECT auth_token FROM ranking_profile WHERE user_id = ?1)), ?6, 1, ?7, ?7)
            ON CONFLICT(user_id) DO UPDATE SET
                github_login = excluded.github_login,
                display_name = excluded.display_name,
                avatar_url = excluded.avatar_url,
                auth_token = COALESCE(excluded.auth_token, ranking_profile.auth_token),
                opted_in = excluded.opted_in,
                is_current = 1,
                updated_at = excluded.updated_at",
            params![
                user_id,
                github_login,
                display_name,
                avatar_url,
                auth_token,
                if opted_in { 1 } else { 0 },
                now
            ],
        )?;
        drop(conn);

        self.get_leaderboard_profile()?
            .ok_or_else(|| AppError::Database("写入排行用户档案后无法读取".to_string()))
    }

    pub fn set_leaderboard_opt_in(&self, opted_in: bool) -> Result<LeaderboardProfile, AppError> {
        let profile = self
            .get_leaderboard_profile()?
            .ok_or_else(|| AppError::InvalidInput("请先使用 GitHub 登录".to_string()))?;
        let now = Local::now().timestamp();
        {
            let conn = lock_conn!(self.conn);
            conn.execute(
                "UPDATE ranking_profile SET opted_in = ?1, updated_at = ?2 WHERE user_id = ?3",
                params![if opted_in { 1 } else { 0 }, now, profile.user_id],
            )?;
        }
        self.get_leaderboard_profile()?
            .ok_or_else(|| AppError::Database("更新排行加入状态后无法读取用户档案".to_string()))
    }

    pub fn sign_out_leaderboard(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("DELETE FROM ranking_profile", [])?;
        conn.execute("DELETE FROM ranking_auth_session", [])?;
        Ok(())
    }

    pub fn create_github_auth_session(
        &self,
        device_code: &str,
        expires_at: i64,
    ) -> Result<String, AppError> {
        let now = Local::now().timestamp();
        let id = Uuid::new_v4().to_string();
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO ranking_auth_session (id, state, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, device_code, now, expires_at],
        )?;
        Ok(id)
    }

    pub fn get_github_auth_session(&self, session_id: &str) -> Result<GithubAuthSession, AppError> {
        let conn = lock_conn!(self.conn);
        conn.query_row(
            "SELECT id, state, expires_at, completed_at
             FROM ranking_auth_session
             WHERE id = ?1",
            params![session_id],
            |row| {
                Ok(GithubAuthSession {
                    id: row.get(0)?,
                    device_code: row.get(1)?,
                    expires_at: row.get(2)?,
                    completed_at: row.get(3)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::InvalidInput("GitHub 登录会话不存在或已失效".to_string()))
    }

    pub fn complete_github_auth_session(&self, session_id: &str) -> Result<(), AppError> {
        let now = Local::now().timestamp();
        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE ranking_auth_session SET completed_at = ?1, error = NULL WHERE id = ?2",
            params![now, session_id],
        )?;
        Ok(())
    }

    pub fn fail_github_auth_session(&self, session_id: &str, error: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "UPDATE ranking_auth_session SET error = ?1 WHERE id = ?2",
            params![error, session_id],
        )?;
        Ok(())
    }

    pub fn get_leaderboard_entries(
        &self,
        range: LeaderboardRange,
    ) -> Result<LeaderboardEntriesResult, AppError> {
        let profile = self.get_leaderboard_profile()?;
        let Some(profile) = profile else {
            return Ok(LeaderboardEntriesResult {
                range: range.as_str().to_string(),
                updated_at: None,
                entries: Vec::new(),
            });
        };

        if !profile.opted_in {
            return Ok(LeaderboardEntriesResult {
                range: range.as_str().to_string(),
                updated_at: None,
                entries: Vec::new(),
            });
        }

        let (window_start, window_end) = range.window();
        let snapshot = self.calculate_local_snapshot(window_start, window_end)?;
        self.upsert_local_snapshot(&profile.user_id, range, window_start, window_end, &snapshot)?;
        self.query_snapshot_entries(&profile.user_id, range, window_start, window_end)
    }

    fn calculate_local_snapshot(
        &self,
        window_start: i64,
        window_end: i64,
    ) -> Result<LocalSnapshot, AppError> {
        let summary = self.get_usage_summary(Some(window_start), Some(window_end), None)?;
        Ok(LocalSnapshot {
            input_tokens: summary.total_input_tokens as i64,
            output_tokens: summary.total_output_tokens as i64,
            cache_read_tokens: summary.total_cache_read_tokens as i64,
            cache_creation_tokens: summary.total_cache_creation_tokens as i64,
            request_count: summary.total_requests as i64,
            total_cost_usd: summary.total_cost,
        })
    }

    fn upsert_local_snapshot(
        &self,
        user_id: &str,
        range: LeaderboardRange,
        window_start: i64,
        window_end: i64,
        snapshot: &LocalSnapshot,
    ) -> Result<(), AppError> {
        let now = Local::now().timestamp();
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO ranking_snapshots (
                user_id, range_key, window_start, window_end,
                total_tokens, input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                request_count, total_cost_usd, snapshot_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(user_id, range_key, window_start, window_end) DO UPDATE SET
                total_tokens = excluded.total_tokens,
                input_tokens = excluded.input_tokens,
                output_tokens = excluded.output_tokens,
                cache_read_tokens = excluded.cache_read_tokens,
                cache_creation_tokens = excluded.cache_creation_tokens,
                request_count = excluded.request_count,
                total_cost_usd = excluded.total_cost_usd,
                snapshot_at = excluded.snapshot_at",
            params![
                user_id,
                range.as_str(),
                window_start,
                window_end,
                snapshot.total_tokens(),
                snapshot.input_tokens,
                snapshot.output_tokens,
                snapshot.cache_read_tokens,
                snapshot.cache_creation_tokens,
                snapshot.request_count,
                snapshot.total_cost_usd,
                now
            ],
        )?;
        Ok(())
    }

    fn query_snapshot_entries(
        &self,
        current_user_id: &str,
        range: LeaderboardRange,
        window_start: i64,
        window_end: i64,
    ) -> Result<LeaderboardEntriesResult, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn.prepare(
            "SELECT
                s.user_id,
                p.github_login,
                p.display_name,
                p.avatar_url,
                s.total_tokens,
                s.input_tokens,
                s.output_tokens,
                s.cache_read_tokens,
                s.cache_creation_tokens,
                s.request_count,
                s.total_cost_usd,
                s.snapshot_at
             FROM ranking_snapshots s
             JOIN ranking_profile p ON p.user_id = s.user_id
             WHERE s.range_key = ?1
               AND s.window_start = ?2
               AND s.window_end = ?3
               AND p.opted_in = 1
             ORDER BY s.total_tokens DESC, s.request_count DESC, p.github_login ASC
             LIMIT 100",
        )?;

        let rows = stmt.query_map(params![range.as_str(), window_start, window_end], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, i64>(11)?,
            ))
        })?;

        let mut entries = Vec::new();
        let mut updated_at: Option<i64> = None;
        for (index, row) in rows.enumerate() {
            let (
                user_id,
                github_login,
                display_name,
                avatar_url,
                total_tokens,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_creation_tokens,
                request_count,
                total_cost_usd,
                snapshot_at,
            ) = row?;
            updated_at = Some(updated_at.map_or(snapshot_at, |v| v.max(snapshot_at)));
            entries.push(LeaderboardEntry {
                rank: index as i64 + 1,
                is_current_user: user_id == current_user_id,
                user_id,
                github_login,
                display_name,
                avatar_url,
                total_tokens,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_creation_tokens,
                request_count,
                total_cost_usd,
            });
        }

        Ok(LeaderboardEntriesResult {
            range: range.as_str().to_string(),
            updated_at,
            entries,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::lock_conn;

    #[test]
    fn leaderboard_entries_require_opt_in() -> Result<(), AppError> {
        let db = Database::memory()?;
        db.upsert_leaderboard_profile("u1", "alice", "Alice", None, false, None)?;

        let result = db.get_leaderboard_entries(LeaderboardRange::Today)?;

        assert!(result.entries.is_empty());
        Ok(())
    }

    #[test]
    fn leaderboard_entry_uses_total_token_consumption() -> Result<(), AppError> {
        let db = Database::memory()?;
        let range = LeaderboardRange::Today;
        let (window_start, window_end) = range.window();
        db.upsert_leaderboard_profile("u1", "alice", "Alice", None, true, None)?;

        {
            let conn = lock_conn!(db.conn);
            conn.execute(
                "INSERT INTO proxy_request_logs (
                    request_id, provider_id, app_type, model,
                    input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                    total_cost_usd, latency_ms, status_code, created_at
                ) VALUES (?1, 'p1', 'claude', 'claude-sonnet', 100, 50, 20, 10, '0.010000', 100, 200, ?2)",
                params!["req-local", window_end],
            )?;
            conn.execute(
                "INSERT INTO ranking_profile (
                    user_id, github_login, display_name, avatar_url, opted_in, is_current, joined_at, updated_at
                ) VALUES ('u2', 'bob', 'Bob', NULL, 1, 0, ?1, ?1)",
                params![window_start],
            )?;
            conn.execute(
                "INSERT INTO ranking_snapshots (
                    user_id, range_key, window_start, window_end, total_tokens,
                    input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                    request_count, total_cost_usd, snapshot_at
                ) VALUES ('u2', ?1, ?2, ?3, 1000, 500, 400, 50, 50, 9, '0.500000', ?3)",
                params![range.as_str(), window_start, window_end],
            )?;
        }

        let result = db.get_leaderboard_entries(range)?;

        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.entries[0].github_login, "bob");
        assert_eq!(result.entries[0].rank, 1);
        assert_eq!(result.entries[1].github_login, "alice");
        assert_eq!(result.entries[1].rank, 2);
        assert_eq!(result.entries[1].total_tokens, 180);
        assert!(result.entries[1].is_current_user);
        Ok(())
    }

    #[test]
    fn leaderboard_range_windows_match_usage_presets() {
        let now = Local::now();
        let expected_start = |days_ago: i64| {
            let base = now - Duration::days(days_ago);
            let start_naive = base
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .expect("valid local day start");
            match Local.from_local_datetime(&start_naive) {
                LocalResult::Single(dt) => dt.timestamp(),
                LocalResult::Ambiguous(dt, _) => dt.timestamp(),
                LocalResult::None => (base - Duration::hours(24)).timestamp(),
            }
        };

        let (today_start, _) = LeaderboardRange::Today.window();
        let (seven_start, _) = LeaderboardRange::SevenDays.window();
        let (thirty_start, _) = LeaderboardRange::ThirtyDays.window();

        assert_eq!(today_start, expected_start(0));
        assert_eq!(seven_start, expected_start(6));
        assert_eq!(thirty_start, expected_start(29));
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_usage_log(
        db: &Database,
        request_id: &str,
        app_type: &str,
        provider_id: &str,
        model: &str,
        data_source: &str,
        created_at: i64,
        input_tokens: i64,
        output_tokens: i64,
        cache_read_tokens: i64,
        cache_creation_tokens: i64,
        total_cost_usd: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(db.conn);
        conn.execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model, request_model,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd,
                total_cost_usd, latency_ms, status_code, created_at, data_source
            ) VALUES (?1, ?2, ?3, ?4, ?4, ?5, ?6, ?7, ?8, '0', '0', '0', '0', ?9, 100, 200, ?10, ?11)",
            params![
                request_id,
                provider_id,
                app_type,
                model,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_creation_tokens,
                total_cost_usd,
                created_at,
                data_source
            ],
        )?;
        Ok(())
    }

    #[test]
    fn leaderboard_snapshot_matches_usage_summary_effective_tokens() -> Result<(), AppError> {
        let db = Database::memory()?;
        let range = LeaderboardRange::Today;
        let (_window_start, window_end) = range.window();
        db.upsert_leaderboard_profile("u1", "alice", "Alice", None, true, None)?;

        insert_usage_log(
            &db,
            "codex-proxy",
            "codex",
            "openai",
            "gpt-5.4",
            "proxy",
            window_end - 120,
            100,
            20,
            10,
            7,
            "0.100000",
        )?;
        insert_usage_log(
            &db,
            "codex-session-dup",
            "codex",
            "_codex_session",
            "gpt-5.4",
            "codex_session",
            window_end - 60,
            100,
            20,
            10,
            0,
            "0.100000",
        )?;
        insert_usage_log(
            &db,
            "gemini-proxy",
            "gemini",
            "google",
            "gemini-2.5-pro",
            "proxy",
            window_end - 120,
            200,
            40,
            30,
            0,
            "0.200000",
        )?;
        insert_usage_log(
            &db,
            "gemini-session-dup",
            "gemini",
            "_gemini_session",
            "gemini-2.5-pro",
            "gemini_session",
            window_end - 60,
            200,
            40,
            30,
            0,
            "0.200000",
        )?;
        insert_usage_log(
            &db,
            "claude-proxy",
            "claude",
            "anthropic",
            "claude-sonnet-4-5",
            "proxy",
            window_end - 90,
            300,
            60,
            20,
            5,
            "0.300000",
        )?;

        let summary = db.get_usage_summary(Some(window_end - 300), Some(window_end), None)?;
        let result = db.get_leaderboard_entries(range)?;
        let current = result
            .entries
            .iter()
            .find(|entry| entry.is_current_user)
            .expect("current user should appear in leaderboard");

        assert_eq!(summary.real_total_tokens, 752);
        assert_eq!(current.total_tokens as u64, summary.real_total_tokens);
        assert_eq!(current.request_count as u64, summary.total_requests);
        Ok(())
    }
}
