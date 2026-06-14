"""Tests for headroom plugin middleware — multi-CCR, proxy detection, compress failure."""
import json, os, sys, unittest

# Add plugin path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "plugins", "headroom"))
import __init__ as plugin


class TestCCRResolution(unittest.TestCase):
    def setUp(self):
        plugin._RESOLVED.clear()
        # Mock _proxy_post to return controlled responses
        self._orig_post = plugin._proxy_post
        plugin._proxy_post = self._mock_post
        self.mock_returns = {}

    def tearDown(self):
        plugin._proxy_post = self._orig_post
        plugin._RESOLVED.clear()

    def _mock_post(self, endpoint, payload, timeout=5):
        if endpoint != "/v1/retrieve":
            return None
        h = payload.get("hash", "")
        content = self.mock_returns.get(h)
        if content is None:
            return None
        return {"original_content": content}

    def test_single_ccr_resolved(self):
        self.mock_returns["abc123"] = "resolved content here"
        msgs = [{"role": "tool", "content": "<<ccr:abc123,string,100B>>"}]
        result, n = plugin._resolve_ccr_in_messages(msgs)
        self.assertEqual(n, 1)
        self.assertEqual(result[0]["content"], "resolved content here")
        # Original unchanged
        self.assertIn("<<ccr:", msgs[0]["content"])

    def test_multi_ccr_in_one_message(self):
        self.mock_returns["aaa111"] = "first part"
        self.mock_returns["bbb222"] = "second part"
        msgs = [{"role": "tool", "content": "prefix <<ccr:aaa111,string,50B>> middle <<ccr:bbb222,string,50B>> suffix"}]
        result, n = plugin._resolve_ccr_in_messages(msgs)
        self.assertEqual(n, 2)
        self.assertEqual(result[0]["content"], "prefix first part middle second part suffix")

    def test_multi_ccr_across_messages(self):
        self.mock_returns["a1"] = "A"
        self.mock_returns["b2"] = "B"
        msgs = [
            {"role": "tool", "content": "<<ccr:a1,string,10B>>"},
            {"role": "tool", "content": "<<ccr:b2,string,10B>>"},
        ]
        result, n = plugin._resolve_ccr_in_messages(msgs)
        self.assertEqual(n, 2)
        self.assertEqual(result[0]["content"], "A")
        self.assertEqual(result[1]["content"], "B")

    def test_unresolved_hash_kept(self):
        msgs = [{"role": "tool", "content": "<<ccr:ghost,string,10B>> text"}]
        result, n = plugin._resolve_ccr_in_messages(msgs)
        self.assertEqual(n, 0)
        self.assertIn("<<ccr:ghost", result[0]["content"])

    def test_cache_reuse(self):
        self.mock_returns["c1"] = "cached"
        msgs1 = [{"role": "tool", "content": "<<ccr:c1,string,10B>>"}]
        result1, n1 = plugin._resolve_ccr_in_messages(msgs1)
        self.assertEqual(n1, 1)
        # Second call — should use cache, no proxy call needed
        self.mock_returns = {}  # clear mock
        msgs2 = [{"role": "tool", "content": "<<ccr:c1,string,10B>>"}]
        result2, n2 = plugin._resolve_ccr_in_messages(msgs2)
        self.assertEqual(n2, 1)
        self.assertEqual(result2[0]["content"], "cached")

    def test_items_compressed_format(self):
        self.mock_returns["def456"] = "full listing"
        msgs = [{"role": "tool", "content": "[80 items compressed to 19, hash=def456]"}]
        result, n = plugin._resolve_ccr_in_messages(msgs)
        self.assertEqual(n, 1)
        self.assertEqual(result[0]["content"], "full listing")

    def test_no_mutation_of_input(self):
        self.mock_returns["m1"] = "resolved"
        msgs = [{"role": "tool", "content": "<<ccr:m1,string,10B>>"}]
        original = msgs[0]["content"]
        plugin._resolve_ccr_in_messages(msgs)
        self.assertEqual(msgs[0]["content"], original)  # unchanged


class TestProxyDetection(unittest.TestCase):
    def test_localhost_detected(self):
        self.assertTrue(plugin._is_proxy("http://127.0.0.1:8787/v1"))
        self.assertTrue(plugin._is_proxy("http://localhost:8787"))
        self.assertTrue(plugin._is_proxy("http://0.0.0.0:8788/v1"))

    def test_remote_not_detected(self):
        self.assertFalse(plugin._is_proxy("https://api.deepseek.com/v1"))
        self.assertFalse(plugin._is_proxy("https://api.openai.com"))
        self.assertFalse(plugin._is_proxy(""))


class TestCompressEngine(unittest.TestCase):
    def test_compress_failure_fallback(self):
        """When hermes_compress is not installed, returns messages unchanged."""
        # Force engine to fail state
        plugin._COMPRESS_ENGINE = False
        msgs = [{"role": "user", "content": "hello"}]
        result = plugin._compress_inline(msgs)
        self.assertIs(result, msgs)  # same reference — no-op


class TestHashExtraction(unittest.TestCase):
    def test_ccr_format(self):
        self.assertEqual(plugin._extract_hash("<<ccr:abc123,string,5KB>>"), "abc123")

    def test_hash_equals_format(self):
        self.assertEqual(plugin._extract_hash("[50 items compressed to 10, hash=def456]"), "def456")

    def test_raw_hash(self):
        self.assertEqual(plugin._extract_hash("abc123"), "abc123")


if __name__ == "__main__":
    unittest.main()
