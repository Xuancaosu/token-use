//! 托盘菜单管理模块
//!
//! 负责系统托盘图标和菜单的创建、更新和事件处理。

use once_cell::sync::Lazy;
use tauri::menu::{Menu, MenuBuilder, MenuItem, Submenu};
use tauri::Manager;

use crate::app_config::AppType;
use crate::error::AppError;
use crate::store::AppState;
use chrono::{Local, LocalResult, TimeZone};

/// 每个 app 分区的子菜单句柄，用于 usage 更新时就地改 label 而非整菜单重建。
/// `create_tray_menu` 每次重建都会整表覆盖写入，保证句柄始终指向当前活跃菜单。
static TRAY_SECTION_SUBMENUS: Lazy<
    std::sync::Mutex<std::collections::HashMap<AppType, Submenu<tauri::Wry>>>,
> = Lazy::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// 托盘菜单文本（国际化）
#[derive(Clone, Copy)]
pub struct TrayTexts {
    pub show_main: &'static str,
    pub quit: &'static str,
    pub _auto_label: &'static str,
}

impl TrayTexts {
    pub fn from_language(language: &str) -> Self {
        match language {
            "en" => Self {
                show_main: "Open main window",
                quit: "Quit",
                _auto_label: "Auto (Failover)",
            },
            "ja" => Self {
                show_main: "メインウィンドウを開く",
                quit: "終了",
                _auto_label: "自動 (フェイルオーバー)",
            },
            _ => Self {
                show_main: "打开主界面",
                quit: "退出",
                _auto_label: "自动 (故障转移)",
            },
        }
    }
}

/// 托盘应用分区配置
pub struct TrayAppSection {
    pub app_type: AppType,
    pub header_label: &'static str,
    pub log_name: &'static str,
}

pub const TRAY_ID: &str = "token-use";
const STATUS_TITLE_REFRESH_SECS: u64 = 60;

pub const TRAY_SECTIONS: [TrayAppSection; 3] = [
    TrayAppSection {
        app_type: AppType::Claude,
        header_label: "Claude",
        log_name: "Claude",
    },
    TrayAppSection {
        app_type: AppType::Codex,
        header_label: "Codex",
        log_name: "Codex",
    },
    TrayAppSection {
        app_type: AppType::Gemini,
        header_label: "Gemini",
        log_name: "Gemini",
    },
];

/// 配色阈值（与前端 `utilizationColor` 语义一致）。
const UTIL_WARN_PCT: f64 = 70.0;
const UTIL_DANGER_PCT: f64 = 90.0;

fn emoji_for_utilization(pct: f64) -> &'static str {
    if pct >= UTIL_DANGER_PCT {
        "\u{1F534}" // 🔴
    } else if pct >= UTIL_WARN_PCT {
        "\u{1F7E0}" // 🟠
    } else {
        "\u{1F7E2}" // 🟢
    }
}

fn format_subscription_summary(
    quota: &crate::services::subscription::SubscriptionQuota,
) -> Option<String> {
    use crate::services::subscription::{
        TIER_FIVE_HOUR, TIER_GEMINI_FLASH, TIER_GEMINI_FLASH_LITE, TIER_GEMINI_PRO, TIER_SEVEN_DAY,
    };
    if !quota.success {
        return None;
    }

    // 按 tool 选取主卡槽 tier 并映射到短 label：
    //   Claude / Codex 沿用时间窗口（h=5 小时，w=7 天）；
    //   Gemini 用模型维度（p=pro，f=flash，l=flash-lite）——Gemini 后端 tier
    //   命名是 gemini_pro / gemini_flash / gemini_flash_lite，与时间窗口不同命名空间。
    //   flash_lite 必须纳入：否则 lite 利用率最高时色标偏低，与前端 footer 行为不一致。
    let parts: Vec<(&'static str, f64)> = match quota.tool.as_str() {
        "gemini" => {
            let mut v = Vec::new();
            if let Some(t) = quota.tiers.iter().find(|t| t.name == TIER_GEMINI_PRO) {
                v.push(("p", t.utilization));
            }
            if let Some(t) = quota.tiers.iter().find(|t| t.name == TIER_GEMINI_FLASH) {
                v.push(("f", t.utilization));
            }
            if let Some(t) = quota
                .tiers
                .iter()
                .find(|t| t.name == TIER_GEMINI_FLASH_LITE)
            {
                v.push(("l", t.utilization));
            }
            v
        }
        _ => {
            let mut v = Vec::new();
            if let Some(t) = quota.tiers.iter().find(|t| t.name == TIER_FIVE_HOUR) {
                v.push(("h", t.utilization));
            }
            if let Some(t) = quota.tiers.iter().find(|t| t.name == TIER_SEVEN_DAY) {
                v.push(("w", t.utilization));
            }
            v
        }
    };

    if parts.is_empty() {
        return None;
    }

    // 色标取所有已选 tier 里最高的利用率——用户更关心"离上限多近"。
    let worst = parts
        .iter()
        .map(|(_, u)| *u)
        .fold(f64::NEG_INFINITY, f64::max);
    if !worst.is_finite() {
        return None;
    }

    let emoji = emoji_for_utilization(worst);
    let body = parts
        .iter()
        .map(|(label, u)| format!("{label}{}%", u.round() as i64))
        .collect::<Vec<_>>()
        .join(" ");
    Some(format!("{emoji} {body}"))
}

