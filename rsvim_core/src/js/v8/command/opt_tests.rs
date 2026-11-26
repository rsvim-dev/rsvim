use super::opt::*;
use crate::js::converter::*;
use crate::tests::js::*;
use compact_str::ToCompactString;

#[test]
#[cfg_attr(miri, ignore)]
fn test1() {
  let mut jsrt = make_js_runtime();
  let context = jsrt.context();
  v8::scope_with_context!(scope, &mut jsrt.isolate, context);

  let a1 = CommandOptionsBuilder::default().build().unwrap();
  let obj1 = a1.to_v8(scope);
  let val1 = CommandOptions::from_v8(scope, obj1);
  assert_eq!(val1, a1);

  let a2 = CommandOptionsBuilder::default()
    .alias(Some("w".to_compact_string()))
    .build()
    .unwrap();
  let obj2 = a2.to_v8(scope);
  let val2 = CommandOptions::from_v8(scope, obj2);
  assert_eq!(val2, a2);
}
