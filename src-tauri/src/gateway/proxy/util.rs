//! 网关请求编排:无状态小工具(纯搬迁)。
use super::*;

/// 安全截断字符串到指定字节数，确保不会切到 UTF-8 多字节字符中间
pub(super) fn safe_truncate(s: &str, max_bytes: usize) -> usize {
    if s.len() <= max_bytes {
        return s.len();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

pub(super) fn short_uuid() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}

/// 从请求中提取会话 ID（用于缓存）
pub(super) fn extract_session_id_from_request(request: &NormalizedRequest) -> Option<String> {
    // 尝试从 previous_response_id 提取会话 ID
    if let Some(prev_id) = &request.previous_response_id {
        // 从 response ID 中提取会话部分（假设格式为 "session_xxx_response_yyy"）
        if let Some(session_part) = prev_id.split('_').nth(1) {
            return Some(format!("session_{}", session_part));
        }
        // 如果格式不匹配，直接使用 previous_response_id 作为会话标识
        return Some(prev_id.clone());
    }

    // 如果没有 previous_response_id，使用消息内容的哈希作为会话标识
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for msg in &request.messages {
        msg.role.hash(&mut hasher);
        if let Some(content) = &msg.content {
            content.to_string().hash(&mut hasher);
        }
    }
    Some(format!("session_{:x}", hasher.finish()))
}
