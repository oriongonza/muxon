//! Integration tests for the blob store.

use anyhow::Result;
use resurreccion_store::Store;
use tempfile::TempDir;

fn temp_store() -> Result<(TempDir, Store)> {
    let dir = TempDir::new()?;
    let path = dir.path().join("test.db");
    let store = Store::open(path.to_str().unwrap())?;
    Ok((dir, store))
}

#[test]
fn blob_put_idempotent() -> Result<()> {
    let (_dir, store) = temp_store()?;

    let data = b"test blob content";

    // Put the same data twice
    let hash1 = store.blob_put(data)?;
    let hash2 = store.blob_put(data)?;

    // Both should return the same hash
    assert_eq!(hash1, hash2);

    Ok(())
}

#[test]
fn blob_get_returns_correct_data() -> Result<()> {
    let (_dir, store) = temp_store()?;

    let data = b"test blob content";

    // Put data and get the hash
    let hash = store.blob_put(data)?;

    // Get the data back
    let retrieved = store.blob_get(&hash)?;

    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved, data);

    Ok(())
}

#[test]
fn blob_get_missing_returns_none() -> Result<()> {
    let (_dir, store) = temp_store()?;

    // Try to get a blob that doesn't exist
    let result = store.blob_get("nonexistent_hash")?;

    assert!(result.is_none());

    Ok(())
}
