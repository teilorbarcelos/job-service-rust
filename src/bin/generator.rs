#![allow(clippy::all)]
use dialoguer::{theme::ColorfulTheme, Select};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

struct Field {
    name: String,
    rust_type: String,
    is_text: bool,
    is_nullable: bool,
}

impl Field {
    fn base_type(&self) -> &str {
        if self.rust_type.starts_with("Option<") {
            let len = self.rust_type.len();
            &self.rust_type[7..len - 1]
        } else {
            &self.rust_type
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && (args[1] == "-h" || args[1] == "--help") {
        print_usage();
        return;
    }

    let (entity_name, fields) = if args.len() < 2 {
        println!("📖 Mage Backend - CLI CRUD Generator (Rust)");
        println!("Nenhum parâmetro fornecido. Iniciando modo interativo...\n");

        let ent_name = prompt_entity_name();
        let flds = prompt_fields();
        (ent_name, flds)
    } else {
        let ent_name = args[1].clone();
        let flds = parse_fields(&args[2..]);
        (ent_name, flds)
    };

    let entity_slug = entity_name.to_lowercase();

    println!("🛠️  Iniciando geração do CRUD para '{}'...", entity_name);

    let mut register_rbac = String::new();
    print!("\n🛡️  Deseja registrar esta feature no RBAC (bootstrap.rs)? [S/n]: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut register_rbac).unwrap();
    let register_rbac = register_rbac.trim().to_lowercase();
    let should_register_rbac =
        register_rbac.is_empty() || register_rbac == "s" || register_rbac == "sim";

    let mut feature_id = entity_slug.clone();
    let mut feature_name = entity_name.clone();
    let mut feature_desc = format!("Gestão de {}", entity_name);

    if should_register_rbac {
        let mut in_id = String::new();
        print!("  ID da Feature (padrão: {}): ", feature_id);
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut in_id).unwrap();
        let in_id = in_id.trim();
        if !in_id.is_empty() {
            feature_id = in_id.to_string();
        }

        let mut in_name = String::new();
        print!("  Nome da Feature (padrão: {}): ", feature_name);
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut in_name).unwrap();
        let in_name = in_name.trim();
        if !in_name.is_empty() {
            feature_name = in_name.to_string();
        }

        let mut in_desc = String::new();
        print!("  Descrição da Feature (padrão: {}): ", feature_desc);
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut in_desc).unwrap();
        let in_desc = in_desc.trim();
        if !in_desc.is_empty() {
            feature_desc = in_desc.to_string();
        }
    }

    let fields_model = generate_fields_model(&fields);
    let fields_struct_create = generate_fields_create(&fields);
    let fields_struct_update = generate_fields_update(&fields);
    let fields_struct_response = generate_fields_response(&fields);
    let response_from_model_mappings = generate_response_from_model_mappings(&fields);
    let service_list_filter_definitions =
        generate_service_list_filter_definitions(&fields, &entity_slug);
    let service_list_search_definitions =
        generate_service_list_search_definitions(&fields, &entity_slug);
    let service_list_order_definitions =
        generate_service_list_order_definitions(&fields, &entity_slug);
    let service_create_fields_mappings = generate_service_create_fields_mappings(&fields);
    let service_update_fields_mappings = generate_service_update_fields_mappings(&fields);
    let controller_search_fields = generate_controller_search_fields(&fields);
    let controller_filter_fields = generate_controller_filter_fields(&fields);
    let create_payload_json = generate_create_payload_json(&fields);
    let assert_create_fields_json = generate_assert_create_fields_json(&fields);
    let update_payload_json = generate_update_payload_json(&fields);
    let assert_update_fields_json = generate_assert_update_fields_json(&fields);
    let migration_fields_sql = generate_migration_fields_sql(&fields);

