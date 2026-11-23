//! APIs for `globalThis` namespace.
//! See WinterCG standard: <https://github.com/wintercg/proposal-common-minimum-api>
//! See MDN javascript documents: <https://developer.mozilla.org/en-US/>

pub mod microtask;
pub mod text_encoder;
pub mod timeout;

#[cfg(test)]
mod microtask_tests;
#[cfg(test)]
mod text_encoder_tests;
#[cfg(test)]
mod timeout_tests;
