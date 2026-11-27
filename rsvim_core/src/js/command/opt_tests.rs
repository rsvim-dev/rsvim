use super::opt::*;
use compact_str::ToCompactString;

#[test]
fn test_options_builder() {
  let o1 = CommandOptionsBuilder::default().build().unwrap();
  assert!(o1.force);
  assert_eq!(o1.alias, None);

  let o2 = CommandOptionsBuilder::default()
    .force(false)
    .alias(Some("w".to_compact_string()))
    .build()
    .unwrap();
  assert!(!o2.force);
  assert_eq!(o2.alias, Some("w".to_compact_string()));
}