    let replacements = vec![
        ("{{EntityName}}", entity_name.as_str()),
        ("{{entity_slug}}", entity_slug.as_str()),
        ("{{FeatureName}}", feature_name.as_str()),
        ("{{FeatureDesc}}", feature_desc.as_str()),
        ("{{FeatureId}}", feature_id.as_str()),
        ("{{FieldsModel}}", fields_model.as_str()),
        ("{{FieldsStructCreate}}", fields_struct_create.as_str()),
        ("{{FieldsStructUpdate}}", fields_struct_update.as_str()),
        ("{{FieldsStructResponse}}", fields_struct_response.as_str()),
        (
            "{{ResponseFromModelMappings}}",
            response_from_model_mappings.as_str(),
        ),
        (
            "{{ServiceListFilterDefinitions}}",
            service_list_filter_definitions.as_str(),
        ),
        (
            "{{ServiceListSearchDefinitions}}",
            service_list_search_definitions.as_str(),
        ),
        (
            "{{ServiceListOrderDefinitions}}",
            service_list_order_definitions.as_str(),
        ),
        (
            "{{ServiceCreateFieldsMappings}}",
            service_create_fields_mappings.as_str(),
        ),
        (
            "{{ServiceUpdateFieldsMappings}}",
            service_update_fields_mappings.as_str(),
        ),
        (
            "{{ControllerSearchFields}}",
            controller_search_fields.as_str(),
        ),
        (
            "{{ControllerFilterFields}}",
            controller_filter_fields.as_str(),
        ),
        ("{{CreatePayloadJson}}", create_payload_json.as_str()),
        (
            "{{AssertCreateFieldsJson}}",
            assert_create_fields_json.as_str(),
        ),
        ("{{UpdatePayloadJson}}", update_payload_json.as_str()),
        (
            "{{AssertUpdateFieldsJson}}",
            assert_update_fields_json.as_str(),
        ),
        ("{{MigrationFieldsSql}}", migration_fields_sql.as_str()),
    ];

    let module_dir = format!("src/modules/{}", entity_slug);
    if !Path::new(&module_dir).exists() {
        fs::create_dir_all(&module_dir).expect("Falha ao criar diretório do módulo");
    }

    write_from_template(
        "templates/crud/model.rs.tpl",
        &format!("src/models/{}.rs", entity_slug),
        &replacements,
    );
    write_from_template(
        "templates/crud/schemas.rs.tpl",
        &format!("src/modules/{}/schemas.rs", entity_slug),
        &replacements,
    );
    write_from_template(
        "templates/crud/service.rs.tpl",
        &format!("src/modules/{}/service.rs", entity_slug),
        &replacements,
    );
    write_from_template(
        "templates/crud/controller.rs.tpl",
        &format!("src/modules/{}/controller.rs", entity_slug),
        &replacements,
    );
    write_from_template(
        "templates/crud/routes.rs.tpl",
        &format!("src/modules/{}/routes.rs", entity_slug),
        &replacements,
    );
    write_from_template(
        "templates/crud/mod.rs.tpl",
        &format!("src/modules/{}/mod.rs", entity_slug),
        &replacements,
    );

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let migration_name = format!("m{}_create_{}_table", timestamp, entity_slug);
    write_from_template(
        "templates/crud/migration.rs.tpl",
        &format!("src/migration/{}.rs", migration_name),
        &replacements,
    );
    register_migration_in_migrator(&migration_name);

    let next_test_idx = find_next_test_index();
    let test_mod_name = format!("t{:02}_{}", next_test_idx, entity_slug);
    write_from_template(
        "templates/crud/test.rs.tpl",
        &format!("tests/compliance/{}.rs", test_mod_name),
        &replacements,
    );
    register_test_module(&test_mod_name);
    register_test_execution(&test_mod_name);
    register_table_truncate(&entity_name);

    register_model(&entity_slug);
    register_module(&entity_name, &entity_slug);
    register_observability(&entity_name, &entity_slug, &feature_name, &feature_desc);

    if should_register_rbac {
        register_rbac_feature(&feature_id, &feature_name, &feature_desc);
    }

    println!(
        "\n✅ CRUD gerado com sucesso para a feature '{}'!",
        entity_name
    );
    println!("💡 Dica: Rode 'cargo build' ou 'cargo test' para validar a compilação e rodar a cobertura.");
}

fn print_usage() {
    println!("📖 Mage Backend - CLI CRUD Generator (Rust)");
    println!("Uso:");
    println!("  cargo run --bin generator <NomeEntidade> [campo:tipo ...]");
    println!("\nTipos suportados: string, text, int, bool, decimal, float, date");
    println!("\nExemplo:");
    println!("  cargo run --bin generator Customer name:string bio:text active:bool price:decimal");
}

