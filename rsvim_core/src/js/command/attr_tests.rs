use super::attr::*;
use crate::js::converter::*;
use crate::tests::js::*;
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
#[cfg_attr(miri, ignore)]
fn test_converter1() {
  let mut jsrt = make_js_runtime();
  let context = jsrt.context();
  v8::scope_with_context!(scope, &mut jsrt.isolate, context);

  let a1 = CommandAttributesBuilder::default().build().unwrap();
  let obj1 = a1.to_v8(scope);
  let val1 = CommandAttributes::from_v8(scope, obj1);
  assert_eq!(val1, a1);

  let a2 = CommandAttributesBuilder::default()
    .nargs(CommandNargs::Any)
    .build()
    .unwrap();
  let obj2 = a2.to_v8(scope);
  let val2 = CommandAttributes::from_v8(scope, obj2);
  assert_eq!(val2, a2);
}
