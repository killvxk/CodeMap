use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::Path;

// ── 数据结构（与 Node.js JSON schema 完全兼容）────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub signature: String,
    #[serde(rename = "startLine")]
    pub start_line: u32,
    #[serde(rename = "endLine")]
    pub end_line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub source: String,
    pub symbols: Vec<String>,
    #[serde(rename = "isExternal")]
    pub is_external: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    pub name: String,
    #[serde(rename = "startLine")]
    pub start_line: u32,
    #[serde(rename = "endLine")]
    pub end_line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,
    pub kind: String,
    #[serde(rename = "startLine")]
    pub start_line: u32,
    #[serde(rename = "endLine")]
    pub end_line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub language: String,
    pub module: String,
    pub hash: String,
    pub lines: u32,
    pub functions: Vec<FunctionInfo>,
    pub classes: Vec<ClassInfo>,
    pub types: Vec<TypeInfo>,
    pub imports: Vec<ImportInfo>,
    pub exports: Vec<String>,
    #[serde(rename = "isEntryPoint")]
    pub is_entry_point: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleEntry {
    pub files: Vec<String>,
    #[serde(rename = "dependsOn")]
    pub depends_on: Vec<String>,
    #[serde(rename = "dependedBy")]
    pub depended_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSummary {
    #[serde(rename = "totalFiles")]
    pub total_files: u32,
    #[serde(rename = "totalFunctions")]
    pub total_functions: u32,
    #[serde(rename = "totalClasses")]
    pub total_classes: u32,
    pub languages: HashMap<String, u32>,
    pub modules: Vec<String>,
    #[serde(rename = "entryPoints")]
    pub entry_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConfig {
    pub languages: Vec<String>,
    #[serde(rename = "excludePatterns")]
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGraph {
    pub version: String,
    pub project: ProjectInfo,
    #[serde(rename = "scannedAt")]
    pub scanned_at: String,
    #[serde(rename = "commitHash")]
    pub commit_hash: Option<String>,
    pub config: GraphConfig,
    pub summary: GraphSummary,
    pub modules: HashMap<String, ModuleEntry>,
    pub files: HashMap<String, FileEntry>,
}

/// meta.json 格式与 Node.js 版本完全兼容：
/// { lastScanAt, commitHash, scanDuration, fileHashes }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaInfo {
    /// 上次扫描时间
    #[serde(rename = "lastScanAt")]
    pub last_scan_at: String,
    /// git commit hash
    #[serde(rename = "commitHash")]
    pub commit_hash: Option<String>,
    /// 扫描耗时（毫秒）
    #[serde(rename = "scanDuration", default)]
    pub scan_duration: u64,
    /// 文件哈希映射（relPath → hash），用于增量更新对比
    #[serde(rename = "fileHashes", default)]
    pub file_hashes: BTreeMap<String, String>,
}

// ── 入口点文件名集合 ──────────────────────────────────────────────────────────

const ENTRY_POINT_NAMES: &[&str] = &[
    "main", "index", "server", "app", "entry", "bootstrap",
];

// ── 公共函数 ──────────────────────────────────────────────────────────────────

/// 创建空图谱
pub fn create_empty_graph(project_name: &str, root_dir: &str) -> CodeGraph {
    let now = chrono_now();
    CodeGraph {
        version: "1.0".to_string(),
        project: ProjectInfo {
            name: project_name.to_string(),
            root: root_dir.to_string(),
        },
        scanned_at: now,
        commit_hash: None,
        config: GraphConfig {
            languages: vec![],
            exclude_patterns: vec![],
        },
        summary: GraphSummary {
            total_files: 0,
            total_functions: 0,
            total_classes: 0,
            languages: HashMap::new(),
            modules: vec![],
            entry_points: vec![],
        },
        modules: HashMap::new(),
        files: HashMap::new(),
    }
}

/// 计算文件哈希（sha256 前 16 字节，与 Node.js 版本格式一致）
pub fn compute_file_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    let hex = hex_encode(&result);
    format!("sha256:{}", &hex[..16])
}

/// 判断文件是否为入口点
pub fn is_entry_point(file_path: &Path) -> bool {
    let stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    ENTRY_POINT_NAMES.contains(&stem.as_str())
}

/// 保存图谱到 .codemap/ 目录，meta.json 格式与 Node.js 完全兼容
pub fn save_graph(output_dir: &Path, graph: &CodeGraph) -> anyhow::Result<()> {
    std::fs::create_dir_all(output_dir)?;
    let graph_json = serde_json::to_string_pretty(graph)?;
    std::fs::write(output_dir.join("graph.json"), graph_json)?;

    // 构建 fileHashes 映射（与 Node.js scan.js 逻辑一致），BTreeMap 自动按键排序
    let file_hashes: BTreeMap<String, String> = graph
        .files
        .iter()
        .map(|(k, v)| (k.clone(), v.hash.clone()))
        .collect();

    let meta = MetaInfo {
        last_scan_at: chrono_now(),
        commit_hash: graph.commit_hash.clone(),
        scan_duration: 0,
        file_hashes,
    };
    let meta_json = serde_json::to_string_pretty(&meta)?;
    std::fs::write(output_dir.join("meta.json"), meta_json)?;
    Ok(())
}

/// 从 .codemap/ 目录加载图谱
pub fn load_graph(output_dir: &Path) -> anyhow::Result<CodeGraph> {
    let data = std::fs::read_to_string(output_dir.join("graph.json"))?;
    Ok(serde_json::from_str(&data)?)
}

/// 从 .codemap/ 目录加载 meta
pub fn load_meta(output_dir: &Path) -> anyhow::Result<MetaInfo> {
    let data = std::fs::read_to_string(output_dir.join("meta.json"))?;
    Ok(serde_json::from_str(&data)?)
}

// ── 内部工具函数 ──────────────────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 获取当前 UTC 时间的 ISO 8601 字符串（简单实现，不依赖 chrono）
pub fn chrono_now() -> String {
    // 使用 std::time 获取 Unix 时间戳，格式化为 ISO 8601
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // 简单的 UTC 时间格式化
    let (y, mo, d, h, mi, s) = unix_to_datetime(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z", y, mo, d, h, mi, s)
}

fn unix_to_datetime(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let total_min = secs / 60;
    let mi = total_min % 60;
    let total_hours = total_min / 60;
    let h = total_hours % 24;
    let total_days = total_hours / 24;

    // 从 1970-01-01 计算年月日
    let mut year = 1970u64;
    let mut days = total_days;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let months = [31u64, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &m in &months {
        if days < m {
            break;
        }
        days -= m;
        month += 1;
    }
    (year, month, days + 1, h, mi, s)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_file_hash() {
        let hash = compute_file_hash(b"hello world");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 7 + 16); // "sha256:" + 16 hex chars
    }

    #[test]
    fn test_is_entry_point() {
        assert!(is_entry_point(Path::new("main.rs")));
        assert!(is_entry_point(Path::new("index.ts")));
        assert!(is_entry_point(Path::new("server.js")));
        assert!(!is_entry_point(Path::new("utils.ts")));
    }

    #[test]
    fn test_create_empty_graph() {
        let g = create_empty_graph("myproject", "/home/user/myproject");
        assert_eq!(g.version, "1.0");
        assert_eq!(g.project.name, "myproject");
        assert_eq!(g.summary.total_files, 0);
    }

    #[test]
    fn test_graph_serialization() {
        let g = create_empty_graph("test", "/tmp/test");
        let json = serde_json::to_string(&g).unwrap();
        let parsed: CodeGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, "1.0");
    }
}
