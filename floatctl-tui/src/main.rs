use anyhow::Result;
use floatctl_tui::{BlockStore, ScratchParser};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // TODO: Load from config
    let home = dirs::home_dir().expect("Could not find home directory");
    let db_path = home.join(".floatctl").join("tui.db");

    // Initialize block store
    let store = BlockStore::new(&db_path).await?;
    println!("✓ BlockStore initialized at {}", db_path.display());

    // Test parsing
    let test_input = r#"ctx::2025-11-15 @ 14:30 - brain boot
  - good morning
  - [project::floatctl-tui] building the TUI
  - testing parser"#;

    let blocks = ScratchParser::parse_entry(test_input);
    println!("\n✓ Parsed {} blocks from test input", blocks.len());

    for block in &blocks {
        store.insert(block).await?;
    }

    println!("✓ Inserted blocks into store");

    let count = store.count().await?;
    println!("✓ Total blocks in store: {}", count);

    println!("\n🚀 floatctl-tui initialized successfully!");
    println!("   Next: Implement TUI interface");

    Ok(())
}