fn prompt_entity_name() -> String {
    loop {
        print!("✍️  Digite o nome da Entidade (ex: Customer): ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_string();
        if !input.is_empty() {
            return input;
        }
        println!("⚠️  O nome da entidade é obrigatório.");
    }
}

fn prompt_fields() -> Vec<Field> {
    let mut fields = Vec::new();
    println!("\n📝 Definição de campos para a entidade.");
    println!("(Campos padrão como 'id', 'active', 'created_at', 'updated_at', etc. são adicionados automaticamente)\n");

    loop {
        print!("  Nome do campo (ou deixe em branco para finalizar): ");
        io::stdout().flush().unwrap();
        let mut name = String::new();
        io::stdin().read_line(&mut name).unwrap();
        let name = name.trim().to_string();
        if name.is_empty() {
            break;
        }

        let name_lower = name.to_lowercase();
        if name_lower == "id"
            || name_lower == "active"
            || name_lower == "created_at"
            || name_lower == "updated_at"
            || name_lower == "is_deleted"
            || name_lower == "deleted_at"
        {
            println!(
                "  ⚠️  O campo '{}' já é adicionado automaticamente pelo core.",
                name
            );
            continue;
        }

        let types = vec!["string", "text", "int", "bool", "decimal", "float", "date"];
        let type_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Selecione o tipo do campo '{}'", name))
            .items(&types)
            .default(0)
            .interact()
            .unwrap();

        let raw_type = types[type_selection];
        let is_text = raw_type == "text";

        let nullability_options = vec!["Nullable (opcional)", "Not Null (obrigatório)"];
        let nullability_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Defina a obrigatoriedade")
            .items(&nullability_options)
            .default(0) // Default is Nullable (as user requested: "com padrão nullable")
            .interact()
            .unwrap();

        let is_nullable = nullability_selection == 0;

        let base_rust_type = match raw_type {
            "int" => "i32",
            "bool" => "bool",
            "decimal" => "Decimal",
            "float" => "f64",
            "date" => "DateTimeWithTimeZone",
            _ => "String",
        };

        let rust_type = if is_nullable {
            format!("Option<{}>", base_rust_type)
        } else {
            base_rust_type.to_string()
        };

        fields.push(Field {
            name,
            rust_type,
            is_text,
            is_nullable,
        });
        println!();
    }
    fields
}

fn parse_fields(args: &[String]) -> Vec<Field> {
    let mut fields = Vec::new();
    for arg in args {
        let parts: Vec<&str> = arg.split(':').collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[0].to_string();
        let name_lower = name.to_lowercase();
        if name_lower == "id"
            || name_lower == "active"
            || name_lower == "created_at"
            || name_lower == "updated_at"
            || name_lower == "is_deleted"
            || name_lower == "deleted_at"
        {
            continue;
        }
        let raw_type = parts[1].to_lowercase();

        let is_nullable = if parts.len() >= 3 {
            let n_part = parts[2].to_lowercase();
            n_part != "notnull" && n_part != "required"
        } else {
            true // default to nullable
        };

        let is_text = raw_type == "text";

        let base_rust_type = match raw_type.as_str() {
            "int" => "i32",
            "bool" => "bool",
            "decimal" => "Decimal",
            "float" => "f64",
            "date" => "DateTimeWithTimeZone",
            _ => "String",
        };

        let rust_type = if is_nullable {
            format!("Option<{}>", base_rust_type)
        } else {
            base_rust_type.to_string()
        };

        fields.push(Field {
            name,
            rust_type,
            is_text,
            is_nullable,
        });
    }
    fields
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    if pascal.is_empty() {
        return pascal;
    }
    let mut chars = pascal.chars();
    chars.next().unwrap().to_lowercase().collect::<String>() + chars.as_str()
}

fn write_from_template(tpl_path: &str, out_path: &str, replacements: &[(&str, &str)]) {
    let tpl_content = fs::read_to_string(tpl_path)
        .unwrap_or_else(|_| panic!("Template {} não encontrado", tpl_path));
    let mut out_content = tpl_content;
    for (placeholder, value) in replacements {
        out_content = out_content.replace(placeholder, value);
    }
    fs::write(out_path, out_content).unwrap_or_else(|_| panic!("Falha ao salvar {}", out_path));
    println!("  📄 [NEW] {}", out_path);
}

fn generate_fields_model(fields: &[Field]) -> String {
    let mut s = String::new();
    s.push_str("    #[sea_orm(primary_key, auto_increment = false)]\n");
    s.push_str("    pub id: String,\n");
    for f in fields {
        s.push_str(&format!("    pub {}: {},\n", f.name, f.rust_type));
    }
    s.push_str("    pub active: bool,\n");
    s.push_str("    pub created_at: DateTimeWithTimeZone,\n");
    s.push_str("    pub updated_at: DateTimeWithTimeZone,\n");
    s.push_str("    pub is_deleted: Option<bool>,\n");
    s.push_str("    pub deleted_at: Option<DateTimeWithTimeZone>,\n");
    s
}

fn generate_fields_create(fields: &[Field]) -> String {
    let mut s = String::new();
    for f in fields {
        s.push_str(&format!("    pub {}: {},\n", f.name, f.rust_type));
    }
    s
}

fn generate_fields_update(fields: &[Field]) -> String {
    let mut s = String::new();
    for f in fields {
        s.push_str(&format!("    pub {}: {},\n", f.name, f.rust_type));
    }
    s.push_str("    pub active: Option<bool>,\n");
    s
}

fn generate_fields_response(fields: &[Field]) -> String {
    let mut s = String::new();
    s.push_str("    pub id: String,\n");
    for f in fields {
        s.push_str(&format!("    pub {}: {},\n", f.name, f.rust_type));
    }
    s.push_str("    pub active: bool,\n");
    s.push_str("    pub created_at: String,\n");
    s.push_str("    pub updated_at: String,\n");
    s
}

fn generate_response_from_model_mappings(fields: &[Field]) -> String {
    let mut s = String::new();
    s.push_str("            id: p.id,\n");
    for f in fields {
        s.push_str(&format!("            {}: p.{},\n", f.name, f.name));
    }
    s.push_str("            active: p.active,\n");
    s.push_str("            created_at: p.created_at.to_rfc3339(),\n");
    s.push_str("            updated_at: p.updated_at.to_rfc3339(),\n");
    s
}

fn generate_service_list_filter_definitions(fields: &[Field], _slug: &str) -> String {
    let mut s = String::new();
    for f in fields {
        let camel = to_camel_case(&f.name);
        let pascal = to_pascal_case(&f.name);
        if f.base_type() == "DateTimeWithTimeZone" {
        } else if f.base_type() == "String" {
            s.push_str(&format!(
                "            FilterDefinition::contains(\"{}\", (Entity, Column::{})),\n",
                camel, pascal
            ));
        } else if f.base_type() == "bool" {
            s.push_str(&format!(
                "            FilterDefinition::boolean(\"{}\", (Entity, Column::{})),\n",
                camel, pascal
            ));
        } else {
            s.push_str(&format!(
                "            FilterDefinition::equals(\"{}\", (Entity, Column::{})),\n",
                camel, pascal
            ));
        }
    }
    for f in fields {
        if f.base_type() == "DateTimeWithTimeZone" {
            let camel = to_camel_case(&f.name);
            let pascal = to_pascal_case(&f.name);
            s.push_str(&format!("        filter_defs.extend(FilterDefinition::date_range(\"{}\", (Entity, Column::{})));\n", camel, pascal));
        }
    }
    s
}

fn generate_service_list_search_definitions(fields: &[Field], _slug: &str) -> String {
    let mut s = String::new();
    for f in fields {
        if f.base_type() == "String" {
            let camel = to_camel_case(&f.name);
            let pascal = to_pascal_case(&f.name);
            s.push_str(&format!(
                "        SearchDefinition::contains(\"{}\", (Entity, Column::{})),\n",
                camel, pascal
            ));
        }
    }
    s
}

fn generate_service_list_order_definitions(fields: &[Field], _slug: &str) -> String {
    let mut s = String::new();
    for f in fields {
        let camel = to_camel_case(&f.name);
        let pascal = to_pascal_case(&f.name);
        if f.base_type() == "String" {
            s.push_str(&format!(
                "        OrderDefinition::case_insensitive(\"{}\", (Entity, Column::{})),\n",
                camel, pascal
            ));
        } else {
            s.push_str(&format!(
                "        OrderDefinition::column(\"{}\", (Entity, Column::{})),\n",
                camel, pascal
            ));
        }
    }
    s
}

fn generate_service_create_fields_mappings(fields: &[Field]) -> String {
    let mut s = String::new();
    for f in fields {
        s.push_str(&format!(
            "            {}: Set(payload.{}),\n",
            f.name, f.name
        ));
    }
    s
}

fn generate_service_update_fields_mappings(fields: &[Field]) -> String {
    let mut s = String::new();
    for f in fields {
        s.push_str(&format!(
            "        active_item.{} = Set(payload.{});\n",
            f.name, f.name
        ));
    }
    s
}

fn generate_controller_search_fields(fields: &[Field]) -> String {
    let search_fields: Vec<String> = fields
        .iter()
        .filter(|f| f.base_type() == "String")
        .map(|f| format!("\"{}\"", to_camel_case(&f.name)))
        .collect();
    search_fields.join(", ")
}

fn generate_controller_filter_fields(fields: &[Field]) -> String {
    let mut filter_fields: Vec<String> = fields
        .iter()
        .map(|f| format!("\"{}\"", to_camel_case(&f.name)))
        .collect();
    filter_fields.push("\"active\"".to_string());
    filter_fields.push("\"createdAt\"".to_string());
    filter_fields.push("\"updatedAt\"".to_string());
    filter_fields.join(", ")
}

fn generate_create_payload_json(fields: &[Field]) -> String {
    let mut parts = Vec::new();
    for f in fields {
        let camel = to_camel_case(&f.name);
        let val = match f.base_type() {
            "String" => format!("\"{} Test\"", f.name),
            "i32" => "1".to_string(),
            "f64" | "Decimal" => "10.5".to_string(),
            "bool" => "true".to_string(),
            "DateTimeWithTimeZone" => "\"2026-05-22T00:00:00Z\"".to_string(),
            _ => "\"\"".to_string(),
        };
        parts.push(format!("        \"{}\": {}", camel, val));
    }
    format!("{{\n{}\n    }}", parts.join(",\n"))
}

fn generate_assert_create_fields_json(fields: &[Field]) -> String {
    let mut parts = Vec::new();
    for f in fields {
        let camel = to_camel_case(&f.name);
        let val = match f.base_type() {
            "String" => format!("    assert_eq!(body[\"{}\"].as_str().unwrap(), \"{} Test\");", camel, f.name),
            "i32" => format!("    assert_eq!(body[\"{}\"].as_i64().unwrap(), 1);", camel),
            "Decimal" => format!("    assert!((body[\"{}\"].as_str().unwrap().parse::<f64>().unwrap() - 10.5).abs() < 0.001);", camel),
            "f64" => format!("    assert!((body[\"{}\"].as_f64().unwrap() - 10.5).abs() < 0.001);", camel),
            "bool" => format!("    assert_eq!(body[\"{}\"].as_bool().unwrap(), true);", camel),
            "DateTimeWithTimeZone" => format!("    assert!(body[\"{}\"].as_str().is_some());", camel),
            _ => String::new(),
        };
        if !val.is_empty() {
            parts.push(val);
        }
    }
    parts.join("\n")
}

fn generate_update_payload_json(fields: &[Field]) -> String {
    let mut parts = Vec::new();
    for f in fields {
        let camel = to_camel_case(&f.name);
        let val = match f.base_type() {
            "String" => format!("\"{} Updated\"", f.name),
            "i32" => "2".to_string(),
            "f64" | "Decimal" => "20.5".to_string(),
            "bool" => "false".to_string(),
            "DateTimeWithTimeZone" => "\"2026-05-22T01:00:00Z\"".to_string(),
            _ => "\"\"".to_string(),
        };
        parts.push(format!("        \"{}\": {}", camel, val));
    }
    parts.push("        \"active\": false".to_string());
    format!("{{\n{}\n    }}", parts.join(",\n"))
}

fn generate_assert_update_fields_json(fields: &[Field]) -> String {
    let mut parts = Vec::new();
    for f in fields {
        let camel = to_camel_case(&f.name);
        let val = match f.base_type() {
            "String" => format!("    assert_eq!(body[\"{}\"].as_str().unwrap(), \"{} Updated\");", camel, f.name),
            "i32" => format!("    assert_eq!(body[\"{}\"].as_i64().unwrap(), 2);", camel),
            "Decimal" => format!("    assert!((body[\"{}\"].as_str().unwrap().parse::<f64>().unwrap() - 20.5).abs() < 0.001);", camel),
            "f64" => format!("    assert!((body[\"{}\"].as_f64().unwrap() - 20.5).abs() < 0.001);", camel),
            "bool" => format!("    assert_eq!(body[\"{}\"].as_bool().unwrap(), false);", camel),
            "DateTimeWithTimeZone" => format!("    assert!(body[\"{}\"].as_str().is_some());", camel),
            _ => String::new(),
        };
        if !val.is_empty() {
            parts.push(val);
        }
    }
    parts.join("\n")
}

fn generate_migration_fields_sql(fields: &[Field]) -> String {
    let mut s = String::new();
    for f in fields {
        let pg_type = match f.base_type() {
            "String" => {
                if f.is_text || f.name == "description" {
                    "TEXT".to_string()
                } else {
                    "VARCHAR(255)".to_string()
                }
            }
            "i32" => "INTEGER DEFAULT 0".to_string(),
            "bool" => "BOOLEAN DEFAULT TRUE".to_string(),
            "Decimal" => "NUMERIC(10, 2)".to_string(),
            "f64" => "DOUBLE PRECISION".to_string(),
            "DateTimeWithTimeZone" => "TIMESTAMP WITH TIME ZONE".to_string(),
            _ => "VARCHAR(255)".to_string(),
        };

        let nullability = if f.is_nullable { "" } else { " NOT NULL" };

        s.push_str(&format!(
            "                {} {}{},\n",
            f.name, pg_type, nullability
        ));
    }
    s
}

fn register_model(slug: &str) {
    let path = "src/models/mod.rs";
    let mut content = fs::read_to_string(path).unwrap_or_default();
    let mod_line = format!("pub mod {};\n", slug);

    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
        fs::write(path, content).expect("Falha ao atualizar src/models/mod.rs");
        println!("  📝 [EDIT] src/models/mod.rs (Registrado '{}')", slug);
    }
}

