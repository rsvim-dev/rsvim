use super::*;
use crate::js::resource::ResourceTable;
use assert_fs::prelude::*;

#[test]
#[cfg_attr(miri, ignore)]
fn open_read_write_close_round_trip() {
  crate::tests::log::init();
  let tmpfile = assert_fs::NamedTempFile::new("fs-neutral.txt").unwrap();
  let resource_table = ResourceTable::to_arc(ResourceTable::new());

  let rid = fs_open_s(
    resource_table.clone(),
    tmpfile.path(),
    FsOpenOptionsBuilder::default()
      .create(true)
      .write(true)
      .read(true)
      .build()
      .unwrap(),
  )
  .unwrap();

  let written =
    fs_write_s(resource_table.clone(), rid, b"hello".to_vec()).unwrap();
  assert_eq!(written, 5);

  // Re-open for reading: the handle above is positioned at the end.
  fs_close_s(resource_table.clone(), rid);
  assert_eq!(fs_read_file_s(tmpfile.path()).unwrap(), b"hello".to_vec());
}

#[test]
#[cfg_attr(miri, ignore)]
fn read_dir_lists_the_temporary_directory() {
  crate::tests::log::init();
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("a.txt").touch().unwrap();
  let resource_table = ResourceTable::to_arc(ResourceTable::new());

  let rid = fs_read_dir_s(resource_table.clone(), dir.path()).unwrap();
  let name = fs_read_dir_next_s(resource_table.clone(), rid)
    .expect("one entry")
    .unwrap();
  assert_eq!(name.name, "a.txt");
  assert!(name.is_file);

  // The iterator is exhausted after the single entry.
  assert!(fs_read_dir_next_s(resource_table.clone(), rid).is_none());
  fs_read_dir_close_s(resource_table, rid);
}

#[test]
#[cfg_attr(miri, ignore)]
fn stat_reports_a_directory() {
  crate::tests::log::init();
  let dir = assert_fs::TempDir::new().unwrap();
  let meta = fs_stat_s(dir.path()).unwrap();
  assert!(meta.is_dir);
  assert!(!meta.is_file);
}

#[test]
#[cfg_attr(miri, ignore)]
fn lstat_on_a_missing_path_is_an_error() {
  crate::tests::log::init();
  let result = fs_lstat_s(Path::new("/nonexistent/rsvim-neutral-test"));
  assert!(result.is_err());
}

#[test]
#[cfg_attr(miri, ignore)]
fn mkdir_then_remove() {
  crate::tests::log::init();
  let dir = assert_fs::TempDir::new().unwrap();
  let nested = dir.path().join("a").join("b");
  fs_mkdir_s(
    &nested,
    FsMkdirOptionsBuilder::default()
      .recursive(true)
      .build()
      .unwrap(),
  )
  .unwrap();
  assert!(nested.is_dir());
}

#[test]
#[cfg_attr(miri, ignore)]
fn link_and_symlink() {
  crate::tests::log::init();
  let dir = assert_fs::TempDir::new().unwrap();
  let target = dir.child("target.txt");
  target.write_str("x").unwrap();
  let hard = dir.path().join("hard.txt");
  fs_link_s(target.path(), &hard).unwrap();
  assert!(hard.is_file());

  let soft = dir.path().join("soft.txt");
  fs_symlink_s(target.path(), &soft, FsSymlinkOptions::File).unwrap();
  assert!(soft.symlink_metadata().unwrap().is_symlink());
}
