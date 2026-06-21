//! UFW backend: reads firewall state from config files (no root needed),
//! executes modifications via pkexec (root needed).
//!
//! Architecture (Solution B):
//!   - /etc/ufw/ufw.conf      → ENABLED status (world-readable)
//!   - /etc/default/ufw        → DEFAULT policies (world-readable)
//!   - /etc/ufw/user.rules     → IPv4 rules (made readable via Apkg postinst)
//!   - /etc/ufw/user6.rules    → IPv6 rules (made readable via Apkg postinst)
//!   - /etc/ufw/applications.d → App profiles (world-readable)
//!   - pkexec ufw ...          → All write operations

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use super::types::*;

const UFW_CONF: &str = "/etc/ufw/ufw.conf";
const UFW_DEFAULTS: &str = "/etc/default/ufw";
const UFW_USER_RULES: &str = "/etc/ufw/user.rules";
const UFW_USER6_RULES: &str = "/etc/ufw/user6.rules";
const UFW_APPS_DIR: &str = "/etc/ufw/applications.d";

// ─── Reading state (no root needed) ──────────────────────────────────────────

/// Read the complete firewall status by parsing config files directly.
/// This does NOT require root privileges.
pub fn read_status() -> Result<UfwStatus, UfwError> {
    let active = read_enabled()?;
    let logging = read_logging()?;
    let (default_incoming, default_outgoing) = read_defaults()?;
    let mut rules = Vec::new();

    // Parse IPv4 rules
    if let Ok(v4_rules) = parse_rules_file(UFW_USER_RULES, false) {
        rules.extend(v4_rules);
    }

    // Parse IPv6 rules
    if let Ok(v6_rules) = parse_rules_file(UFW_USER6_RULES, true) {
        rules.extend(v6_rules);
    }

    // Renumber rules sequentially (combined v4 + v6)
    for (i, rule) in rules.iter_mut().enumerate() {
        rule.number = (i + 1) as u32;
    }

    Ok(UfwStatus {
        active,
        default_incoming,
        default_outgoing,
        rules,
        logging,
    })
}

/// Check if UFW is enabled by reading /etc/ufw/ufw.conf.
fn read_enabled() -> Result<bool, UfwError> {
    let content = fs::read_to_string(UFW_CONF).map_err(|e| UfwError {
        message: format!("Cannot read {UFW_CONF}: {e}"),
    })?;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("ENABLED=") {
            let val = line.strip_prefix("ENABLED=").unwrap_or("no");
            return Ok(val.eq_ignore_ascii_case("yes"));
        }
    }

    Ok(false)
}

/// Read logging level from /etc/ufw/ufw.conf.
fn read_logging() -> Result<String, UfwError> {
    let content = fs::read_to_string(UFW_CONF).unwrap_or_default();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("LOGLEVEL=") {
            let val = line.strip_prefix("LOGLEVEL=").unwrap_or("off");
            return Ok(val.to_string());
        }
    }
    Ok("off".to_string())
}

/// Read default policies from /etc/default/ufw.
fn read_defaults() -> Result<(Policy, Policy), UfwError> {
    let content = fs::read_to_string(UFW_DEFAULTS).map_err(|e| UfwError {
        message: format!("Cannot read {UFW_DEFAULTS}: {e}"),
    })?;

    let mut incoming = Policy::Deny;
    let mut outgoing = Policy::Allow;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("DEFAULT_INPUT_POLICY=") {
            let val = line
                .strip_prefix("DEFAULT_INPUT_POLICY=")
                .unwrap_or("")
                .trim_matches('"');
            if let Some(p) = Policy::from_str(val) {
                incoming = p;
            }
        } else if line.starts_with("DEFAULT_OUTPUT_POLICY=") {
            let val = line
                .strip_prefix("DEFAULT_OUTPUT_POLICY=")
                .unwrap_or("")
                .trim_matches('"');
            if let Some(p) = Policy::from_str(val) {
                outgoing = p;
            }
        }
    }

    Ok((incoming, outgoing))
}

/// Parse rules from a user.rules file by reading `### tuple ###` comment lines.
///
/// Format: `### tuple ### <action> <protocol> <dport> <dst> <sport> <src> <direction>`
fn parse_rules_file(path: &str, v6: bool) -> Result<Vec<UfwRule>, UfwError> {
    let content = fs::read_to_string(path).map_err(|e| UfwError {
        message: format!("Cannot read {path}: {e}"),
    })?;

    let mut rules = Vec::new();
    let mut in_rules = false;

    for line in content.lines() {
        let line = line.trim();

        if line == "### RULES ###" {
            in_rules = true;
            continue;
        }
        if line == "### END RULES ###" {
            break;
        }
        if !in_rules {
            continue;
        }

        if let Some(tuple) = line.strip_prefix("### tuple ###") {
            if let Some(rule) = parse_tuple(tuple.trim(), v6) {
                rules.push(rule);
            }
        }
    }

    Ok(rules)
}