fn register_module(_entity_name: &str, slug: &str) {
    let path = "src/modules/mod.rs";
    let content = fs::read_to_string(path).unwrap_or_default();

    let mod_declaration = format!("pub mod {};", slug);
    if !content.contains(&mod_declaration) {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        lines.insert(0, mod_declaration);

        for i in 0..lines.len() {
            if lines[i].contains("Router::new()") {
                lines[i] = format!(
                    "{}\n        .merge({}::router(db.clone(), cache.clone(), config.clone()))",
                    lines[i], slug
                );
                break;
            }
        }

        let new_content = lines.join("\n");
        fs::write(path, new_content).expect("Falha ao atualizar src/modules/mod.rs");
        println!("  📝 [EDIT] src/modules/mod.rs (Registrado '{}')", slug);
    }
}

fn register_observability(entity_name: &str, slug: &str, feature_name: &str, feature_desc: &str) {
    let path = "src/modules/observability.rs";
    let content = fs::read_to_string(path).expect("Falha ao ler src/modules/observability.rs");

    if !content.contains(&format!("{}::{}Api", slug, entity_name)) {
        let import_pattern = "product::ProductApi,";
        let new_import = format!("product::ProductApi, {}::{}Api,", slug, entity_name);

        let merge_pattern = "ProductApi::openapi(),";
        let new_merge = format!(
            "ProductApi::openapi(),\n        {}Api::openapi(),",
            entity_name
        );

        let tag_pattern = "(name = \"Product\", description = \"Product Catalog & Pricing\"),";
        let new_tag = format!("(name = \"Product\", description = \"Product Catalog & Pricing\"),\n        (name = \"{}\", description = \"{}\"),", feature_name, feature_desc);

        let mut new_content = content.replace(import_pattern, &new_import);
        new_content = new_content.replace(merge_pattern, &new_merge);
        new_content = new_content.replace(tag_pattern, &new_tag);
        fs::write(path, new_content).expect("Falha ao salvar src/modules/observability.rs");
        println!("  📝 [EDIT] src/modules/observability.rs (Registrado API de docs)");
    }
}