fn tier_pct(data: &crate::provider::UsageData) -> Option<f64> {
    match (data.used, data.total) {
        (Some(used), Some(total)) if total > 0.0 => Some(used / total * 100.0),
        _ => None,
    }
}

fn format_script_summary(result: &crate::provider::UsageResult) -> Option<String> {
    use crate::services::subscription::{TIER_FIVE_HOUR, TIER_WEEKLY_LIMIT};

    if !result.success {
        return None;
    }
    let data = result.data.as_ref()?;
    if data.is_empty() {
        return None;
    }

    // commands::provider 的 token_plan 分支把 SubscriptionQuota 的每个 tier
    // 扁平化为一条 UsageData（plan_name 承载 tier 名），所以这里按 plan_name
    // 识别双桶形态，其余 usage 结果（Copilot / balance / 自定义脚本）走 fallback。
    const TOKEN_PLAN_LABELS: &[(&str, &str)] = &[(TIER_FIVE_HOUR, "h"), (TIER_WEEKLY_LIMIT, "w")];

    let mut parts: Vec<(&'static str, f64)> = Vec::new();
    for &(tier_name, label) in TOKEN_PLAN_LABELS {
        let Some(d) = data
            .iter()
            .find(|d| d.plan_name.as_deref() == Some(tier_name))
        else {
            continue;
        };
        if let Some(u) = tier_pct(d) {
            parts.push((label, u));
        }
    }
    if !parts.is_empty() {
        let worst = parts
            .iter()
            .map(|(_, u)| *u)
            .fold(f64::NEG_INFINITY, f64::max);
        let emoji = emoji_for_utilization(worst);
        let body = parts
            .iter()
            .map(|(label, u)| format!("{label}{}%", u.round() as i64))
            .collect::<Vec<_>>()
            .join(" ");
        return Some(format!("{emoji} {body}"));
    }

    let first = data.first()?;
    let pct = tier_pct(first)?;
    let emoji = emoji_for_utilization(pct);
    let plan = first.plan_name.as_deref().unwrap_or("");
    let rounded = pct.round() as i64;
    if plan.is_empty() {
        Some(format!("{} {}%", emoji, rounded))
    } else {
        Some(format!("{} {} {}%", emoji, plan, rounded))
    }
}

fn format_usage_suffix(
    app_state: &AppState,
    app_type: &AppType,
    provider: &crate::provider::Provider,
    provider_id: &str,
) -> Option<String> {
    // 当前脚本是否启用：禁用/删除时不再沿用旧 UsageCache 结果，
    // 并顺手 invalidate，防止后续重建继续命中过期数据。
    if provider.has_usage_script_enabled() {
        // 脚本缓存优先（覆盖 Copilot/coding_plan/balance/自定义脚本），借用访问避免克隆整条 UsageResult。
        if let Some(Some(s)) =
            app_state
                .usage_cache
                .with_script(app_type, provider_id, format_script_summary)
        {
            return Some(format!(" · {s}"));
        }
    } else {
        app_state
            .usage_cache
            .invalidate_script(app_type, provider_id);
    }

    if provider.category.as_deref() == Some("official") {
        if let Some(Some(s)) = app_state
            .usage_cache
            .with_subscription(app_type, format_subscription_summary)
        {
            return Some(format!(" · {s}"));
        }
    }
    None
}

fn trim_trailing_zero(value: String) -> String {
    value
        .strip_suffix(".0")
        .map_or(value.clone(), ToString::to_string)
}

pub fn format_compact_tokens(tokens: u64) -> String {
    const UNITS: [(&str, f64); 3] = [("B", 1_000_000_000.0), ("M", 1_000_000.0), ("K", 1_000.0)];

    if tokens < 1_000 {
        return tokens.to_string();
    }

    for (unit, divisor) in UNITS {
        if tokens as f64 >= divisor {
            let value = tokens as f64 / divisor;
            let number = if value >= 100.0 {
                format!("{value:.0}")
            } else {
                trim_trailing_zero(format!("{value:.1}"))
            };
            return format!("{number}{unit}");
        }
    }

    tokens.to_string()
}

fn local_today_window() -> (i64, i64) {
    let now = Local::now();
    let end = now.timestamp();
    let start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .and_then(|naive| match Local.from_local_datetime(&naive) {
            LocalResult::Single(dt) => Some(dt.timestamp()),
            LocalResult::Ambiguous(earliest, _) => Some(earliest.timestamp()),
            LocalResult::None => None,
        })
        .unwrap_or(end);
    (start, end)
}

pub fn refresh_tray_usage_title(app: &tauri::AppHandle) {
    let Some(app_state) = app.try_state::<AppState>() else {
        return;
    };
    let (start, end) = local_today_window();
    let tokens = app_state
        .db
        .get_usage_summary(Some(start), Some(end), None)
        .map(|summary| summary.real_total_tokens)
        .unwrap_or_else(|err| {
            log::debug!("[Tray] 今日 token 统计刷新失败: {err}");
            0
        });
    let short = format_compact_tokens(tokens);

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Err(err) = tray.set_title(Some(&short)) {
            log::debug!("[Tray] 更新状态栏标题失败: {err}");
        }
        let tooltip = format!("Token Use · Today {short} tokens");
        if let Err(err) = tray.set_tooltip(Some(&tooltip)) {
            log::debug!("[Tray] 更新状态栏提示失败: {err}");
        }
    }
}

