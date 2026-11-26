// #[test]
// fn test_resolve_url_imports() {
//   // Group of tests to be run.
//   let tests = vec![
//     (
//       None,
//       "http://github.com/x/core/tests/006_url_imports.js",
//       "http://github.com/x/core/tests/006_url_imports.js",
//     ),
//     (
//       Some("http://github.com/x/core/tests/006_url_imports.js"),
//       "./005_more_imports.js",
//       "http://github.com/x/core/tests/005_more_imports.js",
//     ),
//     (
//       Some("http://github.com/x/core/tests/006_url_imports.js"),
//       "../005_more_imports.js",
//       "http://github.com/x/core/005_more_imports.js",
//     ),
//     (
//       Some("http://github.com/x/core/tests/006_url_imports.js"),
//       "http://github.com/x/core/tests/005_more_imports.js",
//       "http://github.com/x/core/tests/005_more_imports.js",
//     ),
//   ];
//
//   // Run tests.
//   let loader = UrlModuleLoader::default();
//
//   for (base, specifier, expected) in tests {
//     let url = loader.resolve(base, specifier).unwrap();
//     assert_eq!(url, expected);
//   }
// }
