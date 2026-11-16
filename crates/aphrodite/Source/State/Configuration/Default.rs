//! The configuration mirror's seed.
//!
//! Two values are deliberately NOT the shipped configuration: `AutoExpand`
//! starts false here so the shipped default is reachable through the config
//! layer alone, and a missing config can never turn the feature on. The
//! chain-split switch is seeded the same way by `ChainSplit::Default`.

use super::Struct::Struct;

impl Default for Struct {
	fn default() -> Self {
		Self {
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
		}
	}
}
