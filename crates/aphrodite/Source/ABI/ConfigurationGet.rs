//! `AphroditeConfigurationGet` - read one configuration value from the session.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, State::Bare::Fn::Fn as Bare},
	Guard::Cstr::{Fn::Fn as Cstr, Optional::Fn::Fn as Optional},
};

/// Answer with one configuration value as a bare C string.
///
/// The key set is closed and deliberately small: an unknown key is an error, not
/// an empty answer, because a host reading a typo'd key would otherwise believe
/// the value is unset. Numbers are rendered as decimal strings, so the caller
/// parses rather than the ABI inventing a type tag.
///
/// Like `AphroditeRetrieve`, success is the value itself - there is no JSON
/// envelope to unwrap.
///
/// The key match stays a key match: it is one dispatch that projects one field,
/// and turning it into an enum would only move the same match one call deeper.
/// The settable side is different and does have an enum
/// (`Configuration::Update`), because there the value has to be parsed for its
/// field's type before the session lock is taken, and that parse is what the
/// type carries.
/// # Safety
/// `Handle` and `Key` must each be null or point to a valid, NUL-terminated C
/// string that stays readable and unmoved for the duration of this call. The
/// returned pointer is owned by the caller and must be released with
/// `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeConfigurationGet(Handle:*const c_char, Key:*const c_char) -> *mut c_char {
	let Key = unsafe { Cstr(Key) };

	Bare(Handle, move |s| {
		let Key = Optional(Key)?;

		let Value = match &*Key {
			"model" => s.Model.clone(),
			"api_url" => s.APIURL.clone(),
			"engine_threshold_pct" => s.EngineThresholdPct.to_string(),
			"tool_threshold" => s.ToolThreshold.to_string(),
			"context_engine_enabled" => s.ContextEngineEnabled.to_string(),
			_ => anyhow::bail!("unknown key: {}", Key),
		};

		Ok(Answer::Text(Value))
	})
}
