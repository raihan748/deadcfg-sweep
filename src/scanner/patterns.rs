use std::path::Path;
use regex::Regex;

/// Supported source code languages for config reference scanning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceLanguage {
    Rust,
    Go,
    Python,
    TypeScript,
    JavaScript,
    Java,
    CSharp,
    Shell,
    Ruby,
    Php,
    Unknown,
}

impl SourceLanguage {
    /// Detect programming language from file extension.
    pub fn detect(path: &Path) -> Self {
        let ext = match path.extension().and_then(|s| s.to_str()) {
            Some(e) => e.to_lowercase(),
            None => {
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if name.ends_with("rc") || name == "Dockerfile" || name.starts_with(".env") {
                    return SourceLanguage::Shell;
                }
                return SourceLanguage::Unknown;
            }
        };

        match ext.as_str() {
            "rs" => SourceLanguage::Rust,
            "go" => SourceLanguage::Go,
            "py" => SourceLanguage::Python,
            "ts" | "tsx" => SourceLanguage::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => SourceLanguage::JavaScript,
            "java" | "kt" | "scala" => SourceLanguage::Java,
            "cs" => SourceLanguage::CSharp,
            "sh" | "bash" | "zsh" => SourceLanguage::Shell,
            "rb" => SourceLanguage::Ruby,
            "php" => SourceLanguage::Php,
            _ => SourceLanguage::Unknown,
        }
    }
}

/// Pre-compiled programmatic access patterns for common programming languages.
pub struct LanguagePatterns {
    pub language: SourceLanguage,
    pub api_patterns: Vec<Regex>,
}

impl LanguagePatterns {
    /// Instantiate regex patterns appropriate for the given programming language.
    pub fn for_language(lang: SourceLanguage) -> Self {
        let mut patterns = Vec::new();

        match lang {
            SourceLanguage::Rust => {
                // std::env::var("KEY"), env::var("KEY"), env::var_os("KEY")
                if let Ok(re) = Regex::new(r#"(?:std::)?env::(?:var|var_os)\s*\(\s*["']([^"']+)["']\s*\)"#) {
                    patterns.push(re);
                }
                // config.get("KEY"), config.get_string("KEY")
                if let Ok(re) = Regex::new(r#"config\.(?:get|get_string|get_int|get_bool)\s*\(\s*["']([^"']+)["']\s*\)"#) {
                    patterns.push(re);
                }
            }
            SourceLanguage::Go => {
                // os.Getenv("KEY"), os.LookupEnv("KEY")
                if let Ok(re) = Regex::new(r#"os\.(?:Getenv|LookupEnv)\s*\(\s*["']([^"']+)["']\s*\)"#) {
                    patterns.push(re);
                }
                // viper.GetString("KEY"), viper.Get("KEY")
                if let Ok(re) = Regex::new(r#"viper\.(?:Get|GetString|GetInt|GetBool)\s*\(\s*["']([^"']+)["']\s*\)"#) {
                    patterns.push(re);
                }
                // config.Get("KEY")
                if let Ok(re) = Regex::new(r#"config\.(?:Get|GetString)\s*\(\s*["']([^"']+)["']\s*\)"#) {
                    patterns.push(re);
                }
            }
            SourceLanguage::Python => {
                // os.environ.get("KEY"), os.getenv("KEY"), os.environ["KEY"]
                if let Ok(re) = Regex::new(r#"os\.(?:environ\.get|getenv)\s*\(\s*["']([^"']+)["']"#) {
                    patterns.push(re);
                }
                if let Ok(re) = Regex::new(r#"os\.environ\s*\[\s*["']([^"']+)["']\s*\]"#) {
                    patterns.push(re);
                }
                // settings.KEY
                if let Ok(re) = Regex::new(r#"settings\.([A-Za-z0-9_]+)"#) {
                    patterns.push(re);
                }
                // config.get("KEY")
                if let Ok(re) = Regex::new(r#"config\.get\s*\(\s*["']([^"']+)["']"#) {
                    patterns.push(re);
                }
            }
            SourceLanguage::TypeScript | SourceLanguage::JavaScript => {
                // process.env.KEY
                if let Ok(re) = Regex::new(r#"process\.env\.([A-Za-z0-9_]+)"#) {
                    patterns.push(re);
                }
                // process.env["KEY"]
                if let Ok(re) = Regex::new(r#"process\.env\s*\[\s*["']([^"']+)["']\s*\]"#) {
                    patterns.push(re);
                }
                // import.meta.env.KEY
                if let Ok(re) = Regex::new(r#"import\.meta\.env\.([A-Za-z0-9_]+)"#) {
                    patterns.push(re);
                }
                // config.get("KEY")
                if let Ok(re) = Regex::new(r#"config\.get\s*\(\s*["']([^"']+)["']\s*\)"#) {
                    patterns.push(re);
                }
            }
            SourceLanguage::Java => {
                // System.getenv("KEY"), System.getProperty("KEY")
                if let Ok(re) = Regex::new(r#"System\.(?:getenv|getProperty)\s*\(\s*["']([^"']+)["']\s*\)"#) {
                    patterns.push(re);
                }
                // environment.getProperty("KEY")
                if let Ok(re) = Regex::new(r#"environment\.getProperty\s*\(\s*["']([^"']+)["']\s*\)"#) {
                    patterns.push(re);
                }
            }
            SourceLanguage::CSharp => {
                // Environment.GetEnvironmentVariable("KEY")
                if let Ok(re) = Regex::new(r#"Environment\.GetEnvironmentVariable\s*\(\s*["']([^"']+)["']\s*\)"#) {
                    patterns.push(re);
                }
                // Configuration["KEY"], builder.Configuration["KEY"]
                if let Ok(re) = Regex::new(r#"Configuration\s*\[\s*["']([^"']+)["']\s*\]"#) {
                    patterns.push(re);
                }
            }
            SourceLanguage::Shell => {
                // $KEY or ${KEY}
                if let Ok(re) = Regex::new(r#"\$\{?([A-Za-z_][A-Za-z0-9_]*)\}?"#) {
                    patterns.push(re);
                }
            }
            _ => {
                // Generic string literal capture: "KEY" or 'KEY'
                if let Ok(re) = Regex::new(r#"["']([A-Za-z0-9_.-]+)["']"#) {
                    patterns.push(re);
                }
            }
        }

        Self {
            language: lang,
            api_patterns: patterns,
        }
    }
}
