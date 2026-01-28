use chrono::Utc;
use libc::statvfs;
use serde::Serialize;
use std::collections::HashSet;
use std::ffi::CString;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Clone)]
pub struct StorageCategory {
    pub id: String,
    pub label: String,
    pub bytes: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct RamStorageBreakdown {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct StorageBreakdown {
    pub mount: String,
    pub timestamp: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub categories: Vec<StorageCategory>,
    pub ram_storage: Option<RamStorageBreakdown>,
}

#[derive(Debug, Clone)]
struct StorageConfig {
    mount_path: PathBuf,
    service_paths: Vec<PathBuf>,
    app_paths: Vec<PathBuf>,
}

impl StorageConfig {
    fn from_env() -> Self {
        let mount_path = std::env::var("DISK_MONITOR_PATH").unwrap_or_else(|_| "/".to_string());
        let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "home_automation.db".to_string());
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

        let mut service_paths = vec![
            PathBuf::from("/var/lib/mosquitto"),
            PathBuf::from("/var/lib/zigbee2mqtt"),
            PathBuf::from("/opt/zigbee2mqtt/data"),
            PathBuf::from("/var/lib/home-automation-rs"),
        ];

        if let Ok(extra) = std::env::var("DISK_SERVICE_PATHS") {
            for item in extra.split(',') {
                let trimmed = item.trim();
                if !trimmed.is_empty() {
                    service_paths.push(PathBuf::from(trimmed));
                }
            }
        }

        let mut app_paths = vec![
            PathBuf::from(db_path),
            PathBuf::from(format!("{}/system_monitor.log", home_dir)),
            PathBuf::from(format!("{}/process_monitor.log", home_dir)),
            PathBuf::from(format!("{}/top_cpu_consumers.log", home_dir)),
            PathBuf::from(format!("{}/top_ram_consumers.log", home_dir)),
        ];

        if let Ok(extra) = std::env::var("DISK_APP_PATHS") {
            for item in extra.split(',') {
                let trimmed = item.trim();
                if !trimmed.is_empty() {
                    app_paths.push(PathBuf::from(trimmed));
                }
            }
        }

        Self {
            mount_path: PathBuf::from(mount_path),
            service_paths,
            app_paths,
        }
    }
}

#[derive(Debug)]
struct FsStats {
    total_bytes: u64,
    free_bytes: u64,
}

pub fn compute_storage_breakdown() -> Result<StorageBreakdown, String> {
    let config = StorageConfig::from_env();
    let stats = statvfs_bytes(&config.mount_path)?;

    let used_bytes = stats.total_bytes.saturating_sub(stats.free_bytes);

    let binaries_paths = vec![
        PathBuf::from("/bin"),
        PathBuf::from("/sbin"),
        PathBuf::from("/lib"),
        PathBuf::from("/lib64"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/usr/sbin"),
        PathBuf::from("/usr/lib"),
    ];
    let boot_paths = vec![PathBuf::from("/boot")];
    let home_paths = vec![PathBuf::from("/home"), PathBuf::from("/root")];
    let runtime_paths = vec![
        PathBuf::from("/var"),
        PathBuf::from("/run"),
        PathBuf::from("/tmp"),
    ];

    let binaries_bytes = sum_paths(&binaries_paths);
    let boot_bytes = sum_paths(&boot_paths);
    let home_bytes = sum_paths(&home_paths);
    let runtime_total_bytes = sum_paths(&runtime_paths);

    let service_sizes = sized_paths(&config.service_paths);
    let app_sizes = sized_paths(&config.app_paths);
    let service_bytes = sum_sized(&service_sizes);
    let app_bytes = sum_sized(&app_sizes);

    let mut overlap_bytes = 0u64;
    let overlap_candidates = service_sizes
        .iter()
        .chain(app_sizes.iter())
        .collect::<Vec<_>>();
    for base in &runtime_paths {
        overlap_bytes = overlap_bytes.saturating_add(sum_under(base, &overlap_candidates));
    }
    let runtime_bytes = runtime_total_bytes.saturating_sub(overlap_bytes);

    let known_used = binaries_bytes
        .saturating_add(runtime_bytes)
        .saturating_add(service_bytes)
        .saturating_add(app_bytes)
        .saturating_add(home_bytes)
        .saturating_add(boot_bytes);
    let other_bytes = used_bytes.saturating_sub(known_used);

    let categories = vec![
        StorageCategory {
            id: "binaries".to_string(),
            label: "Binaries".to_string(),
            bytes: binaries_bytes,
        },
        StorageCategory {
            id: "runtime".to_string(),
            label: "Runtime".to_string(),
            bytes: runtime_bytes,
        },
        StorageCategory {
            id: "services".to_string(),
            label: "Services".to_string(),
            bytes: service_bytes,
        },
        StorageCategory {
            id: "app_data".to_string(),
            label: "App Data".to_string(),
            bytes: app_bytes,
        },
        StorageCategory {
            id: "home".to_string(),
            label: "Home".to_string(),
            bytes: home_bytes,
        },
        StorageCategory {
            id: "boot".to_string(),
            label: "Boot".to_string(),
            bytes: boot_bytes,
        },
        StorageCategory {
            id: "other".to_string(),
            label: "Other".to_string(),
            bytes: other_bytes,
        },
        StorageCategory {
            id: "free".to_string(),
            label: "Free".to_string(),
            bytes: stats.free_bytes,
        },
    ];

    Ok(StorageBreakdown {
        mount: config.mount_path.display().to_string(),
        timestamp: Utc::now().to_rfc3339(),
        total_bytes: stats.total_bytes,
        free_bytes: stats.free_bytes,
        used_bytes,
        categories,
        ram_storage: tmpfs_breakdown(),
    })
}

fn statvfs_bytes(path: &Path) -> Result<FsStats, String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| "Invalid path for statvfs".to_string())?;
    let c_path = CString::new(path_str).map_err(|e| format!("Invalid path: {}", e))?;

