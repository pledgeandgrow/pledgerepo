// Prisma plugin — Prisma client generation and query logging in dev.
//
// Features:
//   - Parse schema.prisma files (datasource, generator, model blocks)
//   - Generate TypeScript client types from Prisma schema
//   - Dev-mode query logging middleware
//   - Schema validation and warnings

use anyhow::Result;

/// Parsed Prisma schema
#[derive(Debug, Clone, Default)]
pub struct PrismaSchema {
    pub datasource: Option<PrismaDatasource>,
    pub generator: Option<PrismaGenerator>,
    pub models: Vec<PrismaModel>,
    pub enums: Vec<PrismaEnum>,
}

#[derive(Debug, Clone, Default)]
pub struct PrismaDatasource {
    pub provider: String,
    pub url: String,
}

#[derive(Debug, Clone, Default)]
pub struct PrismaGenerator {
    pub provider: String,
    pub output: String,
}

#[derive(Debug, Clone, Default)]
pub struct PrismaModel {
    pub name: String,
    pub fields: Vec<PrismaField>,
    pub map: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PrismaField {
    pub name: String,
    pub type_name: String,
    pub is_optional: bool,
    pub is_list: bool,
    pub is_id: bool,
    pub is_unique: bool,
    pub is_autoincrement: bool,
    pub is_updated_at: bool,
    pub default: Option<String>,
    pub relation: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PrismaEnum {
    pub name: String,
    pub values: Vec<String>,
}

/// Parse a Prisma schema file
pub fn parse_schema(source: &str) -> Result<PrismaSchema> {
    let mut schema = PrismaSchema::default();
    let mut current_block: Option<(String, String)> = None; // (block_type, block_name)
    let mut current_model: Option<PrismaModel> = None;
    let mut current_enum: Option<PrismaEnum> = None;

    for line in source.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        // Block start: `model User {` or `datasource db {` or `enum Status {`
        if line.ends_with('{') {
            let parts: Vec<&str> = line.strip_suffix('{').unwrap_or(line).trim().splitn(2, ' ').collect();
            if parts.len() >= 2 {
                current_block = Some((parts[0].to_string(), parts[1].to_string()));
                match parts[0] {
                    "model" => {
                        current_model = Some(PrismaModel {
                            name: parts[1].to_string(),
                            ..Default::default()
                        });
                    }
                    "enum" => {
                        current_enum = Some(PrismaEnum {
                            name: parts[1].to_string(),
                            ..Default::default()
                        });
                    }
                    "datasource" => {
                        schema.datasource = Some(PrismaDatasource::default());
                    }
                    "generator" => {
                        schema.generator = Some(PrismaGenerator::default());
                    }
                    _ => {}
                }
            }
            continue;
        }

        // Block end: `}`
        if line == "}" {
            if let Some((block_type, _)) = current_block.take() {
                match block_type.as_str() {
                    "model" => {
                        if let Some(model) = current_model.take() {
                            schema.models.push(model);
                        }
                    }
                    "enum" => {
                        if let Some(en) = current_enum.take() {
                            schema.enums.push(en);
                        }
                    }
                    _ => {}
                }
            }
            continue;
        }

        // Inside a block
        if let Some((block_type, _)) = &current_block {
            match block_type.as_str() {
                "datasource" => {
                    if let Some(ref mut ds) = schema.datasource {
                        if let Some(url) = parse_field_value(line, "url") {
                            ds.url = url;
                        } else if let Some(provider) = parse_field_value(line, "provider") {
                            ds.provider = provider;
                        }
                    }
                }
                "generator" => {
                    if let Some(ref mut generator) = schema.generator {
                        if let Some(provider) = parse_field_value(line, "provider") {
                            generator.provider = provider;
                        } else if let Some(output) = parse_field_value(line, "output") {
                            generator.output = output;
                        }
                    }
                }
                "model" => {
                    if let Some(ref mut model) = current_model {
                        if line.starts_with("@@map(") {
                            model.map = parse_paren_content(line);
                        } else if !line.starts_with("@@") {
                            if let Some(field) = parse_field_line(line) {
                                model.fields.push(field);
                            }
                        }
                    }
                }
                "enum" => {
                    if let Some(ref mut en) = current_enum {
                        if !line.starts_with("//") && !line.is_empty() {
                            en.values.push(line.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(schema)
}

fn parse_field_value(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{} = ", key);
    if line.starts_with(&prefix) {
        let val = line[prefix.len()..].trim();
        // Strip quotes and env() wrapper
        if val.starts_with("env(") {
            return parse_paren_content(val);
        }
        Some(val.trim_matches('"').to_string())
    } else {
        None
    }
}

fn parse_paren_content(s: &str) -> Option<String> {
    if let Some(start) = s.find('(') {
        if let Some(end) = s.rfind(')') {
            let inner = s[start + 1..end].trim();
            return Some(inner.trim_matches('"').to_string());
        }
    }
    None
}

fn parse_field_line(line: &str) -> Option<PrismaField> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let name = parts[0].to_string();
    let type_str = parts[1];
    let is_optional = type_str.ends_with('?');
    let is_list = type_str.ends_with("[]");
    let type_name = type_str.trim_end_matches('?').trim_end_matches("[]").to_string();

    let mut field = PrismaField {
        name,
        type_name,
        is_optional,
        is_list,
        ..Default::default()
    };

    // Parse attributes
    for attr in &parts[2..] {
        match *attr {
            "@id" => field.is_id = true,
            "@unique" => field.is_unique = true,
            "@updatedAt" => field.is_updated_at = true,
            "@default(autoincrement())" => field.is_autoincrement = true,
            _ => {
                if attr.starts_with("@default(") {
                    field.default = parse_paren_content(attr);
                } else if attr.starts_with("@relation(") {
                    field.relation = parse_paren_content(attr);
                }
            }
        }
    }

    Some(field)
}

/// Generate TypeScript types from a parsed Prisma schema
pub fn generate_types(schema: &PrismaSchema) -> String {
    let mut ts = String::new();

    ts.push_str("// Auto-generated by PledgePack Prisma plugin\n\n");

    // Generate enums
    for en in &schema.enums {
        ts.push_str(&format!("export enum {} {{\n", en.name));
        for value in &en.values {
            ts.push_str(&format!("  {} = \"{}\",\n", value, value));
        }
        ts.push_str("}\n\n");
    }

    // Generate model interfaces
    for model in &schema.models {
        ts.push_str(&format!("export interface {} {{\n", model.name));
        for field in &model.fields {
            let ts_type = prisma_type_to_ts(&field.type_name, field.is_list, field.is_optional);
            ts.push_str(&format!("  {}: {};\n", field.name, ts_type));
        }
        ts.push_str("}\n\n");

        // Generate input types for create/update
        ts.push_str(&format!("export interface {}CreateInput {{\n", model.name));
        for field in &model.fields {
            if field.is_autoincrement {
                continue;
            }
            let ts_type = prisma_type_to_ts(&field.type_name, field.is_list, field.is_optional || field.default.is_some());
            ts.push_str(&format!("  {}: {};\n", field.name, ts_type));
        }
        ts.push_str("}\n\n");
    }

    // Generate PrismaClient type
    ts.push_str("export interface PrismaClient {\n");
    for model in &schema.models {
        let lower = to_camel_case(&model.name);
        ts.push_str(&format!("  {}: PrismaModel<{}>;\n", lower, model.name));
    }
    ts.push_str("}\n\n");

    ts.push_str("export interface PrismaModel<T> {\n");
    ts.push_str("  findUnique(args: { where: { id: string } }): Promise<T | null>;\n");
    ts.push_str("  findMany(args?: { where?: Partial<T>; take?: number; skip?: number }): Promise<T[]>;\n");
    ts.push_str("  create(args: { data: T & Record<string, unknown> }): Promise<T>;\n");
    ts.push_str("  update(args: { where: { id: string }; data: Partial<T> }): Promise<T>;\n");
    ts.push_str("  delete(args: { where: { id: string } }): Promise<T>;\n");
    ts.push_str("  count(args?: { where?: Partial<T> }): Promise<number>;\n");
    ts.push_str("}\n");

    ts
}

fn prisma_type_to_ts(type_name: &str, is_list: bool, is_optional: bool) -> String {
    let base = match type_name {
        "String" => "string",
        "Int" => "number",
        "BigInt" => "bigint",
        "Float" => "number",
        "Decimal" => "number",
        "Boolean" => "boolean",
        "DateTime" => "Date",
        "Json" => "Record<string, unknown>",
        "Bytes" => "Uint8Array",
        _ => type_name, // Custom type (model reference or enum)
    };

    let mut ts_type = if is_list {
        format!("{}[]", base)
    } else {
        base.to_string()
    };

    if is_optional {
        ts_type.push_str(" | null");
    }

    ts_type
}

fn to_camel_case(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }
    let mut result = String::new();
    let mut next_upper = false;
    for (i, c) in s.chars().enumerate() {
        if i == 0 {
            result.push(c.to_lowercase().next().unwrap_or(c));
        } else if c == '_' {
            next_upper = true;
        } else if next_upper {
            result.push(c.to_uppercase().next().unwrap_or(c));
            next_upper = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Generate dev-mode query logging middleware code
pub fn generate_query_logger() -> String {
    r#"// PledgePack Prisma query logger (dev mode)
// Wraps Prisma client $use middleware to log queries with timing
export function createQueryLogger(prisma: any) {
  prisma.$use(async (params: any, next: any) => {
    const start = performance.now();
    const result = await next(params);
    const duration = (performance.now() - start).toFixed(2);
    const model = params.model || "unknown";
    const action = params.action;
    console.log(`[prisma] ${model}.${action} — ${duration}ms`);
    return result;
  });
  return prisma;
}
"#.to_string()
}

/// Validate a Prisma schema and return warnings
pub fn validate_schema(schema: &PrismaSchema) -> Vec<String> {
    let mut warnings = Vec::new();

    if schema.datasource.is_none() {
        warnings.push("No datasource block found in schema.prisma".to_string());
    } else if let Some(ref ds) = schema.datasource {
        if ds.provider.is_empty() {
            warnings.push("Datasource provider is not specified".to_string());
        }
        if ds.url.is_empty() {
            warnings.push("Datasource url is not specified".to_string());
        }
    }

    if schema.generator.is_none() {
        warnings.push("No generator block found — client will not be generated".to_string());
    }

    // Check for models without ID fields
    for model in &schema.models {
        if !model.fields.iter().any(|f| f.is_id) {
            warnings.push(format!("Model '{}' has no @id field", model.name));
        }
    }

    if schema.models.is_empty() {
        warnings.push("No models defined in schema".to_string());
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_schema() {
        let schema = r#"
datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

generator client {
  provider = "prisma-client-js"
}

model User {
  id        Int      @id @default(autoincrement())
  email     String   @unique
  name      String?
  posts     Post[]
}

model Post {
  id       Int    @id @default(autoincrement())
  title    String
  author   User   @relation(fields: [authorId], references: [id])
  authorId Int
}

enum Role {
  ADMIN
  USER
}
"#;
        let result = parse_schema(schema);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.models.len(), 2);
        assert_eq!(parsed.enums.len(), 1);
        assert!(parsed.datasource.is_some());
        assert!(parsed.generator.is_some());

        let user = &parsed.models[0];
        assert_eq!(user.name, "User");
        assert!(user.fields.iter().any(|f| f.is_id));
        assert!(user.fields.iter().any(|f| f.name == "email" && f.is_unique));
    }

    #[test]
    fn test_generate_types() {
        let schema = PrismaSchema {
            models: vec![PrismaModel {
                name: "User".to_string(),
                fields: vec![
                    PrismaField { name: "id".to_string(), type_name: "Int".to_string(), is_id: true, is_autoincrement: true, ..Default::default() },
                    PrismaField { name: "email".to_string(), type_name: "String".to_string(), is_unique: true, ..Default::default() },
                    PrismaField { name: "name".to_string(), type_name: "String".to_string(), is_optional: true, ..Default::default() },
                ],
                ..Default::default()
            }],
            enums: vec![PrismaEnum {
                name: "Role".to_string(),
                values: vec!["ADMIN".to_string(), "USER".to_string()],
            }],
            ..Default::default()
        };

        let types = generate_types(&schema);
        assert!(types.contains("export enum Role"));
        assert!(types.contains("export interface User"));
        assert!(types.contains("export interface UserCreateInput"));
        assert!(types.contains("export interface PrismaClient"));
    }

    #[test]
    fn test_validate_schema() {
        let schema = PrismaSchema::default();
        let warnings = validate_schema(&schema);
        assert!(warnings.iter().any(|w| w.contains("datasource")));
        assert!(warnings.iter().any(|w| w.contains("generator")));
        assert!(warnings.iter().any(|w| w.contains("No models")));
    }
}