fn find_next_test_index() -> usize {
    let mut max_idx = 16;
    if let Ok(entries) = fs::read_dir("tests/compliance") {
        for entry in entries {
            if let Ok(entry) = entry {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('t') && name.len() > 3 {
                    if let Some(idx_str) = name[1..3].split('_').next() {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if idx > max_idx {
                                max_idx = idx;
                            }
                        }
                    }
                }
            }
        }
    }
    max_idx + 1
}

fn register_test_module(test_mod_name: &str) {
    let path = "tests/compliance/mod.rs";
    let mut content = fs::read_to_string(path).unwrap_or_default();
    let mod_decl = format!("pub mod {};\n", test_mod_name);
    if !content.contains(&mod_decl) {
        content.push_str(&mod_decl);
        fs::write(path, content).expect("Falha ao atualizar tests/compliance/mod.rs");
        println!(
            "  📝 [EDIT] tests/compliance/mod.rs (Registrado '{}')",
            test_mod_name
        );
    }
}

fn register_test_execution(test_mod_name: &str) {
    let path = "tests/integration_tests.rs";
    let content = fs::read_to_string(path).expect("Falha ao ler tests/integration_tests.rs");
    let run_stmt = format!("compliance::{}::run(&ctx).await;", test_mod_name);
    if !content.contains(&run_stmt) {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut insert_idx = None;
        for (i, line) in lines.iter().enumerate() {
            if line.contains("compliance::t") && line.contains("::run(&ctx).await;") {
                insert_idx = Some(i);
            }
        }
        if let Some(idx) = insert_idx {
            lines.insert(idx + 1, format!("    {}", run_stmt));
            let new_content = lines.join("\n") + "\n";
            fs::write(path, new_content).expect("Falha ao salvar tests/integration_tests.rs");
            println!(
                "  📝 [EDIT] tests/integration_tests.rs (Registrado execução de '{}')",
                test_mod_name
            );
        }
    }
}

