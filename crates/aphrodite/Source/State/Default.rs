//! The seed state every session starts from.
//!
//! The hermes bridge clones a config-seeded `AphroditeState` per session, so
//! these values are the defaults a session has before `Apply::Startup::Fn` runs
//! over it. Two of them are deliberately NOT the shipped configuration:
//! `ChainSplitEnabled` and `AutoExpand` start false here so the shipped
//! default is reachable through the config layer alone, and a missing config
//! can never turn either feature on.

use std::collections::{HashMap, VecDeque};

use super::{
	Const::{DefaultInlineByteBudget, InlineMaximum},
	Struct::AphroditeState,
};

impl Default for AphroditeState {
	fn default() -> Self {
		Self {
			InlineStore:HashMap::with_capacity(InlineMaximum),
			// +1 headroom: the LruCache must never self-evict before the
			// explicit evict loop enforces INLINE_MAX and the byte budget.
			InlineOrder:lru::LruCache::new(std::num::NonZeroUsize::new(InlineMaximum + 1).unwrap()),
			InlineStoreBytes:0,
			InlineStoreByteBudget:DefaultInlineByteBudget,
			RecentMarkers:VecDeque::new(),
			ConvIndex:HashMap::new(),
			ReferencedFiles:VecDeque::new(),
			TurnCounter:0,
			ScannedMessageIndex:0,
			FileTools:vec!["read_file".into(), "write_file".into(), "patch".into(), "search_files".into()],
			APIURL:String::new(),
			Model:"gpt-4o".into(),
			EngineThresholdPct:45,
			EngineMinimumMsgs:8,
			EngineProtectFirst:2,
			EngineProtectLast:5,
			ContextEngineEnabled:true,
			ToolThreshold:512,
			TerminalThreshold:256,
			ConfigurationError:None,
			AutoReload:false,
			DebugLevel:0,
			CatalogMode:"tool".into(),
			ExpandGuidance:false,
			DevMode:false,
			Directives:std::collections::HashMap::new(),
			ActiveDirectives:Vec::new(),
			EphemeralDirectives:Vec::new(),
			FlowBudgetChars:4000,
			SessionInject:String::new(),
			Templates:None,
			AutoExpand:false,
			AutoExpandLimit:102400,
			LiveContainer:false,
			RecursiveDepth:5,
			MaximumRequestBodySize:None,
			RetrieveGuidance:"minimal".into(),
			CatalogIntentHints:false,
			ManualDirectiveTurn:None,
			ToolEvents:VecDeque::new(),
			BgTasks:VecDeque::new(),
			PollWorkerEnabled:true,
			ChainSplitEnabled:false,
			ChainSplitMinimumSegments:2,
			ChainSplitFloor:2,
			ChainSplitMaximumSegments:6,
			SplitEvents:VecDeque::new(),
			SplitSegmentMap:HashMap::new(),
			SplitNextEventID:0,
			LastEmittedMarkerCount:0,
			LastEmittedFileCount:0,
		}
	}
}