pub fn start_tray_usage_title_refresh(app: tauri::AppHandle) {
    refresh_tray_usage_title(&app);
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(STATUS_TITLE_REFRESH_SECS);
        loop {
            tokio::time::sleep(interval).await;
            refresh_tray_usage_title(&app);
        }
    });
}

/// 创建动态托盘菜单
pub fn create_tray_menu(
    app: &tauri::AppHandle,
    _app_state: &AppState,
) -> Result<Menu<tauri::Wry>, AppError> {
    let app_settings = crate::settings::get_settings();
    let tray_texts = TrayTexts::from_language(app_settings.language.as_deref().unwrap_or("zh"));

    let mut menu_builder = MenuBuilder::new(app);

    let show_main_item =
        MenuItem::with_id(app, "show_main", tray_texts.show_main, true, None::<&str>)
            .map_err(|e| AppError::Message(format!("创建打开主界面菜单失败: {e}")))?;
    let quit_item = MenuItem::with_id(app, "quit", tray_texts.quit, true, None::<&str>)
        .map_err(|e| AppError::Message(format!("创建退出菜单失败: {e}")))?;

    menu_builder = menu_builder
        .item(&show_main_item)
        .separator()
        .item(&quit_item);

    let menu = menu_builder
        .build()
        .map_err(|e| AppError::Message(format!("构建菜单失败: {e}")))?;

    TRAY_SECTION_SUBMENUS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clear();

    Ok(menu)
}

