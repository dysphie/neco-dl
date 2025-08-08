use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use rustyline::{Editor, error::ReadlineError};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::time::Duration;

static TITLE_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".workshopItemTitle").unwrap());
static CHANGELOG_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(".changeLogCtn p[id]").unwrap());
static ITEM_SELECTOR: Lazy<Selector> =
    Lazy::new(|| Selector::parse(r#"[id^="sharedfile_"]"#).unwrap());

#[derive(Debug, Deserialize)]
struct Config {
    appid: String,
    steam_cmd: String,
    download_dir: String,
    workshop_cfgs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileInfo {
    path: String,
    hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkshopMetadata {
    title: String,
    changelog_id: String,
    #[serde(default)]
    files: Vec<FileInfo>,
    #[serde(default)]
    collection_ids: Vec<String>,
}

struct WorkshopItem {
    id: String,
    title: String,
    changelog_id: String,
}

struct WorkshopCollection {
    id: String,
    title: String,
    item_ids: Vec<String>,
}

enum ParseResult {
    Item(WorkshopItem),
    Collection(WorkshopCollection),
}

pub struct WorkshopManager {
    config: Config,
    paths: ManagerPaths,
    metadata: HashMap<String, WorkshopMetadata>,
    client: reqwest::Client,
}

struct ManagerPaths {
    local_files: PathBuf,
    workshop_cfgs: Vec<PathBuf>,
    steamcmd: PathBuf,
    metadata_file: PathBuf,
}

impl ManagerPaths {
    fn new(config: &Config) -> Result<Self> {
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;
        let steamcmd = PathBuf::from(&config.steam_cmd);
        let workshop_cfgs = config.workshop_cfgs.iter().map(PathBuf::from).collect();

        Ok(Self {
            local_files: PathBuf::from(&config.download_dir),
            workshop_cfgs,
            steamcmd,
            metadata_file: current_dir.join("metadata.json"),
        })
    }

    fn steamcmd_workshop_path(&self, appid: &str, workshop_id: &str) -> Result<PathBuf> {
        let parent = self
            .steamcmd
            .parent()
            .context("Steam CMD path has no parent directory")?;
        Ok(parent
            .join("steamapps")
            .join("workshop")
            .join("content")
            .join(appid)
            .join(workshop_id))
    }
}

impl WorkshopManager {
    pub async fn new() -> Result<Self> {
        let config = Self::load_config().await?;
        Self::validate_config(&config)?;
        let paths = ManagerPaths::new(&config)?;

        fs::create_dir_all(&paths.local_files)
            .await
            .context("Failed to create download directory")?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        let mut mgr = Self {
            config,
            paths,
            metadata: HashMap::new(),
            client,
        };

        mgr.load_metadata().await?;
        Ok(mgr)
    }

    async fn load_config() -> Result<Config> {
        let content = fs::read_to_string("config.toml")
            .await
            .context("Failed to read config.toml")?;
        toml::from_str(&content).context("Failed to parse config.toml")
    }

    fn validate_config(config: &Config) -> Result<()> {
        if config.appid.trim().is_empty() {
            anyhow::bail!("appid must not be empty in config.toml");
        }
        if config.download_dir.trim().is_empty() {
            anyhow::bail!("download_dir must not be empty in config.toml");
        }
        if config.steam_cmd.trim().is_empty() {
            anyhow::bail!("steam_cmd must not be empty in config.toml");
        }
        Ok(())
    }

    async fn load_metadata(&mut self) -> Result<()> {
        match fs::read_to_string(&self.paths.metadata_file).await {
            Ok(data) => {
                self.metadata =
                    serde_json::from_str(&data).context("Failed to parse metadata.json")?;
            }
            Err(_) => {
                self.metadata = HashMap::new();
            }
        }
        Ok(())
    }

    async fn save_metadata(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(&self.metadata)?;
        fs::write(&self.paths.metadata_file, data)
            .await
            .context("Failed to save metadata")
    }

    async fn fetch_html(&self, url: &str) -> Result<String> {
        self.client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
            .map_err(Into::into)
    }

    async fn parse_workshop_item(&self, workshop_id: &str) -> Result<ParseResult> {
        let changelog_url = format!(
            "https://steamcommunity.com/sharedfiles/filedetails/changelog/{}",
            workshop_id
        );
        let changelog_html = self
            .fetch_html(&changelog_url)
            .await
            .with_context(|| format!("Failed to fetch changelog page for id {}", workshop_id))?;
        let changelog_doc = Html::parse_document(&changelog_html);

        let title = changelog_doc
            .select(&TITLE_SELECTOR)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        if let Some(changelog_id) = changelog_doc
            .select(&CHANGELOG_SELECTOR)
            .next()
            .and_then(|el| el.value().attr("id"))
        {
            return Ok(ParseResult::Item(WorkshopItem {
                id: workshop_id.to_string(),
                title,
                changelog_id: changelog_id.to_string(),
            }));
        }

        let collection_url = format!(
            "https://steamcommunity.com/sharedfiles/filedetails/?id={}",
            workshop_id
        );
        let collection_html = self
            .fetch_html(&collection_url)
            .await
            .with_context(|| format!("Failed to fetch collection page for id {}", workshop_id))?;
        let collection_doc = Html::parse_document(&collection_html);

        let item_ids = collection_doc
            .select(&ITEM_SELECTOR)
            .filter_map(|el| el.value().attr("id"))
            .filter_map(|id| id.strip_prefix("sharedfile_"))
            .map(String::from)
            .collect();

        Ok(ParseResult::Collection(WorkshopCollection {
            id: workshop_id.to_string(),
            title,
            item_ids,
        }))
    }

    async fn quick_update(
        &mut self,
        item: &WorkshopItem,
        collection_id: Option<&str>,
    ) -> Result<bool> {
        let metadata: &mut WorkshopMetadata = match self.metadata.get_mut(&item.id) {
            Some(m) => m,
            None => return Ok(false),
        };

        if metadata.changelog_id != item.changelog_id {
            return Ok(false);
        }

        let files = metadata.files.clone();

        for file_info in &files {
            if !self.verify_file(file_info).await? {
                return Ok(false);
            }
        }

        if let Some(cid) = collection_id {
            let cid_string = cid.to_string();
            if let Some(metadata) = self.metadata.get_mut(&item.id) {
                if !metadata.collection_ids.contains(&cid_string) {
                    metadata.collection_ids.push(cid_string);
                }
            }
        }

        self.save_metadata().await?;

        println!("Successfully downloaded {}", item.id);
        Ok(true)
    }

    async fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        const BUFFER_SIZE: usize = 64 * 1024;
        let mut file = fs::File::open(path)
            .await
            .with_context(|| format!("Failed to open file: {}", path.display()))?;

        let mut context = md5::Context::new();
        let mut buffer = vec![0u8; BUFFER_SIZE];

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            context.consume(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", context.compute()))
    }

    async fn verify_file(&self, file_info: &FileInfo) -> Result<bool> {
        let full_path = self.paths.local_files.join(&file_info.path);

        if !fs::try_exists(&full_path).await? {
            return Ok(false);
        }

        if file_info.hash.is_empty() {
            return Ok(true);
        }

        let current_hash = self.calculate_file_hash(&full_path).await?;
        Ok(current_hash == file_info.hash)
    }

    async fn run_steamcmd(&self, args: &[&str]) -> Result<bool> {
        let mut child = Command::new(&self.paths.steamcmd)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start SteamCMD")?;

        let stdout = child
            .stdout
            .take()
            .context("Failed to capture SteamCMD stdout")?;
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        let mut success = false;
        while let Some(line) = lines.next_line().await? {
            if line.contains("Success. Downloaded item") || line.contains("item state : 4") {
                success = true;
                break;
            }
        }

        let status = child.wait().await?;
        Ok(success || status.success())
    }

    async fn move_and_track_files(&self, src: &Path, dest: &Path) -> Result<Vec<FileInfo>> {
        if !fs::try_exists(src).await? {
            return Ok(Vec::new());
        }

        fs::create_dir_all(dest).await?;
        let mut files = Vec::new();
        self.move_directory(src, dest, &mut files).await?;
        Ok(files)
    }

    async fn move_directory(
        &self,
        src: &Path,
        dest: &Path,
        files: &mut Vec<FileInfo>,
    ) -> Result<()> {
        let mut stack = vec![(src.to_path_buf(), dest.to_path_buf())];

        while let Some((src_dir, dest_dir)) = stack.pop() {
            if !fs::try_exists(&src_dir).await? {
                continue;
            }
            fs::create_dir_all(&dest_dir).await?;

            let mut entries = fs::read_dir(&src_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let src_path = entry.path();
                let dest_path = dest_dir.join(entry.file_name());
                let meta = fs::metadata(&src_path).await?;

                if meta.is_dir() {
                    stack.push((src_path, dest_path));
                } else {
                    let hash = self.calculate_file_hash(&src_path).await?;
                    fs::copy(&src_path, &dest_path).await?;
                    fs::remove_file(&src_path).await?;

                    let relative_path = dest_path
                        .strip_prefix(&self.paths.local_files)
                        .unwrap_or(&dest_path)
                        .to_string_lossy()
                        .to_string();

                    files.push(FileInfo {
                        path: relative_path,
                        hash,
                    });
                }
            }

            // todo: remove empty dir
        }

        Ok(())
    }

    async fn remove_item(&mut self, workshop_id: &str) -> Result<bool> {
        let metadata = match self.metadata.remove(workshop_id) {
            Some(m) => m,
            None => return Ok(false),
        };

        self.save_metadata().await?;

        let mut removed_count = 0;

        for file_info in &metadata.files {
            let full_path = self.paths.local_files.join(&file_info.path);

            if !fs::try_exists(&full_path).await? {
                continue;
            }

            if !file_info.hash.is_empty() && !self.verify_file(file_info).await? {
                println!(
                    "Skipping {} - file modified, delete manually",
                    file_info.path
                );
                continue;
            }

            let meta = fs::metadata(&full_path).await?;
            if meta.is_dir() {
                fs::remove_dir_all(&full_path).await?;
            } else {
                fs::remove_file(&full_path).await?;
            }

            println!("Removed: {}", file_info.path);
            removed_count += 1;
        }

        Ok(removed_count > 0)
    }

    fn display_config_info(&self) {
        println!("\n{:-<60}", " CONFIGURATION ");
        println!("{:<25}: {}", "App ID", self.config.appid);
        println!("{:<25}: {}", "SteamCMD Path", self.config.steam_cmd);
        println!("{:<25}: {}", "Download Directory", self.config.download_dir);
        println!(
            "{:<25}: {} files",
            "Workshop CFGs",
            self.config.workshop_cfgs.len()
        );
    }

    fn display_paths_info(&self) {
        println!("\n{:-<60}", " PATHS ");
        println!(
            "{:<25}: {}",
            "Metadata File",
            self.paths.metadata_file.display()
        );
        println!(
            "{:<25}: {}",
            "Local Files",
            self.paths.local_files.display()
        );
        println!("{:<25}: {}", "SteamCMD", self.paths.steamcmd.display());
        println!("Workshop CFG Locations:");
        for (i, path) in self.paths.workshop_cfgs.iter().enumerate() {
            println!("  {:<23}: {}", format!("Config #{}", i + 1), path.display());
        }
    }

    async fn display_subscription_info(&self) -> Result<()> {
        println!("\n{:-<60}", " SUBSCRIPTIONS ");
        println!("{:<25}: {}", "Total Subscriptions", self.metadata.len());
        Ok(())
    }

    async fn calculate_directory_size(&self, root: &Path) -> Result<u64> {
        let mut total = 0;
        let mut stack = vec![root.to_path_buf()];

        while let Some(path) = stack.pop() {
            if !fs::try_exists(&path).await? {
                continue;
            }

            let mut entries = fs::read_dir(&path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                let meta = fs::metadata(&path).await?;

                if meta.is_dir() {
                    stack.push(path);
                } else {
                    total += meta.len();
                }
            }
        }

        Ok(total)
    }

    async fn display_storage_info(&self) -> Result<()> {
        println!("\n{:-<60}", " STORAGE ");

        let download_dir = &self.paths.local_files;
        let used_space = self.calculate_directory_size(download_dir).await?;

        println!("{:<25}: {}", "Download Directory", download_dir.display());
        println!("{:<25}: {}", "Used Space", format_file_size(used_space));

        Ok(())
    }

    async fn cmd_info(&self) -> Result<()> {
        self.display_config_info();
        self.display_paths_info();
        self.display_subscription_info().await?;
        self.display_storage_info().await?;
        Ok(())
    }

    async fn cmd_download(&mut self, workshop_id: &str) -> Result<()> {
        if workshop_id.is_empty() {
            println!("usage: download <workshop_id>");
            return Ok(());
        }

        self.download_generic(workshop_id).await
    }

    async fn download_generic(&mut self, workshop_id: &str) -> Result<()> {
        let item = self
            .parse_workshop_item(workshop_id)
            .await
            .context("Failed to fetch workshop information")?;

        match item {
            ParseResult::Item(file) => {
                self.download_item(file, None).await?;
            }
            ParseResult::Collection(collection) => {
                self.download_collection(collection).await?;
            }
        }

        Ok(())
    }

    async fn download_item(
        &mut self,
        item: WorkshopItem,
        collection_id: Option<&str>,
    ) -> Result<bool> {
        println!("Downloading {} ({})...", item.id, item.title);
        if self.quick_update(&item, collection_id).await? {
            return Ok(true);
        }

        let args = [
            "+login",
            "anonymous",
            "+workshop_download_item",
            &self.config.appid,
            &item.id,
            "+quit",
        ];

        if !self.run_steamcmd(&args).await? {
            eprintln!("Failed to download {}", item.id);
            return Ok(false);
        }

        let source_path = self
            .paths
            .steamcmd_workshop_path(&self.config.appid, &item.id)
            .context("Failed to compute SteamCMD workshop path")?;

        if !fs::try_exists(&source_path).await? {
            eprintln!("Downloaded files not found at expected location");
            return Ok(false);
        }

        let files = self
            .move_and_track_files(&source_path, &self.paths.local_files)
            .await?;

        if files.is_empty() {
            eprintln!("No files found for workshop item {}", item.id);
            return Ok(false);
        }

        let entry = self
            .metadata
            .entry(item.id.clone())
            .or_insert_with(|| WorkshopMetadata {
                title: item.title.clone(),
                changelog_id: item.changelog_id.clone(),
                files: Vec::new(),
                collection_ids: Vec::new(),
            });

        entry.title = item.title;
        entry.changelog_id = item.changelog_id;
        entry.files = files;

        if let Some(cid) = collection_id {
            let cid_string = cid.to_string();
            if !entry.collection_ids.contains(&cid_string) {
                entry.collection_ids.push(cid_string);
            }
        }

        println!("Successfully downloaded {}", item.id);
        self.save_metadata().await?;
        Ok(true)
    }

    async fn download_collection(&mut self, collection: WorkshopCollection) -> Result<()> {
        println!(
            "Downloading collection: {} ({} items)",
            collection.title,
            collection.item_ids.len()
        );

        for file_id in &collection.item_ids {
            let file = self
                .parse_workshop_item(file_id)
                .await
                .context("Failed to fetch file info in collection")?;

            if let ParseResult::Item(file_item) = file {
                self.download_item(file_item, Some(&collection.id)).await?;
            }
        }

        Ok(())
    }

    async fn cmd_update(&mut self) -> Result<()> {
        let workshop_ids: Vec<String> = self.metadata.keys().cloned().collect();

        if workshop_ids.is_empty() {
            println!("No subscribed items. Use 'download <id>' to add items.");
            return Ok(());
        }

        println!("Checking {} items for updates...", workshop_ids.len());

        for workshop_id in &workshop_ids {
            match self
                .parse_workshop_item(workshop_id)
                .await
                .context("Failed to fetch workshop information")?
            {
                ParseResult::Item(item) => {
                    self.download_item(item, None).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn cmd_list(&self, verbose: bool) -> Result<()> {
        if self.metadata.is_empty() {
            println!("No subscribed items. Use 'download <id>' to add items.");
            return Ok(());
        }

        println!("Subscribed items ({}):", self.metadata.len());

        if verbose {
            println!("{}", "=".repeat(60));
        }

        for (workshop_id, metadata) in &self.metadata {
            if verbose {
                self.print_detailed_item(workshop_id, metadata)?;
            } else {
                let map_name = metadata
                    .files
                    .iter()
                    .find(|f| f.path.ends_with(".bsp"))
                    .and_then(|f| Path::new(&f.path).file_stem())
                    .map(|s| s.to_string_lossy())
                    .unwrap_or_else(|| "no_map".into());

                println!("{:<12} {}", workshop_id, map_name);
            }
        }

        Ok(())
    }

    fn print_detailed_item(&self, workshop_id: &str, metadata: &WorkshopMetadata) -> Result<()> {
        println!("ID: {}", workshop_id);
        println!("Title: {}", metadata.title);

        if !metadata.collection_ids.is_empty() {
            println!("Collections: {}", metadata.collection_ids.join(", "));
        }

        if !metadata.files.is_empty() {
            println!("Files ({}):", metadata.files.len());
            let current_dir = std::env::current_dir()?;
            for file_info in &metadata.files {
                let path = Path::new(&file_info.path);
                let display_path = path.strip_prefix(&current_dir).unwrap_or(path);
                println!("  - {}", display_path.display());
            }
        }

        println!("{}", "-".repeat(40));
        Ok(())
    }

    async fn cmd_remove(&mut self, workshop_id: &str) -> Result<()> {
        if workshop_id.is_empty() {
            println!("usage: remove <workshop_id>");
            return Ok(());
        }

        if self.metadata.contains_key(workshop_id) {
            self.remove_item(workshop_id).await?;
        }

        let mut to_remove = Vec::new();
        for (id, object) in &self.metadata {
            if object.collection_ids.len() == 1 && object.collection_ids[0] == workshop_id {
                to_remove.push(id.clone());
            }
        }

        for id in to_remove {
            self.remove_item(&id).await?;
        }

        Ok(())
    }

    fn generate_workshop_maps_content(&self) -> (String, usize) {
        let mut content = String::from("\"WorkshopMaps\"\n{\n");
        let mut map_count = 0;

        for (workshop_id, metadata) in &self.metadata {
            if let Some(map_name) = self.extract_map_name(metadata) {
                content.push_str(&format!("\t\"{}\"\t\t\"{}\"\n", map_name, workshop_id));
                map_count += 1;
            }
        }

        content.push_str("}\n");
        (content, map_count)
    }

    async fn write_workshop_maps(&self, content: &str) -> Result<()> {
        for cfg_path in &self.paths.workshop_cfgs {
            if let Some(parent) = cfg_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            fs::write(cfg_path, content).await.with_context(|| {
                format!("Failed to write workshop maps to {}", cfg_path.display())
            })?;
        }
        Ok(())
    }

    async fn cmd_generate(&self) -> Result<()> {
        let (content, map_count) = self.generate_workshop_maps_content();
        self.write_workshop_maps(&content).await?;

        println!(
            "Generated workshop maps in {} locations with {} map entries",
            self.paths.workshop_cfgs.len(),
            map_count
        );
        Ok(())
    }

    fn extract_map_name(&self, metadata: &WorkshopMetadata) -> Option<String> {
        metadata
            .files
            .iter()
            .find(|f| f.path.to_lowercase().ends_with(".bsp"))
            .and_then(|f| Path::new(&f.path).file_stem())
            .map(|s| s.to_string_lossy().to_string())
    }

    fn show_help(&self) {
        println!("\nAvailable commands:");
        println!("  download <id>   - Download workshop item or collection");
        println!("  update          - Update all subscribed items");
        println!("  list [-v]       - List subscribed items (use -v for details)");
        println!("  remove <id>     - Remove workshop item or collection");
        println!("                    (collections remove orphaned items)");
        println!("  generate        - Generate workshop_maps.txt file");
        println!("  info            - Show configuration and status information");
        println!("  help            - Show this help");
        println!("  exit            - Exit application");
        println!();
    }

    async fn process_command(&mut self, input: &str) -> Result<bool> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Ok(true);
        }

        match parts[0].to_lowercase().as_str() {
            "download" => {
                if let Some(id) = parts.get(1) {
                    self.cmd_download(id).await?;
                } else {
                    println!("Usage: download <workshop_id>");
                }
            }
            "update" => self.cmd_update().await?,
            "list" => {
                let verbose = parts.contains(&"-v") || parts.contains(&"--verbose");
                self.cmd_list(verbose).await?;
            }
            "remove" => {
                if let Some(id) = parts.get(1) {
                    self.cmd_remove(id).await?;
                } else {
                    println!("Usage: remove <workshop_id>");
                }
            }
            "generate" => self.cmd_generate().await?,
            "info" => self.cmd_info().await?,
            "help" => self.show_help(),
            "exit" | "quit" => return Ok(false),
            "" => {}
            _ => {
                println!(
                    "Unknown command: '{}'. Type 'help' for available commands.",
                    parts[0]
                );
            }
        }

        Ok(true)
    }

    pub async fn run(&mut self) -> Result<()> {
        println!(
            r#"Steam Workshop Manager
Type 'help' for available commands.
"#
        );

        let mut rl = Editor::<()>::new().context("Failed to create readline editor")?;
        let _ = rl.load_history(".history");

        loop {
            match rl.readline("> ") {
                Ok(line) => {
                    rl.add_history_entry(&line);
                    if !self.process_command(&line).await? {
                        break;
                    }
                }
                Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                    break;
                }
                Err(e) => {
                    eprintln!("Readline error: {}", e);
                    break;
                }
            }
        }

        let _ = rl.save_history(".history");
        println!("Goodbye!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut manager = WorkshopManager::new()
        .await
        .context("Failed to initialize workshop manager")?;

    manager.run().await
}

fn format_file_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}
