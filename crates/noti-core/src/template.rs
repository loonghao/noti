use std::collections::HashMap;

use crate::error::NotiError;

/// A message template with variable placeholders.
///
/// Templates use `{{variable_name}}` syntax for variable substitution.
/// Nested braces and whitespace inside placeholders are supported:
/// `{{ name }}`, `{{name}}`, `{{ user.name }}` are all valid.
#[derive(Debug, Clone)]
pub struct MessageTemplate {
    /// Template name for identification.
    pub name: String,
    /// The template body text with `{{variable}}` placeholders.
    pub body: String,
    /// Optional title template.
    pub title: Option<String>,
    /// Default values for variables.
    pub defaults: HashMap<String, String>,
}

impl MessageTemplate {
    /// Create a new template with the given name and body.
    pub fn new(name: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            body: body.into(),
            title: None,
            defaults: HashMap::new(),
        }
    }

    /// Set an optional title template.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set a default value for a variable.
    pub fn with_default(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.defaults.insert(key.into(), value.into());
        self
    }

    /// Render the template body with the given variables.
    ///
    /// Variables in `vars` override defaults. Unknown placeholders are left as-is.
    pub fn render_body(&self, vars: &HashMap<String, String>) -> String {
        render_template(&self.body, vars, &self.defaults)
    }

    /// Render the title template, if present.
    pub fn render_title(&self, vars: &HashMap<String, String>) -> Option<String> {
        self.title
            .as_ref()
            .map(|t| render_template(t, vars, &self.defaults))
    }

    /// Extract variable names from the body and title templates.
    pub fn variables(&self) -> Vec<String> {
        let mut vars = extract_variables(&self.body);
        if let Some(ref title) = self.title {
            for v in extract_variables(title) {
                if !vars.contains(&v) {
                    vars.push(v);
                }
            }
        }
        vars
    }

    /// Validate that all required variables (those without defaults) are provided.
    pub fn validate_vars(&self, vars: &HashMap<String, String>) -> Result<(), NotiError> {
        let template_vars = self.variables();
        let missing: Vec<&str> = template_vars
            .iter()
            .filter(|v| !vars.contains_key(v.as_str()) && !self.defaults.contains_key(v.as_str()))
            .map(|v| v.as_str())
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(NotiError::Validation(format!(
                "template '{}' is missing variables: {}",
                self.name,
                missing.join(", ")
            )))
        }
    }

    /// Render into a [`crate::Message`] with the given variables.
    pub fn render(&self, vars: &HashMap<String, String>) -> crate::Message {
        let text = self.render_body(vars);
        let mut msg = crate::Message::text(text);
        if let Some(title) = self.render_title(vars) {
            msg = msg.with_title(title);
        }
        msg
    }
}