/// Parse a single `### tuple ###` comment line into a UfwRule.
///
/// Format: `<action> <protocol> <dport> <dst> <sport> <src> <direction> [comment=...]`
fn parse_tuple(tuple: &str, v6: bool) -> Option<UfwRule> {
    let parts: Vec<&str> = tuple.split_whitespace().collect();
    if parts.len() < 7 {
        return None;
    }

    let action = Action::from_str(parts[0])?;
    let protocol = Protocol::from_str(parts[1]).unwrap_or(Protocol::Both);
    let dport = parts[2]; // destination port
    let _dst = parts[3]; // destination address
    let _sport = parts[4]; // source port
    let src = parts[5]; // source address
    let direction = Direction::from_str(parts[6]).unwrap_or(Direction::In);

    // Build port display string
    let port = match protocol {
        Protocol::Both => dport.to_string(),
        Protocol::Tcp => {
            if dport == "any" {
                "any".to_string()
            } else {
                format!("{dport}/tcp")
            }
        }
        Protocol::Udp => {
            if dport == "any" {
                "any".to_string()
            } else {
                format!("{dport}/udp")
            }
        }
    };

    // Build source display string
    let from = if src == "0.0.0.0/0" || src == "::/0" {
        if v6 {
            "Anywhere (v6)".to_string()
        } else {
            "Anywhere".to_string()
        }
    } else {
        src.to_string()
    };

    // Build destination display string
    let to = if _dst == "0.0.0.0/0" || _dst == "::/0" {
        if v6 {
            "Anywhere (v6)".to_string()
        } else {
            "Anywhere".to_string()
        }
    } else {
        _dst.to_string()
    };

    Some(UfwRule {
        number: 0, // renumbered later
        port,
        action,
        direction,
        from,
        to,
        v6,
    })
}

// ─── Reading app profiles (no root needed) ───────────────────────────────────

/// Read all application profiles from /etc/ufw/applications.d/.
pub fn read_profiles() -> Result<Vec<AppProfile>, UfwError> {
    let dir = Path::new(UFW_APPS_DIR);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut profiles = Vec::new();

    let entries = fs::read_dir(dir).map_err(|e| UfwError {
        message: format!("Cannot read {UFW_APPS_DIR}: {e}"),
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(&path) {
                let mut file_profiles = parse_app_profiles(&content);
                profiles.append(&mut file_profiles);
            }
        }
    }

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}

/// Parse an INI-style application profile file.
///
/// Format:
/// ```ini
/// [AppName]
/// title=Human Readable Title
/// description=What this app does
/// ports=80,443/tcp
/// ```
fn parse_app_profiles(content: &str) -> Vec<AppProfile> {
    let mut profiles = Vec::new();
    let mut current_section: Option<String> = None;
    let mut fields: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Section header: [AppName]
        if line.starts_with('[') && line.ends_with(']') {
            // Save previous section
            if let Some(name) = current_section.take() {
                profiles.push(AppProfile {
                    name,
                    title: fields.remove("title").unwrap_or_default(),
                    description: fields.remove("description").unwrap_or_default(),
                    ports: fields.remove("ports").unwrap_or_default(),
                });
                fields.clear();
            }
            current_section = Some(line[1..line.len() - 1].to_string());
            continue;
        }

        // Key=value
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }

    // Save last section
    if let Some(name) = current_section.take() {
        profiles.push(AppProfile {
            name,
            title: fields.remove("title").unwrap_or_default(),
            description: fields.remove("description").unwrap_or_default(),
            ports: fields.remove("ports").unwrap_or_default(),
        });
    }

    profiles
}

// ─── Writing operations (require pkexec) ─────────────────────────────────────

/// Enable or disable the firewall via `pkexec ufw enable/disable`.
pub fn set_enabled(enabled: bool) -> Result<String, UfwError> {
    let arg = if enabled { "enable" } else { "disable" };
    run_pkexec_ufw(&["--force", arg])
}

/// Set the UFW logging level.
pub fn set_logging(level: &str) -> Result<String, UfwError> {
    run_pkexec_ufw(&["logging", level])
}

/// Set the default policy for a direction.
pub fn set_default_policy(direction: Direction, policy: Policy) -> Result<String, UfwError> {
    run_pkexec_ufw(&["default", policy.as_ufw_arg(), direction.as_ufw_arg()])
}

