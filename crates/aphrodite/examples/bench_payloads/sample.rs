//! Rust source payload for bench_01_corpus.
//! ~3.5 KB - above TOKEN_COMPRESS_THRESHOLD (1 KB), below CACHE_COMPRESS_THRESHOLD (8 KB).
//! Expected: compressed in token mode, passthrough in cache mode.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct Store {
	data: Mutex<HashMap<String, String>>,
	cap: usize,
	order: Mutex<Vec<String>>,
}

impl Store {
	pub fn new(cap: usize) -> Arc<Self> {
		Arc::new(Self { data: Mutex::new(HashMap::new()), cap, order: Mutex::new(Vec::new()) })
	}
	pub fn get(&self, k: &str) -> Option<String> {
		let d = self.data.lock().unwrap();
		if let Some(v) = d.get(k) {
			let mut o = self.order.lock().unwrap();
			o.retain(|x| x != k);
			o.push(k.to_string());
			Some(v.clone())
		} else {
			None
		}
	}
	pub fn put(&self, k: &str, v: &str) {
		let mut d = self.data.lock().unwrap();
		let mut o = self.order.lock().unwrap();
		if d.contains_key(k) {
			o.retain(|x| x != k);
		} else if d.len() >= self.cap {
			if let Some(evict) = o.first().cloned() {
				o.remove(0);
				d.remove(&evict);
			}
		}
		d.insert(k.to_string(), v.to_string());
		o.push(k.to_string());
	}
	pub fn del(&self, k: &str) -> bool {
		let mut d = self.data.lock().unwrap();
		let mut o = self.order.lock().unwrap();
		o.retain(|x| x != k);
		d.remove(k).is_some()
	}
	pub fn len(&self) -> usize {
		self.data.lock().unwrap().len()
	}
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

pub fn fnv1a(data: &[u8]) -> u64 {
	let mut h: u64 = 14695981039346656037;
	for b in data {
		h ^= *b as u64;
		h = h.wrapping_mul(1099511628211);
	}
	h
}

pub struct BatchProcessor {
	store: Arc<Store>,
	workers: usize,
}
impl BatchProcessor {
	pub fn new(store: Arc<Store>, workers: usize) -> Self {
		Self { store, workers }
	}
	pub fn process_batch(&self, items: &[(String, String)]) -> usize {
		let chunk = (items.len() + self.workers - 1) / self.workers;
		let done = Arc::new(Mutex::new(0usize));
		std::thread::scope(|s| {
			for slice in items.chunks(chunk) {
				let st = self.store.clone();
				let dn = done.clone();
				let sl = slice.to_vec();
				s.spawn(move || {
					for (k, v) in &sl {
						st.put(k, v);
						*dn.lock().unwrap() += 1;
					}
				});
			}
		});
		*done.lock().unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn put_get() {
		let s = Store::new(4);
		s.put("a", "1");
		assert_eq!(s.get("a"), Some("1".into()));
	}
	#[test]
	fn eviction() {
		let s = Store::new(2);
		s.put("a", "1");
		s.put("b", "2");
		s.put("c", "3");
		assert_eq!(s.get("a"), None);
		assert_eq!(s.get("b"), Some("2".into()));
	}
	#[test]
	fn fnv_known() {
		assert_eq!(fnv1a(b"hello"), 0xa430d84680aabd0b);
	}
}