/// 就地更新各 app 分区子菜单的标题（usage 后缀变化时走这条），
/// 避免 `set_menu` 导致用户打开中的菜单被关闭。
/// 句柄由上一次 `create_tray_menu` 填充；为空（从未构建过菜单）时无事发生。
fn update_tray_usage_labels(app: &tauri::AppHandle) {
    let Some(app_state) = app.try_state::<AppState>() else {
        return;
    };
    let handles = match TRAY_SECTION_SUBMENUS.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    for section in TRAY_SECTIONS.iter() {
        let Some(submenu) = handles.get(&section.app_type) else {
            continue;
        };
        let Ok(providers) = app_state.db.get_all_providers(section.app_type.as_str()) else {
            continue;
        };
        let Ok(Some(current_id)) =
            crate::settings::get_effective_current_provider(&app_state.db, &section.app_type)
        else {
            continue;
        };
        let Some(provider) = providers.get(&current_id) else {
            continue;
        };
        let suffix = format_usage_suffix(&app_state, &section.app_type, provider, &current_id)
            .unwrap_or_default();
        let new_label = format!("{} · {}{}", section.header_label, provider.name, suffix);
        if let Err(e) = submenu.set_text(&new_label) {
            log::debug!("[Tray] 更新{}子菜单标题失败: {e}", section.log_name);
        }
    }
}

pub fn refresh_tray_menu(app: &tauri::AppHandle) {
    use crate::store::AppState;

    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(new_menu) = create_tray_menu(app, state.inner()) {
            if let Some(tray) = app.tray_by_id(TRAY_ID) {
                if let Err(e) = tray.set_menu(Some(new_menu)) {
                    log::error!("刷新托盘菜单失败: {e}");
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub fn apply_tray_policy(app: &tauri::AppHandle, dock_visible: bool) {
    use tauri::ActivationPolicy;

    let desired_policy = if dock_visible {
        ActivationPolicy::Regular
    } else {
        ActivationPolicy::Accessory
    };

    if let Err(err) = app.set_dock_visibility(dock_visible) {
        log::warn!("设置 Dock 显示状态失败: {err}");
    }

    if let Err(err) = app.set_activation_policy(desired_policy) {
        log::warn!("设置激活策略失败: {err}");
    }
}

/// 处理托盘菜单事件
pub fn handle_tray_menu_event(app: &tauri::AppHandle, event_id: &str) {
    log::info!("处理托盘菜单事件: {event_id}");

    match event_id {
        "show_main" => {
            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "windows")]
                {
                    let _ = window.set_skip_taskbar(false);
                }
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
                #[cfg(target_os = "linux")]
                {
                    crate::linux_fix::nudge_main_window(window.clone());
                }
                #[cfg(target_os = "macos")]
                {
                    apply_tray_policy(app, true);
                }
            } else if crate::lightweight::is_lightweight_mode() {
                if let Err(e) = crate::lightweight::exit_lightweight_mode(app) {
                    log::error!("退出轻量模式重建窗口失败: {e}");
                }
            }
        }
        "quit" => {
            log::info!("退出应用");
            app.exit(0);
        }
        _ => {
            log::warn!("未处理的菜单事件: {event_id}");
        }
    }
}

/// 合并多次快速触发的"usage 标题软更新"：批量刷新期间多个 usage 命令
/// 同时成功时，只会产生一次就地 `set_text` 批量调用。走软更新而不是
/// `refresh_tray_menu` 整建，避免用户打开中的菜单被 macOS 系统关闭。
static TRAY_REBUILD_SCHEDULED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn schedule_tray_refresh(app: &tauri::AppHandle) {
    use std::sync::atomic::Ordering;
    if TRAY_REBUILD_SCHEDULED.swap(true, Ordering::AcqRel) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // 50ms 合窗：让同一轮 React Query / 托盘批量刷新触发的多个写入
        // 共享一次标题更新。
        std::thread::sleep(std::time::Duration::from_millis(50));
        TRAY_REBUILD_SCHEDULED.store(false, Ordering::Release);
        refresh_tray_usage_title(&app);
        update_tray_usage_labels(&app);
    });
}

#[cfg(test)]
mod tests {
    use super::{
        format_compact_tokens, format_script_summary, format_subscription_summary, TRAY_ID,
    };
    use crate::provider::{UsageData, UsageResult};
    use crate::services::subscription::{
        CredentialStatus, QuotaTier, SubscriptionQuota, TIER_FIVE_HOUR, TIER_WEEKLY_LIMIT,
    };