/// Add a new firewall rule.
pub fn add_rule(params: &RuleParams) -> Result<String, UfwError> {
    let mut args: Vec<String> = Vec::new();

    // Action
    args.push(params.action.as_ufw_arg().to_string());

    // Direction (optional)
    if let Some(dir) = &params.direction {
        args.push(dir.as_ufw_arg().to_string());
    }

    // From clause
    if let Some(from) = &params.from {
        if !from.is_empty() {
            args.push("from".to_string());
            args.push(from.clone());
        }
    }

    // To clause
    if let Some(to) = &params.to {
        if !to.is_empty() {
            args.push("to".to_string());
            args.push(to.clone());
        }
    }

    // Port with optional protocol
    if !params.port.is_empty() {
        // If we have from/to clauses, use "port" keyword
        if params.from.is_some() || params.to.is_some() {
            args.push("port".to_string());
            args.push(params.port.clone());
        } else {
            // Build port/proto string
            let port_str = match &params.protocol {
                Some(Protocol::Tcp) => format!("{}/tcp", params.port),
                Some(Protocol::Udp) => format!("{}/udp", params.port),
                _ => params.port.clone(),
            };
            args.push(port_str);
        }
    }

    // Protocol (when using from/to syntax)
    if (params.from.is_some() || params.to.is_some()) && params.protocol.is_some() {
        if let Some(proto) = params.protocol.as_ref().and_then(|p| p.as_ufw_arg()) {
            args.push("proto".to_string());
            args.push(proto.to_string());
        }
    }

    let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_pkexec_ufw(&args_refs)
}

/// Delete a rule by its number.
pub fn delete_rule(number: u32) -> Result<String, UfwError> {
    run_pkexec_ufw(&["--force", "delete", &number.to_string()])
}

/// Allow an application profile.
pub fn allow_app(name: &str) -> Result<String, UfwError> {
    run_pkexec_ufw(&["allow", name])
}

/// Delete an application profile allow rule.
pub fn delete_app(name: &str) -> Result<String, UfwError> {
    run_pkexec_ufw(&["--force", "delete", "allow", name])
}

/// Check if an app profile is currently allowed by checking the rules.
pub fn is_app_allowed(rules: &[UfwRule], profile: &AppProfile) -> bool {
    // Check if any rule matches this app's ports
    let profile_name_lower = profile.name.to_lowercase();
    rules.iter().any(|r| {
        let port_lower = r.port.to_lowercase();
        port_lower == profile_name_lower && r.action == Action::Allow
    })
}

// ─── Internal helpers ────────────────────────────────────────────────────────

/// Run `pkexec ufw <args>` and return stdout.
fn run_pkexec_ufw(args: &[&str]) -> Result<String, UfwError> {
    let mut cmd_args = vec!["ufw"];
    cmd_args.extend_from_slice(args);

    let output = Command::new("pkexec")
        .args(&cmd_args)
        .output()
        .map_err(|e| UfwError {
            message: format!("Failed to execute pkexec: {e}"),
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        // pkexec returns 126 when user dismisses the dialog
        if output.status.code() == Some(126) {
            Err(UfwError {
                message: "Authentication cancelled".to_string(),
            })
        } else {
            Err(UfwError {
                message: format!(
                    "UFW command failed: {}",
                    if stderr.is_empty() {
                        stdout.to_string()
                    } else {
                        stderr.to_string()
                    }
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tuple_allow_tcp() {
        let rule = parse_tuple("allow tcp 22 0.0.0.0/0 any 0.0.0.0/0 in", false).unwrap();
        assert_eq!(rule.port, "22/tcp");
        assert_eq!(rule.action, Action::Allow);
        assert_eq!(rule.direction, Direction::In);
        assert_eq!(rule.from, "Anywhere");
        assert!(!rule.v6);
    }

    #[test]
    fn test_parse_tuple_deny_any() {
        let rule = parse_tuple("deny any 80 0.0.0.0/0 any 192.168.1.0/24 in", false).unwrap();
        assert_eq!(rule.port, "80");
        assert_eq!(rule.action, Action::Deny);
        assert_eq!(rule.from, "192.168.1.0/24");
    }

    #[test]
    fn test_parse_tuple_v6() {
        let rule = parse_tuple("allow tcp 443 ::/0 any ::/0 in", true).unwrap();
        assert_eq!(rule.port, "443/tcp");
        assert_eq!(rule.from, "Anywhere (v6)");
        assert!(rule.v6);
    }

    #[test]
    fn test_parse_tuple_short() {
        assert!(parse_tuple("allow tcp", false).is_none());
    }

    #[test]
    fn test_parse_app_profiles() {
        let content = r#"
[CUPS]
title=Common UNIX Printing System server
description=CUPS is a printing system with support for IPP
ports=631

[OpenSSH]
title=Secure Shell
description=OpenSSH server
ports=22/tcp
"#;
        let profiles = parse_app_profiles(content);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "CUPS");
        assert_eq!(profiles[0].ports, "631");
        assert_eq!(profiles[1].name, "OpenSSH");
        assert_eq!(profiles[1].ports, "22/tcp");
    }

    #[test]
    fn test_policy_from_str() {
        assert_eq!(Policy::from_str("ALLOW"), Some(Policy::Allow));
        assert_eq!(Policy::from_str("DROP"), Some(Policy::Deny));
        assert_eq!(Policy::from_str("REJECT"), Some(Policy::Reject));
        assert_eq!(Policy::from_str("invalid"), None);
    }
}
