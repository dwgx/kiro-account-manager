use crate::utils::client_id_hash::calculate_client_id_hash;

// ===== Profile ARN 常量与 provider 映射 =====

/// BuilderId 的固定 startUrl
pub const KIRO_BUILDER_ID_START_URL: &str = "https://view.awsapps.com/start";

/// BuilderId 的 clientIdHash（定值，无需每次现算）
///
/// 算法：`sha1(JSON.stringify({ startUrl }))`，即
/// `SHA1('{"startUrl":"https://view.awsapps.com/start"}')`。
/// startUrl 固定，hash 自然也固定，直接当常量兜底，省去重算、也避免
/// 规范化逻辑出错时把 BuilderId 也算偏。常量正确性由 `common.rs` 单测校验。
pub const KIRO_BUILDER_ID_CLIENT_ID_HASH: &str = "e909a0580879b06ece1202964fbe9dda95ea4ce3";

/// 判断给定 startUrl 是否就是 BuilderId 的默认 startUrl（去尾斜杠后比较）。
///
/// 用于拦截 Enterprise 账号回退到 BuilderId 默认值的脏数据：Enterprise 必须用
/// 自己的 `d-xxx` 域名，绝不能落到 `https://view.awsapps.com/start`。
pub fn is_builder_id_start_url(start_url: &str) -> bool {
    start_url.trim().trim_end_matches('/') == KIRO_BUILDER_ID_START_URL.trim_end_matches('/')
}

/// 判断给定 clientIdHash 是否就是 BuilderId 的默认 hash（忽略大小写）。
///
/// 同样用于拦截 Enterprise 误用 BuilderId 默认 hash（`e909a058...`）。
pub fn is_builder_id_client_id_hash(hash: &str) -> bool {
    hash.trim()
        .eq_ignore_ascii_case(KIRO_BUILDER_ID_CLIENT_ID_HASH)
}

/// 校验 Enterprise 账号解析出的 clientIdHash 合法：非空、且不是 BuilderId 默认值。
///
/// Enterprise 回退到 BuilderId 的 hash（或其 startUrl 算出的 hash）正是 issue #119
/// 的根因——文件名错位导致 IDE 找不到 client registration。这里硬性拦下。
pub fn ensure_enterprise_client_id_hash(hash: &str) -> Result<(), String> {
    if hash.trim().is_empty() {
        return Err("Enterprise 账号的 clientIdHash 为空".to_string());
    }
    if is_builder_id_client_id_hash(hash) {
        return Err(
            "Enterprise 账号不能使用 BuilderId 默认 clientIdHash（e909a058...），\
             必须由企业自己的 d-xxx startUrl 算出。请检查账号的 start_url / client_id_hash。"
                .to_string(),
        );
    }
    Ok(())
}

/// 统一解析 IdC（BuilderId / Enterprise）账号的 clientIdHash —— 切号、添加、导入共用。
///
/// 这是 issue #119 的唯一裁决点：以前 switch / add / import 各抄一份「优先用已存
/// hash → BuilderId 用常量兜底 / Enterprise 由 startUrl 现算」的逻辑，容易漂移，
/// 还漏了校验。收敛到这里，规则只有一处：
///
/// 1. 账号已存 `client_id_hash` → 直接用（真实 IDE/CLI token 通常自带）；
/// 2. 否则按 provider 兜底：
///    - BuilderId：startUrl 固定，hash 是定值，直接用常量（也兼容传入的自定义 startUrl）；
///    - Enterprise：必须有自己的 `d-xxx` startUrl，现算；缺失则报错。
/// 3. Enterprise 最终硬校验：hash 绝不能为空、也绝不能等于 BuilderId 默认值
///    （`e909a058...`）。一旦命中即说明数据被污染，写出去文件名必然错位 →
///    IDE 找不到 client registration（issue #119 根因），直接拒绝而非写坏数据。
///
/// `provider` 取 `"BuilderId"` / `"Enterprise"`，其余值视为未知 provider 报错。
pub fn resolve_idc_client_id_hash(
    provider: &str,
    client_id_hash: Option<&str>,
    start_url: Option<&str>,
) -> Result<String, String> {
    // 1. 已存 hash 优先
    let hash = if let Some(hash) = client_id_hash.map(str::trim).filter(|h| !h.is_empty()) {
        hash.to_string()
    } else {
        // 2. 按 provider 兜底
        match provider {
            "BuilderId" => match start_url.map(str::trim).filter(|s| !s.is_empty()) {
                Some(url) => calculate_client_id_hash(url),
                None => KIRO_BUILDER_ID_CLIENT_ID_HASH.to_string(),
            },
            "Enterprise" => {
                let url = start_url
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or("Enterprise 账号必须提供 start_url 或 client_id_hash")?;
                calculate_client_id_hash(url)
            }
            other => return Err(format!("未知的 IdC Provider: {other}")),
        }
    };

    // 3. Enterprise 硬校验
    if provider == "Enterprise" {
        ensure_enterprise_client_id_hash(&hash)?;
    }
    Ok(hash)
}

#[cfg(test)]
mod tests {
    #[test]
    fn builder_id_client_id_hash_constant_matches_algorithm() {
        // 校验：常量 KIRO_BUILDER_ID_CLIENT_ID_HASH 必须等于
        // sha1(JSON.stringify({ startUrl: BUILDER_ID_START_URL }))。
        // 防止以后改 hash 算法/常量时两者漂移。
        use sha1::{Digest, Sha1};
        let input = serde_json::json!({
            "startUrl": super::KIRO_BUILDER_ID_START_URL,
        })
        .to_string();
        let mut hasher = Sha1::new();
        hasher.update(input.as_bytes());
        let computed = hex::encode(hasher.finalize());
        assert_eq!(computed, super::KIRO_BUILDER_ID_CLIENT_ID_HASH);
    }

