/** @type {import('prettier').Config} */
export default {
	// =========================================================================
	// Core Formatting Options
	// =========================================================================
	printWidth: 80,
	useTabs: true,
	tabWidth: 4,
	semi: true,
	singleQuote: false,
	trailingComma: "all",
	bracketSameLine: true,
	bracketSpacing: true,
	arrowParens: "always",
	endOfLine: "lf",
	proseWrap: "always",
	quoteProps: "preserve",

	// =========================================================================
	// HTML / JSX Specifics
	// =========================================================================
	htmlWhitespaceSensitivity: "css",
	jsxSingleQuote: false,
	vueIndentScriptAndStyle: true,
	embeddedLanguageFormatting: "auto",

	// =========================================================================
	// Plugins
	// =========================================================================
	plugins: [
		"@ianvs/prettier-plugin-sort-imports",
		"prettier-plugin-astro",
		"prettier-plugin-organize-attributes",
		"prettier-plugin-packagejson",
		"prettier-plugin-sh",
		"prettier-plugin-toml",
		// MUST BE LAST
		"prettier-plugin-tailwindcss",
	],

	// =========================================================================
	// Plugin: Organize Attributes
	// =========================================================================
	attributeSort: "ASC",
	attributeIgnoreCase: false,
	attributeGroups: ["$DEFAULT", "^data-"],

	// =========================================================================
	// Plugin: Sort Imports (@ianvs/prettier-plugin-sort-imports)
	// =========================================================================
	importOrder: [
		"<THIRD_PARTY_MODULES>",
		"",
		"^[./]",
	],
	importOrderParserPlugins: ["typescript", "jsx", "decorators-legacy"],
	importOrderTypeScriptVersion: "5.5.4",

	// =========================================================================
	// File Overrides
	// =========================================================================
	overrides: [
		// JavaScript / JSX
		{
			files: "*.{js,mjs,cjs,jsx}",
			options: { parser: "babel" },
		},
		// TypeScript / TSX
		{
			files: "*.{ts,mts,cts,tsx}",
			options: { parser: "babel-ts" },
		},
		// Astro
		{
			files: "*.astro",
			options: { parser: "astro" },
		},
		// Svelte
		{
			files: "*.svelte",
			options: { parser: "svelte" },
		},
		// Lua
		{
			files: "*.lua",
			options: { parser: "lua" },
		},
		// TOML
		{
			files: "*.toml",
			options: { parser: "toml" },
		},
		// Markdown
		{
			files: "*.md",
			options: { parser: "markdown" },
		},
		// package.json - strict JSON (no trailing commas/comments)
		{
			files: "package.json",
			options: {
				parser: "json-stringify",
				trailingComma: "none",
			},
		},
		// Other JSON - loose (allows comments)
		{
			files: "*.json",
			excludeFiles: ["package.json"],
			options: {
				parser: "json",
				trailingComma: "none",
			},
		},
	],
};
