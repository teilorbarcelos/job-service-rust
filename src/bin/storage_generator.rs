use dialoguer::{theme::ColorfulTheme, Select};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("==================================================");
    println!("         Storage Provider Scaffolding CLI         ");
    println!("==================================================");

    let selections = &[
        "AWS S3 (Amazon Web Services)",
        "Google Cloud Storage (GCS)",
        "Azure Blob Storage (Microsoft Azure)",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select the Storage Provider to implement")
        .default(0)
        .items(&selections[..])
        .interact()?;

    match selection {
        0 => setup_s3()?,
        1 => setup_gcs()?,
        2 => setup_azure()?,
        _ => unreachable!(),
    }

    println!("\n✅ Scaffolding completed successfully!");
    println!("Run 'cargo test' to verify that everything compiles and all tests pass.");
    Ok(())
}

fn setup_s3() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚙️  Scaffolding AWS S3 Storage Provider...");

    let tpl_path = "templates/storage/s3.rs.tpl";
    let dest_path = "src/infra/storage/s3.rs";
    fs::copy(tpl_path, dest_path)?;
    println!("  - Created src/infra/storage/s3.rs");

    add_dependencies(&["aws-config = \"1.1.7\"", "aws-sdk-s3 = \"1.17.0\""])?;

    register_provider_in_mod("s3", "S3StorageService")?;

    update_env_files(
        "s3",
        &[
            ("AWS_ACCESS_KEY_ID", "dummy_access_key"),
            ("AWS_SECRET_ACCESS_KEY", "dummy_secret_key"),
            ("AWS_S3_BUCKET", "my-s3-bucket"),
        ],
    )?;

    println!("\n💡 S3 Provider added! Remember to set the environment variables:");
    println!("   - AWS_ACCESS_KEY_ID");
    println!("   - AWS_SECRET_ACCESS_KEY");
    println!("   - AWS_S3_BUCKET");
    println!("   - STORAGE_PROVIDER=s3");
    Ok(())
}

fn setup_gcs() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚙️  Scaffolding Google Cloud Storage Provider...");

    let tpl_path = "templates/storage/gcs.rs.tpl";
    let dest_path = "src/infra/storage/gcs.rs";
    fs::copy(tpl_path, dest_path)?;
    println!("  - Created src/infra/storage/gcs.rs");

    add_dependencies(&[
        "google-cloud-storage = \"0.13.0\"",
        "google-cloud-token = \"0.1.2\"",
    ])?;

    register_provider_in_mod("gcs", "GcsStorageService")?;

    update_env_files(
        "gcs",
        &[
            ("GOOGLE_APPLICATION_CREDENTIALS", ""),
            ("GCS_BUCKET", "my-gcs-bucket"),
        ],
    )?;

    println!("\n💡 GCS Provider added! Remember to set the environment variables:");
    println!("   - GOOGLE_APPLICATION_CREDENTIALS (path to service account json)");
    println!("   - GCS_BUCKET");
    println!("   - STORAGE_PROVIDER=gcs");
    Ok(())
}

fn setup_azure() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚙️  Scaffolding Azure Blob Storage Provider...");

    let tpl_path = "templates/storage/azure.rs.tpl";
    let dest_path = "src/infra/storage/azure.rs";
    fs::copy(tpl_path, dest_path)?;
    println!("  - Created src/infra/storage/azure.rs");

    add_dependencies(&[
        "azure_core = \"0.21.0\"",
        "azure_storage = \"0.21.0\"",
        "azure_storage_blobs = \"0.21.0\"",
    ])?;

    register_provider_in_mod("azure", "AzureStorageService")?;

    update_env_files(
        "azure",
        &[
            (
                "AZURE_STORAGE_CONNECTION_STRING",
                "UseDevelopmentStorage=true",
            ),
            ("AZURE_STORAGE_CONTAINER", "my-azure-container"),
        ],
    )?;

    println!("\n💡 Azure Provider added! Remember to set the environment variables:");
    println!("   - AZURE_STORAGE_CONNECTION_STRING");
    println!("   - AZURE_STORAGE_CONTAINER");
    println!("   - STORAGE_PROVIDER=azure");
    Ok(())
}

fn add_dependencies(deps: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let toml_path = "Cargo.toml";
    let mut content = fs::read_to_string(toml_path)?;

    let target = "[dependencies]\n";
    if !content.contains(target) {
        return Err("Could not find [dependencies] block in Cargo.toml".into());
    }

    for dep in deps {
        let parts: Vec<&str> = dep.split('=').collect();
        let dep_name = parts[0].trim();

        if content.contains(dep_name) {
            println!(
                "  - Dependency {} is already present in Cargo.toml",
                dep_name
            );
            continue;
        }

        let replacement = format!("{}{}\n", target, dep);
        content = content.replace(target, &replacement);
        println!("  - Added {} to Cargo.toml", dep_name);
    }

    fs::write(toml_path, content)?;
    Ok(())
}

fn register_provider_in_mod(
    provider_key: &str,
    provider_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mod_path = "src/infra/storage/mod.rs";
    let mut content = fs::read_to_string(mod_path)?;

    let mod_decl = format!("pub mod {};", provider_key);
    if !content.contains(&mod_decl) {
        content = content.replace(
            "pub mod local;",
            &format!("pub mod local;\npub mod {};", provider_key),
        );
        println!("  - Registered mod {} in {}", provider_key, mod_path);
    }

    let match_arm = format!("\"{}\" =>", provider_key);
    if !content.contains(&match_arm) {
        let replacement = format!(
            "\"{}\" => {{\n                info!(\"[Storage] Initializing {}...\");\n                Arc::new({}::{}::new().await?)\n            }}\n            /* {{{{GENERATED_PROVIDERS}}}} */",
            provider_key, provider_name, provider_key, provider_name
        );
        content = content.replace("/* {{GENERATED_PROVIDERS}} */", &replacement);
        println!(
            "  - Registered provider initialization match block in {}",
            mod_path
        );
    }

    fs::write(mod_path, content)?;
    Ok(())
}

fn update_env_files(
    provider_key: &str,
    extra_vars: &[(&str, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    let files = &[".env", ".env.example"];
    for file_path in files {
        if !std::path::Path::new(file_path).exists() {
            continue;
        }
        let content = fs::read_to_string(file_path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut provider_updated = false;

        for line in &mut lines {
            if line.starts_with("STORAGE_PROVIDER=") {
                *line = format!("STORAGE_PROVIDER={}", provider_key);
                provider_updated = true;
            }
        }

        if !provider_updated {
            lines.push(format!("STORAGE_PROVIDER={}", provider_key));
        }

        for &(key, val) in extra_vars {
            let prefix = format!("{}=", key);
            if !lines.iter().any(|l| l.starts_with(&prefix)) {
                lines.push(format!("{}={}", key, val));
            }
        }

        fs::write(file_path, lines.join("\n") + "\n")?;
        println!(
            "  - Updated {} with STORAGE_PROVIDER={}",
            file_path, provider_key
        );
    }
    Ok(())
}
