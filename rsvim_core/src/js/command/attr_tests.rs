use super::attr::*;
use std::str::FromStr;

#[test]
fn test_nargs() {
  assert_eq!(format!("{}", CommandNargs::Zero), "0");
  assert_eq!(CommandNargs::from_str("0"), Ok(CommandNargs::Zero));

  assert_eq!(format!("{}", CommandNargs::One), "1");
  assert_eq!(CommandNargs::from_str("1"), Ok(CommandNargs::One));

  assert_eq!(format!("{}", CommandNargs::Optional), "?");
  assert_eq!(CommandNargs::from_str("?"), Ok(CommandNargs::Optional));

  assert_eq!(format!("{}", CommandNargs::More), "+");
  assert_eq!(CommandNargs::from_str("+"), Ok(CommandNargs::More));

  assert_eq!(format!("{}", CommandNargs::Any), "*");
  assert_eq!(CommandNargs::from_str("*"), Ok(CommandNargs::Any));
}

#[test]
fn test_attributes_builder() {
  let a1 = CommandAttributesBuilder::default().build().unwrap();
  assert!(!a1.bang);
  assert_eq!(a1.nargs, CommandNargs::Zero);

  let a2 = CommandAttributesBuilder::default()
    .bang(true)
    .nargs(CommandNargs::Any)
    .build()
    .unwrap();
  assert!(a2.bang);
  assert_eq!(a2.nargs, CommandNargs::Any);
}