    #[test]
    fn tray_id_is_unique_to_app() {
        assert_eq!(TRAY_ID, "token-use");
        assert_ne!(TRAY_ID, "main");
    }

    #[test]
    fn compact_tokens_keep_status_bar_text_short() {
        assert_eq!(format_compact_tokens(0), "0");
        assert_eq!(format_compact_tokens(999), "999");
        assert_eq!(format_compact_tokens(1_200), "1.2K");
        assert_eq!(format_compact_tokens(12_000), "12K");
        assert_eq!(format_compact_tokens(12_400_000), "12.4M");
        assert_eq!(format_compact_tokens(987_000_000), "987M");
        assert_eq!(format_compact_tokens(1_200_000_000), "1.2B");
    }

    fn make_quota(tool: &str, success: bool, tiers: Vec<QuotaTier>) -> SubscriptionQuota {
        SubscriptionQuota {
            tool: tool.to_string(),
            credential_status: CredentialStatus::Valid,
            credential_message: None,
            success,
            tiers,
            extra_usage: None,
            error: None,
            queried_at: Some(0),
        }
    }

    fn tier(name: &str, utilization: f64) -> QuotaTier {
        QuotaTier {
            name: name.to_string(),
            utilization,
            resets_at: None,
        }
    }

    #[test]
    fn claude_summary_uses_h_and_w_labels() {
        let quota = make_quota(
            "claude",
            true,
            vec![tier("five_hour", 9.0), tier("seven_day", 27.0)],
        );
        let s = format_subscription_summary(&quota).expect("should format");
        assert!(s.contains("h9%"), "expected h9% in {s}");
        assert!(s.contains("w27%"), "expected w27% in {s}");
    }

    #[test]
    fn gemini_summary_uses_p_and_f_labels() {
        let quota = make_quota(
            "gemini",
            true,
            vec![tier("gemini_pro", 15.0), tier("gemini_flash", 42.0)],
        );
        let s = format_subscription_summary(&quota).expect("should format");
        assert!(s.contains("p15%"), "expected p15% in {s}");
        assert!(s.contains("f42%"), "expected f42% in {s}");
    }

    #[test]
    fn gemini_summary_includes_all_three_tiers() {
        let quota = make_quota(
            "gemini",
            true,
            vec![
                tier("gemini_pro", 5.0),
                tier("gemini_flash", 42.0),
                tier("gemini_flash_lite", 80.0),
            ],
        );
        let s = format_subscription_summary(&quota).expect("should format");
        assert!(s.contains("p5%"), "expected p5% in {s}");
        assert!(s.contains("f42%"), "expected f42% in {s}");
        assert!(s.contains("l80%"), "expected l80% in {s}");
    }

    #[test]
    fn gemini_summary_lite_only_still_renders() {
        // flash_lite 如果是 API 返回的唯一 tier，仍应显示（避免前端 footer 能看到、
        // 托盘空白的不对称）。
        let quota = make_quota("gemini", true, vec![tier("gemini_flash_lite", 80.0)]);
        let s = format_subscription_summary(&quota).expect("should format");
        assert!(s.contains("l80%"), "expected l80% in {s}");
    }

    #[test]
    fn gemini_summary_emoji_reflects_highest_tier_including_lite() {
        // lite 是利用率最高的那条 → emoji 必须是红色，不能被 pro/flash 掩盖。
        let quota = make_quota(
            "gemini",
            true,
            vec![
                tier("gemini_pro", 10.0),
                tier("gemini_flash", 20.0),
                tier("gemini_flash_lite", 95.0),
            ],
        );
        let s = format_subscription_summary(&quota).unwrap();
        assert!(
            s.starts_with("\u{1F534}"),
            "expected red emoji (lite worst) in {s}"
        );
    }

    #[test]
    fn worst_emoji_reflects_highest_utilization() {
        // 🔴 = \u{1F534}; 任一 tier ≥ 90% 时预期显示红色。
        let quota = make_quota(
            "claude",
            true,
            vec![tier("five_hour", 10.0), tier("seven_day", 95.0)],
        );
        let s = format_subscription_summary(&quota).unwrap();
        assert!(s.starts_with("\u{1F534}"), "expected red emoji in {s}");
    }