fn register_table_truncate(entity_name: &str) {
    let path = "tests/common/mod.rs";
    let content = fs::read_to_string(path).expect("Falha ao ler tests/common/mod.rs");
    let target_pattern = "public.\\\"Product\\\"";
    let new_pattern = format!("public.\\\"Product\\\", public.\\\"{}\\\"", entity_name);
    if !content.contains(&format!("public.\\\"{}\\\"", entity_name)) {
        let new_content = content.replace(target_pattern, &new_pattern);
        fs::write(path, new_content).expect("Falha ao salvar tests/common/mod.rs");
        println!(
            "  📝 [EDIT] tests/common/mod.rs (Registrado truncate da tabela '{}')",
            entity_name
        );
    }
}

fn register_migration_in_migrator(migration_mod: &str) {
    let path = "src/migration/mod.rs";
    let content = fs::read_to_string(path).expect("Falha ao ler src/migration/mod.rs");
    if !content.contains(migration_mod) {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        lines.insert(2, format!("mod {};", migration_mod));

        for i in 0..lines.len() {
            if lines[i].contains("vec![") {
                lines[i] = lines[i].replace(
                    "vec![",
                    &format!("vec![\n            Box::new({}::Migration),", migration_mod),
                );
                break;
            }
        }

        let new_content = lines.join("\n") + "\n";
        fs::write(path, new_content).expect("Falha ao salvar src/migration/mod.rs");
        println!(
            "  📝 [EDIT] src/migration/mod.rs (Registrado migration '{}')",
            migration_mod
        );
    }
}