    let mut vfs: libc::statvfs = unsafe { std::mem::zeroed() };
    let result = unsafe { statvfs(c_path.as_ptr(), &mut vfs) };
    if result != 0 {
        return Err(format!("statvfs failed for {}", path_str));
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    let block_size = vfs.f_frsize;
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let block_size = vfs.f_frsize as u64;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    let total = vfs.f_blocks * block_size;
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let total = vfs.f_blocks as u64 * block_size;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    let free = vfs.f_bavail * block_size;
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let free = vfs.f_bavail as u64 * block_size;

    Ok(FsStats {
        total_bytes: total,
        free_bytes: free,
    })
}

fn sized_paths(paths: &[PathBuf]) -> Vec<(PathBuf, u64)> {
    paths
        .iter()
        .map(|path| (path.clone(), size_for_path(path)))
        .collect()
}

fn sum_sized(entries: &[(PathBuf, u64)]) -> u64 {
    entries.iter().map(|(_, size)| *size).sum()
}

fn sum_paths(paths: &[PathBuf]) -> u64 {
    paths.iter().map(|path| size_for_path(path)).sum()
}

fn size_for_path(path: &Path) -> u64 {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.is_file() {
            return metadata.len();
        }
        if metadata.is_dir() {
            return dir_size(path);
        }
    }
    0
}

fn dir_size(root: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![root.to_path_buf()];

    while let Some(path) = stack.pop() {
        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let entry_path = entry.path();
            let metadata = match fs::symlink_metadata(&entry_path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };

            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                stack.push(entry_path);
            } else if file_type.is_file() {
                total = total.saturating_add(metadata.len());
            }
        }
    }

    total
}

fn sum_under(base: &Path, entries: &[&(PathBuf, u64)]) -> u64 {
    entries
        .iter()
        .filter(|(path, _)| path_is_under(base, path))
        .map(|(_, size)| *size)
        .sum()
}

fn path_is_under(base: &Path, path: &Path) -> bool {
    let base = match base.canonicalize() {
        Ok(base) => base,
        Err(_) => return false,
    };
    let path = match path.canonicalize() {
        Ok(path) => path,
        Err(_) => return false,
    };
    path.starts_with(base)
}

fn tmpfs_breakdown() -> Option<RamStorageBreakdown> {
    let mounts = match fs::read_to_string("/proc/mounts") {
        Ok(content) => content,
        Err(_) => return None,
    };

    let mut mountpoints = HashSet::new();
    for line in mounts.lines() {
        let mut parts = line.split_whitespace();
        let _device = parts.next();
        let mountpoint = parts.next();
        let fstype = parts.next();
        if let (Some(mountpoint), Some(fstype)) = (mountpoint, fstype)
            && fstype == "tmpfs"
            && (mountpoint == "/run" || mountpoint == "/tmp" || mountpoint == "/dev/shm")
        {
            mountpoints.insert(mountpoint.to_string());
        }
    }

    if mountpoints.is_empty() {
        return None;
    }

    let mut total = 0u64;
    let mut free = 0u64;
    for mount in mountpoints {
        let path = PathBuf::from(mount);
        if let Ok(stats) = statvfs_bytes(&path) {
            total = total.saturating_add(stats.total_bytes);
            free = free.saturating_add(stats.free_bytes);
        }
    }

    let used = total.saturating_sub(free);
    Some(RamStorageBreakdown {
        total_bytes: total,
        free_bytes: free,
        used_bytes: used,
    })
}