    #[test]
    fn is_builder_id_start_url_matches_default_variants() {
        use super::is_builder_id_start_url;
        // 默认值本身、带尾斜杠、带空白都应识别为 BuilderId 默认值
        assert!(is_builder_id_start_url("https://view.awsapps.com/start"));
        assert!(is_builder_id_start_url("https://view.awsapps.com/start/"));
        assert!(is_builder_id_start_url(
            "  https://view.awsapps.com/start  "
        ));
        // 企业 d-xxx 域名不应被误判
        assert!(!is_builder_id_start_url(
            "https://d-90660ceab3.awsapps.com/start"
        ));
    }

    #[test]
    fn is_builder_id_client_id_hash_matches_default_hash() {
        use super::is_builder_id_client_id_hash;
        // 默认 hash 本身、大写、带空白都应识别
        assert!(is_builder_id_client_id_hash(
            "e909a0580879b06ece1202964fbe9dda95ea4ce3"
        ));
        assert!(is_builder_id_client_id_hash(
            "E909A0580879B06ECE1202964FBE9DDA95EA4CE3"
        ));
        assert!(is_builder_id_client_id_hash(
            "  e909a0580879b06ece1202964fbe9dda95ea4ce3  "
        ));
        // 企业账号真实 hash（d-90660ceab3）不应被误判
        assert!(!is_builder_id_client_id_hash(
            "a96ec6ff09e0c558ceca191cdaa0ff2b0e4e3e35"
        ));
    }

    #[test]
    fn ensure_enterprise_client_id_hash_rejects_empty_and_builder_default() {
        use super::ensure_enterprise_client_id_hash;
        // 空 hash 拒绝
        assert!(ensure_enterprise_client_id_hash("").is_err());
        assert!(ensure_enterprise_client_id_hash("   ").is_err());
        // BuilderId 默认 hash 拒绝（issue #119 根因）
        assert!(
            ensure_enterprise_client_id_hash("e909a0580879b06ece1202964fbe9dda95ea4ce3").is_err()
        );
        // 企业自己的 d-xxx hash 通过
        assert!(
            ensure_enterprise_client_id_hash("a96ec6ff09e0c558ceca191cdaa0ff2b0e4e3e35").is_ok()
        );
    }

    #[test]
    fn resolve_idc_client_id_hash_prefers_stored_hash() {
        use super::resolve_idc_client_id_hash;
        // 已存 hash 优先，provider / start_url 都不影响
        let resolved = resolve_idc_client_id_hash(
            "Enterprise",
            Some("a96ec6ff09e0c558ceca191cdaa0ff2b0e4e3e35"),
            Some("https://d-90660ceab3.awsapps.com/start"),
        )
        .unwrap();
        assert_eq!(resolved, "a96ec6ff09e0c558ceca191cdaa0ff2b0e4e3e35");
    }

    #[test]
    fn resolve_idc_client_id_hash_builder_falls_back_to_constant() {
        use super::resolve_idc_client_id_hash;
        // BuilderId 无 hash、无 start_url → 直接用定值常量，不用现算
        let resolved = resolve_idc_client_id_hash("BuilderId", None, None).unwrap();
        assert_eq!(resolved, super::KIRO_BUILDER_ID_CLIENT_ID_HASH);
        // 空白也视为缺失
        let resolved = resolve_idc_client_id_hash("BuilderId", Some("  "), Some("")).unwrap();
        assert_eq!(resolved, super::KIRO_BUILDER_ID_CLIENT_ID_HASH);
    }

    #[test]
    fn resolve_idc_client_id_hash_enterprise_computes_from_start_url() {
        use super::resolve_idc_client_id_hash;
        // Enterprise 无 hash 但有 d-xxx startUrl → 现算，对得上真实 IDE 文件名
        let resolved = resolve_idc_client_id_hash(
            "Enterprise",
            None,
            Some("https://d-90660ceab3.awsapps.com/start"),
        )
        .unwrap();
        assert_eq!(resolved, "a96ec6ff09e0c558ceca191cdaa0ff2b0e4e3e35");
    }

    #[test]
    fn resolve_idc_client_id_hash_enterprise_rejects_missing_and_builder_default() {
        use super::resolve_idc_client_id_hash;
        // Enterprise 既无 hash 又无 startUrl → 报错（不能瞎兜底成 BuilderId）
        assert!(resolve_idc_client_id_hash("Enterprise", None, None).is_err());
        // Enterprise 的 start_url 被污染成 BuilderId 默认值 → 现算出 e909a058 → 硬校验拦下
        assert!(resolve_idc_client_id_hash(
            "Enterprise",
            None,
            Some("https://view.awsapps.com/start"),
        )
        .is_err());
        // Enterprise 直接存了 BuilderId 默认 hash → 同样拦下
        assert!(resolve_idc_client_id_hash(
            "Enterprise",
            Some("e909a0580879b06ece1202964fbe9dda95ea4ce3"),
            None,
        )
        .is_err());
    }

    #[test]
    fn resolve_idc_client_id_hash_rejects_unknown_provider() {
        use super::resolve_idc_client_id_hash;
        assert!(resolve_idc_client_id_hash("Google", None, None).is_err());
    }
}
