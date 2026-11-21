//! Js runtime hooks: promise, import and import.meta, etc.

use crate::js::JsRuntime;
use crate::js::binding::throw_type_error;
use crate::js::module::resolve_import;
use crate::prelude::*;
use crate::util::paths;

/// Called during `Module::instantiate_module`, see:
/// - Rusty V8 API:
///   - <https://docs.rs/v8/latest/v8/struct.Module.html#method.instantiate_module>.
///   - <https://docs.rs/rusty_v8/latest/rusty_v8/type.ResolveModuleCallback.html>
/// - Node V8 API: <https://v8docs.nodesource.com/node-24.1/df/d74/classv8_1_1_module.html#a3313f8faa14b6dc5d37c340d45273bf1>.
pub fn module_resolve_cb<'a>(
  context: v8::Local<'a, v8::Context>,
  specifier: v8::Local<'a, v8::String>,
  _import_attributes: v8::Local<'a, v8::FixedArray>,
  referrer: v8::Local<'a, v8::Module>,
) -> Option<v8::Local<'a, v8::Module>> {
  // Get `CallbackScope` from context.
  v8::callback_scope!(unsafe scope, context);
  let state_rc = JsRuntime::state(scope);
  let state = state_rc.borrow();

  let import_map = state.options.import_map.clone();
  let referrer_id = referrer.get_identity_hash().get();
  let referrer = v8::Global::new(scope, referrer);

  let dependant = state.module_map.get_path(referrer_id, referrer).unwrap();
  let dependant = paths::maybe_parent(&dependant).to_string_lossy();
  let specifier = specifier.to_rust_string_lossy(scope);
  let specifier = resolve_import(&dependant, &specifier, import_map).unwrap();
  trace!(
    "|module_resolve_cb| dependant:{:?}, specifier:{:?}",
    dependant, specifier
  );

  // This call should always give us back the module.
  let module = state.module_map.get(&specifier).unwrap();

  Some(v8::Local::new(scope, module))
}

/// Called the first time import.meta is accessed for a module.
/// See: <https://docs.rs/v8/0.49.0/v8/type.HostInitializeImportMetaObjectCallback.html>.
pub extern "C" fn host_initialize_import_meta_object_cb(
  context: v8::Local<v8::Context>,
  module: v8::Local<v8::Module>,
  meta: v8::Local<v8::Object>,
) {
  // Get `CallbackScope` from context.
  v8::callback_scope!(unsafe scope, context);

  let state_rc = JsRuntime::state(scope);
  let state = state_rc.borrow();

  // Make the module global.
  let module_id = module.get_identity_hash().get();
  let module = v8::Global::new(scope, module);

  let filename = state.module_map.get_path(module_id, module).unwrap();
  let is_main = state.module_map.main().as_ref() == Some(&filename);
  trace!(
    "|host_initialize_import_meta_object_cb| url:{:?}, is_main:{:?}",
    filename, is_main
  );

  // `import.meta.filename`
  let key = v8::String::new(scope, "filename").unwrap();
  let value = v8::String::new(scope, &filename).unwrap();
  meta.create_data_property(scope, key.into(), value.into());

  // `import.meta.dirname`
  let filepath = Path::new(&filename).normalize().unwrap();
  let dirname = paths::maybe_parent(&filepath);
  let key = v8::String::new(scope, "dirname").unwrap();
  let value = v8::String::new(scope, &dirname.to_string_lossy()).unwrap();
  meta.create_data_property(scope, key.into(), value.into());

  // `import.meta.url`
  let url = format!("file://{}", filepath.as_path().to_string_lossy());
  let key = v8::String::new(scope, "url").unwrap();
  let value = v8::String::new(scope, &url).unwrap();
  meta.create_data_property(scope, key.into(), value.into());

  // `import.meta.main`
  let key = v8::String::new(scope, "main").unwrap();
  let value = v8::Boolean::new(scope, is_main);
  meta.create_data_property(scope, key.into(), value.into());

  // `import.meta.resolve()`
  let url = v8::String::new(scope, &filename).unwrap();
  let builder = v8::FunctionBuilder::new(import_meta_resolve).data(url.into());
  let key = v8::String::new(scope, "resolve").unwrap();
  let value =
    v8::FunctionBuilder::<v8::Function>::build(builder, scope).unwrap();
  meta.set(scope, key.into(), value.into());
}

fn import_meta_resolve(
  scope: &mut v8::PinScope,
  args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  // Check for provided arguments.
  if args.length() == 0 {
    throw_type_error(scope, &TheErr::ArgumentsNotEnough);
    return;
  }

  let state_rc = JsRuntime::state(scope);
  let state = state_rc.borrow();
  let base = args.data().to_rust_string_lossy(scope);
  let base = paths::maybe_parent(&base).to_string_lossy();
  let specifier = args.get(0).to_rust_string_lossy(scope);
  trace!(
    "|import_meta_resolve| base:{:?}, specifier:{:?}",
    base, specifier
  );
  let import_map = state.options.import_map.clone();

  match resolve_import(&base, &specifier, import_map) {
    Ok(path) => rv.set(v8::String::new(scope, &path).unwrap().into()),
    Err(e) => throw_type_error(scope, &e),
  };
}

/// Called when a promise rejects with no rejection handler specified.
/// See: <https://docs.rs/v8/0.49.0/v8/type.PromiseRejectCallback.html>.
/// See: <https://v8.dev/features/promise-combinators>.
pub extern "C" fn promise_reject_cb(message: v8::PromiseRejectMessage) {
  // Create a v8 callback-scope.
  v8::callback_scope!(unsafe scope, &message);
  let undefined = v8::undefined(scope).into();
  let event = message.get_event();
  trace!("|promise_reject_cb| event:{event:?}, message:{message:?}");

  use v8::PromiseRejectEvent::PromiseHandlerAddedAfterReject;
  use v8::PromiseRejectEvent::PromiseRejectAfterResolved;
  use v8::PromiseRejectEvent::PromiseRejectWithNoHandler;
  use v8::PromiseRejectEvent::PromiseResolveAfterResolved;

  let reason = match event {
    PromiseHandlerAddedAfterReject
    | PromiseRejectAfterResolved
    | PromiseResolveAfterResolved => undefined,
    PromiseRejectWithNoHandler => message.get_value().unwrap(),
  };

  let promise = message.get_promise();
  let promise = v8::Global::new(scope, promise);

  let state_rc = JsRuntime::state(scope);
  let mut state = state_rc.borrow_mut();

  match event {
    // Note: We might need to "interrupt" the event loop to handle
    // the promise rejection in a timely manner.
    PromiseRejectWithNoHandler => {
      let reason = v8::Global::new(scope, reason);
      state.exceptions.capture_promise_rejection(promise, reason);
      // state.interrupt_handle.interrupt();
    }
    PromiseHandlerAddedAfterReject => {
      state.exceptions.remove_promise_rejection(&promise);
    }
    PromiseRejectAfterResolved | PromiseResolveAfterResolved => {}
  }
}
