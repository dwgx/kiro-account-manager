use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccountProxyProtocol {
    Http,
    Socks5,
}

impl Default for AccountProxyProtocol {
    fn default() -> Self {
        Self::Http
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountProxyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub protocol: AccountProxyProtocol,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl AccountProxyConfig {
    pub fn to_proxy_url(&self) -> Result<String, String> {
        if !self.enabled {
            return Err("Account proxy is disabled".to_string());
        }

        let host = self.host.trim();
        if host.is_empty() {
            return Err("Proxy host is required".to_string());
        }
        if self.port == 0 {
            return Err("Proxy port is required".to_string());
        }

        let scheme = match self.protocol {
            AccountProxyProtocol::Http => "http",
            AccountProxyProtocol::Socks5 => "socks5h",
        };
        let mut url = url::Url::parse(&format!("{scheme}://{host}:{}", self.port))
            .map_err(|error| format!("Invalid proxy address: {error}"))?;

        let username = self
            .username
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let password = self.password.as_deref().filter(|value| !value.is_empty());

        if let Some(username) = username {
            url.set_username(username)
                .map_err(|_| "Invalid proxy username".to_string())?;
            if let Some(password) = password {
                url.set_password(Some(password))
                    .map_err(|_| "Invalid proxy password".to_string())?;
            }
        } else if password.is_some() {
            return Err("Proxy password requires a username".to_string());
        }

        Ok(url.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{AccountProxyConfig, AccountProxyProtocol};

    #[test]
    fn account_proxy_config_uses_remote_dns_for_socks5() {
        let proxy = AccountProxyConfig {
            enabled: true,
            protocol: AccountProxyProtocol::Socks5,
            host: "127.0.0.1".to_string(),
            port: 1080,
            username: None,
            password: None,
        };

        assert_eq!(proxy.to_proxy_url().unwrap(), "socks5h://127.0.0.1:1080");
    }

    #[test]
    fn account_proxy_config_includes_auth_when_configured() {
        let proxy = AccountProxyConfig {
            enabled: true,
            protocol: AccountProxyProtocol::Http,
            host: "proxy.example.test".to_string(),
            port: 8080,
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
        };

        assert_eq!(
            proxy.to_proxy_url().unwrap(),
            "http://user:pass@proxy.example.test:8080/"
        );
    }
}