    #[test]
    fn failure_quota_returns_none() {
        let quota = make_quota("claude", false, vec![tier("five_hour", 50.0)]);
        assert!(format_subscription_summary(&quota).is_none());
    }

    #[test]
    fn unknown_tiers_return_none() {
        let quota = make_quota("claude", true, vec![tier("one_hour", 80.0)]);
        assert!(format_subscription_summary(&quota).is_none());
    }

    #[test]
    fn gemini_without_any_known_tiers_returns_none() {
        // 完全没有 pro/flash/flash_lite 三种 tier 的退化响应 → None。
        let quota = make_quota("gemini", true, vec![tier("some_future_tier", 80.0)]);
        assert!(format_subscription_summary(&quota).is_none());
    }

    fn usage_data(plan_name: Option<&str>, utilization: f64) -> UsageData {
        UsageData {
            plan_name: plan_name.map(String::from),
            extra: None,
            is_valid: Some(true),
            invalid_message: None,
            total: Some(100.0),
            used: Some(utilization),
            remaining: Some(100.0 - utilization),
            unit: Some("%".to_string()),
        }
    }

    fn usage_result(success: bool, data: Vec<UsageData>) -> UsageResult {
        UsageResult {
            success,
            data: if data.is_empty() { None } else { Some(data) },
            error: None,
        }
    }

    #[test]
    fn script_summary_token_plan_two_tiers() {
        let r = usage_result(
            true,
            vec![
                usage_data(Some(TIER_FIVE_HOUR), 12.0),
                usage_data(Some(TIER_WEEKLY_LIMIT), 80.0),
            ],
        );
        let s = format_script_summary(&r).expect("should format");
        assert!(s.contains("h12%"), "expected h12% in {s}");
        assert!(s.contains("w80%"), "expected w80% in {s}");
        assert!(s.starts_with("\u{1F7E0}"), "expected orange emoji in {s}");
    }

    #[test]
    fn script_summary_token_plan_worst_drives_emoji() {
        let r = usage_result(
            true,
            vec![
                usage_data(Some(TIER_FIVE_HOUR), 20.0),
                usage_data(Some(TIER_WEEKLY_LIMIT), 95.0),
            ],
        );
        let s = format_script_summary(&r).unwrap();
        assert!(s.starts_with("\u{1F534}"), "expected red emoji in {s}");
    }

    #[test]
    fn script_summary_token_plan_five_hour_only() {
        let r = usage_result(true, vec![usage_data(Some(TIER_FIVE_HOUR), 8.0)]);
        let s = format_script_summary(&r).expect("should format");
        assert!(s.contains("h8%"), "expected h8% in {s}");
        assert!(
            !s.contains("plan_name"),
            "plan_name should not leak into label: {s}"
        );
    }

    #[test]
    fn script_summary_token_plan_weekly_only() {
        let r = usage_result(true, vec![usage_data(Some(TIER_WEEKLY_LIMIT), 50.0)]);
        let s = format_script_summary(&r).expect("should format");
        assert!(s.contains("w50%"), "expected w50% in {s}");
    }

    #[test]
    fn script_summary_single_bucket_fallback_with_plan_name() {
        let r = usage_result(true, vec![usage_data(Some("Copilot Pro"), 40.0)]);
        let s = format_script_summary(&r).expect("should format");
        assert!(s.contains("Copilot Pro"), "expected plan name in {s}");
        assert!(s.contains("40%"), "expected 40% in {s}");
        assert!(
            !s.contains("h40%"),
            "must not relabel non-token-plan data as h: {s}"
        );
    }

    #[test]
    fn script_summary_single_bucket_fallback_without_plan_name() {
        let r = usage_result(true, vec![usage_data(None, 15.0)]);
        let s = format_script_summary(&r).expect("should format");
        assert_eq!(s, "\u{1F7E2} 15%", "expected emoji + pct only, got {s}");
    }

    #[test]
    fn script_summary_failure_returns_none() {
        let r = usage_result(false, vec![usage_data(Some(TIER_FIVE_HOUR), 12.0)]);
        assert!(format_script_summary(&r).is_none());
    }

    #[test]
    fn script_summary_empty_data_returns_none() {
        let r = usage_result(true, vec![]);
        assert!(format_script_summary(&r).is_none());
    }
}
