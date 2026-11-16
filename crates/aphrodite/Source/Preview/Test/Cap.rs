//! `State::Maximum::Value` cap, end to end.
//!
//! The cap is the one guarantee every arm shares, so it is tested against a
//! generic arm, a structured arm and multibyte content: a cap that only works on
//! ASCII, or only on the generic path, is not a cap.

use super::*;

#[test]
fn TestPreviewCapTruncatesAndKeepsBracket() {
	let _G = CapGuard();
	let Previous = crate::Preview::State::Set::Fn(Some(30));
	let long = format!("some {} prose", "x".repeat(200));
	let p = crate::Preview::Builder::Entry::Fn("text", &long);
	assert!(p.chars().count() <= 30, "preview must respect the cap: {p}");
	assert!(p.ends_with(']'), "self-bracketed preview must keep its closing bracket: {p}");
	assert_eq!(p.chars().count(), 30, "truncated preview should fill the budget exactly: {p}");
	crate::Preview::State::Set::Fn(Previous);
}

#[test]
fn TestPreviewCapUnsetIsUnlimited() {
	let _G = CapGuard();
	crate::Preview::State::Set::Fn(None);
	let long = format!("some {} prose", "y".repeat(200));
	let p = crate::Preview::Builder::Entry::Fn("text", &long);
	assert!(p.chars().count() > 30, "no cap -> full preview expected: {p}");
	assert!(p.contains("yyy"), "got {p}");
}

#[test]
fn TestPreviewCapMultibyteTruncatesOnCharBoundary() {
	let _G = CapGuard();
	let Previous = crate::Preview::State::Set::Fn(Some(25));
	let Content = format!("{}{}", "a\u{00e9}\u{4e2d}\u{1f600}".repeat(30), " end");
	let p = crate::Preview::Builder::Entry::Fn("text", &Content);
	assert!(
		p.chars().count() <= 25,
		"multibyte truncation must stay on a char boundary: {p}"
	);
	crate::Preview::State::Set::Fn(Previous);
}

#[test]
fn TestPreviewCapAppliesToBuildArmToo() {
	let _G = CapGuard();
	let Previous = crate::Preview::State::Set::Fn(Some(20));
	let c = "error[E0432]: unresolved import `crate::foo`\n  --> src/x.rs:1:5\nwarning: unused variable `y`";
	let p = crate::Preview::Builder::Entry::Fn("build", c);
	assert!(p.chars().count() <= 20, "cap must apply to every arm: {p}");
	assert!(p.ends_with(']'));
	crate::Preview::State::Set::Fn(Previous);
}

/// Cap one string directly, bypassing the process-wide atomic and every builder arm.
fn Capped(Preview:String, Maximum:usize) -> String {
	crate::Preview::Render::Cap::Fn::Fn(Preview, &crate::Preview::Limit::Resolve::Fn::Fn(Maximum))
}

/// The boundary table, pinned at the edge where the previous `checked_sub` test misfired.
///
/// "Fits" is `count <= Maximum`, so a preview whose length EQUALS the limit comes back
/// unchanged - a `checked_sub` reads equality as a successful subtraction and cut it to
/// `[...]`. The unit is the Unicode scalar value: a four-scalar multibyte preview fits a limit
/// of four and is cut to three scalars at three, never to a byte or grapheme count.
#[test]
fn TestPreviewCapFitsUnchangedAtBoundary() {
	assert_eq!(
		Capped("[abc]".to_owned(), 5),
		"[abc]",
		"exactly at the limit must not be truncated"
	);
	assert_eq!(
		Capped("abc".to_owned(), 3),
		"abc",
		"exactly at the limit, no bracket to preserve"
	);
	assert_eq!(
		Capped("[abc]".to_owned(), 4),
		"...]",
		"one over the limit keeps the closing bracket"
	);
	assert_eq!(Capped("[abc]".to_owned(), 6), "[abc]", "under the limit must not be truncated");
	assert_eq!(
		Capped("a\u{00e9}\u{4e2d}\u{1f600}".to_owned(), 4),
		"a\u{00e9}\u{4e2d}\u{1f600}",
		"four scalars fit a limit of four"
	);
	assert_eq!(
		Capped("a\u{00e9}\u{4e2d}\u{1f600}".to_owned(), 3),
		"a\u{00e9}\u{4e2d}",
		"the cut counts scalars, not bytes"
	);
	assert_eq!(Capped(String::new(), 1), "", "an empty preview is always inside its budget");
}

/// The fits-unchanged property rather than the table: at every limit, every preview at or under
/// it is returned byte-identical, and every preview over it is capped to at most the limit.
#[test]
fn TestPreviewCapIdentityAtAndBelowLimit() {
	(1..=16usize).for_each(|Maximum| {
		(0..=Maximum).for_each(|Length| {
			let Bracketed = (Length >= 2).then(|| format!("[{}]", "a".repeat(Length - 2)));

			std::iter::once("a".repeat(Length)).chain(Bracketed).for_each(|Preview| {
				assert_eq!(
					Capped(Preview.clone(), Maximum),
					Preview,
					"{Preview:?} at {Maximum} must be unchanged"
				);
			});
		});

		(Maximum + 1..=Maximum + 8).for_each(|Length| {
			let Preview = "a".repeat(Length);
			let Count = Capped(Preview.clone(), Maximum).chars().count();
			assert!(Count <= Maximum, "{Preview:?} at {Maximum} must be capped, got {Count}");
		});
	});
}