fn register_rbac_feature(feature_id: &str, _feature_name: &str, feature_desc: &str) {
    let path = "src/infra/bootstrap.rs";
    let content = fs::read_to_string(path).expect("Falha ao ler src/infra/bootstrap.rs");
    if !content.contains(&format!("\"{}\"", feature_id)) {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        let mut insert_idx = None;
        let mut in_features_data = false;
        for (i, line) in lines.iter().enumerate() {
            if line.contains("let features_data = vec![") {
                in_features_data = true;
                continue;
            }
            if in_features_data && line.trim() == "];" {
                insert_idx = Some(i);
                break;
            }
        }
        if let Some(idx) = insert_idx {
            lines.insert(
                idx,
                format!("        (\"{}\", \"{}\"),", feature_id, feature_desc),
            );
        }

        for i in 0..lines.len() {
            if lines[i].contains("let features = vec![") {
                if let Some(pos) = lines[i].rfind(']') {
                    let (before, after) = lines[i].split_at(pos);
                    lines[i] = format!("{}, \"{}\"{}", before, feature_id, after);
                }
                break;
            }
        }

        let new_content = lines.join("\n") + "\n";
        fs::write(path, new_content).expect("Falha ao salvar src/infra/bootstrap.rs");
        println!(
            "  📝 [EDIT] src/infra/bootstrap.rs (Registrado feature '{}' no RBAC/Seed)",
            feature_id
        );
    }
}