/// Replace `{{key}}` placeholders in `template` using `vars` then `defaults`.
fn render_template(
    template: &str,
    vars: &HashMap<String, String>,
    defaults: &HashMap<String, String>,
) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' && chars.peek() == Some(&'{') {
            // Consume second '{'
            chars.next();
            // Read until '}}'
            let mut var_name = String::new();
            let mut found_close = false;
            while let Some(inner) = chars.next() {
                if inner == '}' && chars.peek() == Some(&'}') {
                    chars.next(); // consume second '}'
                    found_close = true;
                    break;
                }
                var_name.push(inner);
            }

            if found_close {
                let key = var_name.trim();
                if let Some(val) = vars.get(key).or_else(|| defaults.get(key)) {
                    result.push_str(val);
                } else {
                    // Leave placeholder as-is for unknown variables
                    result.push_str("{{");
                    result.push_str(&var_name);
                    result.push_str("}}");
                }
            } else {
                // Unterminated placeholder — emit as-is
                result.push_str("{{");
                result.push_str(&var_name);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Extract variable names from `{{...}}` placeholders.
fn extract_variables(template: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find("}}") {
            let var = after_open[..end].trim().to_string();
            if !var.is_empty() && !vars.contains(&var) {
                vars.push(var);
            }
            rest = &after_open[end + 2..];
        } else {
            break;
        }
    }

    vars
}

/// A registry of named message templates.
#[derive(Debug, Clone, Default)]
pub struct TemplateRegistry {
    templates: HashMap<String, MessageTemplate>,
}

impl TemplateRegistry {
    /// Create an empty template registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a template.
    pub fn register(&mut self, template: MessageTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// Get a template by name.
    pub fn get(&self, name: &str) -> Option<&MessageTemplate> {
        self.templates.get(name)
    }

    /// Remove a template by name.
    pub fn remove(&mut self, name: &str) -> Option<MessageTemplate> {
        self.templates.remove(name)
    }

    /// List all template names.
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.templates.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Number of registered templates.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_render() {
        let tpl = MessageTemplate::new("test", "Hello, {{name}}!");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());
        assert_eq!(tpl.render_body(&vars), "Hello, World!");
    }

    #[test]
    fn test_render_with_whitespace() {
        let tpl = MessageTemplate::new("test", "Hello, {{ name }}!");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        assert_eq!(tpl.render_body(&vars), "Hello, Alice!");
    }

    #[test]
    fn test_render_with_defaults() {
        let tpl = MessageTemplate::new("test", "Hello, {{name}}! Welcome to {{place}}.")
            .with_default("place", "Earth");

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Bob".to_string());
        assert_eq!(tpl.render_body(&vars), "Hello, Bob! Welcome to Earth.");
    }

    #[test]
    fn test_render_vars_override_defaults() {
        let tpl = MessageTemplate::new("test", "Go to {{place}}").with_default("place", "Earth");

        let mut vars = HashMap::new();
        vars.insert("place".to_string(), "Mars".to_string());
        assert_eq!(tpl.render_body(&vars), "Go to Mars");
    }

    #[test]
    fn test_unknown_placeholder_preserved() {
        let tpl = MessageTemplate::new("test", "Hello, {{name}}! {{unknown}}");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Eve".to_string());
        assert_eq!(tpl.render_body(&vars), "Hello, Eve! {{unknown}}");
    }

    #[test]
    fn test_extract_variables() {
        let tpl = MessageTemplate::new("test", "{{greeting}}, {{ name }}! Your code is {{code}}.");
        let vars = tpl.variables();
        assert_eq!(vars, vec!["greeting", "name", "code"]);
    }

    #[test]
    fn test_extract_variables_with_title() {
        let tpl = MessageTemplate::new("test", "Body: {{a}}").with_title("Title: {{b}}");
        let vars = tpl.variables();
        assert!(vars.contains(&"a".to_string()));
        assert!(vars.contains(&"b".to_string()));
    }

    #[test]
    fn test_validate_vars_ok() {
        let tpl = MessageTemplate::new("test", "{{a}} and {{b}}").with_default("b", "default_b");

        let mut vars = HashMap::new();
        vars.insert("a".to_string(), "val_a".to_string());
        assert!(tpl.validate_vars(&vars).is_ok());
    }

    #[test]
    fn test_validate_vars_missing() {
        let tpl = MessageTemplate::new("test", "{{a}} and {{b}}");
        let vars = HashMap::new();
        let err = tpl.validate_vars(&vars);
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("a"));
        assert!(msg.contains("b"));
    }

    #[test]
    fn test_render_message() {
        let tpl = MessageTemplate::new("alert", "Alert: {{message}}").with_title("{{level}} Alert");

        let mut vars = HashMap::new();
        vars.insert("message".to_string(), "disk full".to_string());
        vars.insert("level".to_string(), "CRITICAL".to_string());

        let msg = tpl.render(&vars);
        assert_eq!(msg.text, "Alert: disk full");
        assert_eq!(msg.title, Some("CRITICAL Alert".to_string()));
    }

    #[test]
    fn test_no_placeholders() {
        let tpl = MessageTemplate::new("static", "No variables here.");
        let vars = HashMap::new();
        assert_eq!(tpl.render_body(&vars), "No variables here.");
    }

    #[test]
    fn test_multiple_same_variable() {
        let tpl = MessageTemplate::new("test", "{{x}} and {{x}} again");
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), "val".to_string());
        assert_eq!(tpl.render_body(&vars), "val and val again");
    }

    #[test]
    fn test_template_registry() {
        let mut reg = TemplateRegistry::new();
        assert!(reg.is_empty());

        reg.register(MessageTemplate::new("alert", "Alert: {{msg}}"));
        reg.register(MessageTemplate::new("info", "Info: {{msg}}"));

        assert_eq!(reg.len(), 2);
        assert!(reg.get("alert").is_some());
        assert!(reg.get("info").is_some());
        assert!(reg.get("missing").is_none());

        let names = reg.names();
        assert_eq!(names, vec!["alert", "info"]);

        reg.remove("alert");
        assert_eq!(reg.len(), 1);
        assert!(reg.get("alert").is_none());
    }

    #[test]
    fn test_empty_placeholder() {
        let tpl = MessageTemplate::new("test", "before {{}} after");
        let vars = HashMap::new();
        // Empty placeholder preserved as-is
        assert_eq!(tpl.render_body(&vars), "before {{}} after");
    }

    #[test]
    fn test_adjacent_placeholders() {
        let tpl = MessageTemplate::new("test", "{{a}}{{b}}");
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), "X".to_string());
        vars.insert("b".to_string(), "Y".to_string());
        assert_eq!(tpl.render_body(&vars), "XY");
    }
}
