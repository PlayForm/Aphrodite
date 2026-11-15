//! Shared fixtures for the proxy tests.
//!
//! One file per fixture family; the facade re-exports each fixture under the
//! name the tests call it by.

pub mod Chat;
pub mod Lock;
pub mod State;
pub mod Store;
pub mod Upstream;

pub(crate) use Chat::Fn::ChatCompletionBody;
pub(crate) use Lock::Const::ReloadTestLock;
pub(crate) use State::Fn::{TestState, TestStateWithCCR, TestStateWithFailingCCR, TestStateWithMaximumBody};
pub(crate) use Store::Struct::Struct as FailingCCRStore;
pub(crate) use Upstream::Fn::{HeaderValue, SpawnMockUpstream, TestRequest};
