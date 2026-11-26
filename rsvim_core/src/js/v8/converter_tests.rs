use super::converter::*;
use crate::tests::js::*;

#[test]
#[cfg_attr(miri, ignore)]
fn test_integer1() {
  let mut jsrt = make_js_runtime();
  let context = jsrt.context();
  v8::scope_with_context!(scope, &mut jsrt.isolate, context);

  let a1 = 10_i32;
  let obj1 = a1.to_v8(scope);
  let val1 = i32::from_v8(scope, obj1);
  assert_eq!(val1, a1);

  let a2 = 10_u32;
  let obj2 = a2.to_v8(scope);
  let val2 = u32::from_v8(scope, obj2);
  assert_eq!(val2, a2);
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_number1() {
  let mut jsrt = make_js_runtime();
  let context = jsrt.context();
  v8::scope_with_context!(scope, &mut jsrt.isolate, context);

  let a1 = 1.23_f64;
  let obj1 = a1.to_v8(scope);
  let val1 = f64::from_v8(scope, obj1);
  assert_eq!(val1, a1);

  let a2 = 8_f64;
  let obj2 = a2.to_v8(scope);
  let val2 = f64::from_v8(scope, obj2);
  assert_eq!(val2, a2);
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_bool1() {
  let mut jsrt = make_js_runtime();
  let context = jsrt.context();
  v8::scope_with_context!(scope, &mut jsrt.isolate, context);

  let a1 = true;
  let obj1 = a1.to_v8(scope);
  let val1 = bool::from_v8(scope, obj1);
  assert!(val1);

  let a2 = false;
  let obj2 = a2.to_v8(scope);
  let val2 = bool::from_v8(scope, obj2);
  assert!(!val2);
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_string1() {
  let mut jsrt = make_js_runtime();
  let context = jsrt.context();
  v8::scope_with_context!(scope, &mut jsrt.isolate, context);

  let a1 = "Hello".to_string();
  let obj1 = a1.to_v8(scope);
  let val1 = String::from_v8(scope, obj1);
  assert_eq!(val1, a1);
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_array1() {
  let mut jsrt = make_js_runtime();
  let context = jsrt.context();
  v8::scope_with_context!(scope, &mut jsrt.isolate, context);

  let a1: Vec<i32> = vec![1, 2, 3];
  let obj1 = a1.to_v8(scope);
  let val1 = Vec::<i32>::from_v8(scope, obj1);
  assert_eq!(val1, a1);

  let a2: Vec<String> = vec!["a".to_string(), "b".to_string(), "c".to_string()];
  let obj2 = a2.to_v8(scope);
  let val2 = Vec::<String>::from_v8(scope, obj2);
  assert_eq!(val2, a2);
}
