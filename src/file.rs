use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn read<P: AsRef<Path>>(path: P) -> crate::Result<Vec<u8>> {
    let data = fs::read(path).await?;
    Ok(data)
}

pub async fn read_to_string<P: AsRef<Path>>(path: P) -> crate::Result<String> {
    let content = fs::read_to_string(path).await?;
    Ok(content)
}

pub async fn write<P: AsRef<Path>>(path: P, data: &[u8]) -> crate::Result<()> {
    fs::write(path, data).await?;
    Ok(())
}

pub async fn write_string<P: AsRef<Path>>(path: P, content: &str) -> crate::Result<()> {
    write(path, content.as_bytes()).await
}

pub async fn append<P: AsRef<Path>>(path: P, data: &[u8]) -> crate::Result<()> {
    use tokio::fs::OpenOptions;
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    
    file.write_all(data).await?;
    file.flush().await?;
    Ok(())
}

pub async fn exists<P: AsRef<Path>>(path: P) -> bool {
    fs::metadata(path).await.is_ok()
}

pub async fn is_file<P: AsRef<Path>>(path: P) -> bool {
    match fs::metadata(path).await {
        Ok(metadata) => metadata.is_file(),
        Err(_) => false,
    }
}

pub async fn is_dir<P: AsRef<Path>>(path: P) -> bool {
    match fs::metadata(path).await {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

pub async fn mkdir<P: AsRef<Path>>(path: P) -> crate::Result<()> {
    fs::create_dir_all(path).await?;
    Ok(())
}

pub async fn remove_file<P: AsRef<Path>>(path: P) -> crate::Result<()> {
    fs::remove_file(path).await?;
    Ok(())
}

pub async fn remove_dir<P: AsRef<Path>>(path: P) -> crate::Result<()> {
    fs::remove_dir_all(path).await?;
    Ok(())
}

pub async fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> crate::Result<u64> {
    let bytes_copied = fs::copy(from, to).await?;
    Ok(bytes_copied)
}

pub async fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> crate::Result<()> {
    fs::rename(from, to).await?;
    Ok(())
}

pub async fn list_dir<P: AsRef<Path>>(path: P) -> crate::Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    let mut dir = fs::read_dir(path).await?;
    
    while let Some(entry) = dir.next_entry().await? {
        entries.push(entry.path());
    }
    
    Ok(entries)
}

pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub modified: std::time::SystemTime,
}

pub async fn stat<P: AsRef<Path>>(path: P) -> crate::Result<FileInfo> {
    let path = path.as_ref().to_path_buf();
    let metadata = fs::metadata(&path).await?;
    
    Ok(FileInfo {
        path,
        size: metadata.len(),
        is_file: metadata.is_file(),
        is_dir: metadata.is_dir(),
        modified: metadata.modified()?,
    })
}

pub async fn walk_dir<P: AsRef<Path>>(
    path: P,
    mut callback: impl FnMut(&Path) -> bool,
) -> crate::Result<()> {
    async fn walk_recursive<F>(path: &Path, callback: &mut F) -> crate::Result<()>
    where
        F: FnMut(&Path) -> bool,
    {
        if !callback(path) {
            return Ok(());
        }
        
        if let Ok(metadata) = fs::metadata(path).await {
            if metadata.is_dir() {
                let mut dir = fs::read_dir(path).await?;
                while let Some(entry) = dir.next_entry().await? {
                    walk_recursive(&entry.path(), callback).await?;
                }
            }
        }
        Ok(())
    }
    
    walk_recursive(path.as_ref(), &mut callback).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_operations() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let content = "Hello, rust-eco!";

        // Test write and read
        write_string(&file_path, content).await.unwrap();
        let read_content = read_to_string(&file_path).await.unwrap();
        assert_eq!(content, read_content);

        // Test exists
        assert!(exists(&file_path).await);
        assert!(is_file(&file_path).await);
        assert!(!is_dir(&file_path).await);

        // Test append
        append(&file_path, b" More content").await.unwrap();
        let appended_content = read_to_string(&file_path).await.unwrap();
        assert_eq!(appended_content, "Hello, rust-eco! More content");

        // Test stat
        let info = stat(&file_path).await.unwrap();
        assert!(info.is_file);
        assert!(!info.is_dir);
        assert!(info.size > 0);

        // Test remove
        remove_file(&file_path).await.unwrap();
        assert!(!exists(&file_path).await);
    }

    #[tokio::test]
    async fn test_directory_operations() {
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path().join("test_dir");

        // Test mkdir
        mkdir(&test_dir).await.unwrap();
        assert!(exists(&test_dir).await);
        assert!(is_dir(&test_dir).await);

        // Create some files
        let file1 = test_dir.join("file1.txt");
        let file2 = test_dir.join("file2.txt");
        write_string(&file1, "content1").await.unwrap();
        write_string(&file2, "content2").await.unwrap();

        // Test list_dir
        let entries = list_dir(&test_dir).await.unwrap();
        assert_eq!(entries.len(), 2);

        // Test walk_dir
        let mut paths = Vec::new();
        walk_dir(&test_dir, |path| {
            paths.push(path.to_path_buf());
            true
        }).await.unwrap();
        assert!(!paths.is_empty());

        // Test remove_dir
        remove_dir(&test_dir).await.unwrap();
        assert!(!exists(&test_dir).await);
    }
}